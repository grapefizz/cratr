use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub file_type: String,
    pub can_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesResponse {
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub used_bytes: u64,
    pub total_files: usize,
    pub used_percentage: f64,
    pub formatted_used: String,
    pub max_size_mb: u64,
    pub disk_free_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_used_percentage: f64,
    pub formatted_disk_free: String,
    pub formatted_disk_total: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResponse {
    pub content: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    pub debug_mode: bool,
}

#[cfg(feature = "frontend")]
pub mod frontend;

#[cfg(feature = "frontend")]
pub use frontend::*;
