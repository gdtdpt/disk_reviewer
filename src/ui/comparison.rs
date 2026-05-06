use crate::scanner::{DirNode, Entry};
use crate::snapshot::{diff_level, ChangeType, DiffNode};
use crate::treemap::{layout_treemap, paint_treemap, TreemapAction};
use egui::{Color32, RichText};
use std::collections::HashMap;
use std::sync::Arc;

// ── Shared helper functions ──

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

// ── Rendering logic ──

fn render_treemap_panel(
    ui: &mut egui::Ui,
    root: &DirNode,
    nav_stack: &mut Vec<usize>,
    selected: &mut Option<usize>,
    snap_root: &Arc<DirNode>,
    current_scan: Option<&DirNode>,
    is_left: bool,
) {
    let current = resolve_by_nav_stack(root, nav_stack);
    if let Some(dir) = current {
        let available_height = ui.available_height();
        let panel_width = ui.available_width();
        let treemap_height = (available_height - 40.0).max(200.0);

        // Canvas starts at current cursor position
        let canvas_origin = egui::pos2(ui.min_rect().min.x, ui.cursor().min.y);
        let canvas = egui::Rect::from_min_size(
            canvas_origin,
            egui::vec2(panel_width, treemap_height),
        );

        let nodes = layout_treemap(dir, canvas);

        if let Some(action) = paint_treemap(ui, &nodes, *selected, canvas, None) {
            match action {
                TreemapAction::Click(idx) => *selected = Some(idx),
                TreemapAction::DoubleClick(idx) => {
                    if let Some(Entry::Dir(d)) = dir.children.get(nodes[idx].entry_index) {
                        nav_stack.push(nodes[idx].entry_index);
                        *selected = None;
                        // Auto-sync the other panel
                        if is_left {
                            if let Some(right_dir) = resolve_by_nav_stack(snap_root, nav_stack) {
                                if let Some(ri) = find_matching_dir_index(d, right_dir) {
                                    // We can't mutate right_nav_stack here — handled by caller
                                }
                            }
                        }
                    }
                }
            }
        }

        if !nav_stack.is_empty() {
            ui.horizontal(|ui| {
                if ui.button("<< 上层").clicked() {
                    nav_stack.pop();
                    *selected = None;
                }
                ui.label(
                    RichText::new(format!("深度: {}", nav_stack.len()))
                        .size(11.0)
                        .color(Color32::GRAY),
                );
            });
        }
    } else {
        ui.label(RichText::new("无法解析目录树").color(Color32::RED));
        if ui.button("<< 返回根目录").clicked() {
            nav_stack.clear();
            *selected = None;
        }
    }
}

fn render_diff_panel(
    ui: &mut egui::Ui,
    snap_root: &Arc<DirNode>,
    right_nav_stack: &mut Vec<usize>,
    right_selected: &mut Option<usize>,
    left_nav_stack: &[usize],
    current_scan: Option<&DirNode>,
    diff_cache: &mut Option<Vec<DiffNode>>,
    diff_cache_key: &mut Option<Vec<usize>>,
) {
    let right_current = resolve_by_nav_stack(snap_root, right_nav_stack);
    if let Some(right_dir) = right_current {
        let available_height = ui.available_height();
        let panel_width = ui.available_width();
        let treemap_height = (available_height - 40.0).max(200.0);

        let canvas_origin = egui::pos2(ui.min_rect().min.x, ui.cursor().min.y);
        let canvas = egui::Rect::from_min_size(
            canvas_origin,
            egui::vec2(panel_width, treemap_height),
        );

        let nodes = layout_treemap(right_dir, canvas);

        let needs_recompute = diff_cache_key.as_ref() != Some(right_nav_stack)
            || diff_cache.is_none();
        if needs_recompute {
            *diff_cache = Some(compute_diff(
                right_nav_stack,
                left_nav_stack,
                snap_root,
                current_scan,
            ));
            *diff_cache_key = Some(right_nav_stack.clone());
        }

        let diff_map: HashMap<usize, &DiffNode> = diff_cache
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
            *right_selected,
            canvas,
            Some(&diff_map),
        ) {
            match action {
                TreemapAction::Click(idx) => *right_selected = Some(idx),
                TreemapAction::DoubleClick(idx) => {
                    if let Some(Entry::Dir(d)) = right_dir.children.get(nodes[idx].entry_index) {
                        right_nav_stack.push(nodes[idx].entry_index);
                        *right_selected = None;
                    }
                }
            }
        }

        if !right_nav_stack.is_empty() {
            ui.horizontal(|ui| {
                if ui.button("<< 上层").clicked() {
                    right_nav_stack.pop();
                    *right_selected = None;
                }
                ui.label(
                    RichText::new(format!("深度: {}", right_nav_stack.len()))
                        .size(11.0)
                        .color(Color32::GRAY),
                );
            });
        }
    } else {
        ui.label(RichText::new("暂无快照数据").color(Color32::GRAY));
    }
}

