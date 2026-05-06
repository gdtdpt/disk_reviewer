use crate::scanner::{DirNode, Entry};
use crate::snapshot::{diff_level, ChangeType, DiffNode};
use crate::treemap::{layout_treemap, paint_treemap, TreemapAction};
use egui::{Color32, RichText};
use std::collections::HashMap;
use std::sync::Arc;

/// Data shared between the main window and the comparison viewport.
#[derive(Clone)]
pub struct ComparisonData {
    pub snapshot_root: Arc<DirNode>,
    pub snapshot_name: String,
    pub current_scan: Option<Arc<DirNode>>,
}

/// Persistent state for the comparison window, stored in egui memory.
#[derive(Default, Clone)]
struct ComparisonState {
    left_nav_stack: Vec<usize>,
    right_nav_stack: Vec<usize>,
    left_selected: Option<usize>,
    right_selected: Option<usize>,
    diff_cache: Option<Vec<DiffNode>>,
    diff_cache_key: Option<Vec<usize>>,
}

impl ComparisonState {
    fn load(ctx: &egui::Context) -> Self {
        ctx.data_mut(|d| d.get_temp(egui::Id::new("comparison_state")).unwrap_or_default())
    }

    fn save(self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("comparison_state"), self));
    }
}

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

fn find_matching_dir_index(source_root: &DirNode, target_root: &DirNode) -> Option<usize> {
    target_root.children.iter().enumerate().find_map(|(idx, entry)| {
        if let Entry::Dir(d) = entry {
            if d.name == source_root.name {
                return Some(idx);
            }
        }
        None
    })
}

fn compute_diff(
    right_nav_stack: &[usize],
    left_nav_stack: &[usize],
    snapshot_root: &DirNode,
    current_scan: Option<&DirNode>,
) -> Vec<DiffNode> {
    let right_current = resolve_by_nav_stack(snapshot_root, right_nav_stack);
    if let Some(right_dir) = right_current {
        let left_dir = current_scan.and_then(|root| resolve_by_nav_stack(root, left_nav_stack));
        if let Some(left_d) = left_dir {
            diff_level(right_dir, left_d)
        } else {
            right_dir.children.iter().enumerate().map(|(idx, entry)| DiffNode {
                entry: entry.clone(),
                change: ChangeType::Removed,
                old_size: Some(entry.size()),
                new_size: 0,
                child_index: Some(idx),
            }).collect()
        }
    } else {
        Vec::new()
    }
}

