use crate::scanner::{DirNode, Entry, FileEntry};
use crate::treemap::color::FileCategory;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Unchanged,
    Added,
    Removed,
    Grown,
    Shrunk,
}

#[derive(Debug, Clone)]
pub struct DiffNode {
    pub entry: Entry,
    pub change: ChangeType,
    pub old_size: Option<u64>,
    pub new_size: u64,
    /// Index of this entry within the parent DirNode's children vec.
    /// Used for O(1) mapping from TreemapNode to DiffNode.
    pub child_index: Option<usize>,
}

/// Extract the name for matching (D-19: match by name within same level).
/// Delegates to `Entry::name()` to avoid duplication.
pub fn entry_name(entry: &Entry) -> String {
    entry.name()
}

/// Diff two DirNode trees at one level.
/// Matches entries by name (D-19). O(n + m) per level.
///
/// Each returned `DiffNode` carries a `child_index` recording the entry's position
/// within `old.children` (`Some(idx)`) for entries present in the old tree, or `None`
/// for entries that only exist in the new tree (Added). This enables correct
/// positional mapping to treemap node `entry_index` without name-based lookups.
pub fn diff_level(old: &DirNode, new: &DirNode) -> Vec<DiffNode> {
    // Build name -> index map for old children (first match wins for duplicate names)
    let old_map: HashMap<String, usize> = old
        .children
        .iter()
        .enumerate()
        .map(|(idx, e)| (entry_name(e), idx))
        .collect();
    let new_map: HashMap<String, usize> = new
        .children
        .iter()
        .enumerate()
        .map(|(idx, e)| (entry_name(e), idx))
        .collect();

    let mut result = Vec::new();

    // Entries in new (scan) tree
    for (_new_idx, new_entry) in new.children.iter().enumerate() {
        let name = entry_name(new_entry);
        match old_map.get(&name) {
            None => {
                result.push(DiffNode {
                    entry: new_entry.clone(),
                    change: ChangeType::Added,
                    old_size: None,
                    new_size: new_entry.size(),
                    child_index: None, // not in old tree, no index in old children
                });
            }
            Some(&old_idx) => {
                let old_entry = &old.children[old_idx];
                let old_size = old_entry.size();
                let new_size = new_entry.size();
                let change = if new_size > old_size {
                    ChangeType::Grown
                } else if new_size < old_size {
                    ChangeType::Shrunk
                } else {
                    ChangeType::Unchanged
                };
                result.push(DiffNode {
                    entry: new_entry.clone(),
                    change,
                    old_size: Some(old_size),
                    new_size,
                    child_index: Some(old_idx),
                });
            }
        }
    }

    // Entries only in old tree (removed)
    for (old_idx, old_entry) in old.children.iter().enumerate() {
        let name = entry_name(old_entry);
        if !new_map.contains_key(&name) {
            result.push(DiffNode {
                entry: old_entry.clone(),
                change: ChangeType::Removed,
                old_size: Some(old_entry.size()),
                new_size: 0,
                child_index: Some(old_idx),
            });
        }
    }

    result
}

/// Diff two DirNode subtrees recursively.
/// Returns DiffNode vec for the current level.
pub fn diff_trees_recursive(old: &DirNode, new: &DirNode) -> Vec<DiffNode> {
    diff_level(old, new)
}

// ── Helper for tests ──────────────────────────────────────────

