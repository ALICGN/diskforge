use crate::categories::config_file;
use crate::models::CategoryConfig;
use crate::models::{FileEntry, LogBatch, LogOp, Progress};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const DUPLICATE_FOLDER: &str = "_Duplikate";
pub const MAX_LOG_BATCHES: usize = 30;

static BATCH_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
const EXDEV: i32 = 18;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Structure {
    ExtOnly,
    CategoryOnly,
    CategoryExt,
}

impl Structure {
    pub fn parse(s: &str) -> Self {
        match s {
            "ext" => Structure::ExtOnly,
            "category" => Structure::CategoryOnly,
            _ => Structure::CategoryExt,
        }
    }
    pub fn id(&self) -> &'static str {
        match self {
            Structure::ExtOnly => "ext",
            Structure::CategoryOnly => "category",
            Structure::CategoryExt => "categoryExt",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DupPolicy {
    Skip,
    Folder,
    Delete,
}

impl DupPolicy {
    pub fn parse(s: &str) -> Self {
        match s {
            "folder" => DupPolicy::Folder,
            "delete" => DupPolicy::Delete,
            _ => DupPolicy::Skip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SortOptions {
    pub root: String,
    pub target: String,
    pub structure: Structure,
    pub dry_run: bool,
    pub dup_policy: DupPolicy,
    pub dup_paths: HashSet<String>,
    pub selected_exts: Option<HashSet<String>>,
    pub selected_categories: Option<HashSet<String>>,
    pub preserve_mtime: bool,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Makes a path component safe for NTFS/ext4/FAT and strips characters that
/// Windows cannot store (`<>:"/\|?*` and control chars).
pub fn sanitize_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => out.push('_'),
            c if (c as u32) < 32 => out.push('_'),
            c => out.push(c),
        }
    }
    let trimmed = out.trim().trim_end_matches(['.', ' ']).to_string();
    let trimmed = if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed
    };
    if is_reserved_name(&trimmed) {
        format!("{}_", trimmed)
    } else {
        trimmed
    }
}

/// Windows device names are illegal on NTFS even with an extension.
pub fn is_reserved_name(stem: &str) -> bool {
    let base = stem.split('.').next().unwrap_or("").to_ascii_uppercase();
    matches!(
        base.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6"
            | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6"
            | "LPT7" | "LPT8" | "LPT9"
    )
}

/// Splits `file.tar.gz` into (`file`, `.tar.gz`) so counters stay readable.
fn split_name(name: &str) -> (String, String) {
    if let Some(idx) = name.find('.') {
        if idx > 0 {
            return (name[..idx].to_string(), name[idx..].to_string());
        }
    }
    (name.to_string(), String::new())
}

fn planned_key(p: &Path) -> String {
    p.to_string_lossy().to_lowercase()
}

/// Never overwrites: `foto.jpg` -> `foto (1).jpg` -> `foto (2).jpg` ...
pub fn unique_target(dir: &Path, name: &str, planned: &mut HashSet<String>) -> PathBuf {
    let (stem, ext) = split_name(name);
    let mut candidate = dir.join(name);
    let mut n = 0u32;
    while candidate.exists() || planned.contains(&planned_key(&candidate)) {
        n += 1;
        candidate = dir.join(format!("{} ({}){}", stem, n, ext));
        if n > 100_000 {
            break;
        }
    }
    planned.insert(planned_key(&candidate));
    candidate
}

fn blake3_of(path: &Path) -> std::io::Result<String> {
    crate::hasher::full_hash(path, None, None)
}

/// Copies, verifies with BLAKE3 and atomically renames the finished part file.
fn copy_verified(src: &Path, dst: &Path, preserve_mtime: bool) -> std::io::Result<()> {
    let file_name = dst
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let part = dst.with_file_name(format!(".{}.dfpart", file_name));
    let _ = std::fs::remove_file(&part);

    let result = (|| -> std::io::Result<()> {
        let md = std::fs::metadata(src)?;
        let mut input = std::fs::File::open(src)?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)?;
        let mut buf = vec![0u8; 4 * 1024 * 1024];
        loop {
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            output.write_all(&buf[..n])?;
        }
        output.flush()?;
        output.sync_all()?;
        drop(output);

        let src_hash = blake3_of(src)?;
        let dst_hash = blake3_of(&part)?;
        if src_hash != dst_hash {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "BLAKE3-Verifikation fehlgeschlagen",
            ));
        }
        if preserve_mtime {
            if let Ok(mtime) = md.modified() {
                let _ = filetime::set_file_mtime(&part, filetime::FileTime::from_system_time(mtime));
            }
        }
        let _ = std::fs::set_permissions(&part, md.permissions());
        std::fs::rename(&part, dst)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

/// rename, falling back to copy+verify+delete on EXDEV / other cross-device errors.
fn move_file(src: &Path, dst: &Path, preserve_mtime: bool) -> Result<&'static str, String> {
    match std::fs::rename(src, dst) {
        Ok(()) => Ok("moved"),
        Err(err) => {
            let cross_device = err.raw_os_error() == Some(EXDEV);
            let rename_msg = err.to_string();
            match copy_verified(src, dst, preserve_mtime) {
                Ok(()) => match std::fs::remove_file(src) {
                    Ok(()) => Ok(if cross_device { "copied" } else { "copied" }),
                    Err(remove_err) => Err(format!(
                        "kopiert, Quelle konnte nicht gelöscht werden: {}",
                        remove_err
                    )),
                },
                Err(copy_err) => Err(format!("{} / {}", rename_msg, copy_err)),
            }
        }
    }
}

fn target_dir(
    cfg: &CategoryConfig,
    ext: &str,
    structure: Structure,
    target_root: &Path,
) -> PathBuf {
    let category = sanitize_component(&cfg.category_of_ext(ext));
    let ext_dir = if ext.is_empty() {
        "Ohne_Endung".to_string()
    } else {
        sanitize_component(&ext.to_uppercase())
    };
    match structure {
        Structure::ExtOnly => target_root.join(ext_dir),
        Structure::CategoryOnly => target_root.join(category),
        Structure::CategoryExt => target_root.join(category).join(ext_dir),
    }
}

fn selected(entry: &FileEntry, opts: &SortOptions, category: &str) -> bool {
    if let Some(cats) = opts.selected_categories.as_ref() {
        if !cats.contains(category) {
            return false;
        }
    }
    if let Some(exts) = opts.selected_exts.as_ref() {
        let key = if entry.ext.is_empty() {
            "Ohne_Endung".to_lowercase()
        } else {
            entry.ext.clone()
        };
        if !exts.contains(&key) {
            return false;
        }
    }
    true
}

/// Sorts (or simulates sorting) all matching entries of the last scan.
pub fn sort_files(
    cfg: &CategoryConfig,
    entries: &[FileEntry],
    opts: &SortOptions,
    cancel: &AtomicBool,
    emit: &dyn Fn(Progress),
) -> Result<LogBatch, String> {
    let target_root = PathBuf::from(&opts.target);
    if opts.target.trim().is_empty() {
        return Err("Kein Zielordner gewählt".into());
    }
    if !opts.dry_run {
        if !target_root.exists() {
            std::fs::create_dir_all(&target_root)
                .map_err(|e| format!("Zielordner nicht erstellbar: {}", e))?;
        }
        if !target_root.is_dir() {
            return Err("Zielpfad ist kein Verzeichnis".into());
        }
    }

    let started = Instant::now();
    let mut planned: HashSet<String> = HashSet::new();
    let mut ops: Vec<LogOp> = Vec::new();
    let (mut moved, mut copied, mut deleted, mut skipped) = (0u64, 0u64, 0u64, 0u64);
    let mut bytes = 0u64;
    let mut errors = 0u64;
    let total = entries.len() as u64;
    let mut last_emit = Instant::now() - std::time::Duration::from_secs(1);

    for (i, entry) in entries.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let src = Path::new(&entry.path);
        let category = cfg.category_of_ext(&entry.ext);
        if !selected(entry, opts, &category) {
            continue;
        }
        let is_dup = opts.dup_paths.contains(&entry.path);
        let cancelled = cancel.load(Ordering::Relaxed);
        if cancelled {
            break;
        }

        // ---- duplicate handling first ----
        if is_dup {
            match opts.dup_policy {
                DupPolicy::Skip => {
                    skipped += 1;
                    ops.push(LogOp {
                        src: entry.path.clone(),
                        dst: String::new(),
                        size: entry.size,
                        action: "skipped-duplicate".into(),
                        error: None,
                    });
                    continue;
                }
                DupPolicy::Delete => {
                    if opts.dry_run {
                        ops.push(LogOp {
                            src: entry.path.clone(),
                            dst: String::new(),
                            size: entry.size,
                            action: "would-delete".into(),
                            error: None,
                        });
                    } else {
                        match std::fs::remove_file(src) {
                            Ok(()) => {
                                deleted += 1;
                                bytes += entry.size;
                                ops.push(LogOp {
                                    src: entry.path.clone(),
                                    dst: String::new(),
                                    size: entry.size,
                                    action: "deleted".into(),
                                    error: None,
                                });
                            }
                            Err(e) => {
                                errors += 1;
                                ops.push(LogOp {
                                    src: entry.path.clone(),
                                    dst: String::new(),
                                    size: entry.size,
                                    action: "error".into(),
                                    error: Some(format!("löschen: {}", e)),
                                });
                            }
                        }
                    }
                    continue;
                }
                DupPolicy::Folder => {
                    let dir = target_root.join(DUPLICATE_FOLDER);
                    if !opts.dry_run {
                        if let Err(e) = std::fs::create_dir_all(&dir) {
                            errors += 1;
                            ops.push(LogOp {
                                src: entry.path.clone(),
                                dst: String::new(),
                                size: entry.size,
                                action: "error".into(),
                                error: Some(format!("Ordner {}: {}", dir.display(), e)),
                            });
                            continue;
                        }
                    }
                    let dst = unique_target(&dir, &entry.name, &mut planned);
                    if opts.dry_run {
                        ops.push(LogOp {
                            src: entry.path.clone(),
                            dst: dst.to_string_lossy().to_string(),
                            size: entry.size,
                            action: "planned-duplicate".into(),
                            error: None,
                        });
                    } else {
                        match move_file(src, &dst, opts.preserve_mtime) {
                            Ok(kind) => {
                                if kind == "copied" {
                                    copied += 1
                                } else {
                                    moved += 1
                                }
                                bytes += entry.size;
                                ops.push(LogOp {
                                    src: entry.path.clone(),
                                    dst: dst.to_string_lossy().to_string(),
                                    size: entry.size,
                                    action: "duplicate-moved".into(),
                                    error: None,
                                });
                            }
                            Err(e) => {
                                errors += 1;
                                ops.push(LogOp {
                                    src: entry.path.clone(),
                                    dst: dst.to_string_lossy().to_string(),
                                    size: entry.size,
                                    action: "error".into(),
                                    error: Some(e),
                                });
                            }
                        }
                    }
                    continue;
                }
            }
        }

        // ---- normal sort ----
        let dir = target_dir(cfg, &entry.ext, opts.structure, &target_root);
        if !opts.dry_run {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                errors += 1;
                ops.push(LogOp {
                    src: entry.path.clone(),
                    dst: dir.to_string_lossy().to_string(),
                    size: entry.size,
                    action: "error".into(),
                    error: Some(format!("Zielordner: {}", e)),
                });
                continue;
            }
        }
        let dst = unique_target(&dir, &entry.name, &mut planned);
        if dst == src {
            skipped += 1;
            ops.push(LogOp {
                src: entry.path.clone(),
                dst: entry.path.clone(),
                size: entry.size,
                action: "skipped-same".into(),
                error: None,
            });
            continue;
        }

