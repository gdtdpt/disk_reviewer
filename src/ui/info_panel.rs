use egui::{Sense, Ui};
use crate::treemap::TreemapNode;
use crate::treemap::color::FileCategory;
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
        ui.label(format!("名称: {}", node.label));
        ui.label(format!("大小: {}", format_size(node.size)));
        ui.label(format!("占比: {:.1}%", node.percentage));
        ui.label(if node.is_dir { "类型: 目录" } else { "类型: 文件" });
        if node.is_dir {
            if let Some(dir) = current_dir {
                if let Some(crate::scanner::Entry::Dir(d)) = dir.children.get(node.entry_index) {
                    ui.label(format!("文件数: {}", d.file_count));
                    ui.label(format!(
                        "子目录: {}",
                        d.children
                            .iter()
                            .filter(|c| matches!(c, crate::scanner::Entry::Dir(_)))
                            .count()
                    ));
                }
            }
        }
    } else if let Some(dir) = current_dir {
        ui.label(format!("当前: {}", dir.name));
        ui.label(format!("总大小: {}", format_size(dir.total_size)));
        ui.label(format!("文件数: {}", dir.file_count));
        ui.label(format!("子条目: {}", dir.children.len()));
    } else {
        ui.label("点击色块查看详情");
    }

    // D-10, D-15: 颜色图例（横向排列，5 列 2 行）
    ui.separator();
    ui.heading("图例");
    let categories = [
        FileCategory::Document,
        FileCategory::Image,
        FileCategory::Video,
        FileCategory::Audio,
        FileCategory::Archive,
        FileCategory::Code,
        FileCategory::Executable,
        FileCategory::System,
        FileCategory::Temp,
        FileCategory::Other,
    ];
    ui.columns_const::<5, _>(|columns| {
        for (i, cat) in categories.iter().enumerate() {
            columns[i % 5].horizontal(|ui: &mut egui::Ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(2), cat.color());
                ui.label(cat.label());
            });
        }
    });
}
