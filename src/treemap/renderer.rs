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
pub fn paint_treemap(
    ui: &mut Ui,
    nodes: &[TreemapNode],
    selected_index: Option<usize>,
    canvas_rect: emath::Rect,
) -> Option<TreemapAction> {
    // 分配 painter，不设 Sense（避免 click Sense 消费第一次点击导致双击失效）
    let (response, painter) = ui.allocate_painter(canvas_rect.size(), Sense::hover());
    let response_rect = response.rect;

    // 将节点坐标从 canvas 局部坐标转换为 painter 实际坐标
    let offset = response_rect.min - canvas_rect.min;

    // 辅助：屏幕坐标 → 节点索引
    let pos_to_index = |pos: emath::Pos2| -> Option<usize> {
        for (i, node) in nodes.iter().enumerate().rev() {
            if node.rect.translate(offset).contains(pos) {
                return Some(i);
            }
        }
        None
    };

    for (i, node) in nodes.iter().enumerate() {
        let rect = node.rect.translate(offset);
        if !response_rect.intersects(rect) { continue; }

        painter.rect_filled(rect, CornerRadius::same(1), node.color);
        painter.rect_stroke(
            rect,
            CornerRadius::same(1),
            Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 0, 0, 60)),
            StrokeKind::Middle,
        );

        if selected_index == Some(i) {
            painter.rect_stroke(
                rect.shrink(1.0),
                CornerRadius::same(1),
                Stroke::new(SELECTED_STROKE_WIDTH, Color32::from_rgba_premultiplied(255, 200, 50, 220)),
                StrokeKind::Middle,
            );
        }

        let area = rect.width() * rect.height();
        if area >= LABEL_AREA_THRESHOLD {
            let text_color = if selected_index == Some(i) { Color32::BLACK } else { Color32::WHITE };
            painter.text(
                rect.left_top() + emath::vec2(3.0, 3.0),
                egui::Align2::LEFT_TOP,
                &node.label,
                FontId::proportional(12.0),
                text_color,
            );
        }
    }

    // 交互检测：手动轮询 input_state，不依赖 Sense
    // 这样双击的第一次 click 不会被 Sense 消费
    let pointer = ui.input(|i| i.pointer.clone());
    let interact_pos = pointer.interact_pos();

    // 交互检测：手动轮询 input_state
    let dbl = pointer.button_double_clicked(egui::PointerButton::Primary);
    let clk = pointer.button_clicked(egui::PointerButton::Primary);
    if dbl || clk {
        eprintln!("[input] double={} click={} pos={:?}", dbl, clk, interact_pos);
        // 打印所有 nodes 信息用于调试
        for (i, node) in nodes.iter().enumerate() {
            let r = node.rect.translate(offset);
            eprintln!("  [{}] {} dir={} rect=({:.0},{:.0},{:.0},{:.0})",
                i, node.label, node.is_dir, r.min.x, r.min.y, r.max.x, r.max.y);
        }
    }
    if dbl {
        if let Some(pos) = interact_pos {
            if let Some(idx) = pos_to_index(pos) {
                eprintln!("[input] DoubleClick idx={} label={} is_dir={}",
                    idx, nodes[idx].label, nodes[idx].is_dir);
                return Some(TreemapAction::DoubleClick(idx));
            }
        }
    } else if clk {
        if let Some(pos) = interact_pos {
            if let Some(idx) = pos_to_index(pos) {
                return Some(TreemapAction::Click(idx));
            }
            return Some(TreemapAction::Click(usize::MAX));
        }
    }

    // 悬停提示
    if let Some(pos) = response.hover_pos() {
        for node in nodes.iter().rev() {
            if node.rect.translate(offset).contains(pos) {
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
