use egui::{Color32, CornerRadius, FontId, Sense, Stroke, StrokeKind, Ui, emath};
use egui::emath::Rect;
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
    // 分配 painter，使用 Sense::click() 让 egui 追踪这个区域的输入
    // 同时用 interact_pointer_pos 获取精确的点击/双击位置
    let (response, painter) = ui.allocate_painter(canvas_rect.size(), Sense::click());
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

        // 填充 + 细边框
        painter.rect_filled(rect, CornerRadius::same(1), node.color);
        painter.rect_stroke(
            rect,
            CornerRadius::same(1),
            Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 0, 0, 60)),
            StrokeKind::Middle,
        );

        // 选中高亮：黄色边框
        if selected_index == Some(i) {
            painter.rect_stroke(
                rect.shrink(1.0),
                CornerRadius::same(1),
                Stroke::new(SELECTED_STROKE_WIDTH, Color32::from_rgba_premultiplied(255, 200, 50, 220)),
                StrokeKind::Middle,
            );
        }

        // 标签
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

    // 半透明浮动图例（左上角）
    use crate::treemap::color::FileCategory;
    let legend_items = [
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
    let swatch = 10.0;
    let gap = 5.0;
    let label_w = 38.0; // 每个标签估算宽度
    let total_w = legend_items.len() as f32 * (swatch + gap + label_w) + gap;
    let legend_bg = Rect::from_min_size(
        response_rect.min + emath::vec2(4.0, 4.0),
        emath::vec2(total_w, swatch + 8.0),
    );
    painter.rect_filled(
        legend_bg,
        CornerRadius::same(4),
        Color32::from_rgba_premultiplied(0, 0, 0, 150),
    );
    let mut cx = legend_bg.min.x + gap;
    for cat in &legend_items {
        let swatch_rect = Rect::from_min_size(
            emath::pos2(cx, legend_bg.min.y + 4.0),
            emath::vec2(swatch, swatch),
        );
        painter.rect_filled(swatch_rect, CornerRadius::same(1), cat.color());
        painter.text(
            emath::pos2(cx + swatch + 2.0, legend_bg.min.y + 5.0),
            egui::Align2::LEFT_TOP,
            cat.label(),
            FontId::proportional(9.0),
            Color32::WHITE,
        );
        cx += swatch + gap + label_w;
    }

    // 交互检测：优先检测双击，再检测单击
    // 使用 response.double_clicked() + interact_pointer_pos() 获取精确位置
    if response.double_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Some(idx) = pos_to_index(pos) {
                return Some(TreemapAction::DoubleClick(idx));
            }
        }
    } else if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
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
