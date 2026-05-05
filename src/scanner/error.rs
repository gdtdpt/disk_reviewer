use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScanError {
    #[error("Access denied: {path}")]
    AccessDenied { path: PathBuf },

    #[error("Path not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Win32 error: {0}")]
    Win32(u32),

    #[error("IO error: {0}")]
    Io(Arc<std::io::Error>),
}

impl From<std::io::Error> for ScanError {
    fn from(err: std::io::Error) -> Self {
        ScanError::Io(Arc::new(err))
    }
}
