use egui::{Color32, CornerRadius, FontId, Sense, Stroke, StrokeKind, Ui, emath};
use crate::treemap::TreemapNode;

const LABEL_AREA_THRESHOLD: f32 = 400.0;

pub fn paint_treemap(
    ui: &mut Ui,
    nodes: &[TreemapNode],
    selected_index: Option<usize>,
) -> Option<usize> {
    let size = ui.available_size();
    let (response, painter) = ui.allocate_painter(size, Sense::click());
    let mut clicked_index = None;

    for (i, node) in nodes.iter().enumerate() {
        if !response.rect.intersects(node.rect) { continue; }
        painter.rect_filled(node.rect, CornerRadius::same(1), node.color);
        if selected_index == Some(i) {
            painter.rect_stroke(
                node.rect.shrink(1.0),
                CornerRadius::same(1),
                Stroke::new(2.0, Color32::WHITE),
                StrokeKind::Middle,
            );
        }
        let area = node.rect.width() * node.rect.height();
        if area >= LABEL_AREA_THRESHOLD {
            painter.text(
                node.rect.left_top() + emath::vec2(2.0, 2.0),
                egui::Align2::LEFT_TOP,
                &node.label,
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
    }

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            for (i, node) in nodes.iter().enumerate().rev() {
                if node.rect.contains(pos) {
                    clicked_index = Some(i);
                    break;
                }
            }
        }
    }

    if let Some(pos) = response.hover_pos() {
        for node in nodes.iter().rev() {
            if node.rect.contains(pos) {
                let size_str = format_size(node.size);
                response.on_hover_ui_at_pointer(|ui| {
                    ui.label(&node.label);
                    ui.label(size_str);
                    ui.label(format!("{:.1}%", node.percentage));
                });
                break;
            }
        }
    }

    clicked_index
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut i = 0;
    while size >= 1024.0 && i < UNITS.len() - 1 {
        size /= 1024.0;
        i += 1;
    }
    format!("{:.1} {}", size, UNITS[i])
}
