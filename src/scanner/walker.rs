use std::path::PathBuf;
use crate::scanner::types::DirNode;
use crate::scanner::error::ScanError;

// Phase 1-03 实现此函数
// pub fn scan_directory(path: &std::path::Path) -> Result<DirNode, ScanError> {
//     todo!("Phase 1-03")
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// 创建测试目录结构：
    /// test_root/
    ///   file1.txt (10 bytes)
    ///   file2.txt (20 bytes)
    ///   subdir/
    ///     file3.txt (30 bytes)
    fn create_test_dir() -> PathBuf {
        let test_root = std::env::temp_dir().join("disk_reviewer_test_walk");
        // 清理旧数据
        let _ = fs::remove_dir_all(&test_root);
        fs::create_dir_all(test_root.join("subdir")).unwrap();
        fs::write(test_root.join("file1.txt"), "0123456789").unwrap();       // 10 bytes
        fs::write(test_root.join("file2.txt"), "01234567890123456789").unwrap(); // 20 bytes
        fs::write(test_root.join("subdir").join("file3.txt"), "012345678901234567890123456789").unwrap(); // 30 bytes
        test_root
    }

    #[test]
    fn test_walk_known_directory() {
        let test_dir = create_test_dir();
        let result = scan_directory(&test_dir);
        assert!(result.is_ok(), "扫描应成功: {:?}", result.err());
        let node = result.unwrap();
        assert_eq!(node.path, test_dir);
        // 应有 2 个文件 + 1 个子目录 = 3 个 children
        assert_eq!(node.children.len(), 3, "应有 2 文件 + 1 子目录");
        // 总大小 = 10 + 20 + 30 = 60
        assert_eq!(node.total_size, 60, "总大小应为 60 bytes");
        // 清理
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_walk_empty_directory() {
        let test_root = std::env::temp_dir().join("disk_reviewer_test_empty");
        let _ = fs::remove_dir_all(&test_root);
        fs::create_dir_all(&test_root).unwrap();
        let result = scan_directory(&test_root);
        assert!(result.is_ok(), "空目录扫描应成功");
        let node = result.unwrap();
        assert_eq!(node.children.len(), 0, "空目录应无 children");
        assert_eq!(node.total_size, 0, "空目录总大小为 0");
        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_file_size_accumulation() {
        let test_dir = create_test_dir();
        let result = scan_directory(&test_dir).unwrap();
        // 直接子文件大小: 10 + 20 = 30 (subdir 的 30 在子节点中)
        let direct_file_size: u64 = result.children.iter()
            .filter_map(|e| match e {
                Entry::File(f) => Some(f.size),
                _ => None,
            })
            .sum();
        assert_eq!(direct_file_size, 30, "直接文件大小之和应为 30");
        // 总大小应包含子目录: 10 + 20 + 30 = 60
        assert_eq!(result.total_size, 60, "总大小应包含子目录内容");
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_walk_nonexistent_path() {
        let fake_path = PathBuf::from(r"\\?\C:\this_path_does_not_exist_12345");
        let result = scan_directory(&fake_path);
        assert!(result.is_err(), "不存在的路径应返回错误");
    }
}
