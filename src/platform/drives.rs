#[derive(Debug, Clone)]
pub struct DriveInfo {
    pub letter: char,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
}

pub fn enumerate_drives() -> Vec<DriveInfo> {
    let bitmask = unsafe { windows::Win32::Storage::FileSystem::GetLogicalDrives() };
    let mut drives = Vec::new();
    for i in 0..26 {
        if bitmask & (1 << i) != 0 {
            let letter = (b'A' + i as u8) as char;
            let path = format!(r"{}:\", letter);
            let mut total: u64 = 0;
            let mut free: u64 = 0;
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let result = unsafe {
                windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                    windows::core::PCWSTR(wide.as_ptr()),
                    None,
                    Some(&mut total),
                    Some(&mut free),
                )
            };
            if result.is_ok() {
                drives.push(DriveInfo {
                    letter,
                    total_bytes: total,
                    free_bytes: free,
                    used_bytes: total - free,
                });
            }
        }
    }
    drives
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_drives_not_empty() {
        let drives = enumerate_drives();
        assert!(!drives.is_empty(), "应该至少有一个逻辑盘");
    }

    #[test]
    fn test_enumerate_drives_total_size_positive() {
        let drives = enumerate_drives();
        for drive in &drives {
            assert!(
                drive.total_bytes > 0,
                "盘 {} 的总空间应该 > 0",
                drive.letter
            );
        }
    }

    #[test]
    fn test_enumerate_drives_used_plus_free_lte_total() {
        let drives = enumerate_drives();
        for drive in &drives {
            assert!(
                drive.used_bytes + drive.free_bytes <= drive.total_bytes,
                "盘 {} 的已用 + 可用应 <= 总计",
                drive.letter
            );
        }
    }

    #[test]
    fn test_enumerate_drives_letter_is_uppercase() {
        let drives = enumerate_drives();
        for drive in &drives {
            assert!(
                drive.letter.is_ascii_uppercase(),
                "盘符应为大写字母, 得到: {}",
                drive.letter
            );
        }
    }

    #[test]
    fn test_enumerate_drives_has_c_drive() {
        let drives = enumerate_drives();
        assert!(
            drives.iter().any(|d| d.letter == 'C'),
            "Windows 机器应有 C: 盘"
        );
    }
}
