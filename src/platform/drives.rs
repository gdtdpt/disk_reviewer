#[derive(Debug, Clone)]
pub struct DriveInfo {
    pub letter: char,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
}

// Phase 1-02 实现此函数
// pub fn enumerate_drives() -> Vec<DriveInfo> {
//     vec![]
// }
