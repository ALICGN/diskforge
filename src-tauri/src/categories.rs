use crate::models::{Category, CategoryConfig, CategoryStat, ExtStat, FileEntry};
use std::collections::HashMap;
use std::path::PathBuf;

pub const UNKNOWN_CATEGORY: &str = "Sonstiges";
pub const NO_EXT_KEY: &str = "Ohne_Endung";

impl CategoryConfig {
    pub fn defaults() -> Self {
        let cat = |name: &str, color: &str, exts: &[&str]| Category {
            name: name.to_string(),
            color: color.to_string(),
            exts: exts.iter().map(|e| e.to_string()).collect(),
            builtin: true,
        };
        CategoryConfig {
            version: 1,
            categories: vec![
                cat(
                    "Bilder",
                    "#38bdf8",
                    &[
                        "jpg", "jpeg", "png", "webp", "gif", "bmp", "svg", "tiff", "tif", "raw",
                        "cr2", "cr3", "nef", "arw", "heic", "heif", "avif", "dng", "psd", "ico",
                    ],
                ),
                cat(
                    "Videos",
                    "#a78bfa",
                    &[
                        "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "ts", "mpg",
                        "mpeg", "3gp", "mts", "m2ts", "vob",
                    ],
                ),
                cat(
                    "Audio",
                    "#f472b6",
                    &[
                        "mp3", "flac", "wav", "aac", "ogg", "opus", "m4a", "wma", "aiff", "alac",
                        "ape", "dsf",
                    ],
                ),
                cat(
                    "Dokumente",
                    "#fbbf24",
                    &[
                        "pdf", "doc", "docx", "odt", "txt", "md", "rtf", "xls", "xlsx", "ods",
                        "csv", "ppt", "pptx", "odp", "epub", "mobi", "pages", "numbers", "key",
                        "tex", "log",
                    ],
                ),
                cat(
                    "Archive",
                    "#fb923c",
                    &[
                        "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "zst", "iso", "tar.gz",
                        "tar.bz2", "tar.xz", "tar.zst", "lz4", "cab",
                    ],
                ),
                cat(
                    "Programme & Skripte",
                    "#4ade80",
                    &[
                        "exe",
                        "msi",
                        "deb",
                        "rpm",
                        "pkg.tar.zst",
                        "appimage",
                        "sh",
                        "bash",
                        "zsh",
                        "fish",
                        "py",
                        "rs",
                        "js",
                        "ts",
                        "jsx",
                        "tsx",
                        "json",
                        "yaml",
                        "yml",
                        "toml",
                        "ini",
                        "conf",
                        "service",
                        "desktop",
                        "patch",
                        "diff",
                        "go",
                        "c",
                        "h",
                        "cpp",
                        "hpp",
                        "java",
                        "kt",
                        "lua",
                        "php",
                        "rb",
                        "sql",
                        "html",
                        "css",
                        "scss",
                        "vue",
                        "svelte",
                    ],
                ),
                cat(UNKNOWN_CATEGORY, "#94a3b8", &[]),
            ],
        }
    }

    /// Lowercases, trims dots, removes duplicates and guarantees the fallback category.
    pub fn normalize(&mut self) {
        let mut seen: HashMap<String, ()> = HashMap::new();
        for c in self.categories.iter_mut() {
            c.name = c.name.trim().to_string();
            let mut out: Vec<String> = Vec::new();
            for e in c.exts.iter() {
                let e = e.trim().trim_start_matches('.').to_lowercase();
                if e.is_empty() {
                    continue;
                }
                if seen.contains_key(&e) {
                    continue;
                }
                seen.insert(e.clone(), ());
                out.push(e);
            }
            out.sort();
            out.dedup();
            c.exts = out;
        }
        if !self.categories.iter().any(|c| c.name == UNKNOWN_CATEGORY) {
            self.categories.push(Category {
                name: UNKNOWN_CATEGORY.to_string(),
                color: "#94a3b8".to_string(),
                exts: Vec::new(),
                builtin: true,
            });
        }
        let fallback = self
            .categories
            .iter()
            .position(|c| c.name == UNKNOWN_CATEGORY)
            .unwrap();
        if fallback != self.categories.len() - 1 {
            let c = self.categories.remove(fallback);
            self.categories.push(c);
        }
    }

    pub fn ext_map(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        for c in &self.categories {
            for e in &c.exts {
                m.entry(e.clone()).or_insert_with(|| c.name.clone());
            }
        }
        m
    }

    pub fn category_for(&self, ext: &str, map: &HashMap<String, String>) -> String {
        if ext.is_empty() {
            return UNKNOWN_CATEGORY.to_string();
        }
        map.get(ext)
            .cloned()
            .unwrap_or_else(|| UNKNOWN_CATEGORY.to_string())
    }

