mod app;
mod scanner;
mod platform;
mod treemap;
#[cfg(feature = "snapshot")]
mod snapshot;
mod ui;

use eframe::NativeOptions;
use app::DiskReviewerApp;

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Disk Reviewer",
        options,
        Box::new(|cc| {
            // 配置字体：使用微软雅黑（中英文统一，避免混排不对齐）
            let mut fonts = egui::FontDefinitions::default();

            let chinese_fonts = [
                (r"C:\Windows\Fonts\msyh.ttc", "msyh"),
                (r"C:\Windows\Fonts\simhei.ttf", "simhei"),
            ];
            for (path_str, name) in &chinese_fonts {
                let font_path = std::path::Path::new(path_str);
                if font_path.exists() {
                    if let Ok(font_data) = std::fs::read(font_path) {
                        fonts.font_data.insert(
                            name.to_string(),
                            std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                        );
                        // 设为所有字体族的首选，确保中英文使用同一字体
                        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                            fonts.families.get_mut(&family).unwrap().insert(0, name.to_string());
                        }
                        break;
                    }
                }
            }

            cc.egui_ctx.set_fonts(fonts);

            // 设置全局字体样式
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(12.0));
            style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
            style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(15.0));
            style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(10.0));
            cc.egui_ctx.set_style(style);
            Ok(Box::new(DiskReviewerApp::new(cc)))
        }),
    )
}
