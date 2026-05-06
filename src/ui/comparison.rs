use crate::scanner::{DirNode, Entry};
use crate::snapshot::{diff_level, ChangeType, DiffNode};
use crate::treemap::{layout_treemap, paint_treemap, TreemapAction};
use egui::{Color32, RichText};
use std::collections::HashMap;
use std::sync::Arc;

/// Comparison window state. Created by `open_comparison` and rendered as a
/// large egui::Window. The `open` field is managed by `Window::open()`.
pub struct ComparisonWindow {
    pub open: bool,
    pub snapshot_id: i64,
    pub snapshot_name: String,
    pub snapshot_root: Option<Arc<DirNode>>,
    left_nav_stack: Vec<usize>,
    right_nav_stack: Vec<usize>,
    left_selected: Option<usize>,
    right_selected: Option<usize>,
    diff_cache: Option<Vec<DiffNode>>,
    diff_cache_key: Option<Vec<usize>>,
}

impl ComparisonWindow {
    pub fn new(snapshot_id: i64, snapshot_name: String, snapshot_root: Arc<DirNode>) -> Self {
        Self {
            open: true,
            snapshot_id,
            snapshot_name,
            snapshot_root: Some(snapshot_root),
            left_nav_stack: Vec::new(),
            right_nav_stack: Vec::new(),
            left_selected: None,
            right_selected: None,
            diff_cache: None,
            diff_cache_key: None,
        }
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

/// Render the comparison window. Call this from `update()` when
/// `comparison_state` is `Some`. The window is large and centered.
pub fn comparison_window_ui(
    ctx: &egui::Context,
    window: &mut ComparisonWindow,
    current_scan: Option<&DirNode>,
) {
    let snap_ref: &DirNode = match &window.snapshot_root {
        Some(root) => root,
        None => return,
    };

    // Large centered window. We use a local bool for `Window::open()` because
    // the closure mutably borrows `window`.  The close button writes to egui
    // data so we can pick it up after `.show()` returns.
    let mut is_open = window.open;
    let close_requested = egui::Id::new("comparison_close_request");

    egui::Window::new(format!("⚖ 对比 — {}", window.snapshot_name))
        .open(&mut is_open)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_size(egui::vec2(1200.0, 750.0))
        .min_size(egui::vec2(800.0, 500.0))
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            // ── Title bar ──
            ui.horizontal(|ui| {
                ui.heading(RichText::new(format!(
                    "⚖ 对比: {} vs 当前扫描",
                    window.snapshot_name
                )));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕ 关闭").clicked() {
                        ctx.data_mut(|d| d.insert_temp(close_requested, true));
                    }
                });
            });
            ui.add_space(8.0);

            // ── Side-by-side panels ──
            let available_height = ui.available_height();
            let panel_width = (ui.available_width() - 12.0) * 0.5;
            let treemap_height = (available_height - 60.0).max(200.0);

            ui.horizontal(|ui| {
                // ── Left panel (current scan) ──
                ui.vertical(|ui| {
                    // Title ABOVE the treemap
                    ui.label(RichText::new("📁 当前扫描").heading());
                    ui.separator();

                    // Canvas starts at the current cursor position (after title)
                    let canvas_origin = egui::pos2(ui.min_rect().min.x, ui.cursor().min.y);
                    let canvas = egui::Rect::from_min_size(
                        canvas_origin,
                        egui::vec2(panel_width, treemap_height),
                    );

                    if let Some(scan_root) = current_scan {
                        let left_current = resolve_by_nav_stack(scan_root, &window.left_nav_stack);
                        if let Some(left_dir) = left_current {
                            let nodes = layout_treemap(left_dir, canvas);

                            if let Some(action) = paint_treemap(
                                ui, &nodes, window.left_selected, canvas, None,
                            ) {
                                match action {
                                    TreemapAction::Click(idx) => window.left_selected = Some(idx),
                                    TreemapAction::DoubleClick(idx) => {
                                        if let Some(Entry::Dir(d)) =
                                            left_dir.children.get(nodes[idx].entry_index)
                                        {
                                            window.left_nav_stack.push(nodes[idx].entry_index);
                                            window.left_selected = None;
                                            if let Some(right_dir) = resolve_by_nav_stack(
                                                snap_ref, &window.right_nav_stack,
                                            ) {
                                                if let Some(ri) = find_matching_dir_index(d, right_dir) {
                                                    window.right_nav_stack.push(ri);
                                                    window.right_selected = None;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !window.left_nav_stack.is_empty() {
                                ui.horizontal(|ui| {
                                    if ui.button("<< 上层").clicked() {
                                        window.left_nav_stack.pop();
                                        window.left_selected = None;
                                    }
                                    ui.label(
                                        RichText::new(format!("深度: {}", window.left_nav_stack.len()))
                                            .size(11.0)
                                            .color(Color32::GRAY),
                                    );
                                });
                            }
                        } else {
                            ui.label(RichText::new("无法解析当前扫描树").color(Color32::RED));
                            if ui.button("<< 返回根目录").clicked() {
                                window.left_nav_stack.clear();
                                window.left_selected = None;
                            }
                        }
                    } else {
                        ui.label(RichText::new("暂无扫描结果").color(Color32::GRAY));
                    }
                });

                ui.separator();

                // ── Right panel (snapshot) ──
                ui.vertical(|ui| {
                    // Title ABOVE the treemap
                    ui.label(
                        RichText::new(format!("📸 快照: {}", window.snapshot_name)).heading(),
                    );
                    ui.separator();

                    // Canvas starts at the current cursor position (after title)
                    let canvas_origin = egui::pos2(ui.min_rect().min.x, ui.cursor().min.y);
                    let canvas = egui::Rect::from_min_size(
                        canvas_origin,
                        egui::vec2(ui.available_width(), treemap_height),
                    );

                    let right_current = resolve_by_nav_stack(snap_ref, &window.right_nav_stack);
                    if let Some(right_dir) = right_current {
                        let nodes = layout_treemap(right_dir, canvas);

                        let needs_recompute = window.diff_cache_key.as_ref()
                            != Some(&window.right_nav_stack)
                            || window.diff_cache.is_none();
                        if needs_recompute {
                            window.diff_cache = Some(compute_diff(
                                &window.right_nav_stack,
                                &window.left_nav_stack,
                                snap_ref,
                                current_scan,
                            ));
                            window.diff_cache_key = Some(window.right_nav_stack.clone());
                        }

                        let diff_map: HashMap<usize, &DiffNode> = window
                            .diff_cache
                            .as_ref()
                            .map(|c| {
                                c.iter()
                                    .filter_map(|dn| dn.child_index.map(|i| (i, dn)))
                                    .collect()
                            })
                            .unwrap_or_default();

                        if let Some(action) = paint_treemap(
                            ui,
                            &nodes,
                            window.right_selected,
                            canvas,
                            Some(&diff_map),
                        ) {
                            match action {
                                TreemapAction::Click(idx) => window.right_selected = Some(idx),
                                TreemapAction::DoubleClick(idx) => {
                                    if let Some(Entry::Dir(d)) =
                                        right_dir.children.get(nodes[idx].entry_index)
                                    {
                                        window.right_nav_stack.push(nodes[idx].entry_index);
                                        window.right_selected = None;
                                        if let Some(sr) = current_scan {
                                            if let Some(left_dir) = resolve_by_nav_stack(
                                                sr, &window.left_nav_stack,
                                            ) {
                                                if let Some(li) =
                                                    find_matching_dir_index(d, left_dir)
                                                {
                                                    window.left_nav_stack.push(li);
                                                    window.left_selected = None;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !window.right_nav_stack.is_empty() {
                            ui.horizontal(|ui| {
                                if ui.button("<< 上层").clicked() {
                                    window.right_nav_stack.pop();
                                    window.right_selected = None;
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "深度: {}",
                                        window.right_nav_stack.len()
                                    ))
                                    .size(11.0)
                                    .color(Color32::GRAY),
                                );
                            });
                        }
                    } else {
                        ui.label(RichText::new("暂无快照数据").color(Color32::GRAY));
                    }
                });
            });
        });

    // Sync the open flag back (handles X button, close button, and native close)
    let close_btn_clicked = ctx.data(|d| d.get_temp(close_requested).unwrap_or(false));
    window.open = is_open && !close_btn_clicked;
}
