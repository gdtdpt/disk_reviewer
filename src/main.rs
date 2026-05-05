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
        Box::new(|cc| Ok(Box::new(DiskReviewerApp::new(cc)))),
    )
}
