use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{GetLastError, ERROR_NO_MORE_FILES};
use windows::Win32::Storage::FileSystem::{
    FindClose, FindFirstFileExW, FindNextFileW,
    FindExInfoBasic, FindExSearchNameMatch, FIND_FIRST_EX_LARGE_FETCH,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    WIN32_FIND_DATAW,
};
use windows::core::PCWSTR;

use crate::scanner::error::ScanError;
use crate::scanner::types::{DirNode, Entry, FileEntry};

/// 将路径转换为 \\?\ 前缀的扩展长度 UTF-16 向量（含 null 终止符）
fn to_extended_path(path: &Path) -> Vec<u16> {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path_str = abs.as_os_str();
    // canonicalize() 在 Windows 上可能已返回 \\?\ 前缀路径，避免重复添加
    let already_extended = path_str.to_string_lossy().starts_with(r"\\?\");
    if already_extended {
        path_str.encode_wide().chain(std::iter::once(0)).collect()
    } else {
        OsString::from(r"\\?\")
            .encode_wide()
            .chain(abs.as_os_str().encode_wide())
            .chain(std::iter::once(0))
            .collect()
    }
}

/// 从 WIN32_FIND_DATAW 的 cFileName 提取 Rust String
fn find_data_to_string(data: &WIN32_FIND_DATAW) -> String {
    let name_slice = &data.cFileName;
    let len = name_slice.iter().position(|&c| c == 0).unwrap_or(name_slice.len());
    if len == 0 {
        return String::new();
    }
    OsString::from_wide(&name_slice[..len])
        .to_string_lossy()
        .into_owned()
}

/// 从 WIN32_FIND_DATAW 提取文件大小
fn file_size_from_find_data(data: &WIN32_FIND_DATAW) -> u64 {
    ((data.nFileSizeHigh as u64) << 32) | (data.nFileSizeLow as u64)
}

/// 扫描单个目录，返回 DirNode（含直接子文件和子目录的 DirNode）
///
/// 使用 rayon::scope() 实现并行目录遍历（D-01）：
/// - 每个子目录作为一个 rayon 任务提交，工作窃取调度器自动负载均衡
/// - 通过 crossbeam channel 将子目录结果传回主扫描线程
pub fn scan_directory(path: &Path) -> Result<DirNode, ScanError> {
    let mut node = DirNode {
        path: path.to_path_buf(),
        name: path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned()),
        total_size: 0,
        file_count: 0,
        children: Vec::new(),
        access_denied: false,
        dominant_cat: crate::treemap::color::FileCategory::Other,
    };

    // 构造搜索路径: \\?\C:\target\*
    let mut search_path = to_extended_path(path);
    let star: Vec<u16> = OsString::from(r"\*")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    if search_path.last() == Some(&0) {
        search_path.pop();
    }
    search_path.extend(star);

    let mut find_data = WIN32_FIND_DATAW::default();

    let handle = unsafe {
        FindFirstFileExW(
            PCWSTR(search_path.as_ptr()),
            FindExInfoBasic,
            &mut find_data as *mut _ as *mut _,
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        )
    };

    let handle = match handle {
        Ok(h) => h,
        Err(_) => {
            let err = unsafe { GetLastError() };
            if err.0 == 5 {
                // D-04: 返回 AccessDenied 错误，由调用方记录为 Entry::AccessDenied
                return Err(ScanError::AccessDenied { path: path.to_path_buf() });
            }
            return Err(ScanError::Win32(err.0));
        }
    };

    let mut total_size: u64 = 0;
    let mut file_count: u64 = 0;
    let mut subdirs: Vec<PathBuf> = Vec::new();

    loop {
        let name = find_data_to_string(&find_data);

        if name.is_empty() || name == "." || name == ".." {
            // 跳过
        } else {
            let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;
            let is_reparse = (find_data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0) != 0;

            if is_dir {
                let full_path = path.join(&name);
                if is_reparse {
                    // D-03: 不跟随符号链接/junction，标记为 Symlink
                    node.children.push(Entry::Symlink(full_path));
                } else {
                    subdirs.push(full_path);
                }
            } else {
                let size = file_size_from_find_data(&find_data);
                node.children.push(Entry::File(FileEntry {
                    name: name.clone(),
                    size,
                }));
                total_size += size;
                file_count += 1;
            }
        }

        let success = unsafe { FindNextFileW(handle, &mut find_data) };
        if let Err(_) = success {
            let err = unsafe { GetLastError() };
            if err == ERROR_NO_MORE_FILES {
                break;
            }
            // D-05: 其他错误（如文件被删除），跳过
        }
    }

    unsafe { FindClose(handle).ok() };

    // D-01: 使用 rayon::scope() 并行扫描子目录
    if !subdirs.is_empty() {
        let (tx, rx) = crossbeam_channel::bounded(subdirs.len());

        rayon::scope(|s| {
            for subdir_path in subdirs {
                let tx = tx.clone();
                s.spawn(move |_s| {
                    let result = scan_directory(&subdir_path);
                    tx.send((subdir_path, result)).ok();
                });
            }
        });
        drop(tx); // 关闭发送端，确保迭代器能结束

        // 收集 rayon 并行扫描结果
        for (subdir_path, result) in rx {
            match result {
                Ok(child_node) => {
                    total_size += child_node.total_size;
                    file_count += child_node.file_count;
                    node.children.push(Entry::Dir(child_node));
                }
                Err(ScanError::AccessDenied { .. }) => {
                    // D-04: 记录无权限目录，不中断
                    node.children.push(Entry::AccessDenied { path: subdir_path });
                }
                Err(_) => {
                    // D-05: 其他错误跳过
                }
            }
        }
    }

    node.total_size = total_size;
    node.file_count = file_count;
    Ok(node)
}

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
