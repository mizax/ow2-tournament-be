use crate::storage::{FileMetadata, Storage, StorageError};
use async_trait::async_trait;
use chrono::Utc;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use ulid::Ulid;
use crate::storage::util::compose_sharded_path;

/// File system based implementation of Storage
pub struct FileSystemStorage {
    root_dir: PathBuf,
    shard_levels: usize,  // Number of directory levels for sharding
    shard_chars: usize,   // Number of characters per directory level
    serve_url: String,
}

impl FileSystemStorage {
    /// Create a storage with a custom sharding configuration
    pub fn with_sharding<P: AsRef<Path>>(root_dir: P, shard_levels: usize, shard_chars: usize, serve_url: String) -> Self {
        let root = PathBuf::from(root_dir.as_ref());
        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&root).unwrap_or_else(|e| {
            log::warn!("Failed to create a storage directory: {}", e);
        });
        
        Self { 
            root_dir: root,
            shard_levels,
            shard_chars,
            serve_url,
        }
    }
    
    // Generate a sharded path based on file content or a ULID
    fn generate_sharded_path(&self, file_name: &str) -> String {
        // Generate a ULID for this file (includes timestamp + random)
        let ulid = Ulid::new();
        let ulid_str = ulid.to_string();

        // Extract the file extension
        let extension = Path::new(file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        // Use the random part of the ULID for sharding (last 16 characters)
        // ULID format: 10 chars timestamp + 16 chars random (in base32)
        let random_part = &ulid_str[10..];
        let sharded_path = compose_sharded_path(random_part, self.shard_levels, self.shard_chars);

        // Create the final file name with the ULID and original extension
        let final_file_name = if extension.is_empty() {
            ulid_str.clone()
        } else {
            format!("{}.{}", ulid_str, extension)
        };

        // Combine the path and filename
        let full_path = format!("{}{}", sharded_path, final_file_name);

        full_path
    }
    
    fn resolve_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        let path = Path::new(path);
        
        // Prevent directory traversal attacks
        let canonical_root = self.root_dir.canonicalize()
            .map_err(|e| StorageError::ConfigError(format!("Invalid root directory: {}", e)))?;
            
        let full_path = self.root_dir.join(path);
        
        // Ensure the path is within the root directory
        let canonical_path = full_path.canonicalize().unwrap_or(full_path.clone());
        if !canonical_path.starts_with(&canonical_root) && canonical_path.exists() {
            return Err(StorageError::InvalidPath(path.to_path_buf()));
        }
        
        Ok(full_path)
    }
    
    async fn ensure_parent_dir(&self, path: &Path) -> Result<(), StorageError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e))?;
        }
        Ok(())
    }
    
    async fn get_file_metadata(&self, path: &Path, orig_content_type: Option<&str>) -> Result<FileMetadata, StorageError> {
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| StorageError::NotFound(e.to_string()))?;
            
        let relative_path = path.strip_prefix(&self.root_dir)
            .map_err(|_| StorageError::InvalidPath(path.to_path_buf()))?
            .to_string_lossy()
            .to_string();
            
        // Try to determine content type
        let content_type = mime_guess::from_path(path)
            .first_raw()
            .or_else(|| orig_content_type)
            .map(|s| s.to_string());
            
        // Create a URL (in this case, it's just a local path)
        let url = self.get_url(&relative_path).await?;
            
        Ok(FileMetadata {
            path: relative_path,
            size: metadata.len(),
            content_type,
            modified: metadata.modified()
                .map(|t| t.into())
                .unwrap_or_else(|_| Utc::now()),
            url,
        })
    }
}

