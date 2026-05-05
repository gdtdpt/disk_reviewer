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
            if ui.button(&scan_result.name).clicked() {
                clicked_depth = Some(0);
            }
            // 沿 nav_stack 逐级显示路径段
            let mut current = scan_result;
            for (depth, &idx) in nav_stack.iter().enumerate() {
                ui.label(">");
                if let Some(crate::scanner::Entry::Dir(dir)) = current.children.get(idx) {
                    if ui.button(&dir.name).clicked() {
                        clicked_depth = Some(depth + 1);
                    }
                    current = dir;
                }
            }
        });
    });
    clicked_depth
}
