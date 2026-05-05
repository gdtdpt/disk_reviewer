pub struct DiskReviewerApp {
    // Phase 1: 扫描结果占位
    // 后续计划会添加 scanner handle, channel receiver 等字段
}

impl DiskReviewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {}
    }
}

impl eframe::App for DiskReviewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Disk Reviewer");
            ui.label("扫描引擎开发中... (Phase 1)");
        });
    }
}
