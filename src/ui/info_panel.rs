use egui::Ui;
use crate::treemap::TreemapNode;
use crate::scanner::types::format_size;
use crate::scanner::DirNode;

/// 文件列表用户操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileListAction {
    None,
    Select(usize),
    Drill(usize),
}

/// 详情区域固定高度（防止文件列表跳动）
const DETAIL_ROW_H: f32 = 18.0;
const DETAIL_MAX_ROWS: usize = 8; // 最多显示行数（含 heading + separator）
const DETAIL_RESERVED_H: f32 = DETAIL_ROW_H * DETAIL_MAX_ROWS as f32;

pub fn info_panel_ui(
    ui: &mut Ui,
    selected: Option<&TreemapNode>,
    current_dir: Option<&DirNode>,
    nodes: &[TreemapNode],
) -> FileListAction {
    let mut action = FileListAction::None;

    ui.heading("详情");
    ui.separator();

    // 在固定高度区域内绘制详情内容，避免列表跳动
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), DETAIL_RESERVED_H),
        egui::Sense::hover(),
    );
    let painter = ui.painter().clone();
    let mut y = rect.min.y;
    let x = rect.min.x + 4.0;
    let line_h = DETAIL_ROW_H;

    let mut draw_line = |text: &str, color: egui::Color32| {
        let galley = painter.layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(12.0),
            color,
        );
        painter.galley(egui::pos2(x, y), galley, color);
        y += line_h;
    };

    if let Some(node) = selected {
        draw_line(&format!("名称: {}", node.label), ui.style().visuals.text_color());
        draw_line(&format!("大小: {}", format_size(node.size)), ui.style().visuals.text_color());
        draw_line(&format!("占比: {:.1}%", node.percentage), ui.style().visuals.text_color());
        draw_line(if node.is_dir { "类型: 目录" } else { "类型: 文件" }, ui.style().visuals.text_color());
        if node.is_dir {
            if let Some(dir) = current_dir {
                if let Some(crate::scanner::Entry::Dir(d)) = dir.children.get(node.entry_index) {
                    draw_line(&format!("文件数: {}", d.file_count), ui.style().visuals.text_color());
                    draw_line(
                        &format!(
                            "子目录: {}",
                            d.children
                                .iter()
                                .filter(|c| matches!(c, crate::scanner::Entry::Dir(_)))
                                .count()
                        ),
                        ui.style().visuals.text_color(),
                    );
                }
            }
        }
    } else if let Some(dir) = current_dir {
        draw_line(&format!("当前: {}", dir.name), ui.style().visuals.text_color());
        draw_line(&format!("总大小: {}", format_size(dir.total_size)), ui.style().visuals.text_color());
        draw_line(&format!("文件数: {}", dir.file_count), ui.style().visuals.text_color());
        draw_line(&format!("子条目: {}", dir.children.len()), ui.style().visuals.text_color());
    } else {
        draw_line("点击色块查看详情", ui.style().visuals.text_color());
    }

    // 文件列表
    if !nodes.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(format!("共 {} 项", nodes.len())).size(11.0).color(egui::Color32::GRAY));
        ui.add_space(4.0);

        let list_action = crate::ui::file_list::file_list_ui(ui, nodes, selected_index(nodes, selected));
        action = list_action;
    }

    action
}

/// 找到选中节点在 nodes 列表中的索引
fn selected_index(nodes: &[TreemapNode], selected: Option<&TreemapNode>) -> Option<usize> {
    selected.and_then(|s| {
        nodes.iter().position(|n| n.entry_index == s.entry_index)
    })
}
