use egui::{Color32, CornerRadius, FontId, Sense, Stroke, StrokeKind, Ui, emath};
use crate::treemap::{TreemapAction, TreemapNode};

const LABEL_AREA_THRESHOLD: f32 = 400.0;
const SELECTED_STROKE_WIDTH: f32 = 2.0;

/// 绘制 Treemap，返回用户交互动作
///
/// 交互模式：
/// - 单击色块 → 选中（TreemapAction::Click）
/// - 单击空白 → 取消选中（TreemapAction::Click(usize::MAX)）
/// - 双击目录色块 → 下钻（TreemapAction::DoubleClick）
///
/// canvas_rect 参数指定了绘制区域的实际位置，确保矩形坐标与 painter 原点一致。
pub fn paint_treemap(
    ui: &mut Ui,
    nodes: &[TreemapNode],
    selected_index: Option<usize>,
    canvas_rect: emath::Rect,
) -> Option<TreemapAction> {
    // 使用 canvas_rect 分配 painter，确保响应区域与绘制区域一致
    let (response, painter) = ui.allocate_painter(canvas_rect.size(), Sense::click_and_drag());
    let response_rect = response.rect;

    // 将节点坐标从 canvas 局部坐标转换为 painter 实际坐标
    let offset = response_rect.min - canvas_rect.min;

    for (i, node) in nodes.iter().enumerate() {
        let rect = node.rect.translate(offset);
        if !response_rect.intersects(rect) { continue; }

        // 绘制填充矩形
        painter.rect_filled(rect, CornerRadius::same(1), node.color);

        // 选中高亮：半透明白色叠加 + 细边框
        if selected_index == Some(i) {
            painter.rect_filled(
                rect.shrink(1.0),
                CornerRadius::same(1),
                Color32::from_rgba_premultiplied(255, 255, 255, 40),
            );
            painter.rect_stroke(
                rect.shrink(1.0),
                CornerRadius::same(1),
                Stroke::new(SELECTED_STROKE_WIDTH, Color32::from_rgba_premultiplied(255, 255, 255, 180)),
                StrokeKind::Middle,
            );
        }

        // 标签：面积足够大时显示
        let area = rect.width() * rect.height();
        if area >= LABEL_AREA_THRESHOLD {
            painter.text(
                rect.left_top() + emath::vec2(3.0, 3.0),
                egui::Align2::LEFT_TOP,
                &node.label,
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
    }

    // 处理交互
    if response.double_clicked() {
        // 双击 → 下钻
        if let Some(pos) = response.interact_pointer_pos() {
            for (i, node) in nodes.iter().enumerate().rev() {
                let rect = node.rect.translate(offset);
                if rect.contains(pos) {
                    return Some(TreemapAction::DoubleClick(i));
                }
            }
        }
    } else if response.clicked() {
        // 单击 → 选中 或 取消选中
        if let Some(pos) = response.interact_pointer_pos() {
            for (i, node) in nodes.iter().enumerate().rev() {
                let rect = node.rect.translate(offset);
                if rect.contains(pos) {
                    return Some(TreemapAction::Click(i));
                }
            }
            // 点击空白区域 → 取消选中
            return Some(TreemapAction::Click(usize::MAX));
        }
    }

    // 悬停提示
    if let Some(pos) = response.hover_pos() {
        for node in nodes.iter().rev() {
            let rect = node.rect.translate(offset);
            if rect.contains(pos) {
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

    None
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
