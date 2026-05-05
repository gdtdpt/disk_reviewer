use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("Access denied: {path}")]
    AccessDenied { path: PathBuf },

    #[error("Path not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Win32 error: {0}")]
    Win32(u32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
