use crate::categories::{load_categories, save_categories as write_categories};
use crate::models::CategoryConfig;
use crate::models::{
    DupGroup, FileEntry, FileMeta, LogBatch, MountInfo, Progress, ScanSummary,
};
use crate::mover::{self, DupPolicy, SortOptions, Structure, UndoResult};
use crate::scanner::{self, ScanOptions};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

pub const EV_PROGRESS: &str = "df://progress";
pub const EV_SCAN_DONE: &str = "df://scan-done";
pub const EV_DUPS_DONE: &str = "df://dups-done";
pub const EV_SORT_DONE: &str = "df://sort-done";

#[derive(Clone)]
pub struct AppState {
    pub entries: Arc<Mutex<Vec<FileEntry>>>,
    pub summary: Arc<Mutex<Option<ScanSummary>>>,
    pub scan_root: Arc<Mutex<String>>,
    pub categories: Arc<Mutex<CategoryConfig>>,
    pub dups: Arc<Mutex<Vec<DupGroup>>>,
    pub cancel: Arc<AtomicBool>,
    pub busy: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            summary: Arc::new(Mutex::new(None)),
            scan_root: Arc::new(Mutex::new(String::new())),
            categories: Arc::new(Mutex::new(load_categories())),
            dups: Arc::new(Mutex::new(Vec::new())),
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn begin(state: &AppState) -> Result<BusyGuard, String> {
    if state
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Es läuft bereits ein Vorgang — bitte abbrechen oder warten.".into());
    }
    state.cancel.store(false, Ordering::SeqCst);
    Ok(BusyGuard(state.busy.clone()))
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    #[serde(default)]
    pub recursive: Option<bool>,
    #[serde(default)]
    pub honor_ignore_list: Option<bool>,
    #[serde(default)]
    pub extra_ignores: Option<Vec<String>>,
    #[serde(default)]
    pub follow_symlinks: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SortRequest {
    pub target: String,
    #[serde(default)]
    pub structure: Option<String>,
    #[serde(default)]
    pub dry_run: Option<bool>,
    #[serde(default)]
    pub duplicate_policy: Option<String>,
    #[serde(default)]
    pub duplicate_paths: Option<Vec<String>>,
    #[serde(default)]
    pub selected_exts: Option<Vec<String>>,
    #[serde(default)]
    pub selected_categories: Option<Vec<String>>,
    #[serde(default)]
    pub preserve_mtime: Option<bool>,
}

fn progress_emitter(app: AppHandle) -> impl Fn(Progress) + Send + Sync + 'static {
    move |p: Progress| {
        let _ = app.emit(EV_PROGRESS, &p);
    }
}

#[tauri::command]
pub fn list_mounts() -> Vec<MountInfo> {
    crate::mounts::list_mounts()
}

#[tauri::command]
pub fn system_info() -> serde_json::Value {
    serde_json::json!({
        "threads": num_cpus::get(),
        "version": env!("CARGO_PKG_VERSION"),
        "ffprobe": which_ffprobe(),
        "platform": std::env::consts::OS,
    })
}

fn which_ffprobe() -> bool {
    std::process::Command::new("ffprobe")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_categories(state: State<'_, AppState>) -> CategoryConfig {
    state.categories.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_categories(
    state: State<'_, AppState>,
    config: CategoryConfig,
) -> Result<CategoryConfig, String> {
    let mut cfg = config;
    cfg.normalize();
    write_categories(&cfg)?;
    *state.categories.lock().unwrap() = cfg.clone();
    Ok(cfg)
}

#[tauri::command]
pub fn reset_categories(state: State<'_, AppState>) -> Result<CategoryConfig, String> {
    let mut cfg = CategoryConfig::defaults();
    cfg.normalize();
    write_categories(&cfg)?;
    *state.categories.lock().unwrap() = cfg.clone();
    Ok(cfg)
}

#[tauri::command]
pub async fn pick_directory(app: AppHandle, title: Option<String>) -> Option<String> {
    let app2 = app.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        app2.dialog()
            .file()
            .set_title(title.unwrap_or_else(|| "Ordner wählen".into()))
            .blocking_pick_folder()
    })
    .await
    .ok()
    .flatten();
    res.map(|fp| fp.to_string())
}

