use crate::models::FileMeta;
use std::path::Path;
use std::process::Command;
use std::time::UNIX_EPOCH;

pub const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "svg", "avif", "heic", "heif",
    "ico", "psd",
];
pub const VIDEO_EXTS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "ts", "mpg", "mpeg", "3gp", "mts",
    "m2ts", "vob",
];
pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "ogg", "opus", "m4a", "wma", "aiff", "alac", "ape",
];

pub fn kind_of(ext: &str) -> &'static str {
    if IMAGE_EXTS.contains(&ext) {
        "image"
    } else if VIDEO_EXTS.contains(&ext) {
        "video"
    } else if AUDIO_EXTS.contains(&ext) {
        "audio"
    } else {
        "file"
    }
}

fn to_ms(t: std::io::Result<std::time::SystemTime>) -> Option<i64> {
    t.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
}

/// Reads metadata for one path. Never panics; problems land in `error`.
pub fn read_meta(path_str: &str, want_probe: bool) -> FileMeta {
    let path = Path::new(path_str);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path_str.to_string());
    let ext = crate::scanner::extension_of(&name);
    let kind = kind_of(&ext);

    let mut meta = FileMeta {
        path: path_str.to_string(),
        name,
        ext: ext.clone(),
        size: 0,
        width: None,
        height: None,
        duration_s: None,
        bitrate_kbps: None,
        mtime_ms: None,
        ctime_ms: None,
        btime_ms: None,
        inode: 0,
        readonly: false,
        kind: kind.to_string(),
        exists: false,
        error: None,
    };

    let md = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            meta.error = Some(e.to_string());
            return meta;
        }
    };
    meta.exists = true;
    meta.size = md.len();
    meta.readonly = md.permissions().readonly();
    meta.mtime_ms = to_ms(md.modified());
    meta.btime_ms = to_ms(md.created());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.inode = md.ino();
        meta.ctime_ms = Some(md.ctime() * 1000);
    }

    if !want_probe {
        return meta;
    }

    match kind {
        "image" => match imagesize::size(path) {
            Ok(s) => {
                meta.width = Some(s.width as u32);
                meta.height = Some(s.height as u32);
            }
            Err(e) => {
                meta.error = Some(format!("Bildgröße: {}", e));
            }
        },
        "video" | "audio" => {
            if let Some((w, h, dur, br)) = ffprobe(path) {
                meta.width = w;
                meta.height = h;
                meta.duration_s = dur;
                meta.bitrate_kbps = br;
            } else if kind == "video" {
                if let Some((w, h, dur)) = mp4_meta(path) {
                    meta.width = Some(w);
                    meta.height = Some(h);
                    meta.duration_s = dur;
                } else {
                    meta.error = Some("keine Video-Metadaten (ffprobe/mp4parse)".into());
                }
            }
        }
        _ => {}
    }
    meta
}

fn json_num(v: &serde_json::Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| {
        x.as_f64()
            .or_else(|| x.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

/// Resolution, duration (seconds) and bitrate (bit/s) as reported by ffprobe.
pub type ProbeInfo = (Option<u32>, Option<u32>, Option<f64>, Option<u64>);

/// ffprobe (if installed) for resolution, duration and bitrate.
pub fn ffprobe(path: &Path) -> Option<ProbeInfo> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,duration,bit_rate:format=duration,bit_rate",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let stream = v.get("streams").and_then(|s| s.get(0));
    let format = v.get("format");
    let width = stream.and_then(|s| json_num(s, "width")).map(|f| f as u32);
    let height = stream.and_then(|s| json_num(s, "height")).map(|f| f as u32);
    let duration = stream
        .and_then(|s| json_num(s, "duration"))
        .or_else(|| format.and_then(|f| json_num(f, "duration")));
    let bitrate = stream
        .and_then(|s| json_num(s, "bit_rate"))
        .or_else(|| format.and_then(|f| json_num(f, "bit_rate")))
        .map(|b| (b / 1000.0).round() as u64);
    Some((width, height, duration, bitrate))
}

/// mp4parse fallback (MP4/MOV only, no external binary required).
pub fn mp4_meta(path: &Path) -> Option<(u32, u32, Option<f64>)> {
    let mut f = std::io::BufReader::new(std::fs::File::open(path).ok()?);
    let ctx = mp4parse::read_mp4(&mut f).ok()?;
    let timescale = ctx.timescale.map(|t| t.0).unwrap_or(0);
    let mut best: Option<(u32, u32, Option<f64>)> = None;
    for track in ctx.tracks.iter() {
        if track.track_type != mp4parse::TrackType::Video {
            continue;
        }
        if let Some(tkhd) = track.tkhd.as_ref() {
            let w = tkhd.width >> 16;
            let h = tkhd.height >> 16;
            if w == 0 || h == 0 {
                continue;
            }
            let dur = if timescale > 0 && tkhd.duration > 0 {
                Some(tkhd.duration as f64 / timescale as f64)
            } else {
                None
            };
            best = Some((w, h, dur));
            break;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn kind_detection() {
        assert_eq!(kind_of("jpg"), "image");
        assert_eq!(kind_of("mkv"), "video");
        assert_eq!(kind_of("flac"), "audio");
        assert_eq!(kind_of("txt"), "file");
    }

    #[test]
    fn missing_file_is_reported_not_panicked() {
        let m = read_meta("/nonexistent/nope.jpg", true);
        assert!(!m.exists);
        assert!(m.error.is_some());
    }

    #[test]
    fn timestamps_and_size_are_filled() {
        let p = std::env::temp_dir().join("df-meta-test.txt");
        fs::write(&p, b"12345").unwrap();
        let m = read_meta(&p.to_string_lossy(), false);
        assert!(m.exists);
        assert_eq!(m.size, 5);
        assert!(m.mtime_ms.unwrap() > 0);
        assert!(m.ctime_ms.unwrap() > 0);
        let _ = fs::remove_file(&p);
    }
}
