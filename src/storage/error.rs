use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("File not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),
    
    #[error("Storage configuration error: {0}")]
    ConfigError(String),
    
    // Prepare for future implementations
    #[error("Service error: {0}")]
    ServiceError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}