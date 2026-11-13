use crate::models::{DupGroup, FileEntry, Progress};
use blake3::Hasher;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const PARTIAL_THRESHOLD: u64 = 50 * 1024 * 1024; // 50 MB
const PARTIAL_WINDOW: u64 = 64 * 1024; // 64 KB head + tail
const READ_CHUNK: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct DupOptions {
    pub min_size: u64,
    pub partial_threshold: u64,
}

impl Default for DupOptions {
    fn default() -> Self {
        Self {
            min_size: 1,
            partial_threshold: PARTIAL_THRESHOLD,
        }
    }
}

#[derive(Clone)]
struct Cand {
    size: u64,
    entry: FileEntry,
    hash: String,
    partial: bool,
}

pub(crate) struct Counters {
    files: AtomicU64,
    bytes: AtomicU64,
    total_bytes: u64,
    total_files: u64,
}

impl Counters {
    fn emit(&self, started: Instant, last: &Mutex<Instant>, current: &str, emit: &dyn Fn(Progress)) {
        let mut guard = match last.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if guard.elapsed() < Duration::from_millis(200) {
            return;
        }
        *guard = Instant::now();
        let files = self.files.load(Ordering::Relaxed);
        let bytes = self.bytes.load(Ordering::Relaxed);
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        emit(Progress {
            task: "dedupe".into(),
            phase: "hashing".into(),
            files,
            bytes,
            total_bytes: self.total_bytes,
            done_bytes: bytes,
            speed: bytes as f64 / elapsed,
            current: current.to_string(),
            message: format!("{}/{} Dateien gehasht", files, self.total_files),
            elapsed_ms: started.elapsed().as_millis() as u64,
            eta_s: if bytes > 0 {
                ((self.total_bytes.saturating_sub(bytes)) as f64) / (bytes as f64 / elapsed)
            } else {
                0.0
            },
            ..Default::default()
        });
    }
}

