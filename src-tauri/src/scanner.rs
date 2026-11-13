use crate::models::{FileEntry, Progress};
use ignore::{WalkBuilder, WalkState};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const IGNORED_DIR_NAMES: &[&str] = &[
    "$RECYCLE.BIN",
    "System Volume Information",
    "node_modules",
    ".git",
    ".hg",
    ".svn",
    "lost+found",
    "@eaDir",
    "#recycle",
    ".cache",
    ".thumbnails",
    ".Trash",
];

pub const IGNORED_DIR_PREFIXES: &[&str] = &[".Trash-", ".df-trash", "$Extend"];

const MAX_ENTRIES: usize = 2_000_000;
const COMPOUND_EXTS: &[&str] = &[
    "pkg.tar.zst",
    "pkg.tar.xz",
    "tar.gz",
    "tar.bz2",
    "tar.xz",
    "tar.zst",
    "tar.lz4",
];

pub fn is_ignored_dir(name: &str, extra: &[String]) -> bool {
    if IGNORED_DIR_NAMES
        .iter()
        .any(|n| n.eq_ignore_ascii_case(name))
    {
        return true;
    }
    if IGNORED_DIR_PREFIXES.iter().any(|p| {
        name.len() >= p.len() && name.as_bytes()[..p.len()].eq_ignore_ascii_case(p.as_bytes())
    }) {
        return true;
    }
    extra.iter().any(|e| {
        let e = e.trim_end_matches('/');
        !e.is_empty() && (e.eq_ignore_ascii_case(name) || Path::new(e) == Path::new(name))
    })
}

/// Extension extraction: lowercase, compound-aware (`foo.pkg.tar.zst` -> `pkg.tar.zst`).
pub fn extension_of(name: &str) -> String {
    let lower = name.to_lowercase();
    for c in COMPOUND_EXTS {
        if lower.ends_with(&format!(".{}", c)) {
            return c.to_string();
        }
    }
    match Path::new(name).extension() {
        Some(e) => e.to_string_lossy().to_lowercase(),
        None => String::new(),
    }
}

#[derive(Clone, Debug)]
pub struct ScanOptions {
    pub recursive: bool,
    pub honor_ignore_list: bool,
    pub extra_ignores: Vec<String>,
    pub follow_symlinks: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            honor_ignore_list: true,
            extra_ignores: Vec::new(),
            follow_symlinks: false,
        }
    }
}

pub struct RawScan {
    pub entries: Vec<FileEntry>,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub skipped: u64,
    pub cancelled: bool,
    pub truncated: bool,
    pub elapsed_ms: u64,
    pub errors: Vec<String>,
}

/// Sammelzustand, der von allen Walker-Threads geteilt wird.
struct Acc {
    entries: Vec<FileEntry>,
    files: u64,
    dirs: u64,
    bytes: u64,
    skipped: u64,
    truncated: bool,
    errors: Vec<String>,
}

/// Thread-lokaler Puffer: sammelt Einträge und schreibt sie gebündelt in `Acc`.
struct Buf {
    acc: Arc<Mutex<Acc>>,
    entries: Vec<FileEntry>,
    files: u64,
    dirs: u64,
    bytes: u64,
    skipped: u64,
    errors: Vec<String>,
    current: String,
}

const FLUSH_EVERY: usize = 512;

impl Buf {
    fn new(acc: Arc<Mutex<Acc>>) -> Self {
        Self {
            acc,
            entries: Vec::with_capacity(FLUSH_EVERY),
            files: 0,
            dirs: 0,
            bytes: 0,
            skipped: 0,
            errors: Vec::new(),
            current: String::new(),
        }
    }

