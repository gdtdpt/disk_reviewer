use egui::emath::Rect;
use egui::Color32;
use crate::treemap::color::FileCategory;

#[derive(Debug, Clone)]
pub struct TreemapNode {
    pub rect: Rect,
    pub label: String,
    pub color: Color32,
    pub category: FileCategory,
    pub depth: u32,
    pub entry_index: usize,
    pub is_dir: bool,
    pub size: u64,
    pub percentage: f32,
}