        if opts.dry_run {
            ops.push(LogOp {
                src: entry.path.clone(),
                dst: dst.to_string_lossy().to_string(),
                size: entry.size,
                action: "planned".into(),
                error: None,
            });
        } else {
            match move_file(src, &dst, opts.preserve_mtime) {
                Ok(kind) => {
                    if kind == "copied" {
                        copied += 1
                    } else {
                        moved += 1
                    }
                    bytes += entry.size;
                    ops.push(LogOp {
                        src: entry.path.clone(),
                        dst: dst.to_string_lossy().to_string(),
                        size: entry.size,
                        action: kind.into(),
                        error: None,
                    });
                }
                Err(e) => {
                    errors += 1;
                    ops.push(LogOp {
                        src: entry.path.clone(),
                        dst: dst.to_string_lossy().to_string(),
                        size: entry.size,
                        action: "error".into(),
                        error: Some(e),
                    });
                }
            }
        }

        if last_emit.elapsed() >= std::time::Duration::from_millis(120) {
            last_emit = Instant::now();
            emit(Progress {
                task: "sort".into(),
                phase: if opts.dry_run { "simulating" } else { "moving" }.into(),
                files: (i + 1) as u64,
                bytes,
                done_bytes: (i + 1) as u64,
                total_bytes: total,
                current: entry.path.clone(),
                message: format!("{}/{} — {}", i + 1, total, entry.name),
                elapsed_ms: started.elapsed().as_millis() as u64,
                ..Default::default()
            });
        }
    }

    let batch = LogBatch {
        id: format!(
            "{}-{}-{}",
            now_ms(),
            std::process::id(),
            BATCH_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ),
        started_ms: now_ms(),
        root: opts.root.clone(),
        target: opts.target.clone(),
        structure: opts.structure.id().to_string(),
        dry_run: opts.dry_run,
        undone: false,
        moved,
        copied,
        deleted,
        skipped,
        bytes,
        errors,
        duration_ms: started.elapsed().as_millis() as u64,
        ops,
    };

    if !opts.dry_run {
        if let Err(e) = append_log(&batch) {
            eprintln!("move_log konnte nicht geschrieben werden: {}", e);
        }
    }

    emit(Progress {
        task: "sort".into(),
        phase: "done".into(),
        files: batch.ops.len() as u64,
        bytes,
        message: format!(
            "{} verschoben, {} kopiert, {} Fehler",
            batch.moved, batch.copied, batch.errors
        ),
        elapsed_ms: batch.duration_ms,
        ..Default::default()
    });

    Ok(batch)
}

