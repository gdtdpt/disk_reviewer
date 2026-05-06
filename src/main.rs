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
            // 配置字体：加载等宽字体 + 中文 fallback
            let mut fonts = egui::FontDefinitions::default();

            // 1. 加载等宽字体（consola.ttf），作为 Monospace 族首选
            let mono_fonts = [
                (r"C:\Windows\Fonts\consola.ttf", "consola"),
                (r"C:\Windows\Fonts\CascadiaMono.ttf", "cascadia_mono"),
            ];
            let mut mono_name = None;
            for (path_str, name) in &mono_fonts {
                let font_path = std::path::Path::new(path_str);
                if font_path.exists() {
                    if let Ok(font_data) = std::fs::read(font_path) {
                        fonts.font_data.insert(
                            name.to_string(),
                            std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                        );
                        mono_name = Some(name.to_string());
                        break;
                    }
                }
            }

            // 2. 加载中文字体作为 fallback（用于中文字符渲染）
            let chinese_fonts = [
                (r"C:\Windows\Fonts\msyh.ttc", "msyh"),
                (r"C:\Windows\Fonts\simhei.ttf", "simhei"),
            ];
            let mut chinese_name = None;
            for (path_str, name) in &chinese_fonts {
                let font_path = std::path::Path::new(path_str);
                if font_path.exists() {
                    if let Ok(font_data) = std::fs::read(font_path) {
                        fonts.font_data.insert(
                            name.to_string(),
                            std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                        );
                        chinese_name = Some(name.to_string());
                        break;
                    }
                }
            }

            // 3. 设置字体族：等宽字体优先，中文字体作为 fallback
            if let Some(mono) = &mono_name {
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, mono.clone());
            }
            if let Some(ch) = &chinese_name {
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push(ch.clone());
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, ch.clone());
            }

            cc.egui_ctx.set_fonts(fonts);

            // 4. 设置全局字体样式（等宽，egui 会自动处理 DPI 缩放）
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles.insert(egui::TextStyle::Body, egui::FontId::monospace(12.0));
            style.text_styles.insert(egui::TextStyle::Button, egui::FontId::monospace(12.0));
            style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::monospace(15.0));
            style.text_styles.insert(egui::TextStyle::Small, egui::FontId::monospace(10.0));
            cc.egui_ctx.set_style(style);
            Ok(Box::new(DiskReviewerApp::new(cc)))
        }),
    )
}
