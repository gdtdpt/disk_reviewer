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

            // 选中背景高亮（淡蓝色，不遮挡文字）
            let bg_color = if is_selected {
                Color32::from_rgba_premultiplied(100, 160, 255, 30)
            } else {
                Color32::TRANSPARENT
            };

            // 用 Sense::click() 让整行响应点击，鼠标指针为手型
            let row_height = 22.0;
            let (rect, response) = ui.allocate_at_least(
                egui::vec2(ui.available_width(), row_height),
                Sense::click(),
            );

            // 绘制选中背景
            if bg_color != Color32::TRANSPARENT {
                ui.painter().rect_filled(rect, 2.0, bg_color);
            }

            // 在行区域内绘制内容（不使用 child_ui，直接在 rect 内 painter 绘制）
            let painter = ui.painter();

            // 12x12 颜色色块（垂直居中）
            let swatch_y = rect.center().y - 6.0;
            let swatch_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x + 4.0, swatch_y),
                egui::vec2(12.0, 12.0),
            );
            painter.rect_filled(swatch_rect, 1.0, node.color);

            // 名称（左对齐，选中加粗）
            let name_text = if node.is_dir {
                format!("📁 {}", node.label)
            } else {
                node.label.clone()
            };
            let name_color = if is_selected {
                Color32::BLACK
            } else {
                ui.style().visuals.text_color()
            };
            let name_galley = if is_selected {
                painter.layout_no_wrap(name_text, egui::FontId::proportional(12.0), name_color)
            } else {
                painter.layout_no_wrap(name_text, egui::FontId::proportional(12.0), name_color)
            };
            let name_pos = egui::pos2(
                rect.min.x + 22.0,
                rect.center().y - name_galley.size().y / 2.0,
            );
            painter.galley(name_pos, name_galley, name_color);

            // 占比（右对齐）
            let pct_text = format!("{:.1}%", node.percentage);
            let pct_galley = painter.layout_no_wrap(
                pct_text,
                egui::FontId::proportional(11.0),
                Color32::GRAY,
            );
            let pct_pos = egui::pos2(
                rect.max.x - pct_galley.size().x - 8.0,
                rect.center().y - pct_galley.size().y / 2.0,
            );
            painter.galley(pct_pos, pct_galley, Color32::GRAY);

            // 大小（占比左侧）
            let size_text = format_size(node.size);
            let size_galley = painter.layout_no_wrap(
                size_text,
                egui::FontId::proportional(11.0),
                Color32::GRAY,
            );
            let size_pos = egui::pos2(
                pct_pos.x - size_galley.size().x - 12.0,
                rect.center().y - size_galley.size().y / 2.0,
            );
            painter.galley(size_pos, size_galley, Color32::GRAY);

            // 交互检测：使用 response 的内置方法
            if response.hovered() {
                // 鼠标悬停时显示手型指针
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if response.double_clicked() {
                if node.is_dir {
                    action = FileListAction::Drill(i);
                }
            } else if response.clicked() && !is_selected {
                // 只有点击未选中的项才触发选中，避免已选中项被反复取消
                action = FileListAction::Select(i);
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
