use egui::Ui;
use crate::treemap::TreemapNode;
use crate::treemap::renderer::format_size;
use crate::scanner::DirNode;

pub fn info_panel_ui(
    ui: &mut Ui,
    selected: Option<&TreemapNode>,
    current_dir: Option<&DirNode>,
) {
    ui.heading("详情");
    ui.separator();

    if let Some(node) = selected {
        ui.label(egui::RichText::new(format!("名称: {}", node.label)).monospace().size(12.0));
        ui.label(egui::RichText::new(format!("大小: {}", format_size(node.size))).monospace().size(12.0));
        ui.label(egui::RichText::new(format!("占比: {:.1}%", node.percentage)).monospace().size(12.0));
        ui.label(egui::RichText::new(if node.is_dir { "类型: 目录" } else { "类型: 文件" }).monospace().size(12.0));
        if node.is_dir {
            if let Some(dir) = current_dir {
                if let Some(crate::scanner::Entry::Dir(d)) = dir.children.get(node.entry_index) {
                    ui.label(egui::RichText::new(format!("文件数: {}", d.file_count)).monospace().size(12.0));
                    ui.label(egui::RichText::new(format!(
                        "子目录: {}",
                        d.children
                            .iter()
                            .filter(|c| matches!(c, crate::scanner::Entry::Dir(_)))
                            .count()
                    )).monospace().size(12.0));
                }
            }
        }
    } else if let Some(dir) = current_dir {
        ui.label(egui::RichText::new(format!("当前: {}", dir.name)).monospace().size(12.0));
        ui.label(egui::RichText::new(format!("总大小: {}", format_size(dir.total_size))).monospace().size(12.0));
        ui.label(egui::RichText::new(format!("文件数: {}", dir.file_count)).monospace().size(12.0));
        ui.label(egui::RichText::new(format!("子条目: {}", dir.children.len())).monospace().size(12.0));
    } else {
        ui.label(egui::RichText::new("点击色块查看详情").monospace().size(12.0));
    }

}
