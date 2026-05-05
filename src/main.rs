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
            // 配置中文字体：从 Windows 系统目录加载微软雅黑
            let mut fonts = egui::FontDefinitions::default();
            let font_path = std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc");
            if font_path.exists() {
                if let Ok(font_data) = std::fs::read(font_path) {
                    fonts.font_data.insert(
                        "microsoft_yahei".to_owned(),
                        std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                    );
                    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                        fonts.families.get_mut(&family).unwrap().insert(0, "microsoft_yahei".to_owned());
                    }
                    cc.egui_ctx.set_fonts(fonts);
                }
            }
            Ok(Box::new(DiskReviewerApp::new(cc)))
        }),
    )
}