pub fn read_log() -> Vec<LogBatch> {
    let path = config_file("move_log.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Vec<LogBatch>>(&text).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn write_log(batches: &[LogBatch]) -> Result<(), String> {
    let path = config_file("move_log.json");
    let text = serde_json::to_string_pretty(batches).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {}", path.display(), e))
}

pub fn append_log(batch: &LogBatch) -> Result<(), String> {
    let mut batches = read_log();
    batches.push(batch.clone());
    if batches.len() > MAX_LOG_BATCHES {
        let overflow = batches.len() - MAX_LOG_BATCHES;
        batches.drain(0..overflow);
    }
    write_log(&batches)
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UndoResult {
    pub batch_id: String,
    pub restored: u64,
    pub failed: Vec<String>,
    pub unrestorable: u64,
}

/// Rolls back a batch in reverse order (dst -> src), never overwriting.
pub fn undo_batch(batch_id: &str) -> Result<UndoResult, String> {
    let mut batches = read_log();
    let idx = batches
        .iter()
        .position(|b| b.id == batch_id)
        .ok_or_else(|| "Batch nicht im Protokoll gefunden".to_string())?;
    if batches[idx].undone {
        return Err("Dieser Vorgang wurde bereits rückgängig gemacht".into());
    }
    if batches[idx].dry_run {
        return Err("Simulationen können nicht rückgängig gemacht werden".into());
    }
    let ops = batches[idx].ops.clone();

    let mut restored = 0u64;
    let mut failed: Vec<String> = Vec::new();
    let mut unrestorable = 0u64;

    for op in ops.iter().rev() {
        match op.action.as_str() {
            "deleted" => {
                unrestorable += 1;
                failed.push(format!("gelöscht (nicht wiederherstellbar): {}", op.src));
            }
            "moved" | "copied" | "duplicate-moved" => {
                let src = Path::new(&op.src);
                let dst = Path::new(&op.dst);
                if !dst.exists() {
                    failed.push(format!("Ziel fehlt: {}", op.dst));
                    continue;
                }
                if src.exists() {
                    failed.push(format!("Quelle existiert wieder: {}", op.src));
                    continue;
                }
                if let Some(parent) = src.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        failed.push(format!("{}: {}", parent.display(), e));
                        continue;
                    }
                }
                match std::fs::rename(dst, src) {
                    Ok(()) => restored += 1,
                    Err(_) => match copy_verified(dst, src, true) {
                        Ok(()) => match std::fs::remove_file(dst) {
                            Ok(()) => restored += 1,
                            Err(e) => failed.push(format!("{}: {}", op.dst, e)),
                        },
                        Err(e) => failed.push(format!("{}: {}", op.dst, e)),
                    },
                }
            }
            _ => {}
        }
    }

    batches[idx].undone = true;
    write_log(&batches)?;

    Ok(UndoResult {
        batch_id: batch_id.to_string(),
        restored,
        failed,
        unrestorable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CategoryConfig;
    use std::fs;

    fn tmpdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("df-move-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn cfg() -> CategoryConfig {
        let mut c = CategoryConfig::defaults();
        c.normalize();
        c
    }

    static LOG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Keeps the move log of parallel tests out of the real user config and makes
    /// log writes mutually exclusive (set_var is process-global).
    struct HomeGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        path: PathBuf,
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            std::env::remove_var("DISKFORGE_HOME");
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn test_home(tag: &str) -> HomeGuard {
        let lock = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = tmpdir(&format!("home-{}", tag));
        std::env::set_var("DISKFORGE_HOME", &path);
        HomeGuard { _lock: lock, path }
    }

    fn entries_of(root: &Path) -> Vec<FileEntry> {
        let cancel = AtomicBool::new(false);
        crate::scanner::scan_tree(root, &crate::scanner::ScanOptions::default(), &cancel, |_p| {})
            .entries
    }

    fn opts(root: &Path, target: &Path, structure: Structure, dry: bool) -> SortOptions {
        SortOptions {
            root: root.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
            structure,
            dry_run: dry,
            dup_policy: DupPolicy::Skip,
            dup_paths: HashSet::new(),
            selected_exts: None,
            selected_categories: None,
            preserve_mtime: true,
        }
    }

    #[test]
    fn sanitize_and_reserved() {
        assert_eq!(sanitize_component("a/b:c*?"), "a_b_c__");
        assert_eq!(sanitize_component("CON"), "CON_");
        assert_eq!(sanitize_component("   "), "_");
        assert_eq!(sanitize_component("normal.txt"), "normal.txt");
    }

    #[test]
    fn unique_target_counts_up() {
        let dir = tmpdir("unique");
        fs::write(dir.join("a.txt"), b"x").unwrap();
        let mut planned = HashSet::new();
        let p1 = unique_target(&dir, "a.txt", &mut planned);
        assert_eq!(p1.file_name().unwrap(), "a (1).txt");
        let p2 = unique_target(&dir, "a.txt", &mut planned);
        assert_eq!(p2.file_name().unwrap(), "a (2).txt");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_does_not_touch_disk() {
        let _home = test_home("dry");
        let root = tmpdir("dry");
        fs::write(root.join("pic.jpg"), b"jpegdata").unwrap();
        fs::write(root.join("doc.pdf"), b"pdfdata").unwrap();
        let target = tmpdir("dry-target");
        let entries = entries_of(&root);
        let cancel = AtomicBool::new(false);
        let o = opts(&root, &target, Structure::CategoryExt, true);
        let batch = sort_files(&cfg(), &entries, &o, &cancel, &|_p| {}).unwrap();
        assert!(batch.dry_run);
        assert_eq!(batch.ops.len(), 2);
        assert!(batch.ops.iter().all(|o| o.action == "planned"));
        assert!(root.join("pic.jpg").exists());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn hierarchical_sort_moves_files() {
        let _home = test_home("hier");
        let root = tmpdir("hier");
        fs::write(root.join("pic.jpg"), b"jpegdata").unwrap();
        fs::write(root.join("clip.MP4"), b"mp4data").unwrap();
        fs::write(root.join("README"), b"x").unwrap();
        let target = tmpdir("hier-target");
        let entries = entries_of(&root);
        let cancel = AtomicBool::new(false);
        let o = opts(&root, &target, Structure::CategoryExt, false);
        let batch = sort_files(&cfg(), &entries, &o, &cancel, &|_p| {}).unwrap();
        assert_eq!(batch.errors, 0, "{:?}", batch.ops);
        assert_eq!(batch.moved, 3);
        assert!(target.join("Bilder/JPG/pic.jpg").exists());
        assert!(target.join("Videos/MP4/clip.MP4").exists());
        assert!(target.join("Sonstiges/Ohne_Endung/README").exists());
        assert!(!root.join("pic.jpg").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn structure_modes() {
        let _home = test_home("modes");
        let root = tmpdir("modes");
        fs::write(root.join("a.jpg"), b"x").unwrap();
        let entries = entries_of(&root);
        let cancel = AtomicBool::new(false);
        let target = tmpdir("modes-target");
        let b = sort_files(&cfg(), &entries, &opts(&root, &target, Structure::ExtOnly, false), &cancel, &|_p| {}).unwrap();
        assert_eq!(b.moved, 1);
        assert!(target.join("JPG/a.jpg").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn duplicate_policy_skip_and_delete() {
        let _home = test_home("dups");
        let root = tmpdir("dups");
        let target = tmpdir("dups-target");
        fs::write(root.join("keep.jpg"), b"1111").unwrap();
        fs::write(root.join("dup.jpg"), b"1111").unwrap();
        let entries = entries_of(&root);
        let cancel = AtomicBool::new(false);
        let mut o = opts(&root, &target, Structure::CategoryExt, false);
        o.dup_paths.insert(root.join("dup.jpg").to_string_lossy().to_string());
        let b = sort_files(&cfg(), &entries, &o, &cancel, &|_p| {}).unwrap();
        assert_eq!(b.skipped, 1);
        assert_eq!(b.moved, 1);
        assert!(root.join("dup.jpg").exists());

        let mut o2 = o.clone();
        o2.dup_policy = DupPolicy::Delete;
        let b2 = sort_files(&cfg(), &entries, &o2, &cancel, &|_p| {}).unwrap();
        assert_eq!(b2.deleted, 1);
        assert!(!root.join("dup.jpg").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn undo_restores_files() {
        let _home = test_home("undo");
        let root = tmpdir("undo");
        fs::write(root.join("u1.jpg"), b"abc").unwrap();
        fs::write(root.join("u2.pdf"), b"defg").unwrap();
        let target = tmpdir("undo-target");
        let entries = entries_of(&root);
        let cancel = AtomicBool::new(false);
        let b = sort_files(&cfg(), &entries, &opts(&root, &target, Structure::CategoryExt, false), &cancel, &|_p| {}).unwrap();
        assert!(target.join("Bilder/JPG/u1.jpg").exists());
        let res = undo_batch(&b.id).unwrap();
        assert_eq!(res.restored, 2);
        assert!(root.join("u1.jpg").exists());
        assert!(root.join("u2.pdf").exists());
        assert!(!target.join("Bilder/JPG/u1.jpg").exists());
        // second undo must be refused
        assert!(undo_batch(&b.id).is_err());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn preserves_mtime_on_copy() {
        let root = tmpdir("mtime");
        let src_dir = tmpdir("mtime-src");
        let file = root.join("m.jpg");
        fs::write(&file, b"data").unwrap();
        let old = filetime::FileTime::from_unix_time(1_600_000_000, 0);
        filetime::set_file_mtime(&file, old).unwrap();
        let dst = src_dir.join("m.jpg");
        copy_verified(&file, &dst, true).unwrap();
        let md = fs::metadata(&dst).unwrap();
        let mtime = filetime::FileTime::from_last_modification_time(&md);
        assert_eq!(mtime.unix_seconds(), 1_600_000_000);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&src_dir);
    }
}