    fn flush(&mut self) {
        if self.entries.is_empty()
            && self.files == 0
            && self.dirs == 0
            && self.skipped == 0
            && self.errors.is_empty()
        {
            return;
        }
        if let Ok(mut a) = self.acc.lock() {
            let room = MAX_ENTRIES.saturating_sub(a.entries.len());
            if self.entries.len() > room {
                a.truncated = true;
                if room > 0 {
                    a.entries.extend(self.entries.drain(..room));
                } else {
                    self.entries.clear();
                }
            } else {
                a.entries.append(&mut self.entries);
            }
            a.files += self.files;
            a.dirs += self.dirs;
            a.bytes += self.bytes;
            a.skipped += self.skipped;
            if a.errors.len() < 200 {
                let take = 200 - a.errors.len();
                a.errors.extend(self.errors.drain(..).take(take));
            }
        }
        self.entries.clear();
        self.files = 0;
        self.dirs = 0;
        self.bytes = 0;
        self.skipped = 0;
        self.errors.clear();
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Parallel scan. `emit` is called from the walker threads at most ~4x/second.
pub fn scan_tree<F>(root: &Path, opts: &ScanOptions, cancel: &AtomicBool, emit: F) -> RawScan
where
    F: Fn(Progress) + Send + Sync,
{
    let started = Instant::now();
    let acc = Arc::new(Mutex::new(Acc {
        entries: Vec::new(),
        files: 0,
        dirs: 0,
        bytes: 0,
        skipped: 0,
        truncated: false,
        errors: Vec::new(),
    }));
    // Grobe Taktung der Fortschritts-Events: nur ein Thread darf emittieren.
    let last_emit = Arc::new(Mutex::new((Instant::now(), 0u64, 0u64)));
    let emit_seq = Arc::new(AtomicU64::new(0));

    let honor = opts.honor_ignore_list;
    let extra: Arc<Vec<String>> = Arc::new(opts.extra_ignores.clone());
    let recursive = opts.recursive;

    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(false)
        .hidden(false)
        .follow_links(opts.follow_symlinks)
        .threads(num_cpus::get().max(1))
        .max_depth(if recursive { None } else { Some(1) });

    if honor {
        let extra = extra.clone();
        builder.filter_entry(move |ent| {
            if ent.depth() == 0 {
                return true;
            }
            if ent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = ent.file_name().to_string_lossy();
                return !is_ignored_dir(&name, &extra);
            }
            true
        });
    }

    builder.build_parallel().run(|| {
        let acc = acc.clone();
        let last_emit = last_emit.clone();
        let emit_seq = emit_seq.clone();
        let mut buf = Buf::new(acc.clone());
        let emit = &emit;

        Box::new(move |res| {
            if cancel.load(Ordering::Relaxed) {
                buf.flush();
                return WalkState::Quit;
            }

            match res {
                Ok(ent) => {
                    let ft = ent.file_type();
                    let is_dir = ft.map(|t| t.is_dir()).unwrap_or(false);
                    if is_dir {
                        if ent.depth() > 0 {
                            buf.dirs += 1;
                        }
                    } else if ft.map(|t| t.is_file()).unwrap_or(false) {
                        match ent.metadata() {
                            Ok(meta) => {
                                let size = meta.len();
                                let name = ent.file_name().to_string_lossy().to_string();
                                let path = ent.path().to_string_lossy().to_string();
                                buf.current.clone_from(&path);
                                let ext = extension_of(&name);
                                buf.files += 1;
                                buf.bytes += size;
                                buf.entries.push(FileEntry {
                                    path,
                                    name,
                                    ext,
                                    size,
                                });
                            }
                            Err(e) => {
                                buf.skipped += 1;
                                if buf.errors.len() < 20 {
                                    buf.errors.push(format!("{}: {}", ent.path().display(), e));
                                }
                            }
                        }
                    } else {
                        buf.skipped += 1;
                    }
                }
                Err(e) => {
                    buf.skipped += 1;
                    if buf.errors.len() < 20 {
                        buf.errors.push(e.to_string());
                    }
                }
            }

            if buf.entries.len() >= FLUSH_EVERY {
                buf.flush();
            }

            if let Ok(mut gate) = last_emit.try_lock() {
                let dt = gate.0.elapsed();
                if dt >= Duration::from_millis(220) {
                    gate.0 = Instant::now();
                    let (files, dirs, bytes) = if let Ok(a) = acc.lock() {
                        (a.files + buf.files, a.dirs + buf.dirs, a.bytes + buf.bytes)
                    } else {
                        (buf.files, buf.dirs, buf.bytes)
                    };
                    let dsecs = dt.as_secs_f64().max(0.001);
                    let speed = files.saturating_sub(gate.1) as f64 / dsecs;
                    let _ = bytes;
                    gate.1 = files;
                    let _ = emit_seq.fetch_add(1, Ordering::Relaxed);
                    emit(Progress {
                        task: "scan".into(),
                        phase: "scanning".into(),
                        files,
                        dirs,
                        bytes,
                        speed,
                        current: buf.current.clone(),
                        message: format!("{} Dateien", files),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                        ..Default::default()
                    });
                }
            }

            WalkState::Continue
        })
    });

    let (entries, files, dirs, bytes, skipped, truncated, errors) = match acc.lock() {
        Ok(mut a) => (
            std::mem::take(&mut a.entries),
            a.files,
            a.dirs,
            a.bytes,
            a.skipped,
            a.truncated,
            std::mem::take(&mut a.errors),
        ),
        Err(_) => (Vec::new(), 0, 0, 0, 0, false, Vec::new()),
    };

    RawScan {
        entries,
        files,
        dirs,
        bytes,
        skipped,
        cancelled: cancel.load(Ordering::Relaxed),
        truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
        errors,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    fn tmpdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("df-scan-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn extension_normalisation() {
        assert_eq!(extension_of("A.JPG"), "jpg");
        assert_eq!(extension_of("archiv.tar.gz"), "tar.gz");
        assert_eq!(extension_of("pkg.pkg.tar.zst"), "pkg.tar.zst");
        assert_eq!(extension_of("README"), "");
        assert_eq!(extension_of(".bashrc"), "");
    }

    #[test]
    fn ignore_list_matches() {
        assert!(is_ignored_dir("$RECYCLE.BIN", &[]));
        assert!(is_ignored_dir("system volume information", &[]));
        assert!(is_ignored_dir(".Trash-1000", &[]));
        // multibyte names must not panic on prefix slicing
        assert!(!is_ignored_dir("皮卡丘的文件夹", &[]));
        assert!(!is_ignored_dir("日本語", &[]));
        assert!(is_ignored_dir("node_modules", &[]));
        assert!(is_ignored_dir("MeinOrdner", &["meinordner".to_string()]));
        assert!(!is_ignored_dir("Bilder", &[]));
    }

    #[test]
    fn scans_recursively_and_skips_ignored() {
        let root = tmpdir("basic");
        fs::write(root.join("a.jpg"), b"12345").unwrap();
        fs::write(root.join("b.JPG"), b"123").unwrap();
        fs::write(root.join("README"), b"x").unwrap();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/c.mp4"), b"1234567").unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/big.js"), b"1234567890").unwrap();

        let cancel = AtomicBool::new(false);
        let res = scan_tree(&root, &ScanOptions::default(), &cancel, |_p| {});
        assert_eq!(res.files, 4, "node_modules must be ignored");
        assert_eq!(res.bytes, 16);
        assert!(res.entries.iter().any(|e| e.ext == "jpg" && e.size == 5));
        assert!(res.entries.iter().any(|e| e.ext == "mp4"));

        // non-recursive
        let opts = ScanOptions {
            recursive: false,
            ..Default::default()
        };
        let flat = scan_tree(&root, &opts, &cancel, |_p| {});
        assert_eq!(flat.files, 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cancel_aborts_scan() {
        let root = tmpdir("cancel");
        for i in 0..50 {
            let mut f = fs::File::create(root.join(format!("f{}.txt", i))).unwrap();
            f.write_all(b"hello").unwrap();
        }
        let cancel = AtomicBool::new(true);
        let res = scan_tree(&root, &ScanOptions::default(), &cancel, |_p| {});
        assert!(res.cancelled);
        assert!(res.files < 50);
        let _ = fs::remove_dir_all(&root);
    }
}