    /// Builds the per-category / per-extension aggregation used by the UI.
    pub fn aggregate(&self, entries: &[FileEntry]) -> Vec<CategoryStat> {
        let map = self.ext_map();
        // key = "extension" (empty string -> Ohne_Endung)
        let mut by_ext: HashMap<String, (u64, u64)> = HashMap::new();
        for e in entries {
            let slot = by_ext.entry(e.ext.clone()).or_insert((0, 0));
            slot.0 += 1;
            slot.1 += e.size;
        }

        let mut stats: Vec<CategoryStat> = self
            .categories
            .iter()
            .map(|c| CategoryStat {
                name: c.name.clone(),
                color: c.color.clone(),
                count: 0,
                size: 0,
                exts: Vec::new(),
            })
            .collect();
        let index_by_name: HashMap<String, usize> = stats
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.clone(), i))
            .collect();

        for (ext, (count, size)) in by_ext.into_iter() {
            let cat_name = self.category_for(&ext, &map);
            let idx = *index_by_name
                .get(&cat_name)
                .unwrap_or(&stats.len().saturating_sub(1));
            let label = if ext.is_empty() {
                NO_EXT_KEY.to_string()
            } else {
                ext.to_uppercase()
            };
            let s = &mut stats[idx];
            s.count += count;
            s.size += size;
            s.exts.push(ExtStat {
                ext: label,
                count,
                size,
                category: s.name.clone(),
            });
        }

        for s in stats.iter_mut() {
            s.exts
                .sort_by(|a, b| b.size.cmp(&a.size).then(b.count.cmp(&a.count)));
        }
        stats
    }

    pub fn category_of_ext(&self, ext: &str) -> String {
        let map = self.ext_map();
        self.category_for(ext, &map)
    }
}

pub fn config_dir() -> PathBuf {
    // FYLR_HOME is used by the test-suite to keep logs out of the real profile.
    if let Ok(custom) = std::env::var("FYLR_HOME") {
        if !custom.trim().is_empty() {
            let dir = PathBuf::from(custom);
            let _ = std::fs::create_dir_all(&dir);
            return dir;
        }
    }
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("fylr");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn config_file(name: &str) -> PathBuf {
    config_dir().join(name)
}

pub fn load_categories() -> CategoryConfig {
    let path = config_file("categories.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(mut cfg) = serde_json::from_str::<CategoryConfig>(&text) {
            cfg.normalize();
            return cfg;
        }
    }
    let mut cfg = CategoryConfig::defaults();
    cfg.normalize();
    let _ = save_categories(&cfg);
    cfg
}

pub fn save_categories(cfg: &CategoryConfig) -> Result<(), String> {
    let mut cfg = cfg.clone();
    cfg.normalize();
    let text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    let path = config_file("categories.json");
    std::fs::write(&path, text).map_err(|e| format!("{}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_all_categories() {
        let mut cfg = CategoryConfig::defaults();
        cfg.normalize();
        let map = cfg.ext_map();
        assert_eq!(cfg.category_for("jpg", &map), "Bilder");
        assert_eq!(
            cfg.category_for("JPG".to_lowercase().as_str(), &map),
            "Bilder"
        );
        assert_eq!(cfg.category_for("zst", &map), "Archive");
        assert_eq!(cfg.category_for("pkg.tar.zst", &map), "Programme & Skripte");
        assert_eq!(cfg.category_for("qqq", &map), UNKNOWN_CATEGORY);
        assert_eq!(cfg.category_for("", &map), UNKNOWN_CATEGORY);
    }

    #[test]
    fn aggregate_counts_and_fallback() {
        let mut cfg = CategoryConfig::defaults();
        cfg.normalize();
        let entries = vec![
            FileEntry {
                path: "/a/x.jpg".into(),
                name: "x.jpg".into(),
                ext: "jpg".into(),
                size: 10,
            },
            FileEntry {
                path: "/a/y.JPG".into(),
                name: "y.JPG".into(),
                ext: "jpg".into(),
                size: 20,
            },
            FileEntry {
                path: "/a/z.qqq".into(),
                name: "z.qqq".into(),
                ext: "qqq".into(),
                size: 5,
            },
            FileEntry {
                path: "/a/README".into(),
                name: "README".into(),
                ext: "".into(),
                size: 3,
            },
        ];
        let stats = cfg.aggregate(&entries);
        let bilder = stats.iter().find(|s| s.name == "Bilder").unwrap();
        assert_eq!(bilder.count, 2);
        assert_eq!(bilder.size, 30);
        let sonstiges = stats.iter().find(|s| s.name == UNKNOWN_CATEGORY).unwrap();
        assert_eq!(sonstiges.count, 2);
        assert!(sonstiges.exts.iter().any(|e| e.ext == NO_EXT_KEY));
        // fallback must stay last
        assert_eq!(stats.last().unwrap().name, UNKNOWN_CATEGORY);
    }

    #[test]
    fn duplicate_ext_moves_are_deduplicated() {
        let mut cfg = CategoryConfig::defaults();
        cfg.categories[1].exts.push("jpg".to_string()); // Videos claims jpg
        cfg.normalize();
        let map = cfg.ext_map();
        assert_eq!(cfg.category_for("jpg", &map), "Bilder");
    }
}
