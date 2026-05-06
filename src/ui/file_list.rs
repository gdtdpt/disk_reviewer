use egui::{Color32, RichText, Sense, Ui};

use crate::treemap::TreemapNode;
use crate::ui::info_panel::FileListAction;

/// 文件列表 UI
///
/// 返回用户操作（单击选中 / 双击下钻）
pub fn file_list_ui(
    ui: &mut Ui,
    nodes: &[TreemapNode],
    selected_index: Option<usize>,
) -> FileListAction {
    let mut action = FileListAction::None;

    // 表头
    ui.horizontal(|ui| {
        ui.label(RichText::new("名称").size(11.0).color(Color32::GRAY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("大小").size(11.0).color(Color32::GRAY));
            ui.add_space(60.0);
            ui.label(RichText::new("%").size(11.0).color(Color32::GRAY));
        });
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (i, node) in nodes.iter().enumerate() {
            let is_selected = selected_index == Some(i);

            // 选中背景高亮
            let bg_color = if is_selected {
                Color32::from_rgba_premultiplied(255, 200, 50, 40)
            } else {
                Color32::TRANSPARENT
            };

            let (response, painter) = ui.allocate_painter(
                egui::vec2(ui.available_width(), 20.0),
                Sense::click(),
            );

            if bg_color != Color32::TRANSPARENT {
                painter.rect_filled(response.rect, 2.0, bg_color);
            }

            // 布局：颜色色块 + 名称 + 占比 + 大小
            let mut child_ui = ui.child_ui(response.rect, egui::Layout::left_to_right(egui::Align::Center), None);

            // 12x12 颜色色块
            let swatch_rect = egui::Rect::from_min_size(
                child_ui.cursor().min,
                egui::vec2(12.0, 12.0),
            );
            child_ui.painter().rect_filled(swatch_rect, 1.0, node.color);
            child_ui.advance_cursor_after_rect(swatch_rect);
            child_ui.add_space(4.0);

            // 名称（左对齐）
            let name_text = if node.is_dir {
                format!("📁 {}", node.label)
            } else {
                node.label.clone()
            };
            child_ui.label(RichText::new(name_text).size(12.0));

            // 右侧：占比 + 大小
            child_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format_size(node.size))
                        .size(11.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("{:.1}%", node.percentage))
                        .size(11.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(8.0);
            });

            // 交互检测
            let pointer = ui.input(|i| i.pointer.clone());
            let interact_pos = pointer.interact_pos();

            if response.hovered() && interact_pos.map_or(false, |p| response.rect.contains(p)) {
                if pointer.button_double_clicked(egui::PointerButton::Primary) {
                    if node.is_dir {
                        action = FileListAction::Drill(i);
                    }
                } else if pointer.button_clicked(egui::PointerButton::Primary) {
                    action = FileListAction::Select(i);
                }
            }
        }
    });

    action
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut i = 0;
    while size >= 1024.0 && i < UNITS.len() - 1 {
        size /= 1024.0;
        i += 1;
    }
    format!("{:.1} {}", size, UNITS[i])
}
