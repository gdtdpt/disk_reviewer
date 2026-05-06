use egui::{Color32, RichText, StrokeKind};

use crate::scanner::types::format_size;
use crate::snapshot::SnapshotMeta;

/// User actions returned from the snapshot dialog UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotAction {
    None,
    Create(String),       // name
    Delete(i64),          // snapshot id
    Rename(i64, String),  // id, new name
    Load(i64),            // snapshot id
    OpenComparison(i64),  // snapshot id
}

/// Snapshot management dialog state.
#[derive(Debug, Clone)]
pub struct SnapshotDialog {
    pub open: bool,
    pub snapshots: Vec<SnapshotMeta>,
    pub selected_id: Option<i64>,
    pub new_name_buffer: String,
    pub rename_buffer: String,
    pub renaming_id: Option<i64>,
    pub delete_confirm_id: Option<i64>,
    /// Whether the default name has been pre-filled this session.
    pub default_name_prefilled: bool,
}

impl Default for SnapshotDialog {
    fn default() -> Self {
        Self {
            open: false,
            snapshots: Vec::new(),
            selected_id: None,
            new_name_buffer: String::new(),
            rename_buffer: String::new(),
            renaming_id: None,
            delete_confirm_id: None,
            default_name_prefilled: false,
        }
    }
}

/// Pre-fill the default name if not already done.
pub fn maybe_prefill_default_name(dialog: &mut SnapshotDialog) {
    if !dialog.default_name_prefilled {
        dialog.new_name_buffer = crate::snapshot::SnapshotStorage::default_name();
        dialog.default_name_prefilled = true;
    }
}

