use crate::scanner::{DirNode, Entry};
use crate::snapshot::{diff_level, ChangeType, DiffNode};
use crate::treemap::{layout_treemap, paint_treemap, TreemapAction};
use egui::{Color32, RichText};
use std::collections::HashMap;
use std::sync::Arc;

/// Side-by-side comparison window state.
pub struct ComparisonWindow {
    pub open: bool,
    pub snapshot_id: i64,
    pub snapshot_name: String,
    pub snapshot_root: Option<Arc<DirNode>>,
    pub left_nav_stack: Vec<usize>,
    pub right_nav_stack: Vec<usize>,
    pub left_selected: Option<usize>,
    pub right_selected: Option<usize>,
    pub diff_cache: Option<Vec<DiffNode>>,
    pub diff_cache_key: Option<Vec<usize>>,
}

/// Traverse a DirNode tree following a nav stack.
fn resolve_by_nav_stack<'a>(root: &'a DirNode, nav_stack: &[usize]) -> Option<&'a DirNode> {
    let mut current = root;
    for &idx in nav_stack {
        match current.children.get(idx) {
            Some(Entry::Dir(d)) => current = d,
            _ => return None,
        }
    }
    Some(current)
}

/// Find the child index in `target_root` that matches the directory name from `source_root`.
fn find_matching_dir_index(source_root: &DirNode, target_root: &DirNode) -> Option<usize> {
    target_root
        .children
        .iter()
        .enumerate()
        .find_map(|(idx, entry)| {
            if let Entry::Dir(d) = entry {
                if d.name == source_root.name {
                    return Some(idx);
                }
            }
            None
        })
}

/// Compute diff for the right-side (snapshot) view.
fn compute_diff_for_right(
    right_nav_stack: &[usize],
    left_nav_stack: &[usize],
    snapshot_root: &DirNode,
    current_scan: Option<&DirNode>,
) -> Vec<DiffNode> {
    let right_current = resolve_by_nav_stack(snapshot_root, right_nav_stack);

    if let Some(right_dir) = right_current {
        let left_dir_for_diff: Option<&DirNode> =
            current_scan.and_then(|root| resolve_by_nav_stack(root, left_nav_stack));
        if let Some(left_d) = left_dir_for_diff {
            diff_level(right_dir, left_d)
        } else {
            right_dir
                .children
                .iter()
                .enumerate()
                .map(|(idx, entry)| DiffNode {
                    entry: entry.clone(),
                    change: ChangeType::Removed,
                    old_size: Some(entry.size()),
                    new_size: 0,
                    child_index: Some(idx),
                })
                .collect()
        }
    } else {
        Vec::new()
    }
}