/// Render the comparison UI inside a viewport.
/// Called each frame by the viewport callback.
pub fn comparison_window_ui(ctx: &egui::Context, data: &ComparisonData) {
    // Request continuous repaint so the viewport stays alive
    ctx.request_repaint();

    let mut state = ComparisonState::load(ctx);

    egui::CentralPanel::default().show(ctx, |ui| {
        let snap_ref: &DirNode = &data.snapshot_root;
        let scan_ref: Option<&DirNode> = data.current_scan.as_deref();

        // Title bar
        ui.horizontal(|ui| {
            ui.heading(RichText::new(format!(
                "⚖ 对比: {} vs 当前扫描",
                data.snapshot_name
            )));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ 关闭").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
        ui.add_space(8.0);

        // Side-by-side panels — each panel uses vertical layout: title on top, treemap below
        let available_width = ui.available_width();
        let available_height = ui.available_height();
        let panel_width = (available_width - 12.0) * 0.5;

        ui.horizontal(|ui| {
            // ── Left panel: current scan ──
            ui.allocate_ui(egui::vec2(panel_width, available_height), |ui| {
                // Title ABOVE the treemap (vertical layout)
                ui.label(RichText::new("📁 当前扫描").heading());
                ui.separator();

                let treemap_height = if !state.left_nav_stack.is_empty() {
                    ui.available_height() - 30.0
                } else {
                    ui.available_height()
                };

                if let Some(scan_root) = scan_ref {
                    let left_current = resolve_by_nav_stack(scan_root, &state.left_nav_stack);
                    if let Some(left_dir) = left_current {
                        let canvas = egui::Rect::from_min_size(
                            egui::pos2(0.0, 0.0),
                            egui::vec2(panel_width, treemap_height.max(200.0)),
                        );
                        let nodes = layout_treemap(left_dir, canvas);

                        if let Some(action) = paint_treemap(ui, &nodes, state.left_selected, canvas, None) {
                            match action {
                                TreemapAction::Click(idx) => state.left_selected = Some(idx),
                                TreemapAction::DoubleClick(idx) => {
                                    if let Some(Entry::Dir(d)) = left_dir.children.get(nodes[idx].entry_index) {
                                        state.left_nav_stack.push(nodes[idx].entry_index);
                                        state.left_selected = None;
                                        if let Some(ri) = find_matching_dir_index(d,
                                            resolve_by_nav_stack(snap_ref, &state.right_nav_stack).unwrap_or(snap_ref))
                                        {
                                            state.right_nav_stack.push(ri);
                                            state.right_selected = None;
                                        }
                                    }
                                }
                            }
                        }

                        if !state.left_nav_stack.is_empty() {
                            ui.horizontal(|ui| {
                                if ui.button("<< 上层").clicked() {
                                    state.left_nav_stack.pop();
                                    state.left_selected = None;
                                }
                                ui.label(RichText::new(format!("深度: {}", state.left_nav_stack.len()))
                                    .size(11.0).color(Color32::GRAY));
                            });
                        }
                    } else {
                        ui.label(RichText::new("无法解析当前扫描树").color(Color32::RED));
                        if ui.button("<< 返回根目录").clicked() {
                            state.left_nav_stack.clear();
                            state.left_selected = None;
                        }
                    }
                } else {
                    ui.label(RichText::new("暂无扫描结果").color(Color32::GRAY));
                }
            });

            ui.separator();

            // ── Right panel: snapshot with diff overlay ──
            ui.allocate_ui(egui::vec2(ui.available_width(), available_height), |ui| {
                // Title ABOVE the treemap (vertical layout)
                ui.label(RichText::new(format!("📸 快照: {}", data.snapshot_name)).heading());
                ui.separator();

                let treemap_height = if !state.right_nav_stack.is_empty() {
                    ui.available_height() - 30.0
                } else {
                    ui.available_height()
                };

                let right_current = resolve_by_nav_stack(snap_ref, &state.right_nav_stack);
                if let Some(right_dir) = right_current {
                    let canvas = egui::Rect::from_min_size(
                        egui::pos2(0.0, 0.0),
                        egui::vec2(ui.available_width(), treemap_height.max(200.0)),
                    );
                    let nodes = layout_treemap(right_dir, canvas);

                    let needs_recompute = state.diff_cache_key.as_ref() != Some(&state.right_nav_stack)
                        || state.diff_cache.is_none();
                    if needs_recompute {
                        state.diff_cache = Some(compute_diff(
                            &state.right_nav_stack, &state.left_nav_stack,
                            snap_ref, scan_ref,
                        ));
                        state.diff_cache_key = Some(state.right_nav_stack.clone());
                    }

                    let diff_map: HashMap<usize, &DiffNode> = state.diff_cache.as_ref()
                        .map(|c| c.iter().filter_map(|dn| dn.child_index.map(|i| (i, dn))).collect())
                        .unwrap_or_default();

                    if let Some(action) = paint_treemap(ui, &nodes, state.right_selected, canvas, Some(&diff_map)) {
                        match action {
                            TreemapAction::Click(idx) => state.right_selected = Some(idx),
                            TreemapAction::DoubleClick(idx) => {
                                if let Some(Entry::Dir(d)) = right_dir.children.get(nodes[idx].entry_index) {
                                    state.right_nav_stack.push(nodes[idx].entry_index);
                                    state.right_selected = None;
                                    if let Some(sr) = scan_ref {
                                        if let Some(li) = find_matching_dir_index(d,
                                            resolve_by_nav_stack(sr, &state.left_nav_stack).unwrap_or(sr))
                                        {
                                            state.left_nav_stack.push(li);
                                            state.left_selected = None;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !state.right_nav_stack.is_empty() {
                        ui.horizontal(|ui| {
                            if ui.button("<< 上层").clicked() {
                                state.right_nav_stack.pop();
                                state.right_selected = None;
                            }
                            ui.label(RichText::new(format!("深度: {}", state.right_nav_stack.len()))
                                .size(11.0).color(Color32::GRAY));
                        });
                    }
                } else {
                    ui.label(RichText::new("暂无快照数据").color(Color32::GRAY));
                }
            });
        });
    });

    state.save(ctx);
}