#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    options: Option<ScanRequest>,
) -> Result<ScanSummary, String> {
    let st = state.inner().clone();
    let guard = begin(&st)?;
    let opts = options.unwrap_or_default();
    let scan_opts = ScanOptions {
        recursive: opts.recursive.unwrap_or(true),
        honor_ignore_list: opts.honor_ignore_list.unwrap_or(true),
        extra_ignores: opts.extra_ignores.unwrap_or_default(),
        follow_symlinks: opts.follow_symlinks.unwrap_or(false),
    };

    let path = std::path::PathBuf::from(&root);
    if !path.is_dir() {
        return Err(format!("Kein Verzeichnis: {}", root));
    }

    let cancel = st.cancel.clone();
    let cats = st.categories.lock().unwrap().clone();
    let emit = progress_emitter(app.clone());
    let entries_store = st.entries.clone();
    let summary_store = st.summary.clone();
    let root_store = st.scan_root.clone();
    let dups_store = st.dups.clone();
    let threads = num_cpus::get();

    let summary = tauri::async_runtime::spawn_blocking(move || {
        let raw = scanner::scan_tree(&path, &scan_opts, &cancel, |p| emit(p));
        let categories = cats.aggregate(&raw.entries);
        let summary = ScanSummary {
            root: root.to_string(),
            files: raw.files,
            dirs: raw.dirs,
            bytes: raw.bytes,
            skipped: raw.skipped,
            duration_ms: raw.elapsed_ms,
            cancelled: raw.cancelled,
            truncated: raw.truncated,
            threads,
            categories,
            errors: raw.errors,
        };
        *entries_store.lock().unwrap() = raw.entries;
        *summary_store.lock().unwrap() = Some(summary.clone());
        *root_store.lock().unwrap() = root.clone();
        dups_store.lock().unwrap().clear();
        summary
    })
    .await
    .map_err(|e| e.to_string())?;

    let _ = app.emit(EV_SCAN_DONE, &summary);
    drop(guard);
    Ok(summary)
}

#[tauri::command]
pub fn cancel_task(state: State<'_, AppState>) -> bool {
    state.cancel.store(true, Ordering::SeqCst);
    true
}

#[tauri::command]
pub fn get_summary(state: State<'_, AppState>) -> Option<ScanSummary> {
    state.summary.lock().unwrap().clone()
}

#[tauri::command]
pub async fn find_duplicates(
    app: AppHandle,
    state: State<'_, AppState>,
    min_size: Option<u64>,
) -> Result<Vec<DupGroup>, String> {
    let st = state.inner().clone();
    let guard = begin(&st)?;
    let entries = st.entries.lock().unwrap().clone();
    if entries.is_empty() {
        return Err("Noch kein Scan vorhanden — bitte zuerst ein Verzeichnis scannen.".into());
    }
    let cancel = st.cancel.clone();
    let emit = progress_emitter(app.clone());
    let store = st.dups.clone();
    let opts = crate::hasher::DupOptions {
        min_size: min_size.unwrap_or(1),
        ..Default::default()
    };

    let (groups, cancelled) = tauri::async_runtime::spawn_blocking(move || {
        crate::hasher::find_duplicates(&entries, &opts, &cancel, move |p| emit(p))
    })
    .await
    .map_err(|e| e.to_string())?;

    if cancelled {
        drop(guard);
        return Err("Abgebrochen".into());
    }
    *store.lock().unwrap() = groups.clone();
    let _ = app.emit(EV_DUPS_DONE, &groups);
    drop(guard);
    Ok(groups)
}

#[tauri::command]
pub fn get_duplicates(state: State<'_, AppState>) -> Vec<DupGroup> {
    state.dups.lock().unwrap().clone()
}

