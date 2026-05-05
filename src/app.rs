use crossbeam_channel::{bounded, Receiver, TryRecvError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::platform::drives::{self, DriveInfo};
use crate::scanner::{scan_directory, AggThresholds, DirNode, Entry, ScanEvent};
use crate::treemap;

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
}

impl DiskReviewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let drives = drives::enumerate_drives();
        Self {
            drives,
            scan_result: None,
            scan_progress: None,
            event_receiver: None,
            status_message: "就绪".to_string(),
            cancel_token: None,
            nav_stack: Vec::new(),
            selected_index: None,
            treemap_nodes: Vec::new(),
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
        let mut receiver_match = false;
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
                        receiver_match = true; // signal that receiver is done
                        break;
                    }
                }
            }
            if !receiver_match {
                self.event_receiver = Some(receiver);
            }
            if count > 0 {
                ctx.request_repaint();
            }
        }
        if needs_rebuild {
            self.rebuild_treemap();
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

    fn rebuild_treemap(&mut self) {
        if let Some(dir) = self.current_dir() {
            // plan 02-02 实现 layout_treemap 后替换
            // self.treemap_nodes = crate::treemap::layout_treemap(dir);
            let _ = dir; // suppress unused warning until layout_treemap exists
        }
    }
}

impl eframe::App for DiskReviewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 消费扫描事件
        self.consume_events(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Disk Reviewer");
            ui.separator();

            // 驱动器列表
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
            ui.label(&self.status_message);

            // 扫描结果预览
            if let Some(result) = &self.scan_result {
                ui.label(format!(
                    "根目录: {}  总大小: {:.1} MB  文件数: {}",
                    result.path.display(),
                    result.total_size as f64 / 1e6,
                    result.file_count
                ));
            }
        });
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