fn make_test_dir(name: &str, children: Vec<Entry>) -> DirNode {
    let total_size = children.iter().map(|e| e.size()).sum();
    let file_count = children.len() as u64;
    DirNode {
        path: PathBuf::from(format!(r"C:\{}", name)),
        name: name.to_string(),
        total_size,
        file_count,
        children,
        access_denied: false,
        dominant_cat: FileCategory::Other,
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 1: diff_level with identical trees returns all Unchanged
    #[test]
    fn test_identical_trees_all_unchanged() {
        let children = vec![
            Entry::File(FileEntry {
                name: "a.txt".to_string(),
                size: 100,
            }),
            Entry::File(FileEntry {
                name: "b.txt".to_string(),
                size: 200,
            }),
        ];
        let old = make_test_dir("root", children.clone());
        let new = make_test_dir("root", children);

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 2);
        for d in &diff {
            assert_eq!(d.change, ChangeType::Unchanged);
            assert!(d.old_size.is_some());
            assert_eq!(d.old_size.unwrap(), d.new_size);
        }
    }

    /// Test 2: diff_level where new has extra entry -> Added
    #[test]
    fn test_new_has_extra_entry_added() {
        let old = make_test_dir(
            "root",
            vec![Entry::File(FileEntry {
                name: "a.txt".to_string(),
                size: 100,
            })],
        );
        let new = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 100,
                }),
                Entry::File(FileEntry {
                    name: "new_file.txt".to_string(),
                    size: 50,
                }),
            ],
        );

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 2);

        let added: Vec<_> = diff
            .iter()
            .filter(|d| d.change == ChangeType::Added)
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].old_size, None);
        assert_eq!(added[0].new_size, 50);
    }

    /// Test 3: diff_level where old has extra entry -> Removed
    #[test]
    fn test_old_has_extra_entry_removed() {
        let old = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 100,
                }),
                Entry::File(FileEntry {
                    name: "deleted.txt".to_string(),
                    size: 75,
                }),
            ],
        );
        let new = make_test_dir(
            "root",
            vec![Entry::File(FileEntry {
                name: "a.txt".to_string(),
                size: 100,
            })],
        );

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 2);

        let removed: Vec<_> = diff
            .iter()
            .filter(|d| d.change == ChangeType::Removed)
            .collect();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].old_size, Some(75));
        assert_eq!(removed[0].new_size, 0);
    }

    /// Test 4: diff_level where entry size increased -> Grown
    #[test]
    fn test_entry_size_increased_grown() {
        let old = make_test_dir(
            "root",
            vec![Entry::File(FileEntry {
                name: "a.txt".to_string(),
                size: 100,
            })],
        );
        let new = make_test_dir(
            "root",
            vec![Entry::File(FileEntry {
                name: "a.txt".to_string(),
                size: 200,
            })],
        );

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].change, ChangeType::Grown);
        assert_eq!(diff[0].old_size, Some(100));
        assert_eq!(diff[0].new_size, 200);
    }

    /// Test 5: diff_level where entry size decreased -> Shrunk
    #[test]
    fn test_entry_size_decreased_shrunk() {
        let old = make_test_dir(
            "root",
            vec![Entry::File(FileEntry {
                name: "a.txt".to_string(),
                size: 300,
            })],
        );
        let new = make_test_dir(
            "root",
            vec![Entry::File(FileEntry {
                name: "a.txt".to_string(),
                size: 150,
            })],
        );

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].change, ChangeType::Shrunk);
        assert_eq!(diff[0].old_size, Some(300));
        assert_eq!(diff[0].new_size, 150);
    }

    /// Test 6: DiffNode count equals union of old + new entry count
    #[test]
    fn test_diff_node_count_equals_union() {
        let old = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 100,
                }),
                Entry::File(FileEntry {
                    name: "b.txt".to_string(),
                    size: 200,
                }),
                Entry::File(FileEntry {
                    name: "only_old.txt".to_string(),
                    size: 50,
                }),
            ],
        );
        let new = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 100,
                }),
                Entry::File(FileEntry {
                    name: "b.txt".to_string(),
                    size: 200,
                }),
                Entry::File(FileEntry {
                    name: "only_new.txt".to_string(),
                    size: 80,
                }),
            ],
        );
        // Union: a.txt, b.txt, only_old.txt, only_new.txt = 4
        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 4);
    }

    /// Test 7: diff_level with empty old tree -> all Added
    #[test]
    fn test_empty_old_all_added() {
        let old = make_test_dir("root", vec![]);
        let new = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 100,
                }),
                Entry::File(FileEntry {
                    name: "b.txt".to_string(),
                    size: 200,
                }),
            ],
        );

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 2);
        for d in &diff {
            assert_eq!(d.change, ChangeType::Added);
        }
    }

    /// Test 8: diff_level with empty new tree -> all Removed
    #[test]
    fn test_empty_new_all_removed() {
        let old = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 100,
                }),
                Entry::File(FileEntry {
                    name: "b.txt".to_string(),
                    size: 200,
                }),
            ],
        );
        let new = make_test_dir("root", vec![]);

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 2);
        for d in &diff {
            assert_eq!(d.change, ChangeType::Removed);
        }
    }

    /// Test 9: diff_level with both empty -> empty result
    #[test]
    fn test_both_empty() {
        let old = make_test_dir("root", vec![]);
        let new = make_test_dir("root", vec![]);

        let diff = diff_level(&old, &new);
        assert!(diff.is_empty());
    }

    /// Test 10: diff_trees_recursive on nested DirNode trees (2 levels)
    #[test]
    fn test_recursive_nested_dir_diff() {
        let old = make_test_dir(
            "root",
            vec![Entry::Dir(DirNode {
                path: PathBuf::from(r"C:\root\sub"),
                name: "sub".to_string(),
                total_size: 300,
                file_count: 2,
                children: vec![
                    Entry::File(FileEntry {
                        name: "old_file.txt".to_string(),
                        size: 100,
                    }),
                    Entry::File(FileEntry {
                        name: "common.txt".to_string(),
                        size: 200,
                    }),
                ],
                access_denied: false,
                dominant_cat: FileCategory::Other,
            })],
        );
        let new = make_test_dir(
            "root",
            vec![Entry::Dir(DirNode {
                path: PathBuf::from(r"C:\root\sub"),
                name: "sub".to_string(),
                total_size: 400,
                file_count: 2,
                children: vec![
                    Entry::File(FileEntry {
                        name: "new_file.txt".to_string(),
                        size: 150,
                    }),
                    Entry::File(FileEntry {
                        name: "common.txt".to_string(),
                        size: 250,
                    }),
                ],
                access_denied: false,
                dominant_cat: FileCategory::Other,
            })],
        );

        // At root level: "sub" dir matched by name, sizes changed
        let diff = diff_trees_recursive(&old, &new);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].change, ChangeType::Grown);
        assert_eq!(diff[0].old_size, Some(300));
        assert_eq!(diff[0].new_size, 400);

        // At the subdirectory level: diff children
        if let Entry::Dir(old_sub) = &old.children[0] {
            if let Entry::Dir(new_sub) = &new.children[0] {
                let sub_diff = diff_level(old_sub, new_sub);
                assert_eq!(sub_diff.len(), 3); // old_file (removed), new_file (added), common (grown)

                let removed: Vec<_> = sub_diff
                    .iter()
                    .filter(|d| d.change == ChangeType::Removed)
                    .collect();
                assert_eq!(removed.len(), 1);
                assert_eq!(entry_name(&removed[0].entry), "old_file.txt");

                let added: Vec<_> = sub_diff
                    .iter()
                    .filter(|d| d.change == ChangeType::Added)
                    .collect();
                assert_eq!(added.len(), 1);
                assert_eq!(entry_name(&added[0].entry), "new_file.txt");

                let grown: Vec<_> = sub_diff
                    .iter()
                    .filter(|d| d.change == ChangeType::Grown)
                    .collect();
                assert_eq!(grown.len(), 1);
                assert_eq!(grown[0].old_size, Some(200));
                assert_eq!(grown[0].new_size, 250);
            }
        }
    }

    /// Test 11: Name-based matching (D-19): entries matched by name, not position
    #[test]
    fn test_name_based_matching_not_position() {
        // Same entries but in different order
        let old = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "z.txt".to_string(),
                    size: 100,
                }),
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 200,
                }),
            ],
        );
        let new = make_test_dir(
            "root",
            vec![
                Entry::File(FileEntry {
                    name: "a.txt".to_string(),
                    size: 250,
                }),
                Entry::File(FileEntry {
                    name: "z.txt".to_string(),
                    size: 100,
                }),
            ],
        );

        let diff = diff_level(&old, &new);
        // Should identify: a.txt grew (200->250), z.txt unchanged (100->100)
        assert_eq!(diff.len(), 2);

        let a_entry = diff.iter().find(|d| entry_name(&d.entry) == "a.txt").unwrap();
        assert_eq!(a_entry.change, ChangeType::Grown);
        assert_eq!(a_entry.old_size, Some(200));
        assert_eq!(a_entry.new_size, 250);

        let z_entry = diff.iter().find(|d| entry_name(&d.entry) == "z.txt").unwrap();
        assert_eq!(z_entry.change, ChangeType::Unchanged);
    }

    /// Test 12: Dir entries with same name but different children matched and compared by size
    #[test]
    fn test_dir_entries_same_name_different_size() {
        let old = make_test_dir(
            "root",
            vec![Entry::Dir(DirNode {
                path: PathBuf::from(r"C:\root\mydir"),
                name: "mydir".to_string(),
                total_size: 500,
                file_count: 3,
                children: vec![
                    Entry::File(FileEntry {
                        name: "x.txt".to_string(),
                        size: 200,
                    }),
                    Entry::File(FileEntry {
                        name: "y.txt".to_string(),
                        size: 300,
                    }),
                ],
                access_denied: false,
                dominant_cat: FileCategory::Other,
            })],
        );
        let new = make_test_dir(
            "root",
            vec![Entry::Dir(DirNode {
                path: PathBuf::from(r"C:\root\mydir"),
                name: "mydir".to_string(),
                total_size: 800,
                file_count: 4,
                children: vec![
                    Entry::File(FileEntry {
                        name: "x.txt".to_string(),
                        size: 300,
                    }),
                    Entry::File(FileEntry {
                        name: "z.txt".to_string(),
                        size: 500,
                    }),
                ],
                access_denied: false,
                dominant_cat: FileCategory::Other,
            })],
        );

        let diff = diff_level(&old, &new);
        assert_eq!(diff.len(), 1);
        // "mydir" matched by name, size grew from 500 to 800
        assert_eq!(entry_name(&diff[0].entry), "mydir");
        assert_eq!(diff[0].change, ChangeType::Grown);
        assert_eq!(diff[0].old_size, Some(500));
        assert_eq!(diff[0].new_size, 800);
    }
}