pub fn quick_hash(path: &Path) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut hasher = Hasher::new();
    hasher.update(&len.to_le_bytes());
    let mut buf = vec![0u8; PARTIAL_WINDOW as usize];
    let n = f.read(&mut buf)?;
    hasher.update(&buf[..n]);
    if len > PARTIAL_WINDOW * 2 {
        f.seek(SeekFrom::Start(len - PARTIAL_WINDOW))?;
        let n = f.read(&mut buf)?;
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

pub fn full_hash(
    path: &Path,
    counters: Option<&Counters>,
    cancel: Option<&AtomicBool>,
) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "abgebrochen",
                ));
            }
        }
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if n >= 256 * 1024 {
            hasher.update_rayon(&buf[..n]);
        } else {
            hasher.update(&buf[..n]);
        }
        if let Some(c) = counters {
            c.bytes.fetch_add(n as u64, Ordering::Relaxed);
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Two-stage duplicate detection: size grouping -> (quick hash for big files) -> full BLAKE3.
pub fn find_duplicates<F>(
    entries: &[FileEntry],
    opts: &DupOptions,
    cancel: &AtomicBool,
    emit: F,
) -> (Vec<DupGroup>, bool)
where
    F: Fn(Progress) + Send + Sync,
{
    let started = Instant::now();
    let last = Mutex::new(Instant::now() - Duration::from_secs(1));

    // Stage 0: group by exact byte size, only groups > 1 matter.
    let mut by_size: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for e in entries {
        if e.size >= opts.min_size.max(1) {
            by_size.entry(e.size).or_default().push(e.clone());
        }
    }
    let mut candidates: Vec<FileEntry> = Vec::new();
    for (_size, group) in by_size.into_iter() {
        if group.len() > 1 {
            candidates.extend(group);
        }
    }

    let total_bytes: u64 = candidates.iter().map(|c| c.size).sum();
    let counters = Counters {
        files: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
        total_bytes,
        total_files: candidates.len() as u64,
    };
    let emit_ref: &(dyn Fn(Progress) + Sync) = &emit;

    // Stage 1: parallel hashing (partial for big files).
    let stage1: Vec<Option<Cand>> = candidates
        .par_iter()
        .map(|entry| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let path = Path::new(&entry.path);
            let big = entry.size > opts.partial_threshold;
            let hash = if big {
                quick_hash(path).ok()
            } else {
                full_hash(path, Some(&counters), Some(cancel)).ok()
            };
            counters.files.fetch_add(1, Ordering::Relaxed);
            if !big {
                counters.bytes.fetch_add(entry.size, Ordering::Relaxed);
            }
            counters.emit(started, &last, &entry.name, emit_ref);
            hash.map(|h| Cand {
                size: entry.size,
                entry: entry.clone(),
                hash: h,
                partial: big,
            })
        })
        .collect();

    let stage1: Vec<Cand> = stage1.into_iter().flatten().collect();

    let mut buckets: HashMap<(u64, String), Vec<Cand>> = HashMap::new();
    for c in stage1 {
        buckets
            .entry((c.size, c.hash.clone()))
            .or_default()
            .push(c);
    }

    // Stage 2: full hash for partial-hash buckets that still collide.
    let needs_full: Vec<FileEntry> = buckets
        .values()
        .filter(|g| g.len() > 1 && g[0].partial)
        .flat_map(|g| g.iter().map(|c| c.entry.clone()))
        .collect();

    let full: HashMap<String, String> = if needs_full.is_empty() {
        HashMap::new()
    } else {
        needs_full
            .par_iter()
            .filter_map(|e| {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let h = full_hash(Path::new(&e.path), Some(&counters), Some(cancel)).ok()?;
                counters.files.fetch_add(1, Ordering::Relaxed);
                counters.emit(started, &last, &e.name, emit_ref);
                Some((e.path.clone(), h))
            })
            .collect()
    };

    let mut final_buckets: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();
    for (_key, group) in buckets.into_iter() {
        if group.len() < 2 {
            continue;
        }
        for c in group {
            let hash = if c.partial {
                match full.get(&c.entry.path) {
                    Some(h) => h.clone(),
                    None => continue,
                }
            } else {
                c.hash
            };
            final_buckets
                .entry((c.size, hash))
                .or_default()
                .push(c.entry);
        }
    }

    let mut groups: Vec<DupGroup> = final_buckets
        .into_iter()
        .filter(|(_k, v)| v.len() > 1)
        .enumerate()
        .map(|(i, ((size, hash), mut files))| {
            files.sort_by(|a, b| a.path.cmp(&b.path));
            DupGroup {
                id: i,
                size,
                hash,
                files,
            }
        })
        .collect();

    // Biggest wasted space first.
    groups.sort_by(|a, b| {
        let wa = a.size * (a.files.len() as u64 - 1);
        let wb = b.size * (b.files.len() as u64 - 1);
        wb.cmp(&wa)
    });
    for (i, g) in groups.iter_mut().enumerate() {
        g.id = i;
    }

    emit(Progress {
        task: "dedupe".into(),
        phase: "done".into(),
        files: counters.files.load(Ordering::Relaxed),
        bytes: total_bytes,
        total_bytes,
        done_bytes: total_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
        message: format!("{} Duplikat-Gruppen", groups.len()),
        ..Default::default()
    });

    (groups, cancel.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    fn tmpdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("df-hash-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn entry(p: &Path) -> FileEntry {
        FileEntry {
            path: p.to_string_lossy().to_string(),
            name: p.file_name().unwrap().to_string_lossy().to_string(),
            ext: "dat".into(),
            size: fs::metadata(p).unwrap().len(),
        }
    }

    #[test]
    fn finds_identical_files_only() {
        let dir = tmpdir("dups");
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        let c = dir.join("c.bin");
        let d = dir.join("d.bin");
        for p in [&a, &b] {
            let mut f = fs::File::create(p).unwrap();
            f.write_all(b"identical content 123").unwrap();
        }
        let mut f = fs::File::create(&c).unwrap();
        f.write_all(b"different content 456").unwrap(); // same length, different bytes
        let mut f = fs::File::create(&d).unwrap();
        f.write_all(b"shorter").unwrap();

        let entries = vec![entry(&a), entry(&b), entry(&c), entry(&d)];
        let cancel = AtomicBool::new(false);
        let (groups, _) = find_duplicates(&entries, &DupOptions::default(), &cancel, |_p| {});
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[0].size, 21);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_hash_resolves_big_files() {
        let dir = tmpdir("partial");
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        let big = 2 * 1024 * 1024;
        let mut buf = vec![7u8; big];
        fs::write(&a, &buf).unwrap();
        buf[big / 2] = 9;
        fs::write(&b, &buf).unwrap();

        let entries = vec![entry(&a), entry(&b)];
        let cancel = AtomicBool::new(false);
        let opts = DupOptions {
            min_size: 1,
            partial_threshold: 1024, // force partial path
        };
        let (groups, _) = find_duplicates(&entries, &opts, &cancel, |_p| {});
        assert!(groups.is_empty(), "mid-file difference must be detected");
        let _ = fs::remove_dir_all(&dir);
    }
}
