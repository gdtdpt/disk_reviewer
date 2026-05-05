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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 构造一个包含 N 个子条目的 DirNode
    fn make_dir_with_entries(n: usize, base_size: u64) -> DirNode {
        let mut children = Vec::new();
        for i in 0..n {
            children.push(Entry::File(FileEntry {
                name: format!("file_{}", i),
                size: base_size + i as u64,
            }));
        }
        let total_size: u64 = children.iter().map(|e| e.size()).sum();
        DirNode {
            path: PathBuf::from(r"C:\test"),
            name: "test".to_string(),
            total_size,
            file_count: n as u64,
            children,
            access_denied: false,
        }
    }

    #[test]
    fn test_others_aggregation_above_threshold() {
        // 创建 1500 个条目，默认阈值 max_entries=1000
        let mut node = make_dir_with_entries(1500, 100);
        let thresholds = AggThresholds::default();
        node.finish(&thresholds);

        // 聚合后应包含 Others 条目
        let has_others = node.children.iter().any(|e| matches!(e, Entry::Others(_)));
        assert!(has_others, "超过阈值后应产生 Others 条目");

        // 聚合后条目数应明显减少
        assert!(node.children.len() < 1500, "聚合后条目数应减少");
    }

    #[test]
    fn test_others_size_correct() {
        let mut node = make_dir_with_entries(1500, 100);
        let original_total = node.total_size;
        let thresholds = AggThresholds::default();
        node.finish(&thresholds);

        // Others 条目的 size 应 > 0
        let others_size: u64 = node.children.iter()
            .filter_map(|e| match e {
                Entry::Others(o) => Some(o.size),
                _ => None,
            })
            .sum();
        assert!(others_size > 0, "Others 大小应 > 0");

        // finish() 后 total_size 不被修改
        assert_eq!(node.total_size, original_total, "finish() 不应修改 total_size");
    }

    #[test]
    fn test_no_aggregation_below_threshold() {
        // 创建 500 个条目，低于默认阈值 1000
        let mut node = make_dir_with_entries(500, 1000);
        let original_count = node.children.len();
        let thresholds = AggThresholds::default();
        node.finish(&thresholds);

        // 不应有 Others 条目
        let has_others = node.children.iter().any(|e| matches!(e, Entry::Others(_)));
        assert!(!has_others, "未超过阈值时不应产生 Others");

        // 条目数不变
        assert_eq!(node.children.len(), original_count, "未超过阈值时条目数不变");
    }

    #[test]
    fn test_others_entry_count() {
        let mut node = make_dir_with_entries(1500, 100);
        let thresholds = AggThresholds::default();
        node.finish(&thresholds);

        // 找到 Others 条目，其 entry_count 应 > 0
        for child in &node.children {
            if let Entry::Others(o) = child {
                assert!(o.entry_count > 0, "Others 的 entry_count 应 > 0");
                assert_eq!(o.entries.len() as u64, o.entry_count,
                    "entries.len() 应等于 entry_count");
                // Others 内条目大小之和应等于 Others.size
                let inner_size: u64 = o.entries.iter().map(|e| e.size()).sum();
                assert_eq!(inner_size, o.size, "Others 内部条目大小之和应等于 Others.size");
                return;
            }
        }
        panic!("1500 条目、默认阈值下应产生 Others 条目");
    }

    #[test]
    fn test_access_denied_entry_size_is_zero() {
        let entry = Entry::AccessDenied { path: PathBuf::from(r"C:\System") };
        assert_eq!(entry.size(), 0, "AccessDenied 条目 size 应为 0");
    }

    #[test]
    fn test_symlink_entry_size_is_zero() {
        let entry = Entry::Symlink(PathBuf::from(r"C:\link"));
        assert_eq!(entry.size(), 0, "Symlink 条目 size 应为 0");
    }
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    DirEntry {
        path: PathBuf,
        size: u64,
        file_count: u64,
    },
    Progress {
        files_scanned: u64,
        bytes_scanned: u64,
        current_path: PathBuf,
    },
    AccessDenied {
        path: PathBuf,
    },
    Error {
        path: PathBuf,
        error: crate::scanner::error::ScanError,
    },
    Complete {
        root: DirNode,
        duration: std::time::Duration,
        total_files: u64,
        access_denied_count: u64,
    },
}
