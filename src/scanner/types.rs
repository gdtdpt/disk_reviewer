use std::path::PathBuf;
use crate::treemap::color::FileCategory;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DirNode {

    pub path: PathBuf,
    pub name: String,
    pub total_size: u64,
    pub file_count: u64,
    pub children: Vec<Entry>,
    pub access_denied: bool,
    pub dominant_cat: FileCategory,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Entry {
    File(FileEntry),
    Dir(DirNode),
    Others(OthersEntry),
    Symlink(PathBuf),
    AccessDenied { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// Others 聚合阈值配置（SCAN-05）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggThresholds {
    pub max_entries: usize,
    pub top_n: usize,
    pub min_relative_size: f64,
}

impl Default for AggThresholds {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            top_n: 500,
            min_relative_size: 0.001,
        }
    }
}

impl DirNode {
    /// 后处理：递归地对超过阈值的子目录执行 Others 聚合（SCAN-05）
    pub fn finish(&mut self, thresholds: &AggThresholds) {
        // 先递归处理子目录
        for child in &mut self.children {
            if let Entry::Dir(ref mut dir) = child {
                dir.finish(thresholds);
            }
        }

        // 预计算主导颜色（缓存，避免 layout_treemap 时重复递归）
        self.dominant_cat = crate::treemap::color::compute_dominant(self);

        // 如果未超过阈值，不需要聚合
        if self.children.len() <= thresholds.max_entries {
            return;
        }

        // 按 size 降序排序
        self.children.sort_by_key(|e| std::cmp::Reverse(e.size()));

        // 保留 top_n 个
        if self.children.len() > thresholds.top_n {
            let rest = self.children.split_off(thresholds.top_n);
            let min_size = (thresholds.min_relative_size * self.total_size as f64) as u64;
            let mut significant = Vec::new();
            let mut insignificant = Vec::new();
            for entry in rest {
                if entry.size() >= min_size {
                    significant.push(entry);
                } else {
                    insignificant.push(entry);
                }
            }
            self.children.extend(significant);
            if !insignificant.is_empty() {
                let others_size: u64 = insignificant.iter().map(|e| e.size()).sum();
                self.children.push(Entry::Others(OthersEntry {
                    name: "Others".to_string(),
                    size: others_size,
                    entry_count: insignificant.len() as u64,
                    entries: insignificant,
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};
    use std::path::PathBuf;

    #[test]
    fn test_serde_roundtrip_dirnode_mixed_children() {
        let dir = DirNode {
            path: PathBuf::from(r"C:\test"),
            name: "test".to_string(),
            total_size: 350,
            file_count: 3,
            children: vec![
                Entry::File(FileEntry { name: "readme.txt".to_string(), size: 100 }),
                Entry::Dir(DirNode {
                    path: PathBuf::from(r"C:\test\sub"),
                    name: "sub".to_string(),
                    total_size: 200,
                    file_count: 2,
                    children: vec![
                        Entry::File(FileEntry { name: "a.exe".to_string(), size: 200 }),
                    ],
                    access_denied: false,
                    dominant_cat: FileCategory::Other,
                }),
                Entry::Others(OthersEntry {
                    name: "Others".to_string(),
                    size: 50,
                    entry_count: 5,
                    entries: vec![
                        Entry::File(FileEntry { name: "tiny1.tmp".to_string(), size: 25 }),
                        Entry::File(FileEntry { name: "tiny2.tmp".to_string(), size: 25 }),
                    ],
                }),
                Entry::Symlink(PathBuf::from(r"C:\test\link")),
                Entry::AccessDenied { path: PathBuf::from(r"C:\test\secret") },
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        };

        let json = serde_json::to_string(&dir).expect("serialize DirNode");
        let parsed: DirNode = serde_json::from_str(&json).expect("deserialize DirNode");
        assert_eq!(parsed.path, dir.path);
        assert_eq!(parsed.name, dir.name);
        assert_eq!(parsed.total_size, dir.total_size);
        assert_eq!(parsed.children.len(), 5);
    }

    #[test]
    fn test_serde_entry_dir_roundtrip() {
        let entry = Entry::Dir(DirNode {
            path: PathBuf::from(r"C:\parent\child"),
            name: "child".to_string(),
            total_size: 500,
            file_count: 10,
            children: vec![
                Entry::File(FileEntry { name: "data.bin".to_string(), size: 500 }),
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        });
        let json = serde_json::to_string(&entry).expect("serialize Entry::Dir");
        let parsed: Entry = serde_json::from_str(&json).expect("deserialize Entry::Dir");
        match &parsed {
            Entry::Dir(d) => {
                assert_eq!(d.name, "child");
                assert_eq!(d.total_size, 500);
                assert_eq!(d.children.len(), 1);
            }
            _ => panic!("Expected Entry::Dir"),
        }
    }

    #[test]
    fn test_serde_entry_others_roundtrip() {
        let entry = Entry::Others(OthersEntry {
            name: "Others".to_string(),
            size: 1000,
            entry_count: 50,
            entries: vec![
                Entry::File(FileEntry { name: "a.tmp".to_string(), size: 500 }),
                Entry::File(FileEntry { name: "b.tmp".to_string(), size: 500 }),
            ],
        });
        let json = serde_json::to_string(&entry).expect("serialize Entry::Others");
        let parsed: Entry = serde_json::from_str(&json).expect("deserialize Entry::Others");
        match &parsed {
            Entry::Others(o) => {
                assert_eq!(o.name, "Others");
                assert_eq!(o.size, 1000);
                assert_eq!(o.entry_count, 50);
                assert_eq!(o.entries.len(), 2);
            }
            _ => panic!("Expected Entry::Others"),
        }
    }

    #[test]
    fn test_serde_entry_access_denied_roundtrip() {
        let entry = Entry::AccessDenied { path: PathBuf::from(r"C:\secret") };
        let json = serde_json::to_string(&entry).expect("serialize Entry::AccessDenied");
        let parsed: Entry = serde_json::from_str(&json).expect("deserialize Entry::AccessDenied");
        match &parsed {
            Entry::AccessDenied { path } => {
                assert_eq!(*path, PathBuf::from(r"C:\secret"));
            }
            _ => panic!("Expected Entry::AccessDenied"),
        }
    }

    #[test]
    fn test_serde_file_category_roundtrip() {
        for cat in [
            FileCategory::Document,
            FileCategory::Image,
            FileCategory::Video,
            FileCategory::Audio,
            FileCategory::Archive,
            FileCategory::Code,
            FileCategory::Executable,
            FileCategory::System,
            FileCategory::Temp,
            FileCategory::Other,
        ] {
            let json = serde_json::to_string(&cat).expect("serialize FileCategory");
            let parsed: FileCategory = serde_json::from_str(&json).expect("deserialize FileCategory");
            assert_eq!(parsed, cat, "FileCategory round-trip failed");
        }
    }

    #[test]
    fn test_serde_symlink_roundtrip() {
        let entry = Entry::Symlink(PathBuf::from(r"C:\Users\link_target"));
        let json = serde_json::to_string(&entry).expect("serialize Entry::Symlink");
        let parsed: Entry = serde_json::from_str(&json).expect("deserialize Entry::Symlink");
        assert_eq!(parsed, entry);
    }

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
            dominant_cat: crate::treemap::color::FileCategory::Other,
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