// ── Standalone comparison app (separate process) ──

/// State for the standalone comparison app launched as a separate process.
pub struct ComparisonApp {
    snapshot_name: String,
    snapshot_root: Arc<DirNode>,
    current_scan: Option<Arc<DirNode>>,
    left_nav_stack: Vec<usize>,
    right_nav_stack: Vec<usize>,
    left_selected: Option<usize>,
    right_selected: Option<usize>,
    diff_cache: Option<Vec<DiffNode>>,
    diff_cache_key: Option<Vec<usize>>,
}

impl ComparisonApp {
    pub fn new(
        snapshot_id: i64,
        snapshot_name: String,
        snapshot_root: Arc<DirNode>,
        current_scan: Option<Arc<DirNode>>,
    ) -> Self {
        Self {
            snapshot_name,
            snapshot_root,
            current_scan,
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
        egui::CentralPanel::default().show(ctx, |ui| {
            // ── Title bar (no close button — use native window X) ──
            ui.heading(RichText::new(format!(
                "⚖ 对比: {} vs 当前扫描",
                self.snapshot_name
            )));
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(8.0);

            // ── Side-by-side panels ──
            ui.horizontal(|ui| {
                // ── Left panel (current scan) ──
                ui.vertical(|ui| {
                    ui.label(RichText::new("📁 当前扫描").heading());
                    ui.separator();

                    if let Some(scan_root) = &self.current_scan {
                        render_treemap_panel(
                            ui,
                            scan_root,
                            &mut self.left_nav_stack,
                            &mut self.left_selected,
                            &self.snapshot_root,
                            Some(scan_root),
                            true,
                        );
                    } else {
                        ui.label(RichText::new("暂无扫描结果 — 请在主窗口执行扫描后打开对比")
                            .color(Color32::GRAY));
                    }
                });

                ui.separator();

                // ── Right panel (snapshot with diff) ──
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("📸 快照: {}", self.snapshot_name)).heading(),
                    );
                    ui.separator();

                    render_diff_panel(
                        ui,
                        &self.snapshot_root,
                        &mut self.right_nav_stack,
                        &mut self.right_selected,
                        &self.left_nav_stack,
                        self.current_scan.as_deref(),
                        &mut self.diff_cache,
                        &mut self.diff_cache_key,
                    );
                });
            });
        });
    }
}

/// Launch the comparison as a separate OS process.
/// Serializes the current scan to a temp file and passes the path via CLI args.
pub fn launch_comparison_process(
    snapshot_id: i64,
    snapshot_name: &str,
    current_scan: Option<Arc<DirNode>>,
) -> std::io::Result<()> {
    let exe_path = std::env::current_exe()?;

    // Serialize current scan to temp file
    let scan_data_path = if let Some(scan) = &current_scan {
        let json = crate::snapshot::serialize_tree(scan)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let path = std::env::temp_dir().join("disk_reviewer_scan_data.json");
        std::fs::write(&path, json)?;
        Some(path)
    } else {
        None
    };

    let mut cmd = std::process::Command::new(exe_path);
    cmd.args([
        "--comparison",
        &snapshot_id.to_string(),
        snapshot_name,
    ]);

    if let Some(path) = scan_data_path {
        cmd.args(["--scan-data", path.to_str().unwrap_or("")]);
    }

    cmd.spawn()?;
    Ok(())
}
