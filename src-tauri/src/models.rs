use serde::{Deserialize, Serialize};

/// Progress payload emitted to the frontend via `df://progress`.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub task: String,
    pub phase: String,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub total_bytes: u64,
    pub done_bytes: u64,
    pub speed: f64,
    pub current: String,
    pub message: String,
    pub elapsed_ms: u64,
    pub eta_s: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub exts: Vec<String>,
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryConfig {
    pub version: u32,
    pub categories: Vec<Category>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExtStat {
    pub ext: String,
    pub count: u64,
    pub size: u64,
    pub category: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStat {
    pub name: String,
    pub color: String,
    pub count: u64,
    pub size: u64,
    pub exts: Vec<ExtStat>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub root: String,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub skipped: u64,
    pub duration_ms: u64,
    pub cancelled: bool,
    pub truncated: bool,
    pub threads: usize,
    pub categories: Vec<CategoryStat>,
    pub errors: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    pub id: usize,
    pub size: u64,
    pub hash: String,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_s: Option<f64>,
    pub bitrate_kbps: Option<u64>,
    pub mtime_ms: Option<i64>,
    pub ctime_ms: Option<i64>,
    pub btime_ms: Option<i64>,
    pub inode: u64,
    pub readonly: bool,
    pub kind: String,
    pub exists: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MountInfo {
    pub path: String,
    pub source: String,
    pub fs: String,
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub readonly: bool,
    pub removable: bool,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogOp {
    pub src: String,
    pub dst: String,
    pub size: u64,
    pub action: String,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogBatch {
    pub id: String,
    pub started_ms: i64,
    pub root: String,
    pub target: String,
    pub structure: String,
    pub dry_run: bool,
    #[serde(default)]
    pub undone: bool,
    pub moved: u64,
    pub copied: u64,
    pub deleted: u64,
    pub skipped: u64,
    pub bytes: u64,
    pub errors: u64,
    pub duration_ms: u64,
    pub ops: Vec<LogOp>,
}
