use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for different storage types
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum StorageConfig {
    #[serde(rename = "file_system")]
    FileSystem {
        root_dir: String,
        shard_levels: Option<String>, // Number of directory levels
        shard_chars: Option<String>,  // Characters per level
        serve_url: Option<String>,
    },
}

/// Metadata for stored files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Path to the file within the storage
    pub path: String,

    /// Size of the file in bytes
    pub size: u64,

    /// Content type (MIME type) of the file
    pub content_type: Option<String>,

    /// When the file was created or last modified
    pub modified: DateTime<Utc>,

    /// URL to access the file (might be CDN, pre-signed URL, or local path)
    pub url: String,
}
