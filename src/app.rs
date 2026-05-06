use crossbeam_channel::{bounded, Receiver, TryRecvError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::platform::drives::{self, DriveInfo};
use crate::scanner::{scan_directory, AggThresholds, DirNode, Entry, ScanEvent};
use crate::treemap::{paint_treemap, TreemapAction};
use crate::ui::breadcrumb::breadcrumb_ui;
use egui::{Color32, emath::{pos2, vec2, Rect}};

#[cfg(feature = "snapshot")]
use crate::snapshot::SnapshotStorage;

pub struct DiskReviewerApp {
    pub drives: Vec<DriveInfo>,
    pub scan_result: Option<Arc<DirNode>>,
    pub scan_progress: Option<ScanEvent>,
    event_receiver: Option<Receiver<ScanEvent>>,
    pub status_message: String,
    cancel_token: Option<Arc<AtomicBool>>,
    // Phase 2: Treemap 状态
    pub nav_stack: Vec<usize>,
    pub selected_index: Option<usize>,
    pub treemap_nodes: Vec<crate::treemap::TreemapNode>,
    needs_rebuild: bool,
    last_canvas_rect: Option<Rect>,
    pending_resize: Option<Rect>,
    // Phase 3: Snapshot management state
    #[cfg(feature = "snapshot")]
    pub snapshot_manager: Option<SnapshotStorage>,
    #[cfg(feature = "snapshot")]
    pub snapshot_dialog_open: bool,
    #[cfg(feature = "snapshot")]
    pub snapshot_dialog_state: crate::ui::snapshot_dialog::SnapshotDialog,
    // Phase 3 Plan 04: Comparison window state
    #[cfg(feature = "snapshot")]
    pub comparison_state: Option<crate::ui::comparison::ComparisonWindow>,
}

impl DiskReviewerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let drives = drives::enumerate_drives();

        #[cfg(feature = "snapshot")]
        let (snapshot_manager, status_message) = {
            let db_path = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("disk_reviewer")
                .join("snapshots.db");
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            match SnapshotStorage::new(&db_path) {
                Ok(storage) => (Some(storage), "就绪".to_string()),
                Err(e) => {
                    (None, format!("快照数据库初始化失败: {}", e))
                }
            }
        };

        #[cfg(not(feature = "snapshot"))]
        let status_message = "就绪".to_string();

        Self {
            drives,
            scan_result: None,
            scan_progress: None,
            event_receiver: None,
            status_message,
            cancel_token: None,
            nav_stack: Vec::new(),
            selected_index: None,
            treemap_nodes: Vec::new(),
            needs_rebuild: false,
            last_canvas_rect: None,
            pending_resize: None,
            #[cfg(feature = "snapshot")]
            snapshot_manager,
            #[cfg(feature = "snapshot")]
            snapshot_dialog_open: false,
            #[cfg(feature = "snapshot")]
            snapshot_dialog_state: crate::ui::snapshot_dialog::SnapshotDialog::default(),
            #[cfg(feature = "snapshot")]
            comparison_state: None,
        }
    }

    fn start_scan(&mut self, path: PathBuf) {
        // WR-04: 取消前一次扫描，防止多线程并发
        if let Some(token) = &self.cancel_token {
            token.store(true, Ordering::SeqCst);
        }

        let (sender, receiver) = bounded::<ScanEvent>(256);
        self.event_receiver = Some(receiver);
        self.status_message = format!("正在扫描: {}", path.display());
        self.scan_result = None;
        self.scan_progress = None;

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_token = Some(cancel.clone());

        // 在后台线程启动扫描（UI 线程保持响应）
        // scan_directory() 内部使用 rayon::scope() 并行遍历子目录（D-01）
        thread::spawn(move || {
            let start = std::time::Instant::now();
            match scan_directory(&path) {
                Ok(mut root) => {
                    // 检查是否已被取消
                    if cancel.load(Ordering::SeqCst) {
                        return;
                    }
                    // SCAN-05: 执行 Others 聚合后处理
                    let thresholds = AggThresholds::default();
                    root.finish(&thresholds);

                    let total_files = root.file_count;
                    let access_denied_count = count_access_denied(&root);
                    // 通过 channel 推送完成事件（SCAN-03 增量推送）
                    sender.send(ScanEvent::Complete {
                        root,
                        duration: start.elapsed(),
                        total_files,
                        access_denied_count,
                    }).ok();
                }
                Err(e) => {
                    if !cancel.load(Ordering::SeqCst) {
                        sender.send(ScanEvent::Error {
                            path: path.clone(),
                            error: e,
                        }).ok();
                    }
                }
            }
        });
    }

    fn consume_events(&mut self, ctx: &egui::Context) {
        // Take the receiver out temporarily to avoid borrow conflicts
        let mut receiver_done = false;
        let mut needs_rebuild = false;
        if let Some(receiver) = self.event_receiver.take() {
            let mut count = 0;
            loop {
                match receiver.try_recv() {
                    Ok(event) => {
                        match &event {
                            ScanEvent::Complete { root, duration, total_files, access_denied_count } => {
                                self.scan_result = Some(Arc::new(root.clone()));
                                self.nav_stack = Vec::new(); // 空 = 根层级
                                needs_rebuild = true;
                                self.status_message = format!(
                                    "扫描完成: {} 个文件, 耗时 {:.1}s, {} 个目录无权限",
                                    total_files,
                                    duration.as_secs_f64(),
                                    access_denied_count
                                );
                            }
                            ScanEvent::AccessDenied { path } => {
                                self.status_message = format!("无权限: {}", path.display());
                            }
                            _ => {}
                        }
                        self.scan_progress = Some(event);
                        count += 1;
                        if count >= 100 {
                            break; // 每帧最多消费 100 个事件
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        receiver_done = true;
                        break;
                    }
                }
            }
            if !receiver_done {
                self.event_receiver = Some(receiver);
            }
            if count > 0 {
                ctx.request_repaint();
            }
        }
        if needs_rebuild {
            self.needs_rebuild = true;
        }
    }

    /// 扫描期间持续请求重绘，确保切换回应用时 UI 能立即更新
    fn request_repaint_while_scanning(&self, ctx: &egui::Context) {
        if self.event_receiver.is_some() {
            // 扫描仍在进行，约 200ms 后再次请求重绘
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    fn current_dir(&self) -> Option<&DirNode> {
        let root = self.scan_result.as_ref()?;
        let mut current = root.as_ref();
        for &idx in &self.nav_stack {
            current = current.children.get(idx).and_then(|e| match e {
                Entry::Dir(d) => Some(d),
                _ => None,
            })?;
        }
        Some(current)
    }

    fn drill_down(&mut self, child_index: usize) {
        if let Some(dir) = self.current_dir() {
            if let Some(node) = self.treemap_nodes.get(child_index) {
                let orig_idx = node.entry_index;
                if let Some(crate::scanner::Entry::Dir(_)) = dir.children.get(orig_idx) {
                    self.nav_stack.push(orig_idx);
                    self.selected_index = None;
                    self.needs_rebuild = true;
                }
            }
        }
    }

    fn navigate_to_depth(&mut self, depth: usize) {
        self.nav_stack.truncate(depth);
        self.selected_index = None;
        self.needs_rebuild = true;
    }

    fn rebuild_treemap(&mut self, canvas: Rect) {
        if let Some(dir) = self.current_dir() {
            self.treemap_nodes = crate::treemap::layout_treemap(dir, canvas);
        } else {
            self.treemap_nodes.clear();
        }
    }

    /// Load a snapshot into the treemap view (replaces scan_result).
    #[cfg(feature = "snapshot")]
    fn load_snapshot_into_view(&mut self, snapshot_id: i64) {
        if let Some(manager) = &self.snapshot_manager {
            match manager.load_snapshot(snapshot_id) {
                Ok(root) => {
                    self.scan_result = Some(Arc::new(root));
                    self.nav_stack.clear();
                    self.selected_index = None;
                    self.needs_rebuild = true;
                    self.snapshot_dialog_open = false;
                    self.status_message = format!("已加载快照 #{}", snapshot_id);
                }
                Err(e) => {
                    self.status_message = format!("加载快照失败: {}", e);
                }
            }
        }
    }

    /// Save current scan result as a snapshot.
    #[cfg(feature = "snapshot")]
    fn save_current_snapshot(&mut self, name: &str) {
        if let (Some(manager), Some(scan_result)) = (&mut self.snapshot_manager, &self.scan_result) {
            match manager.save_snapshot(name, scan_result) {
                Ok(id) => {
                    self.status_message = format!("快照已保存: {} (#{})", name, id);
                }
                Err(e) => {
                    self.status_message = format!("保存快照失败: {}", e);
                }
            }
        } else if self.scan_result.is_none() {
            self.status_message = "没有可保存的扫描结果".to_string();
        }
    }

    /// Open the comparison window for a given snapshot.
    #[cfg(feature = "snapshot")]
    fn open_comparison(&mut self, snapshot_id: i64, snapshot_name: String) {
        if let Some(manager) = &self.snapshot_manager {
            match manager.load_snapshot(snapshot_id) {
                Ok(root) => {
                    self.comparison_state = Some(crate::ui::comparison::ComparisonWindow {
                        open: true,
                        snapshot_id,
                        snapshot_name,
                        snapshot_root: Some(Arc::new(root)),
                        left_nav_stack: Vec::new(),
                        right_nav_stack: Vec::new(),
                        left_selected: None,
                        right_selected: None,
                        diff_cache: None,
                    });
                    self.snapshot_dialog_open = false;
                }
                Err(e) => {
                    self.status_message = format!("加载快照失败: {}", e);
                }
            }
        }
    }
}

impl eframe::App for DiskReviewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.consume_events(ctx);
        self.request_repaint_while_scanning(ctx);

        // 面包屑在顶部 — take-and-restore 模式避免借用冲突
        let nav_action: Option<usize> = self.scan_result.as_ref().and_then(|root| {
            let mut depth = None;
            egui::TopBottomPanel::top("breadcrumb").show(ctx, |ui| {
                depth = breadcrumb_ui(ui, root, &self.nav_stack);
            });
            depth
        });
        if let Some(d) = nav_action {
            self.navigate_to_depth(d);
        }

        // 右侧详情面板 (D-14: 固定宽度 320px)
        // 使用 take-and-restore 模式处理列表交互
        let mut list_action = crate::ui::info_panel::FileListAction::None;
        let selected_for_panel = self.selected_index.and_then(|i| self.treemap_nodes.get(i));
        egui::SidePanel::right("detail_panel")
            .exact_width(320.0)
            .show(ctx, |ui| {
                list_action = crate::ui::info_panel::info_panel_ui(
                    ui,
                    selected_for_panel,
                    self.current_dir(),
                    &self.treemap_nodes,
                );
            });

        // 处理文件列表交互（在闭包外，可修改 self）
        let list_handled = match list_action {
            crate::ui::info_panel::FileListAction::Select(i) => {
                self.selected_index = Some(i);
                true
            }
            crate::ui::info_panel::FileListAction::Drill(i) => {
                self.drill_down(i);
                true
            }
            crate::ui::info_panel::FileListAction::None => false,
        };

        // 左侧 Treemap 画布 (D-14: 剩余 ~70%)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Disk Reviewer");
            ui.separator();

            // 驱动器列表（保留）
            ui.label("逻辑盘:");
            let scan_requests: Vec<PathBuf> = self.drives.iter().filter_map(|drive| {
                let mut clicked = false;
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{}: 总计 {:.1} GB  可用 {:.1} GB",
                        drive.letter,
                        drive.total_bytes as f64 / 1e9,
                        drive.free_bytes as f64 / 1e9,
                    ));
                    if ui.button("扫描").clicked() {
                        clicked = true;
                    }
                });
                if clicked {
                    Some(PathBuf::from(format!(r"{}:\", drive.letter)))
                } else {
                    None
                }
            }).collect();
            for path in scan_requests {
                self.start_scan(path);
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                #[cfg(feature = "snapshot")]
                if ui.button("快照").clicked() {
                    self.snapshot_dialog_open = !self.snapshot_dialog_open;
                    if self.snapshot_dialog_open {
                        if let Some(manager) = &self.snapshot_manager {
                            match manager.list_snapshots() {
                                Ok(list) => self.snapshot_dialog_state.snapshots = list,
                                Err(e) => self.status_message = format!("加载快照列表失败: {}", e),
                            }
                        }
                    }
                }
            });

            // 颜色图例（单行横向排列，色块与文字垂直居中对齐）
            ui.horizontal(|ui| {
                use crate::treemap::color::FileCategory;
                for cat in [
                    FileCategory::Document,
                    FileCategory::Image,
                    FileCategory::Video,
                    FileCategory::Audio,
                    FileCategory::Archive,
                    FileCategory::Code,
                    FileCategory::Executable,
                    FileCategory::System,
                    FileCategory::Temp,
                    FileCategory::Other,
                ] {
                    ui.horizontal(|ui| {
                        // 用 painter 在文字基线高度绘制色块，确保垂直居中
                        let text_height = ui.text_style_height(&egui::TextStyle::Body);
                        let swatch_size = 10.0;
                        let y_offset = (text_height - swatch_size) / 2.0;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(swatch_size, text_height),
                            egui::Sense::hover(),
                        );
                        let swatch_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x, rect.min.y + y_offset),
                            egui::vec2(swatch_size, swatch_size),
                        );
                        ui.painter().rect_filled(swatch_rect, egui::CornerRadius::same(1), cat.color());
                        ui.label(egui::RichText::new(cat.label()).size(11.0));
                    });
                }
            });
            ui.separator();

            // Treemap 画布
            if !self.treemap_nodes.is_empty() || self.scan_result.is_some() {
                let canvas_rect = Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(ui.available_width(), ui.available_height().max(200.0)),
                );

                // 布局重建逻辑：
                // 1. 首次/导航/下钻时立即重建
                // 2. 窗口 resize 时：记录待重建尺寸，鼠标释放后才重建（避免拖动卡顿）
                let canvas_changed = self.last_canvas_rect != Some(canvas_rect);
                if self.needs_rebuild {
                    if let Some(_dir) = self.current_dir() {
                        self.rebuild_treemap(canvas_rect);
                    }
                    self.needs_rebuild = false;
                    self.last_canvas_rect = Some(canvas_rect);
                    self.pending_resize = None;
                } else if canvas_changed {
                    // 记录待重建尺寸，等鼠标释放
                    self.pending_resize = Some(canvas_rect);
                }

                // 鼠标释放时执行待处理的 resize 重建
                if let Some(pending_rect) = self.pending_resize {
                    let pointer = ui.input(|i| i.pointer.clone());
                    if !pointer.button_down(egui::PointerButton::Primary) {
                        // 鼠标已释放，执行重建
                        if let Some(_dir) = self.current_dir() {
                            self.rebuild_treemap(pending_rect);
                        }
                        self.last_canvas_rect = Some(pending_rect);
                        self.pending_resize = None;
                    }
                }

                // paint_treemap 返回双击下钻的目录索引或单击选中的索引
                if let Some(action) = paint_treemap(
                    ui, &self.treemap_nodes, self.selected_index, canvas_rect, None,
                ) {
                    match action {
                        TreemapAction::DoubleClick(child_index) => {
                            self.drill_down(child_index);
                            return;
                        }
                        TreemapAction::Click(child_index) => {
                            if child_index == usize::MAX {
                                // 点击空白区域取消选中，但文件列表已处理操作时不取消
                                if !list_handled {
                                    self.selected_index = None;
                                }
                            } else {
                                self.selected_index = Some(child_index);
                            }
                        }
                    }
                }

            }
        });

        // Snapshot management dialog (D-23)
        #[cfg(feature = "snapshot")]
        {
            if self.snapshot_dialog_open {
                let scan_available = self.scan_result.is_some();
                let action = crate::ui::snapshot_dialog::snapshot_dialog_ui(
                    ctx,
                    &mut self.snapshot_dialog_state,
                    scan_available,
                );
                use crate::ui::snapshot_dialog::SnapshotAction;
                match action {
                    SnapshotAction::Create(name) => self.save_current_snapshot(&name),
                    SnapshotAction::Delete(id) => {
                        if let Some(mgr) = &mut self.snapshot_manager {
                            if let Err(e) = mgr.delete_snapshot(id) {
                                self.status_message = format!("删除失败: {}", e);
                            } else {
                                self.status_message = format!("已删除快照 #{}", id);
                            }
                        }
                    }
                    SnapshotAction::Rename(id, name) => {
                        if let Some(mgr) = &mut self.snapshot_manager {
                            if let Err(e) = mgr.rename_snapshot(id, &name) {
                                self.status_message = format!("重命名失败: {}", e);
                            } else {
                                self.status_message = format!("快照已重命名为: {}", name);
                            }
                        }
                    }
                    SnapshotAction::Load(id) => self.load_snapshot_into_view(id),
                    SnapshotAction::OpenComparison(id) => {
                        // Find the snapshot name before calling open_comparison
                        // (avoids borrow conflict with &self.snapshot_manager)
                        let name = self.snapshot_dialog_state.snapshots.iter()
                            .find(|s| s.id == id)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| format!("快照 #{}", id));
                        self.open_comparison(id, name);
                    }
                    SnapshotAction::None => {}
                }
            }
        }

        // Comparison window rendering (after snapshot dialog)
        #[cfg(feature = "snapshot")]
        if let Some(comp) = &mut self.comparison_state {
            let scan = self.scan_result.as_ref().map(|r| r.as_ref());
            crate::ui::comparison::comparison_window_ui(ctx, comp, scan);
        }
    }
}

fn count_access_denied(node: &DirNode) -> u64 {
    let mut count = 0;
    for child in &node.children {
        match child {
            crate::scanner::Entry::AccessDenied { .. } => count += 1,
            crate::scanner::Entry::Dir(d) => count += count_access_denied(d),
            _ => {}
        }
    }
    count
}
