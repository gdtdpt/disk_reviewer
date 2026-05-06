use crate::scanner::{DirNode, Entry, FileEntry};
use crate::treemap::color::categorize_entry;
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

    // 3. Run squarified layout (D3-style algorithm)
    let sizes: Vec<f64> = items.iter().map(|&(_, _, s)| s as f64).collect();
    let nrects = squarify(&sizes, 0.0, 0.0, 1.0, 1.0);

    // 4. Scale to canvas + assemble TreemapNode
    let result: Vec<_> = items.into_iter().zip(nrects.into_iter())
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
                d.dominant_cat
            } else {
                categorize_entry(entry)
            };
            crate::treemap::TreemapNode {
                rect,
                label: entry_name(entry),
                color: cat.color(),
                category: cat,
                depth: 0,
                entry_index,
                is_dir: matches!(entry, Entry::Dir(_)),
                size,
                percentage: (size as f64 / total_size * 100.0) as f32,
            }
        })
        .collect();
    result
}

#[derive(Clone, Copy)]
struct NRect { x: f32, y: f32, w: f32, h: f32 }

/// Row in the squarified layout
struct Row {
    /// Indices into the sizes array
    indices: Vec<usize>,
    sum: f64,
    /// true = horizontal split (row on top, remaining below)
    /// false = vertical split (row on left, remaining on right)
    dice: bool,
}

/// D3-style squarified treemap algorithm (recursive).
///
/// Based on Bruls et al. (2000) "Squarified Treemaps", matching D3's implementation.
/// Key insight: worst ratio uses `max(dy/dx, dx/dy) / total` as alpha,
/// and `max(maxVal/beta, beta/minVal)` as the ratio metric.
///
/// Improvement over standard squarify: when a row has only one element and
/// its aspect ratio would exceed MAX_ASPECT, we force-add the next element.
/// This prevents the "first block fills entire row / column" problem that
/// occurs with highly skewed data (e.g. Windows dir taking 46% of C:).
const MAX_ASPECT: f32 = 2.5;

fn squarify(sizes: &[f64], x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<NRect> {
    let n = sizes.len();
    if n == 0 { return Vec::new(); }
    if n == 1 {
        return vec![NRect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }];
    }

    let total: f64 = sizes.iter().sum();
    if total == 0.0 { return Vec::new(); }

    let dx = x1 - x0;
    let dy = y1 - y0;

    // Build one row using greedy worst-ratio algorithm
    let mut row_sum = sizes[0];
    let mut row_min = sizes[0];
    let mut row_max = sizes[0];
    let mut j = 1;

    let alpha = (dy / dx).max(dx / dy) as f64 / total;
    let mut beta = row_sum * row_sum * alpha;
    let mut min_ratio = (row_max / beta).max(beta / row_min);

    while j < n {
        let v = sizes[j];
        let new_sum = row_sum + v;
        let new_min = row_min.min(v);
        let new_max = row_max.max(v);
        let new_beta = new_sum * new_sum * alpha;
        let new_ratio = (new_max / new_beta).max(new_beta / new_min);

        if new_ratio > min_ratio { break; }

        row_sum = new_sum;
        row_min = new_min;
        row_max = new_max;
        beta = new_beta;
        min_ratio = new_ratio;
        j += 1;
    }

    // Improvement: if row has only one element and aspect ratio is too high,
    // force-add the next element to prevent "filling entire row/column"
    if j == 1 {
        let dice = dx < dy;
        let row_ratio = (row_sum / total) as f32;
        let aspect = if dice {
            let row_h = row_ratio * dy;
            dx.max(row_h) / dx.min(row_h).max(0.001)
        } else {
            let row_w = row_ratio * dx;
            row_w.max(dy) / row_w.min(dy).max(0.001)
        };
        if aspect > MAX_ASPECT && n >= 2 {
            row_sum += sizes[1];
            j = 2;
        }
    }

    let dice = dx < dy;
    let mut result = Vec::new();

    if dice {
        // Horizontal split: row on top, remaining below
        let row_h = (row_sum / total) as f32 * dy;
        // Recursively layout elements within the row
        if j == 1 {
            result.push(NRect { x: x0, y: y0, w: x1 - x0, h: row_h });
        } else if j == n {
            // All elements in row — lay out horizontally
            let mut ox = x0;
            for k in 0..j {
                let w = (sizes[k] / row_sum) as f32 * (x1 - x0);
                result.push(NRect { x: ox, y: y0, w, h: row_h });
                ox += w;
            }
        } else {
            result.extend(squarify(&sizes[0..j], x0, y0, x1, y0 + row_h));
        }
        // Recursively layout remaining elements below
        if j < n {
            result.extend(squarify(&sizes[j..], x0, y0 + row_h, x1, y1));
        }
    } else {
        // Vertical split: row on left, remaining on right
        let row_w = (row_sum / total) as f32 * dx;
        if j == 1 {
            result.push(NRect { x: x0, y: y0, w: row_w, h: y1 - y0 });
        } else if j == n {
            let mut oy = y0;
            for k in 0..j {
                let h = (sizes[k] / row_sum) as f32 * (y1 - y0);
                result.push(NRect { x: x0, y: oy, w: row_w, h });
                oy += h;
            }
        } else {
            result.extend(squarify(&sizes[0..j], x0, y0, x0 + row_w, y1));
        }
        if j < n {
            result.extend(squarify(&sizes[j..], x0 + row_w, y0, x1, y1));
        }
    }

    result
}

fn entry_name(entry: &Entry) -> String {
    entry.name()
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
            dominant_cat: crate::treemap::color::FileCategory::Other,
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
        let y_coords: Vec<f32> = result.iter().map(|n| n.rect.min.y).collect();
        let all_same_y = y_coords.iter().all(|&y| (y - y_coords[0]).abs() < 0.001);
        assert!(!all_same_y, "squarified 布局不应所有矩形在同一行");
        for node in &result {
            let w = node.rect.width();
            let h = node.rect.height();
            let ratio = w.max(h) / w.min(h).max(0.001);
            assert!(ratio < 3.0,
                "矩形 {} 的长宽比 {} 应小于 3:1", node.label, ratio);
        }
    }
}