#[async_trait]
impl Storage for FileSystemStorage {
    async fn store_file_from_bytes(&self, path: &str, filename: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<FileMetadata, StorageError> {
        // If the path is empty or ends with /, generate a sharded path
        let (file_path, _) = if path.is_empty() || path.ends_with('/') {
            // Extract the base directory from the provided path
            let base_dir = if path.is_empty() { "uploads" } else { path.trim_end_matches('/') };
            
            // Generate a random file name with sharding
            let full_path = format!("{}/{}", base_dir, self.generate_sharded_path(filename));
            
            // Resolve the actual paths
            let file_path = self.resolve_path(&full_path)?;
            
            // Ensure parent directories exist
            self.ensure_parent_dir(&file_path).await?;
            
            (file_path, full_path)
        } else {
            // Use the provided path directly
            let file_path = self.resolve_path(path)?;
            self.ensure_parent_dir(&file_path).await?;
            (file_path, path.to_string())
        };
        
        // Write the file data
        let mut file = File::create(&file_path)
            .await
            .map_err(|e| StorageError::Io(e))?;
            
        file.write_all(&data)
            .await
            .map_err(|e| StorageError::Io(e))?;
        
        file.sync_all().await?;
            
        self.get_file_metadata(&file_path, content_type).await
    }
    
    async fn store_file_from_path(&self, source_path: &Path, dest_path: &str, content_type: Option<&str>) -> Result<FileMetadata, StorageError> {
        // If dest_path is empty or ends with /, generate a sharded path
        let (dest_file_path, _) = if dest_path.is_empty() || dest_path.ends_with('/') {
            // Extract the base directory from the provided path
            let base_dir = if dest_path.is_empty() { "uploads" } else { dest_path.trim_end_matches('/') };
            
            // Get the original file name from the source path
            let file_name = source_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file");
                
            // Generate a random file name with sharding
            let full_path = format!("{}/{}", base_dir, self.generate_sharded_path(file_name));
            
            // Resolve the actual paths
            let dest_file_path = self.resolve_path(&full_path)?;
            
            // Ensure parent directories exist
            self.ensure_parent_dir(&dest_file_path).await?;
            
            (dest_file_path, full_path)
        } else {
            // Use the provided path directly
            let dest_file_path = self.resolve_path(dest_path)?;
            self.ensure_parent_dir(&dest_file_path).await?;
            (dest_file_path, dest_path.to_string())
        };
        
        // Copy the file
        fs::copy(source_path, &dest_file_path)
            .await
            .map_err(|e| StorageError::Io(e))?;
            
        self.get_file_metadata(&dest_file_path, content_type).await
    }
    
    async fn get_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let file_path = self.resolve_path(path)?;
        
        fs::read(&file_path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => StorageError::NotFound(path.to_string()),
                std::io::ErrorKind::PermissionDenied => StorageError::PermissionDenied(path.to_string()),
                _ => StorageError::Io(e),
            })
    }
    
    async fn get_metadata(&self, path: &str) -> Result<FileMetadata, StorageError> {
        let file_path = self.resolve_path(path)?;
        self.get_file_metadata(&file_path, None).await
    }
    
    async fn delete_file(&self, path: &str) -> Result<(), StorageError> {
        let file_path = self.resolve_path(path)?;
        
        fs::remove_file(&file_path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => StorageError::NotFound(path.to_string()),
                std::io::ErrorKind::PermissionDenied => StorageError::PermissionDenied(path.to_string()),
                _ => StorageError::Io(e),
            })
    }
    
    async fn file_exists(&self, path: &str) -> Result<bool, StorageError> {
        let file_path = self.resolve_path(path)?;
        Ok(file_path.exists())
    }
    
    async fn get_url(&self, path: &str) -> Result<String, StorageError> {
        Ok(format!("{}/api/files/{}", self.serve_url.clone(), path))
    }
}

#[cfg(test)]
mod tests {
    use super::FileSystemStorage;
    use crate::storage::Storage;

    #[tokio::test]
    async fn store_and_retrieve_file_from_bytes() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let storage = FileSystemStorage::with_sharding(
            temp_dir.path(),
            2,
            2,
            "http://example.com".to_string(),
        );

        let data = b"hello world".to_vec();
        let metadata = storage
            .store_file_from_bytes("", "test.txt", data.clone(), Some("text/plain"))
            .await
            .expect("store_file_from_bytes");

        assert!(metadata.path.starts_with("uploads/"));
        assert_eq!(metadata.size, data.len() as u64);
        assert!(metadata.url.contains("/api/files/"));

        let stored = storage
            .get_file(&metadata.path)
            .await
            .expect("get_file");
        assert_eq!(stored, data);

        let exists = storage
            .file_exists(&metadata.path)
            .await
            .expect("file_exists");
        assert!(exists);

        storage
            .delete_file(&metadata.path)
            .await
            .expect("delete_file");

        let exists = storage
            .file_exists(&metadata.path)
            .await
            .expect("file_exists");
        assert!(!exists);
    }
}
