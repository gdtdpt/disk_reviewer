use egui::Ui;
use crate::treemap::TreemapNode;
use crate::treemap::renderer::format_size;
use crate::scanner::DirNode;

/// 文件列表用户操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileListAction {
    None,
    Select(usize),
    Drill(usize),
}

pub fn info_panel_ui(
    ui: &mut Ui,
    selected: Option<&TreemapNode>,
    current_dir: Option<&DirNode>,
    nodes: &[TreemapNode],
) -> FileListAction {
    let mut action = FileListAction::None;

    ui.heading("详情");
    ui.separator();

    // 固定详情区域高度（heading + separator + 最多6行信息），防止列表跳动
    // heading ~28px + separator ~8px + 6行×18px = ~144px，取整 150px
    let reserved_height = 150.0;
    let content_height = ui.available_height() - reserved_height;
    if content_height > 0.0 {
        // 用不可见的占位区域固定空间
        ui.allocate_exact_size(egui::vec2(ui.available_width(), reserved_height), egui::Sense::hover());
    }

    if let Some(node) = selected {
        ui.label(egui::RichText::new(format!("名称: {}", node.label)).size(12.0));
        ui.label(egui::RichText::new(format!("大小: {}", format_size(node.size))).size(12.0));
        ui.label(egui::RichText::new(format!("占比: {:.1}%", node.percentage)).size(12.0));
        ui.label(egui::RichText::new(if node.is_dir { "类型: 目录" } else { "类型: 文件" }).size(12.0));
        if node.is_dir {
            if let Some(dir) = current_dir {
                if let Some(crate::scanner::Entry::Dir(d)) = dir.children.get(node.entry_index) {
                    ui.label(egui::RichText::new(format!("文件数: {}", d.file_count)).size(12.0));
                    ui.label(egui::RichText::new(format!(
                        "子目录: {}",
                        d.children
                            .iter()
                            .filter(|c| matches!(c, crate::scanner::Entry::Dir(_)))
                            .count()
                    )).size(12.0));
                }
            }
        }
    } else if let Some(dir) = current_dir {
        ui.label(egui::RichText::new(format!("当前: {}", dir.name)).size(12.0));
        ui.label(egui::RichText::new(format!("总大小: {}", format_size(dir.total_size))).size(12.0));
        ui.label(egui::RichText::new(format!("文件数: {}", dir.file_count)).size(12.0));
        ui.label(egui::RichText::new(format!("子条目: {}", dir.children.len())).size(12.0));
    } else {
        ui.label(egui::RichText::new("点击色块查看详情").size(12.0));
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