/// Render the comparison view as a large centered window.
pub fn comparison_window_ui(
    ctx: &egui::Context,
    comparison: &mut ComparisonWindow,
    current_scan: Option<&DirNode>,
) {
    if !comparison.open {
        return;
    }

    let mut close_requested = false;

    // Use a large window centered on screen — acts as a "virtual second window"
    egui::Window::new(format!("对比: {} vs 当前扫描", comparison.snapshot_name))
        .open(&mut comparison.open)
        .resizable(true)
        .default_size([1200.0, 700.0])
        .min_size([800.0, 500.0])
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            // Side-by-side panels
            let panel_height = ui.available_height();
            let panel_width = (ui.available_width() - 12.0) * 0.5;

            ui.horizontal(|ui| {
                // ── Left panel: current scan ──
                ui.allocate_ui(egui::vec2(panel_width, panel_height), |ui| {
                    ui.label(RichText::new("📁 当前扫描").heading());
                    ui.separator();

                    if let Some(scan_root) = current_scan {
                        let left_current =
                            resolve_by_nav_stack(scan_root, &comparison.left_nav_stack);

                        if let Some(left_dir) = left_current {
                            let treemap_height = if !comparison.left_nav_stack.is_empty() {
                                ui.available_height() - 30.0
                            } else {
                                ui.available_height()
                            };
                            let canvas_rect = egui::Rect::from_min_size(
                                egui::pos2(0.0, 0.0),
                                egui::vec2(panel_width, treemap_height.max(200.0)),
                            );

                            let left_nodes = layout_treemap(left_dir, canvas_rect);

                            if let Some(action) = paint_treemap(
                                ui,
                                &left_nodes,
                                comparison.left_selected,
                                canvas_rect,
                                None,
                            ) {
                                match action {
                                    TreemapAction::Click(idx) => {
                                        comparison.left_selected = Some(idx);
                                    }
                                    TreemapAction::DoubleClick(idx) => {
                                        if let Some(Entry::Dir(d)) =
                                            left_dir.children.get(left_nodes[idx].entry_index)
                                        {
                                            comparison.left_nav_stack
                                                .push(left_nodes[idx].entry_index);
                                            comparison.left_selected = None;

                                            // Auto-sync right
                                            if let Some(snap_root) = &comparison.snapshot_root {
                                                let snap_ref: &DirNode = snap_root;
                                                if let Some(right_idx) = find_matching_dir_index(
                                                    d,
                                                    resolve_by_nav_stack(
                                                        snap_ref,
                                                        &comparison.right_nav_stack,
                                                    )
                                                    .unwrap_or(snap_ref),
                                                ) {
                                                    comparison.right_nav_stack.push(right_idx);
                                                    comparison.right_selected = None;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !comparison.left_nav_stack.is_empty() {
                                ui.horizontal(|ui| {
                                    if ui.button("<< 上层").clicked() {
                                        comparison.left_nav_stack.pop();
                                        comparison.left_selected = None;
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "深度: {}",
                                            comparison.left_nav_stack.len()
                                        ))
                                        .size(11.0)
                                        .color(Color32::GRAY),
                                    );
                                });
                            }
                        } else {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("无法解析当前扫描树")
                                        .color(Color32::RED),
                                );
                                if ui.button("<< 返回根目录").clicked() {
                                    comparison.left_nav_stack.clear();
                                    comparison.left_selected = None;
                                }
                            });
                        }
                    } else {
                        ui.label(RichText::new("暂无扫描结果").color(Color32::GRAY));
                    }
                });

                ui.separator();

                // ── Right panel: snapshot with diff overlay ──
                ui.allocate_ui(egui::vec2(ui.available_width(), panel_height), |ui| {
                    ui.label(
                        RichText::new(format!("📸 快照: {}", comparison.snapshot_name)).heading(),
                    );
                    ui.separator();

                    if let Some(snapshot_root) = &comparison.snapshot_root {
                        let snap_arc = snapshot_root.clone();
                        let snap_ref: &DirNode = &snap_arc;
                        let right_current =
                            resolve_by_nav_stack(snap_ref, &comparison.right_nav_stack);

                        if let Some(right_dir) = right_current {
                            let treemap_height = if !comparison.right_nav_stack.is_empty() {
                                ui.available_height() - 30.0
                            } else {
                                ui.available_height()
                            };
                            let canvas_rect = egui::Rect::from_min_size(
                                egui::pos2(0.0, 0.0),
                                egui::vec2(ui.available_width(), treemap_height.max(200.0)),
                            );

                            let right_nodes = layout_treemap(right_dir, canvas_rect);

                            // Diff cache
                            let needs_recompute = comparison.diff_cache_key.as_ref()
                                != Some(&comparison.right_nav_stack)
                                || comparison.diff_cache.is_none();

                            if needs_recompute {
                                let new_diff = compute_diff_for_right(
                                    &comparison.right_nav_stack,
                                    &comparison.left_nav_stack,
                                    snap_ref,
                                    current_scan,
                                );
                                comparison.diff_cache = Some(new_diff);
                                comparison.diff_cache_key =
                                    Some(comparison.right_nav_stack.clone());
                            }

                            let diff_map: HashMap<usize, &DiffNode> = comparison
                                .diff_cache
                                .as_ref()
                                .map(|cache| {
                                    cache
                                        .iter()
                                        .filter_map(|dn| dn.child_index.map(|idx| (idx, dn)))
                                        .collect()
                                })
                                .unwrap_or_default();

                            if let Some(action) = paint_treemap(
                                ui,
                                &right_nodes,
                                comparison.right_selected,
                                canvas_rect,
                                Some(&diff_map),
                            ) {
                                match action {
                                    TreemapAction::Click(idx) => {
                                        comparison.right_selected = Some(idx);
                                    }
                                    TreemapAction::DoubleClick(idx) => {
                                        if let Some(Entry::Dir(d)) =
                                            right_dir.children.get(right_nodes[idx].entry_index)
                                        {
                                            comparison.right_nav_stack
                                                .push(right_nodes[idx].entry_index);
                                            comparison.right_selected = None;

                                            // Auto-sync left
                                            if let Some(scan_root) = current_scan {
                                                if let Some(left_idx) = find_matching_dir_index(
                                                    d,
                                                    resolve_by_nav_stack(
                                                        scan_root,
                                                        &comparison.left_nav_stack,
                                                    )
                                                    .unwrap_or(scan_root),
                                                ) {
                                                    comparison.left_nav_stack.push(left_idx);
                                                    comparison.left_selected = None;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !comparison.right_nav_stack.is_empty() {
                                ui.horizontal(|ui| {
                                    if ui.button("<< 上层").clicked() {
                                        comparison.right_nav_stack.pop();
                                        comparison.right_selected = None;
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "深度: {}",
                                            comparison.right_nav_stack.len()
                                        ))
                                        .size(11.0)
                                        .color(Color32::GRAY),
                                    );
                                });
                            }
                        } else {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("无法解析快照树")
                                        .color(Color32::RED),
                                );
                                if ui.button("<< 返回根目录").clicked() {
                                    comparison.right_nav_stack.clear();
                                    comparison.right_selected = None;
                                }
                            });
                        }
                    } else {
                        ui.label(RichText::new("暂无快照数据").color(Color32::GRAY));
                    }
                });
            });

            // Close button at bottom
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ 关闭").clicked() {
                    close_requested = true;
                }
            });
        });

    if close_requested {
        comparison.open = false;
    }
}