#[tauri::command]
pub async fn file_meta(paths: Vec<String>, probe: Option<bool>) -> Result<Vec<FileMeta>, String> {
    let probe = probe.unwrap_or(true);
    let metas: Vec<FileMeta> = tauri::async_runtime::spawn_blocking(move || {
        paths
            .par_iter()
            .map(|p| crate::metadata::read_meta(p, probe))
            .collect()
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(metas)
}

fn build_sort_options(state: &AppState, req: &SortRequest, force_dry: bool) -> SortOptions {
    let root = state.scan_root.lock().unwrap().clone();
    SortOptions {
        root,
        target: req.target.clone(),
        structure: Structure::parse(req.structure.as_deref().unwrap_or("categoryExt")),
        dry_run: force_dry || req.dry_run.unwrap_or(false),
        dup_policy: DupPolicy::parse(req.duplicate_policy.as_deref().unwrap_or("skip")),
        dup_paths: req
            .duplicate_paths
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect::<HashSet<String>>(),
        selected_exts: req
            .selected_exts
            .as_ref()
            .map(|v| v.iter().map(|e| e.to_lowercase()).collect()),
        selected_categories: req
            .selected_categories
            .as_ref()
            .map(|v| v.iter().cloned().collect()),
        preserve_mtime: req.preserve_mtime.unwrap_or(true),
    }
}

#[tauri::command]
pub async fn plan_sort(
    state: State<'_, AppState>,
    request: SortRequest,
) -> Result<LogBatch, String> {
    let st = state.inner().clone();
    let guard = begin(&st)?;
    let entries = st.entries.lock().unwrap().clone();
    if entries.is_empty() {
        return Err("Noch kein Scan vorhanden".into());
    }
    let cfg = st.categories.lock().unwrap().clone();
    let opts = build_sort_options(&st, &request, true);
    let cancel = AtomicBool::new(false);
    let batch = mover::sort_files(&cfg, &entries, &opts, &cancel, &|_p| {})?;
    drop(guard);
    Ok(batch)
}

#[tauri::command]
pub async fn execute_sort(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SortRequest,
) -> Result<LogBatch, String> {
    let st = state.inner().clone();
    let guard = begin(&st)?;
    let entries = st.entries.lock().unwrap().clone();
    if entries.is_empty() {
        return Err("Noch kein Scan vorhanden".into());
    }
    let cfg = st.categories.lock().unwrap().clone();
    let opts = build_sort_options(&st, &request, false);
    let cancel = st.cancel.clone();
    let emit = progress_emitter(app.clone());

    let batch = tauri::async_runtime::spawn_blocking(move || {
        mover::sort_files(&cfg, &entries, &opts, &cancel, &emit)
    })
    .await
    .map_err(|e| e.to_string())??;

    // Files that moved no longer exist where the scan saw them.
    if !batch.dry_run {
        let moved_src: HashSet<&str> = batch
            .ops
            .iter()
            .filter(|o| matches!(o.action.as_str(), "moved" | "copied" | "deleted" | "duplicate-moved"))
            .map(|o| o.src.as_str())
            .collect();
        let mut entries = st.entries.lock().unwrap();
        entries.retain(|e| !moved_src.contains(e.path.as_str()));
        if let Some(summary) = st.summary.lock().unwrap().as_mut() {
            let cats = st.categories.lock().unwrap().clone();
            summary.categories = cats.aggregate(&entries);
            summary.files = entries.len() as u64;
            summary.bytes = entries.iter().map(|e| e.size).sum();
        }
    }

    let _ = app.emit(EV_SORT_DONE, &batch);
    drop(guard);
    Ok(batch)
}

#[tauri::command]
pub fn get_log() -> Vec<LogBatch> {
    mover::read_log()
}

#[tauri::command]
pub async fn undo_last() -> Result<UndoResult, String> {
    let log = mover::read_log();
    let last = log
        .iter()
        .rev()
        .find(|b| !b.dry_run && !b.undone)
        .ok_or_else(|| "Kein rückgängig-machbarer Vorgang im Protokoll".to_string())?
        .id
        .clone();
    tauri::async_runtime::spawn_blocking(move || mover::undo_batch(&last))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn undo_batch(batch_id: String) -> Result<UndoResult, String> {
    tauri::async_runtime::spawn_blocking(move || mover::undo_batch(&batch_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn reveal_path(app: AppHandle, path: String) -> bool {
    let mut p = std::path::PathBuf::from(&path);
    if p.is_file() {
        if let Some(parent) = p.parent() {
            p = parent.to_path_buf();
        }
    }
    let uri = format!("file://{}", p.to_string_lossy());
    // Prefer the desktop portal, fall back to xdg-open.
    let ok = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
            &format!("array:string:{}", uri),
            "string:",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        return true;
    }
    let _ = app;
    std::process::Command::new("xdg-open")
        .arg(&p)
        .spawn()
        .is_ok()
}
