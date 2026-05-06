mod app;
mod scanner;
mod platform;
mod treemap;
mod snapshot;
mod ui;

use eframe::NativeOptions;

fn setup_fonts(cc: &eframe::CreationContext<'_>) {
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
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts.families.get_mut(&family).unwrap().insert(0, name.to_string());
                }
                break;
            }
        }
    }
    cc.egui_ctx.set_fonts(fonts);

    let mut style = (*cc.egui_ctx.style()).clone();
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(12.0));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
    style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(15.0));
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(10.0));
    cc.egui_ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check for --comparison mode: --comparison <snapshot_id> <snapshot_name>
    if args.len() >= 4 && args[1] == "--comparison" {
        let snapshot_id: i64 = args[2].parse().unwrap_or(0);
        let snapshot_name = args[3].clone();

        // Load snapshot from DB
        #[cfg(feature = "snapshot")]
        {
            use crate::snapshot::SnapshotStorage;
            let db_path = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("disk_reviewer")
                .join("snapshots.db");
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let storage = SnapshotStorage::new(&db_path).expect("无法连接快照数据库");
            let root = storage.load_snapshot(snapshot_id)
                .expect("无法加载快照");

            let options = NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([1200.0, 750.0])
                    .with_min_inner_size([800.0, 500.0])
                    .with_title(format!("⚖ 对比 — {}", snapshot_name)),
                ..Default::default()
            };
            eframe::run_native(
                &format!("⚖ 对比 — {}", snapshot_name),
                options,
                Box::new(move |cc| {
                    setup_fonts(cc);
                    Ok(Box::new(
                        crate::ui::comparison::ComparisonApp::new(
                            snapshot_id,
                            snapshot_name.clone(),
                            std::sync::Arc::new(root),
                        ),
                    ))
                }),
            )
        }
    } else {
        // Normal main app mode
        let options = NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1024.0, 768.0]),
            ..Default::default()
        };
        eframe::run_native(
            "Disk Reviewer",
            options,
            Box::new(|cc| {
                setup_fonts(cc);
                Ok(Box::new(app::DiskReviewerApp::new(cc)))
            }),
        )
    }
}
