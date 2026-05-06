use egui::Ui;
use crate::scanner::DirNode;

/// 面包屑导航 UI
/// 返回用户点击的层级深度（0 = 根），None 表示未点击
pub fn breadcrumb_ui(
    ui: &mut Ui,
    scan_result: &DirNode,
    nav_stack: &[usize],
) -> Option<usize> {
    let mut clicked_depth = None;
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal(|ui| {
            // 根节点按钮
            if ui.add(egui::Button::new(egui::RichText::new(&scan_result.name).monospace().size(12.0))).clicked() {
                clicked_depth = Some(0);
            }
            // 沿 nav_stack 逐级显示路径段
            let mut current = scan_result;
            for (depth, &idx) in nav_stack.iter().enumerate() {
                ui.label(egui::RichText::new(">").monospace().size(12.0));
                if let Some(crate::scanner::Entry::Dir(dir)) = current.children.get(idx) {
                    if ui.add(egui::Button::new(egui::RichText::new(&dir.name).monospace().size(12.0))).clicked() {
                        clicked_depth = Some(depth + 1);
                    }
                    current = dir;
                }
            }
        });
    });
    clicked_depth
}
