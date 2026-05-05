use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct DirNode {
    pub path: PathBuf,
    pub name: String,
    pub total_size: u64,
    pub file_count: u64,
    pub children: Vec<Entry>,
    pub access_denied: bool,
}

#[derive(Debug, Clone)]
pub enum Entry {
    File(FileEntry),
    Dir(DirNode),
    Others(OthersEntry),
    Symlink(PathBuf),
    AccessDenied { path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct OthersEntry {
    pub name: String,
    pub size: u64,
    pub entry_count: u64,
    pub entries: Vec<Entry>,
}

impl Entry {
    pub fn size(&self) -> u64 {
        match self {
            Entry::File(f) => f.size,
            Entry::Dir(d) => d.total_size,
            Entry::Others(o) => o.size,
            _ => 0,
        }
    }
}
