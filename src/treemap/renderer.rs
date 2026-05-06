use egui::{Color32, CornerRadius, FontId, Mesh, Pos2, Sense, Stroke, StrokeKind, Ui, emath, Vec2};
use crate::treemap::{TreemapAction, TreemapNode};
use crate::treemap::color::FileCategory;
use crate::snapshot::{ChangeType, DiffNode};

const SELECTED_STROKE_WIDTH: f32 = 2.0;

/// 根据色块面积返回字体大小（面积越大字体越大）
/// 返回 (font_size, show_detail) — show_detail 控制是否显示大小/占比后缀
fn font_for_area(area: f32) -> (f32, bool) {
    const S: f32 = 9.0;   // 小字
    const M: f32 = 11.0;  // 中字
    const L: f32 = 13.0;  // 大字
    const XL: f32 = 15.0; // 特大字
    match area {
        a if a >= 8000.0 => (XL, true),   // 特大色块：大字 + 显示大小
        a if a >= 3000.0 => (L, true),    // 大色块：大字 + 显示大小
        a if a >= 800.0  => (M, false),   // 中等色块：中字，仅显示名称
        a if a >= 200.0  => (S, false),   // 小色块：小字，仅显示名称
        _ => (0.0, false),                // 太小不显示标签
    }
}

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
        let (font_size, show_detail) = font_for_area(area);
        if font_size > 0.0 {
            let text_color = if selected_index == Some(i) { Color32::BLACK } else { Color32::WHITE };
            let label_font = FontId::proportional(font_size);
            let label_text = if show_detail {
                format!("{}  {:.1}%", node.label, node.percentage)
            } else {
                node.label.clone()
            };
            painter.text(
                rect.left_top() + emath::vec2(3.0, 3.0),
                egui::Align2::LEFT_TOP,
                label_text,
                label_font,
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

/// Overlay a semi-transparent color + icon on a treemap rectangle based on change type.
/// Colors per D-22: Added=green, Removed=red, Grown=orange, Shrunk=blue, Unchanged=skip.
pub fn paint_diff_overlay(
    painter: &egui::Painter,
    rect: emath::Rect,
    change: ChangeType,
) {
    let overlay_color = match change {
        ChangeType::Added   => Color32::from_rgba_unmultiplied(0, 200, 0, 80),
        ChangeType::Removed => Color32::from_rgba_unmultiplied(200, 0, 0, 80),
        ChangeType::Grown   => Color32::from_rgba_unmultiplied(255, 165, 0, 80),
        ChangeType::Shrunk  => Color32::from_rgba_unmultiplied(0, 100, 200, 80),
        ChangeType::Unchanged => return,
    };
    painter.rect_filled(rect, CornerRadius::same(1), overlay_color);

    // Icon in top-right corner
    let icon = match change {
        ChangeType::Added   => "+",
        ChangeType::Removed => "-",
        ChangeType::Grown   => "\u{2191}", // unicode up arrow
        ChangeType::Shrunk  => "\u{2193}", // unicode down arrow
        ChangeType::Unchanged => return,
    };
    let icon_pos = rect.right_top() + emath::vec2(-12.0, 2.0);
    painter.text(
        icon_pos,
        egui::Align2::LEFT_TOP,
        icon,
        FontId::proportional(10.0),
        Color32::WHITE,
    );
}

/// Variant of paint_treemap that accepts diff data for the right-side panel.
/// Shows color overlays and enhanced tooltips with change details.
pub fn paint_treemap_with_diff(
    ui: &mut Ui,
    nodes: &[TreemapNode],
    selected_index: Option<usize>,
    canvas_rect: emath::Rect,
    diff_map: &std::collections::HashMap<usize, &DiffNode>,
) -> Option<TreemapAction> {
    let (response, painter) = ui.allocate_painter(canvas_rect.size(), Sense::hover());
    let response_rect = response.rect;

    let offset = response_rect.min - canvas_rect.min;

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

        // Gradient fill (same as paint_treemap)
        let base_color = node.color;
        let end_color = node.category.gradient_end();
        let gradient_start = rect.top() + rect.height() * 0.6;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(Pos2::new(rect.left(), rect.top()), base_color);
        mesh.colored_vertex(Pos2::new(rect.right(), rect.top()), base_color);
        mesh.colored_vertex(Pos2::new(rect.right(), gradient_start), base_color);
        mesh.colored_vertex(Pos2::new(rect.left(), gradient_start), base_color);
        mesh.colored_vertex(Pos2::new(rect.right(), rect.bottom()), end_color);
        mesh.colored_vertex(Pos2::new(rect.left(), rect.bottom()), end_color);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
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

        // Diff overlay: color + icon
        if let Some(diff_node) = diff_map.get(&node.entry_index) {
            paint_diff_overlay(&painter, rect, diff_node.change);
        }

        let area = rect.width() * rect.height();
        let (font_size, show_detail) = font_for_area(area);
        if font_size > 0.0 {
            let text_color = if selected_index == Some(i) { Color32::BLACK } else { Color32::WHITE };
            let label_font = FontId::proportional(font_size);
            let label_text = if show_detail {
                format!("{}  {:.1}%", node.label, node.percentage)
            } else {
                node.label.clone()
            };
            painter.text(
                rect.left_top() + emath::vec2(3.0, 3.0),
                egui::Align2::LEFT_TOP,
                label_text,
                label_font,
                text_color,
            );
        }
    }

    // Interaction detection (same as paint_treemap)
    let pointer = ui.input(|i| i.pointer.clone());
    let interact_pos = pointer.interact_pos();

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

    // Hover tooltip with diff info
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

                    // Diff tooltip enhancement
                    if let Some(diff_node) = diff_map.get(&node.entry_index) {
                        if diff_node.change != ChangeType::Unchanged {
                            let delta_text = match diff_node.change {
                                ChangeType::Added => "(新增)".to_string(),
                                ChangeType::Removed => "(已删除)".to_string(),
                                ChangeType::Grown | ChangeType::Shrunk => {
                                    let old = diff_node.old_size.unwrap_or(0);
                                    let new = diff_node.new_size;
                                    let diff = if new > old { new - old } else { old - new };
                                    let sign = if new > old { "+" } else { "-" };
                                    format!("{}{} (之前: {})",
                                        sign, format_size(diff), format_size(old))
                                }
                                _ => String::new(),
                            };
                            ui.label(egui::RichText::new(delta_text).size(12.0).color(Color32::YELLOW));
                        }
                    }
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
