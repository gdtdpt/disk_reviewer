use crate::scanner::DirNode;

/// Serialize a DirNode tree to a JSON string.
pub fn serialize_tree(node: &DirNode) -> Result<String, serde_json::Error> {
    serde_json::to_string(node)
}

/// Deserialize a JSON string back to a DirNode tree.
pub fn deserialize_tree(json: &str) -> Result<DirNode, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialize a single Entry to JSON.
pub fn serialize_subtree(entry: &crate::scanner::Entry) -> Result<String, serde_json::Error> {
    serde_json::to_string(entry)
}

/// Deserialize a single Entry from JSON.
pub fn deserialize_subtree(json: &str) -> Result<crate::scanner::Entry, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{FileEntry, OthersEntry};
    use crate::treemap::color::FileCategory;
    use std::path::PathBuf;

    fn build_3level_tree() -> DirNode {
        // Level 3: deepest
        let leaf = DirNode {
            path: PathBuf::from(r"C:\a\b\c"),
            name: "c".to_string(),
            total_size: 100,
            file_count: 1,
            children: vec![
                Entry::File(FileEntry { name: "deep.txt".to_string(), size: 100 }),
            ],
            access_denied: false,
            dominant_cat: FileCategory::Document,
        };
        // Level 2
        let mid = DirNode {
            path: PathBuf::from(r"C:\a\b"),
            name: "b".to_string(),
            total_size: 300,
            file_count: 2,
            children: vec![
                Entry::Dir(leaf),
                Entry::File(FileEntry { name: "mid.exe".to_string(), size: 200 }),
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        };
        // Level 1: root
        DirNode {
            path: PathBuf::from(r"C:\a"),
            name: "a".to_string(),
            total_size: 600,
            file_count: 3,
            children: vec![
                Entry::Dir(mid),
                Entry::File(FileEntry { name: "root.zip".to_string(), size: 300 }),
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        }
    }

    use crate::scanner::Entry;

    #[test]
    fn test_serialize_tree_contains_path() {
        let root = build_3level_tree();
        let json = serialize_tree(&root).expect("serialize should succeed");
        assert!(json.contains(r#""path":"C:\\a""#), "JSON should contain root path");
        assert!(json.contains(r#""name":"a""#), "JSON should contain root name");
    }

    #[test]
    fn test_roundtrip_basic() {
        let root = build_3level_tree();
        let json = serialize_tree(&root).expect("serialize");
        let parsed = deserialize_tree(&json).expect("deserialize");
        assert_eq!(parsed.path, root.path);
        assert_eq!(parsed.name, root.name);
        assert_eq!(parsed.total_size, root.total_size);
        assert_eq!(parsed.children.len(), root.children.len());
    }

    #[test]
    fn test_roundtrip_3levels() {
        let root = build_3level_tree();
        let json = serialize_tree(&root).expect("serialize");
        let parsed = deserialize_tree(&json).expect("deserialize");

        // Level 1
        assert_eq!(parsed.name, "a");
        assert_eq!(parsed.children.len(), 2);

        // Level 2
        let mid = match &parsed.children[0] {
            Entry::Dir(d) => d,
            _ => panic!("expected Dir"),
        };
        assert_eq!(mid.name, "b");

        // Level 3
        let leaf = match &mid.children[0] {
            Entry::Dir(d) => d,
            _ => panic!("expected Dir"),
        };
        assert_eq!(leaf.name, "c");
        assert_eq!(leaf.total_size, 100);
        match &leaf.children[0] {
            Entry::File(f) => assert_eq!(f.name, "deep.txt"),
            _ => panic!("expected File"),
        }
    }

    #[test]
    fn test_roundtrip_others_entry() {
        let root = DirNode {
            path: PathBuf::from(r"C:\agg"),
            name: "agg".to_string(),
            total_size: 500,
            file_count: 10,
            children: vec![
                Entry::Others(OthersEntry {
                    name: "Others".to_string(),
                    size: 500,
                    entry_count: 10,
                    entries: vec![
                        Entry::File(FileEntry { name: "x.tmp".to_string(), size: 250 }),
                        Entry::File(FileEntry { name: "y.tmp".to_string(), size: 250 }),
                    ],
                }),
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        };
        let json = serialize_tree(&root).expect("serialize");
        let parsed = deserialize_tree(&json).expect("deserialize");
        match &parsed.children[0] {
            Entry::Others(o) => {
                assert_eq!(o.name, "Others");
                assert_eq!(o.size, 500);
                assert_eq!(o.entry_count, 10);
                assert_eq!(o.entries.len(), 2);
            }
            _ => panic!("expected Others"),
        }
    }

    #[test]
    fn test_roundtrip_access_denied() {
        let root = DirNode {
            path: PathBuf::from(r"C:\restricted"),
            name: "restricted".to_string(),
            total_size: 0,
            file_count: 0,
            children: vec![
                Entry::AccessDenied { path: PathBuf::from(r"C:\restricted\secret") },
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        };
        let json = serialize_tree(&root).expect("serialize");
        let parsed = deserialize_tree(&json).expect("deserialize");
        match &parsed.children[0] {
            Entry::AccessDenied { path } => {
                assert_eq!(*path, PathBuf::from(r"C:\restricted\secret"));
            }
            _ => panic!("expected AccessDenied"),
        }
    }

    #[test]
    fn test_roundtrip_symlink() {
        let root = DirNode {
            path: PathBuf::from(r"C:\links"),
            name: "links".to_string(),
            total_size: 0,
            file_count: 0,
            children: vec![
                Entry::Symlink(PathBuf::from(r"C:\links\shortcut")),
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        };
        let json = serialize_tree(&root).expect("serialize");
        let parsed = deserialize_tree(&json).expect("deserialize");
        assert_eq!(parsed.children[0], Entry::Symlink(PathBuf::from(r"C:\links\shortcut")));
    }

    #[test]
    fn test_roundtrip_100_children() {
        let mut children = Vec::new();
        for i in 0..100 {
            children.push(Entry::File(FileEntry {
                name: format!("file_{}.txt", i),
                size: (i as u64 + 1) * 10,
            }));
        }
        let total_size: u64 = children.iter().map(|e| e.size()).sum();
        let root = DirNode {
            path: PathBuf::from(r"C:\bulk"),
            name: "bulk".to_string(),
            total_size,
            file_count: 100,
            children,
            access_denied: false,
            dominant_cat: FileCategory::Other,
        };
        let json = serialize_tree(&root).expect("serialize 100 children");
        let parsed = deserialize_tree(&json).expect("deserialize 100 children");
        assert_eq!(parsed.children.len(), 100);
        assert_eq!(parsed.total_size, total_size);
    }
}
