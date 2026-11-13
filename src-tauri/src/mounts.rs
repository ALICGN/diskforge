use crate::models::MountInfo;
use std::collections::HashMap;
use std::path::Path;

/// /proc/self/mounts escapes spaces and other special bytes as octal sequences.
pub fn unescape_mount(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            let oct = &s[i + 1..i + 4];
            if let Ok(v) = u8::from_str_radix(oct, 8) {
                out.push(v as char);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn space_of(path: &str) -> (u64, u64, u64) {
    #[cfg(unix)]
    unsafe {
        use std::ffi::CString;
        let Ok(c) = CString::new(path) else {
            return (0, 0, 0);
        };
        let mut st: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c.as_ptr(), &mut st) == 0 {
            let bs = st.f_frsize as u64;
            let total = st.f_blocks as u64 * bs;
            let free = st.f_bavail as u64 * bs;
            let used = (st.f_blocks as u64).saturating_sub(st.f_bfree as u64) * bs;
            return (total, used, free);
        }
    }
    (0, 0, 0)
}

fn is_removable(source: &str) -> bool {
    let dev = Path::new(source)
        .file_name()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default();
    if !dev.starts_with("sd") && !dev.starts_with("nvme") && !dev.starts_with("mmcblk") {
        return false;
    }
    let base: String = dev.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
    let base = base.trim_end_matches('p').to_string();
    let flag = std::fs::read_to_string(format!("/sys/block/{}/removable", base))
        .unwrap_or_default()
        .trim()
        .to_string();
    flag == "1" || flag.starts_with("1")
}

struct RawMount {
    source: String,
    path: String,
    fs: String,
    ro: bool,
}

fn parse_mounts(text: &str) -> Vec<RawMount> {
    let mut out = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let source = unescape_mount(parts[0]);
        let path = unescape_mount(parts[1]);
        let fs = parts[2].to_string();
        let options = parts[3];
        let ro = options.split(',').any(|o| o == "ro");
        out.push(RawMount { source, path, fs, ro });
    }
    out
}

fn interesting(m: &RawMount) -> bool {
    if m.path == "/boot" || m.path.starts_with("/boot/") {
        return false;
    }
    if m.fs == "squashfs" || m.source.starts_with("/dev/loop") {
        return false;
    }
    if m.path.starts_with("/var/") || m.path == "/var" || m.path.starts_with("/run/user") {
        return false;
    }
    // btrfs subvolumes (/root, /srv, /.snapshots, ...) would list the same
    // device over and over; only the real entry points are interesting.
    if m.fs == "btrfs"
        && !matches!(m.path.as_str(), "/" | "/home")
        && !m.path.starts_with("/run/media/")
        && !m.path.starts_with("/media/")
        && !m.path.starts_with("/mnt/")
    {
        return false;
    }
    if m.source.starts_with("/dev/") {
        return true;
    }
    matches!(
        m.fs.as_str(),
        "fuse.ntfs-3g" | "fuseblk" | "ntfs3" | "ntfs" | "exfat" | "vfat" | "fuse.exfat"
    ) || m.path.starts_with("/run/media/")
        || m.path.starts_with("/media/")
        || m.path.starts_with("/mnt/")
}

pub fn list_mounts() -> Vec<MountInfo> {
    let text = std::fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let raws = parse_mounts(&text);

    // Prefer the most specific mount for each path.
    let mut by_path: HashMap<String, usize> = HashMap::new();
    let mut mounts: Vec<MountInfo> = Vec::new();
    for m in raws.into_iter().filter(interesting) {
        if let Some(&idx) = by_path.get(&m.path) {
            mounts[idx].readonly = m.ro;
            continue;
        }
        if !Path::new(&m.path).exists() {
            continue;
        }
        let (total, used, free) = space_of(&m.path);
        let label = Path::new(&m.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| m.path.clone());
        by_path.insert(m.path.clone(), mounts.len());
        mounts.push(MountInfo {
            path: m.path.clone(),
            source: m.source.clone(),
            fs: m.fs.clone(),
            total,
            used,
            free,
            readonly: m.ro,
            removable: is_removable(&m.source) || m.path.starts_with("/run/media"),
            label,
        });
    }

    mounts.sort_by(|a, b| {
        let rank = |p: &str| {
            if p.starts_with("/run/media") {
                0
            } else if p.starts_with("/media") || p.starts_with("/mnt") {
                1
            } else if p == "/home" {
                2
            } else if p == "/" {
                4
            } else {
                3
            }
        };
        rank(&a.path).cmp(&rank(&b.path)).then(a.path.cmp(&b.path))
    });
    mounts
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
/dev/nvme0n1p2 / ext4 rw,noatime 0 0
/dev/nvme0n1p1 /boot vfat rw 0 0
/dev/sdb2 /run/media/ali/Externe\\040HD fuseblk ro,nosuid,nodev 0 0
tmpfs /run tmpfs rw 0 0
/dev/loop0 /var/lib/snapd/snap/core squashfs ro 0 0
server:/export /mnt/nfs nfs4 rw 0 0
";

    #[test]
    fn unescape_works() {
        assert_eq!(unescape_mount("/run/media/a/My\\040Disk"), "/run/media/a/My Disk");
        assert_eq!(unescape_mount("plain"), "plain");
    }

    #[test]
    fn parses_and_filters_mounts() {
        let raws = parse_mounts(SAMPLE);
        assert_eq!(raws.len(), 6);
        let filtered: Vec<&RawMount> = raws.iter().filter(|m| interesting(m)).collect();
        let paths: Vec<&str> = filtered.iter().map(|m| m.path.as_str()).collect();
        assert!(paths.contains(&"/run/media/ali/Externe HD"));
        assert!(!paths.contains(&"/boot"));
        assert!(!paths.iter().any(|p| p.contains("snapd")));
        let ext = filtered.iter().find(|m| m.path.contains("Externe")).unwrap();
        assert!(ext.ro);
        assert_eq!(ext.fs, "fuseblk");
    }

    #[test]
    fn live_mounts_are_listed() {
        let list = list_mounts();
        assert!(!list.is_empty(), "at least / must be detected");
        assert!(list.iter().any(|m| m.path == "/"));
    }
}
