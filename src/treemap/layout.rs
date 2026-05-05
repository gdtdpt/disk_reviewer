use crate::scanner::{DirNode, Entry, FileEntry};
use egui::emath::{pos2, vec2, Rect};
use std::path::PathBuf;
use egui::Color32;

pub fn layout_treemap(dir: &DirNode, canvas: Rect) -> Vec<crate::treemap::TreemapNode> {
    todo!()
}

fn entry_name(entry: &Entry) -> String {
    match entry {
        Entry::File(f) => f.name.clone(),
        Entry::Dir(d) => d.name.clone(),
        Entry::Others(o) => o.name.clone(),
        Entry::Symlink(p) => p.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?").to_string(),
        Entry::AccessDenied { path } => path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?").to_string(),
    }
}

fn is_dir(entry: &Entry) -> bool {
    matches!(entry, Entry::Dir(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dir(children: Vec<(String, u64)>) -> DirNode {
        let mut entries = Vec::new();
        for (name, size) in &children {
            entries.push(Entry::File(FileEntry {
                name: name.clone(),
                size: *size,
            }));
        }
        let total_size: u64 = entries.iter().map(|e| e.size()).sum();
        DirNode {
            path: PathBuf::from(r"C:\test"),
            name: "test".to_string(),
            total_size,
            file_count: entries.len() as u64,
            children: entries,
            access_denied: false,
        }
    }

    fn canvas() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(1.0, 1.0))
    }

    #[test]
    fn test_empty_dir_returns_empty_vec() {
        let dir = make_dir(vec![]);
        let result = layout_treemap(&dir, canvas());
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_child_fills_canvas() {
        let dir = make_dir(vec![("only.txt".to_string(), 100)]);
        let result = layout_treemap(&dir, canvas());
        assert_eq!(result.len(), 1);
        let n = &result[0];
        assert!((n.rect.min.x - 0.0).abs() < 0.001);
        assert!((n.rect.min.y - 0.0).abs() < 0.001);
        assert!((n.rect.max.x - 1.0).abs() < 0.001);
        assert!((n.rect.max.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_two_children_area_ratio() {
        let dir = make_dir(vec![("a.txt".to_string(), 100), ("b.txt".to_string(), 200)]);
        let result = layout_treemap(&dir, canvas());
        assert_eq!(result.len(), 2);
        let area_a = result[0].rect.area();
        let area_b = result[1].rect.area();
        let ratio = area_a / area_b;
        assert!((ratio - 0.5).abs() < 0.05,
            "面积比应接近 0.5，实际 {}", ratio);
        let total = area_a + area_b;
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_total_area_preserved() {
        let dir = make_dir(vec![
            ("a.txt".to_string(), 100),
            ("b.txt".to_string(), 200),
            ("c.txt".to_string(), 300),
            ("d.txt".to_string(), 400),
        ]);
        let result = layout_treemap(&dir, canvas());
        let total_area: f32 = result.iter().map(|n| n.rect.area()).sum();
        assert!((total_area - 1.0).abs() < 0.01,
            "总面积应等于 1.0，实际 {}", total_area);
    }

    #[test]
    fn test_zero_size_entries_filtered() {
        let mut dir = make_dir(vec![("a.txt".to_string(), 100)]);
        dir.children.push(Entry::AccessDenied { path: PathBuf::from(r"C:\denied") });
        let result = layout_treemap(&dir, canvas());
        assert_eq!(result.len(), 1, "AccessDenied 应被过滤");
    }

    #[test]
    fn test_equal_sizes_equal_areas() {
        let dir = make_dir(vec![
            ("a.txt".to_string(), 100),
            ("b.txt".to_string(), 100),
            ("c.txt".to_string(), 100),
            ("d.txt".to_string(), 100),
        ]);
        let result = layout_treemap(&dir, canvas());
        assert_eq!(result.len(), 4);
        let areas: Vec<f32> = result.iter().map(|n| n.rect.area()).collect();
        for i in 1..areas.len() {
            assert!((areas[i] - areas[0]).abs() < 0.01,
                "等 size 条目面积应相等");
        }
    }
}
