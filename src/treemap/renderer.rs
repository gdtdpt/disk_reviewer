use egui::{Color32, CornerRadius, FontId, Mesh, Pos2, Sense, Stroke, StrokeKind, Ui, emath, Vec2};
use crate::treemap::{TreemapAction, TreemapNode};
use crate::treemap::color::FileCategory;

const LABEL_AREA_THRESHOLD: f32 = 400.0;
const SELECTED_STROKE_WIDTH: f32 = 2.0;
const BASE_FONT_SIZE: f32 = 12.0;

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

    // 根据 DPI 缩放字体大小
    let dpi_scale = ui.ctx().pixels_per_point();
    let font_size = (BASE_FONT_SIZE * dpi_scale).max(10.0);
    let label_font = FontId::proportional(font_size);

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

        // 垂直渐变填充：顶部 60% 纯色基色，底部 40% 渐变为同色系浅色
        let base_color = node.color;
        let end_color = node.category.gradient_end();
        let gradient_start = rect.top() + rect.height() * 0.6;
        let mut mesh = Mesh::default();
        // 纯色区域（上 60%）：4 个顶点
        mesh.colored_vertex(Pos2::new(rect.left(), rect.top()), base_color);       // 0: 左上
        mesh.colored_vertex(Pos2::new(rect.right(), rect.top()), base_color);      // 1: 右上
        mesh.colored_vertex(Pos2::new(rect.right(), gradient_start), base_color);  // 2: 渐变起点右
        mesh.colored_vertex(Pos2::new(rect.left(), gradient_start), base_color);   // 3: 渐变起点左
        // 渐变区域（下 40%）：2 个底边顶点
        mesh.colored_vertex(Pos2::new(rect.right(), rect.bottom()), end_color);    // 4: 右下
        mesh.colored_vertex(Pos2::new(rect.left(), rect.bottom()), end_color);     // 5: 左下
        // 纯色区域（2 个三角形）
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        // 渐变区域（2 个三角形）
        mesh.add_triangle(3, 2, 4);
        mesh.add_triangle(3, 4, 5);
        painter.add(egui::Shape::Mesh(std::sync::Arc::new(mesh)));

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
                label_font.clone(),
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
    if dbl {
        if let Some(pos) = interact_pos {
            if let Some(idx) = pos_to_index(pos) {
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

    // 悬停提示：大字体、宽窗口、支持长名称换行
    if let Some(pos) = response.hover_pos() {
        for node in nodes.iter().rev() {
            if node.rect.translate(offset).contains(pos) {
                let size_str = format_size(node.size);
                response.on_hover_ui_at_pointer(|ui| {
                    ui.set_min_width(200.0);
                    ui.label(egui::RichText::new(&node.label).size(14.0).strong());
                    ui.label(egui::RichText::new(size_str).size(13.0));
                    ui.label(egui::RichText::new(format!("{:.1}%", node.percentage)).size(13.0));
                    let type_str = if node.is_dir { "目录" } else { "文件" };
                    ui.label(egui::RichText::new(type_str).size(12.0).color(Color32::GRAY));
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
