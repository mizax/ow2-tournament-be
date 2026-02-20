mod error;
mod file_system;
mod models;
mod util;

pub use error::StorageError;
pub use file_system::FileSystemStorage;
pub use models::{FileMetadata, StorageConfig};

use async_trait::async_trait;
use std::path::Path;

/// Storage trait defines operations for storing and retrieving files
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a file from bytes
    async fn store_file_from_bytes(
        &self,
        path: &str,
        filename: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<FileMetadata, StorageError>;

    /// Store a file from a local path
    async fn store_file_from_path(
        &self,
        source_path: &Path,
        dest_path: &str,
        content_type: Option<&str>,
    ) -> Result<FileMetadata, StorageError>;

    /// Retrieve a file
    async fn get_file(&self, path: &str) -> Result<Vec<u8>, StorageError>;

    /// Get file metadata
    async fn get_metadata(&self, path: &str) -> Result<FileMetadata, StorageError>;

    /// Delete a file
    async fn delete_file(&self, path: &str) -> Result<(), StorageError>;

    /// Check if a file exists
    async fn file_exists(&self, path: &str) -> Result<bool, StorageError>;

    /// Get a URL for the file (useful for CDN or S3 pre-signed URLs)
    async fn get_url(&self, path: &str) -> Result<String, StorageError>;
}

/// Factory for creating storage implementations
pub async fn create_storage(config: StorageConfig) -> Result<Box<dyn Storage>, StorageError> {
    match config {
        StorageConfig::FileSystem {
            root_dir,
            shard_levels,
            shard_chars,
            serve_url,
        } => {
            let shard_levels_usize = match shard_levels {
                None => None,
                Some(v) => Some(v.parse().unwrap()),
            };
            let shard_chars_usize = match shard_chars {
                None => None,
                Some(v) => Some(v.parse().unwrap()),
            };
            let shard_levels = shard_levels_usize.unwrap_or(2);
            let shard_chars = shard_chars_usize.unwrap_or(2);

            Ok(Box::new(FileSystemStorage::with_sharding(
                root_dir,
                shard_levels,
                shard_chars,
                serve_url.unwrap_or_default(),
            )))
        }
    }
}
