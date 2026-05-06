use egui::{Color32, RichText, Ui, Window};

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
        }
    }
}


/// Render the snapshot management dialog.
///
/// Takes `&egui::Context` so it can be called outside of a Ui scope.
/// Returns the user action to be processed by the caller.
pub fn snapshot_dialog_ui(
    ctx: &egui::Context,
    dialog: &mut SnapshotDialog,
    scan_available: bool,
) -> SnapshotAction {
    let mut action = SnapshotAction::None;

    if !dialog.open {
        return action;
    }

    // Delete confirmation dialog (T-03-05: confirmation before delete)
    if let Some(delete_id) = dialog.delete_confirm_id {
        let mut confirmed = false;
        let mut cancelled = false;
        Window::new("确认删除")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("确定要删除快照 #{} 吗？此操作不可撤销。", delete_id));
                ui.horizontal(|ui| {
                    if ui.button("删除").clicked() {
                        confirmed = true;
                    }
                    if ui.button("取消").clicked() {
                        cancelled = true;
                    }
                });
            });
        if confirmed {
            action = SnapshotAction::Delete(delete_id);
            dialog.delete_confirm_id = None;
        } else if cancelled {
            dialog.delete_confirm_id = None;
        }
        // Don't show the main dialog while delete confirm is open
        return action;
    }

    let mut close_dialog = false;

    Window::new("快照管理")
        .open(&mut dialog.open)
        .resizable(true)
        .default_size([480.0, 360.0])
        .show(ctx, |ui| {
            // Create new snapshot section
            ui.horizontal(|ui| {
                ui.label("名称:");
                ui.text_edit_singleline(&mut dialog.new_name_buffer);
                let create_enabled = scan_available;
                let create_btn = ui.add_enabled(
                    create_enabled,
                    egui::Button::new("新建"),
                );
                if create_btn.clicked() {
                    let name = if dialog.new_name_buffer.trim().is_empty() {
                        // D-18: default name with timestamp
                        chrono::Local::now().format("快照 %Y-%m-%d %H:%M").to_string()
                    } else {
                        dialog.new_name_buffer.trim().to_string()
                    };
                    action = SnapshotAction::Create(name);
                    dialog.new_name_buffer.clear();
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
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for snap in &dialog.snapshots {
                            let is_selected = dialog.selected_id == Some(snap.id);
                            let row_height = 40.0;

                            let (rect, response) = ui.allocate_at_least(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Sense::click(),
                            );

                            // Selection background
                            if is_selected {
                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    Color32::from_rgba_premultiplied(100, 160, 255, 30),
                                );
                            }

                            let painter = ui.painter();
                            let x = rect.min.x + 4.0;
                            let line_h = 16.0;

                            // Name (bold if selected)
                            let name_galley = painter.layout_no_wrap(
                                snap.name.clone(),
                                egui::FontId::proportional(12.0),
                                ui.style().visuals.text_color(),
                            );
                            painter.galley(
                                egui::pos2(x, rect.min.y + 4.0),
                                name_galley,
                                ui.style().visuals.text_color(),
                            );

                            // Metadata line: time | size | root_path
                            let meta_text = format!(
                                "{}  |  {}  |  {} 个文件  |  {}",
                                snap.created_at, format_size(snap.total_size), snap.total_files, snap.root_path
                            );
                            let meta_galley = painter.layout_no_wrap(
                                meta_text,
                                egui::FontId::proportional(10.0),
                                Color32::GRAY,
                            );
                            painter.galley(
                                egui::pos2(x, rect.min.y + 4.0 + line_h),
                                meta_galley,
                                Color32::GRAY,
                            );

                            if response.clicked() {
                                dialog.selected_id = Some(snap.id);
                            }
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
                                action = SnapshotAction::Rename(sel_id, dialog.rename_buffer.clone());
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
                let load_btn = ui.add_enabled(
                    sel_selected,
                    egui::Button::new("加载"),
                );
                if load_btn.clicked() {
                    if let Some(id) = dialog.selected_id {
                        action = SnapshotAction::Load(id);
                    }
                }

                let rename_btn = ui.add_enabled(
                    sel_selected,
                    egui::Button::new("重命名"),
                );
                if rename_btn.clicked() {
                    if let Some(id) = dialog.selected_id {
                        if let Some(snap) = dialog.snapshots.iter().find(|s| s.id == id) {
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("关闭").clicked() {
                        close_dialog = true;
                    }
                });
            });
        });

    if close_dialog {
        dialog.open = false;
    }

    // If the window's X button was clicked, dialog.open is already false
    if !dialog.open {
        action = SnapshotAction::None;
    }

    action
}