/// Render the snapshot management dialog as a centered modal overlay.
///
/// Takes `&egui::Context` so it can be called outside of a Ui scope.
/// Returns the user action to be processed by the caller.
pub fn snapshot_dialog_ui(
    ctx: &egui::Context,
    dialog: &mut SnapshotDialog,
    scan_available: bool,
    save_in_progress: bool,
) -> SnapshotAction {
    let mut action = SnapshotAction::None;

    if !dialog.open {
        return action;
    }

    // Delete confirmation dialog
    if let Some(delete_id) = dialog.delete_confirm_id {
        let mut confirmed = false;
        let mut cancelled = false;

        // Fullscreen overlay for delete confirm
        egui::Area::new("delete_confirm_overlay".into())
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                egui::Frame::window(ui.style())
                    .fill(egui::Color32::from_gray(40))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_min_width(320.0);
                        ui.label(
                            RichText::new(format!(
                                "确定要删除快照 #{} 吗？\n此操作不可撤销。",
                                delete_id
                            ))
                            .size(14.0),
                        );
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new(
                                    RichText::new("确认删除").color(Color32::WHITE),
                                ))
                                .clicked()
                            {
                                confirmed = true;
                            }
                            if ui.button("取消").clicked() {
                                cancelled = true;
                            }
                        });
                    });
            });

        if confirmed {
            action = SnapshotAction::Delete(delete_id);
            dialog.delete_confirm_id = None;
        } else if cancelled {
            dialog.delete_confirm_id = None;
        }
        return action;
    }

    let mut close_dialog = false;

    // Main dialog as centered modal overlay
    egui::Area::new("snapshot_dialog_overlay".into())
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::window(ui.style())
                .fill(egui::Color32::from_gray(32))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(80)))
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.set_min_width(520.0);
                    ui.set_max_width(640.0);

                    ui.heading(RichText::new("快照管理").size(18.0));
                    ui.add_space(8.0);

                    // Create new snapshot section
                    ui.horizontal(|ui| {
                        ui.label("名称:");
                        let response = ui.text_edit_singleline(&mut dialog.new_name_buffer);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            // Enter triggers create
                            let name = if dialog.new_name_buffer.trim().is_empty() {
                                crate::snapshot::SnapshotStorage::default_name()
                            } else {
                                dialog.new_name_buffer.trim().to_string()
                            };
                            action = SnapshotAction::Create(name);
                            dialog.new_name_buffer.clear();
                            dialog.default_name_prefilled = false;
                        }

                        let create_enabled = scan_available && !save_in_progress;
                        let create_btn = ui.add_enabled(
                            create_enabled,
                            egui::Button::new("新建"),
                        );
                        if create_btn.clicked() {
                            let name = if dialog.new_name_buffer.trim().is_empty() {
                                crate::snapshot::SnapshotStorage::default_name()
                            } else {
                                dialog.new_name_buffer.trim().to_string()
                            };
                            action = SnapshotAction::Create(name);
                            dialog.new_name_buffer.clear();
                            dialog.default_name_prefilled = false;
                        }

                        if save_in_progress {
                            ui.label(
                                RichText::new("⏳ 保存中...")
                                    .size(12.0)
                                    .color(Color32::YELLOW),
                            );
                        }
                    });

                    if !scan_available {
                        ui.label(
                            RichText::new("（无扫描结果可供保存）")
                                .size(11.0)
                                .color(Color32::GRAY),
                        );
                    }

                    ui.separator();

                    // Snapshot list
                    if dialog.snapshots.is_empty() {
                        ui.label(RichText::new("暂无快照").color(Color32::GRAY));
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for snap in &dialog.snapshots {
                                    let is_selected = dialog.selected_id == Some(snap.id);
                                    let row_height = 44.0;

                                    let (rect, response) = ui.allocate_at_least(
                                        egui::vec2(ui.available_width(), row_height),
                                        egui::Sense::click(),
                                    );

                                    // Selection background — more visible
                                    if is_selected {
                                        ui.painter().rect_filled(
                                            rect,
                                            3.0,
                                            Color32::from_rgba_premultiplied(60, 120, 220, 60),
                                        );
                                        ui.painter().rect_stroke(
                                            rect,
                                            3.0,
                                            egui::Stroke::new(
                                                1.5,
                                                Color32::from_rgba_premultiplied(
                                                    100, 160, 255, 180,
                                                ),
                                            ),
                                            StrokeKind::Inside,
                                        );
                                    } else if response.hovered() {
                                        ui.painter().rect_filled(
                                            rect,
                                            3.0,
                                            Color32::from_rgba_premultiplied(255, 255, 255, 10),
                                        );
                                    }

                                    let painter = ui.painter();
                                    let x = rect.min.x + 8.0;
                                    let line_h = 16.0;

                                    // Name
                                    let name_color = if is_selected {
                                        Color32::WHITE
                                    } else {
                                        ui.style().visuals.text_color()
                                    };
                                    let name_galley = painter.layout_no_wrap(
                                        snap.name.clone(),
                                        egui::FontId::proportional(13.0),
                                        name_color,
                                    );
                                    painter.galley(
                                        egui::pos2(x, rect.min.y + 4.0),
                                        name_galley,
                                        name_color,
                                    );

                                    // Metadata line
                                    let meta_text = format!(
                                        "{}  |  {}  |  {} 个文件  |  {}",
                                        snap.created_at,
                                        format_size(snap.total_size),
                                        snap.total_files,
                                        snap.root_path
                                    );
                                    let meta_galley = painter.layout_no_wrap(
                                        meta_text,
                                        egui::FontId::proportional(10.0),
                                        if is_selected {
                                            Color32::from_gray(180)
                                        } else {
                                            Color32::GRAY
                                        },
                                    );
                                    painter.galley(
                                        egui::pos2(x, rect.min.y + 4.0 + line_h),
                                        meta_galley,
                                        if is_selected {
                                            Color32::from_gray(180)
                                        } else {
                                            Color32::GRAY
                                        },
                                    );

                                    if response.clicked() {
                                        dialog.selected_id = Some(snap.id);
                                    }

                                    // Separator line
                                    ui.painter().hline(
                                        rect.min.x..=rect.max.x,
                                        rect.max.y,
                                        egui::Stroke::new(
                                            0.5,
                                            Color32::from_rgba_premultiplied(255, 255, 255, 15),
                                        ),
                                    );
                                }
                            });
                    }

                    ui.separator();

                    // Rename section
                    if let Some(sel_id) = dialog.selected_id {
                        let is_renaming = dialog.renaming_id == Some(sel_id);
                        if is_renaming {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut dialog.rename_buffer);
                                if ui.button("确认").clicked() {
                                    if !dialog.rename_buffer.is_empty() {
                                        action = SnapshotAction::Rename(
                                            sel_id,
                                            dialog.rename_buffer.clone(),
                                        );
                                        dialog.renaming_id = None;
                                        dialog.rename_buffer.clear();
                                    }
                                }
                                if ui.button("取消").clicked() {
                                    dialog.renaming_id = None;
                                    dialog.rename_buffer.clear();
                                }
                            });
                        }
                    }

                    ui.separator();

                    // Action buttons row
                    let sel_selected = dialog.selected_id.is_some();
                    ui.horizontal(|ui| {
                        // Load button removed — snapshots are for comparison only

                        let rename_btn = ui.add_enabled(
                            sel_selected,
                            egui::Button::new("重命名"),
                        );
                        if rename_btn.clicked() {
                            if let Some(id) = dialog.selected_id {
                                if let Some(snap) =
                                    dialog.snapshots.iter().find(|s| s.id == id)
                                {
                                    dialog.rename_buffer = snap.name.clone();
                                }
                                dialog.renaming_id = Some(id);
                            }
                        }

                        let delete_btn = ui.add_enabled(
                            sel_selected,
                            egui::Button::new("删除"),
                        );
                        if delete_btn.clicked() {
                            if let Some(id) = dialog.selected_id {
                                dialog.delete_confirm_id = Some(id);
                            }
                        }

                        let compare_btn = ui.add_enabled(
                            sel_selected && scan_available,
                            egui::Button::new("对比"),
                        );
                        if compare_btn.clicked() {
                            if let Some(id) = dialog.selected_id {
                                action = SnapshotAction::OpenComparison(id);
                            }
                        }

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.button("关闭").clicked() {
                                    close_dialog = true;
                                }
                            },
                        );
                    });
                });
        });

    if close_dialog {
        dialog.open = false;
        action = SnapshotAction::None;
    }

    action
}
