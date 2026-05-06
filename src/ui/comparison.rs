use crate::scanner::{DirNode, Entry};
use crate::snapshot::{diff_level, ChangeType, DiffNode};
use crate::treemap::{layout_treemap, paint_treemap, TreemapAction};
use egui::RichText;
use std::collections::HashMap;
use std::sync::Arc;

/// Data shared between the main window and the comparison window.
#[derive(Clone)]
pub struct ComparisonData {
    /// The snapshot directory tree to compare against.
    pub snapshot_root: Arc<DirNode>,
    pub snapshot_name: String,
    /// The current scan result (if any) — updated from main window.
    pub current_scan: Option<Arc<DirNode>>,
    /// Request to close the comparison window.
    pub close_requested: bool,
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

/// Comparison window app — runs in a separate native window.
pub struct ComparisonApp {
    data: Arc<std::sync::Mutex<ComparisonData>>,
    left_nav_stack: Vec<usize>,
    right_nav_stack: Vec<usize>,
    left_selected: Option<usize>,
    right_selected: Option<usize>,
    diff_cache: Option<Vec<DiffNode>>,
    diff_cache_key: Option<Vec<usize>>,
}

impl ComparisonApp {
    pub fn new(data: Arc<std::sync::Mutex<ComparisonData>>) -> Self {
        Self {
            data,
            left_nav_stack: Vec::new(),
            right_nav_stack: Vec::new(),
            left_selected: None,
            right_selected: None,
            diff_cache: None,
            diff_cache_key: None,
        }
    }
}

impl eframe::App for ComparisonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if close was requested
        {
            let data = self.data.lock().unwrap();
            if data.close_requested {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let (snapshot_root, snapshot_name, current_scan) = {
                let data = self.data.lock().unwrap();
                let snap_arc = data.snapshot_root.clone();
                let scan_arc = data.current_scan.clone();
                (
                    snap_arc,
                    data.snapshot_name.clone(),
                    scan_arc,
                )
            };

            let snap_ref: &DirNode = &snapshot_root;
            let scan_ref: Option<&DirNode> = current_scan.as_deref();

            // Title bar
            ui.horizontal(|ui| {
                ui.heading(RichText::new(format!(
                    "⚖ 对比: {} vs 当前扫描",
                    snapshot_name
                )));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕ 关闭").clicked() {
                        self.data.lock().unwrap().close_requested = true;
                    }
                });
            });
            ui.add_space(4.0);

            // Side-by-side panels
            let panel_height = ui.available_height();
            let panel_width = (ui.available_width() - 12.0) * 0.5;

            ui.horizontal(|ui| {
                // ── Left panel: current scan ──
                ui.allocate_ui(egui::vec2(panel_width, panel_height), |ui| {
                    ui.label(RichText::new("📁 当前扫描").heading());
                    ui.separator();

                    if let Some(scan_root) = scan_ref {
                        let left_current =
                            resolve_by_nav_stack(scan_root, &self.left_nav_stack);

                        if let Some(left_dir) = left_current {
                            let treemap_height = if !self.left_nav_stack.is_empty() {
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
                                ui, &left_nodes, self.left_selected, canvas_rect, None,
                            ) {
                                match action {
                                    TreemapAction::Click(idx) => {
                                        self.left_selected = Some(idx);
                                    }
                                    TreemapAction::DoubleClick(idx) => {
                                        if let Some(Entry::Dir(d)) =
                                            left_dir.children.get(left_nodes[idx].entry_index)
                                        {
                                            self.left_nav_stack
                                                .push(left_nodes[idx].entry_index);
                                            self.left_selected = None;

                                            // Auto-sync right
                                            if let Some(right_idx) = find_matching_dir_index(
                                                d,
                                                resolve_by_nav_stack(
                                                    snap_ref,
                                                    &self.right_nav_stack,
                                                )
                                                .unwrap_or(snap_ref),
                                            ) {
                                                self.right_nav_stack.push(right_idx);
                                                self.right_selected = None;
                                            }
                                        }
                                    }
                                }
                            }

                            if !self.left_nav_stack.is_empty() {
                                ui.horizontal(|ui| {
                                    if ui.button("<< 上层").clicked() {
                                        self.left_nav_stack.pop();
                                        self.left_selected = None;
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "深度: {}",
                                            self.left_nav_stack.len()
                                        ))
                                        .size(11.0)
                                        .color(egui::Color32::GRAY),
                                    );
                                });
                            }
                        } else {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("无法解析当前扫描树")
                                        .color(egui::Color32::RED),
                                );
                                if ui.button("<< 返回根目录").clicked() {
                                    self.left_nav_stack.clear();
                                    self.left_selected = None;
                                }
                            });
                        }
                    } else {
                        ui.label(RichText::new("暂无扫描结果").color(egui::Color32::GRAY));
                    }
                });

                ui.separator();

                // ── Right panel: snapshot with diff overlay ──
                ui.allocate_ui(egui::vec2(ui.available_width(), panel_height), |ui| {
                    ui.label(
                        RichText::new(format!("📸 快照: {}", snapshot_name)).heading(),
                    );
                    ui.separator();

                    let right_current =
                        resolve_by_nav_stack(snap_ref, &self.right_nav_stack);

                    if let Some(right_dir) = right_current {
                        let treemap_height = if !self.right_nav_stack.is_empty() {
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
                        let needs_recompute = self.diff_cache_key.as_ref()
                            != Some(&self.right_nav_stack)
                            || self.diff_cache.is_none();

                        if needs_recompute {
                            let new_diff = compute_diff_for_right(
                                &self.right_nav_stack,
                                &self.left_nav_stack,
                                snap_ref,
                                scan_ref,
                            );
                            self.diff_cache = Some(new_diff);
                            self.diff_cache_key = Some(self.right_nav_stack.clone());
                        }

                        let diff_map: HashMap<usize, &DiffNode> = self
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
                            self.right_selected,
                            canvas_rect,
                            Some(&diff_map),
                        ) {
                            match action {
                                TreemapAction::Click(idx) => {
                                    self.right_selected = Some(idx);
                                }
                                TreemapAction::DoubleClick(idx) => {
                                    if let Some(Entry::Dir(d)) =
                                        right_dir.children.get(right_nodes[idx].entry_index)
                                    {
                                        self.right_nav_stack
                                            .push(right_nodes[idx].entry_index);
                                        self.right_selected = None;

                                        // Auto-sync left
                                        if let Some(scan_root) = scan_ref {
                                            if let Some(left_idx) = find_matching_dir_index(
                                                d,
                                                resolve_by_nav_stack(
                                                    scan_root,
                                                    &self.left_nav_stack,
                                                )
                                                .unwrap_or(scan_root),
                                            ) {
                                                self.left_nav_stack.push(left_idx);
                                                self.left_selected = None;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !self.right_nav_stack.is_empty() {
                            ui.horizontal(|ui| {
                                if ui.button("<< 上层").clicked() {
                                    self.right_nav_stack.pop();
                                    self.right_selected = None;
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "深度: {}",
                                        self.right_nav_stack.len()
                                    ))
                                    .size(11.0)
                                    .color(egui::Color32::GRAY),
                                );
                            });
                        }
                    } else {
                        ui.label(RichText::new("暂无快照数据").color(egui::Color32::GRAY));
                    }
                });
            });
        });
    }
}
