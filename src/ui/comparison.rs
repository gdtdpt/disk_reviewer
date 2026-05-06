use crate::scanner::{DirNode, Entry};
use crate::snapshot::{diff_level, ChangeType, DiffNode};
use crate::treemap::{layout_treemap, paint_treemap, TreemapAction};
use std::collections::HashMap;
use std::sync::Arc;

/// Side-by-side comparison window state.
/// Left panel shows current scan, right panel shows snapshot with diff overlay.
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
}

/// Traverse a DirNode tree following a nav stack (indices into children at each level).
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

/// Render the comparison window: side-by-side treemaps with diff overlay on the right.
pub fn comparison_window_ui(
    ctx: &egui::Context,
    comparison: &mut ComparisonWindow,
    current_scan: Option<&DirNode>,
) {
    if !comparison.open {
        return;
    }

    let mut is_open = comparison.open;

    egui::Window::new(format!("对比: {} vs 当前扫描", comparison.snapshot_name))
        .open(&mut is_open)
        .resizable(true)
        .default_size([960.0, 600.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // ── Left panel: current scan ──
                let left_width = ui.available_width() * 0.5;
                ui.allocate_ui(
                    egui::vec2(left_width, ui.available_height()),
                    |ui| {
                        ui.label(
                            egui::RichText::new("当前扫描")
                                .heading(),
                        );
                        ui.separator();

                        if let Some(scan_root) = current_scan {
                            let left_current =
                                resolve_by_nav_stack(scan_root, &comparison.left_nav_stack);

                            if let Some(left_dir) = left_current {
                                let canvas_rect = egui::Rect::from_min_size(
                                    egui::pos2(0.0, 0.0),
                                    egui::vec2(
                                        ui.available_width(),
                                        ui.available_height().max(200.0),
                                    ),
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
                                            // Drill down on left
                                            if let Some(Entry::Dir(_)) =
                                                left_dir.children.get(left_nodes[idx].entry_index)
                                            {
                                                comparison.left_nav_stack
                                                    .push(left_nodes[idx].entry_index);
                                                comparison.left_selected = None;
                                            }
                                        }
                                    }
                                }

                                // Breadcrumb / back navigation hint
                                if !comparison.left_nav_stack.is_empty() {
                                    ui.horizontal(|ui| {
                                        if ui.button("<< 上层").clicked() {
                                            comparison.left_nav_stack.pop();
                                            comparison.left_selected = None;
                                        }
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "深度: {}",
                                                comparison.left_nav_stack.len()
                                            ))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY),
                                        );
                                    });
                                }
                            } else {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("无法解析当前扫描树")
                                            .color(egui::Color32::RED),
                                    );
                                    if ui.button("<< 返回根目录").clicked() {
                                        comparison.left_nav_stack.clear();
                                        comparison.left_selected = None;
                                    }
                                });
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("暂无扫描结果")
                                    .color(egui::Color32::GRAY),
                            );
                        }
                    },
                );

                ui.separator();

                // ── Right panel: snapshot with diff overlay ──
                ui.allocate_ui(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "快照: {}",
                                comparison.snapshot_name
                            ))
                            .heading(),
                        );
                        ui.separator();

                        if let Some(snapshot_root) = &comparison.snapshot_root {
                            let snap_ref: &DirNode = snapshot_root;
                            let right_current =
                                resolve_by_nav_stack(snap_ref, &comparison.right_nav_stack);

                            if let Some(right_dir) = right_current {
                                let canvas_rect = egui::Rect::from_min_size(
                                    egui::pos2(0.0, 0.0),
                                    egui::vec2(
                                        ui.available_width(),
                                        ui.available_height().max(200.0),
                                    ),
                                );

                                let right_nodes = layout_treemap(right_dir, canvas_rect);

                                // Compute diff between snapshot (old) and current scan (new)
                                // If we have a current scan at the same level, use it; otherwise empty
                                let left_dir_for_diff: Option<&DirNode> =
                                    current_scan.and_then(|root| {
                                        resolve_by_nav_stack(
                                            root,
                                            &comparison.left_nav_stack,
                                        )
                                    });

                                let diff_nodes = if let Some(left_d) = left_dir_for_diff {
                                    /*
                                     * Diff semantics for D-22:
                                     *   old = snapshot entries, new = current scan entries.
                                     *   Added  = in new scan but not in snapshot
                                     *   Removed = in snapshot but not in new scan
                                     *   Grown/Shrunk = matched by name, size changed
                                     */
                                    diff_level(right_dir, left_d)
                                } else {
                                    // No current scan at this level: mark all snapshot entries as Removed
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
                                };

                                // Build diff_map: entry_index -> &DiffNode
                                // Match by child_index (position in the snapshot's children)
                                // instead of by label to avoid name-collision bugs.
                                let diff_map: HashMap<usize, &DiffNode> = diff_nodes
                                    .iter()
                                    .filter_map(|dn| {
                                        dn.child_index
                                            .map(|idx| (idx, dn))
                                    })
                                    .collect();

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
                                            if let Some(Entry::Dir(_)) = right_dir
                                                .children
                                                .get(right_nodes[idx].entry_index)
                                            {
                                                comparison.right_nav_stack
                                                    .push(right_nodes[idx].entry_index);
                                                comparison.right_selected = None;
                                            }
                                        }
                                    }
                                }

                                // Breadcrumb / back navigation hint
                                if !comparison.right_nav_stack.is_empty() {
                                    ui.horizontal(|ui| {
                                        if ui.button("<< 上层").clicked() {
                                            comparison.right_nav_stack.pop();
                                            comparison.right_selected = None;
                                        }
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "深度: {}",
                                                comparison.right_nav_stack.len()
                                            ))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY),
                                        );
                                    });
                                }
                            } else {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("无法解析快照树")
                                            .color(egui::Color32::RED),
                                    );
                                    if ui.button("<< 返回根目录").clicked() {
                                        comparison.right_nav_stack.clear();
                                        comparison.right_selected = None;
                                    }
                                });
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("暂无快照数据")
                                    .color(egui::Color32::GRAY),
                            );
                        }
                    },
                );
            });
        });

    comparison.open = is_open;
}
