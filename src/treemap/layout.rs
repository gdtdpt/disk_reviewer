use crate::scanner::{DirNode, Entry, FileEntry};
use crate::treemap::color::{categorize_entry, dominant_category};
use egui::emath::{pos2, vec2, Rect};
use std::path::PathBuf;

pub fn layout_treemap(dir: &DirNode, canvas: Rect) -> Vec<crate::treemap::TreemapNode> {
    let total_size = dir.total_size as f64;
    if total_size == 0.0 {
        return Vec::new();
    }

    // 1. Collect valid entries (size > 0), keep &Entry reference
    let mut items: Vec<(usize, &Entry, u64)> = Vec::new();
    for (idx, child) in dir.children.iter().enumerate() {
        let size = child.size();
        if size == 0 { continue; }
        items.push((idx, child, size));
    }

    if items.is_empty() {
        return Vec::new();
    }

    // 2. Sort by size descending
    items.sort_by_key(|&(_, _, size)| std::cmp::Reverse(size));

    // 3. Run squarified layout
    let sizes: Vec<f64> = items.iter().map(|&(_, _, s)| s as f64).collect();
    let nrects = squarify_recursive(&sizes, 0.0, 0.0, 1.0, 1.0);

    // 4. Scale to canvas + assemble TreemapNode
    items.into_iter().zip(nrects.into_iter())
        .map(|((entry_index, entry, size), nr)| {
            let rect = Rect::from_min_size(
                pos2(
                    canvas.min.x + nr.x * canvas.width(),
                    canvas.min.y + nr.y * canvas.height(),
                ),
                vec2(
                    nr.w * canvas.width(),
                    nr.h * canvas.height(),
                ),
            );
            let cat = if let Entry::Dir(d) = entry {
                dominant_category(d)
            } else {
                categorize_entry(entry)
            };
            crate::treemap::TreemapNode {
                rect,
                label: entry_name(entry),
                color: cat.color(),
                depth: 0,
                entry_index,
                is_dir: matches!(entry, Entry::Dir(_)),
                size,
                percentage: (size as f64 / total_size * 100.0) as f32,
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct NRect { x: f32, y: f32, w: f32, h: f32 }

fn squarify_recursive(sizes: &[f64], x: f32, y: f32, w: f32, h: f32) -> Vec<NRect> {
    let n = sizes.len();
    if n == 0 { return Vec::new(); }
    if n == 1 { return vec![NRect { x, y, w, h }]; }

    let total: f64 = sizes.iter().sum();
    if total == 0.0 { return Vec::new(); }

    let short_side = w.min(h);
    let long_side = w.max(h);
    let mut row = vec![sizes[0]];
    let mut row_sum = sizes[0];
    let mut remaining = &sizes[1..];

    while !remaining.is_empty() {
        let current_worst = worst_ratio(&row, row_sum, short_side, long_side, total);
        let mut new_row = row.clone();
        new_row.push(remaining[0]);
        let new_worst = worst_ratio(&new_row, row_sum + remaining[0], short_side, long_side, total);
        if new_worst <= current_worst {
            row_sum += remaining[0];
            row = new_row;
            remaining = &remaining[1..];
        } else {
            break;
        }
    }

    let row_total: f64 = row.iter().sum();
    let row_ratio = (row_total / total) as f32;
    let mut result = Vec::new();
    let mut offset = 0.0f32;

    if w >= h {
        // 行沿长边（w）方向铺满，行高 = (row_sum / total) * short_side
        let row_h = row_ratio * h;
        for &size in &row {
            let sw = (size as f32 / row_total as f32) * w;
            result.push(NRect { x: x + offset, y, w: sw, h: row_h });
            offset += sw;
        }
        // 剩余空间在行下方
        result.extend(squarify_recursive(remaining, x, y + row_h, w, h - row_h));
    } else {
        // 行沿长边（h）方向铺满，行宽 = (row_sum / total) * short_side
        let row_w = row_ratio * w;
        for &size in &row {
            let sh = (size as f32 / row_total as f32) * h;
            result.push(NRect { x, y: y + offset, w: row_w, h: sh });
            offset += sh;
        }
        // 剩余空间在行右侧
        result.extend(squarify_recursive(remaining, x + row_w, y, w - row_w, h));
    }
    result
}

fn worst_ratio(row: &[f64], row_sum: f64, short_side: f32, long_side: f32, total: f64) -> f32 {
    if row.is_empty() || row_sum == 0.0 || total == 0.0 {
        return f32::MAX;
    }
    let row_ratio = row_sum as f32 / total as f32;
    // 行在短边方向上的厚度
    let row_thickness = row_ratio * short_side;
    // 行沿长边方向铺满，每个矩形沿长边方向的长度 = (size / row_sum) * long_side
    row.iter().map(|&s| {
        let w = (s as f32 / row_sum as f32) * long_side;
        let h = row_thickness;
        let mn = w.min(h);
        let mx = w.max(h);
        if mn <= 0.0 { f32::MAX } else { mx / mn }
    }).fold(0.0f32, f32::max)
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

fn _is_dir(entry: &Entry) -> bool {
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
        let total_area: f32 = result.iter().map(|n| n.rect.area()).sum();
        assert!((total_area - 1.0).abs() < 0.01);
        // Each node's area should be proportional to its size
        let total_size: u64 = result.iter().map(|n| n.size).sum();
        for node in &result {
            let expected_pct = node.size as f32 / total_size as f32;
            let actual_pct = node.rect.area() / total_area;
            assert!((actual_pct - expected_pct).abs() < 0.05,
                "节点 {} 的面积占比 {} 应接近 {}",
                node.label, actual_pct, expected_pct);
        }
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

    #[test]
    fn test_squarified_not_all_in_one_row() {
        // 6 个等尺寸条目在 1x1 画布上，squarified 应该分成多行
        // 如果全部水平排列，则所有 rect 的 y 相同（都在同一行）
        let dir = make_dir(vec![
            ("a.txt".to_string(), 100),
            ("b.txt".to_string(), 100),
            ("c.txt".to_string(), 100),
            ("d.txt".to_string(), 100),
            ("e.txt".to_string(), 100),
            ("f.txt".to_string(), 100),
        ]);
        let result = layout_treemap(&dir, canvas());
        assert_eq!(result.len(), 6);
        // 验证不是所有矩形都在同一行（y 坐标不完全相同）
        let y_coords: Vec<f32> = result.iter().map(|n| n.rect.min.y).collect();
        let all_same_y = y_coords.iter().all(|&y| (y - y_coords[0]).abs() < 0.001);
        assert!(!all_same_y, "squarified 布局不应所有矩形在同一行");
        // 验证矩形的长宽比接近 1:1（没有极端长条）
        for node in &result {
            let w = node.rect.width();
            let h = node.rect.height();
            let ratio = w.max(h) / w.min(h).max(0.001);
            assert!(ratio < 3.0,
                "矩形 {} 的长宽比 {} 应小于 3:1", node.label, ratio);
        }
    }
}
