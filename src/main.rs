mod app;
mod scanner;
mod platform;
mod treemap;
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
            // 配置字体：等宽字体 Consolas + 中文 fallback（微软雅黑）
            let mut fonts = egui::FontDefinitions::default();

            // 1. 加载中文字体作为 fallback（用于中文字符渲染）
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
                        break;
                    }
                }
            }

            // 2. 设置等宽字体优先（Consolas），中文字体作为 fallback
            //    egui 会在等宽字体中找不到中文字符时自动回退到中文字体
            fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "Consolas".to_string());
            if fonts.font_data.contains_key("msyh") {
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("msyh".to_string());
            } else if fonts.font_data.contains_key("simhei") {
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("simhei".to_string());
            }

            // 3. Proportional 也使用等宽 + 中文 fallback
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "Consolas".to_string());
            if fonts.font_data.contains_key("msyh") {
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().push("msyh".to_string());
            } else if fonts.font_data.contains_key("simhei") {
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().push("simhei".to_string());
            }

            cc.egui_ctx.set_fonts(fonts);

            // 4. 根据 DPI 设置全局字体缩放
            let dpi = cc.egui_ctx.native_pixels_per_point().unwrap_or(1.0);
            let base_size = 14.0 * dpi;
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles.insert(egui::TextStyle::Body, egui::FontId::monospace(base_size));
            style.text_styles.insert(egui::TextStyle::Button, egui::FontId::monospace(base_size));
            style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::monospace(base_size * 1.3));
            style.text_styles.insert(egui::TextStyle::Small, egui::FontId::monospace(base_size * 0.85));
            cc.egui_ctx.set_style(style);
            Ok(Box::new(DiskReviewerApp::new(cc)))
        }),
    )
}
