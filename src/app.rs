use crossbeam_channel::{bounded, Receiver, TryRecvError};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use crate::platform::drives::{self, DriveInfo};
use crate::scanner::{scan_directory, DirNode, ScanEvent};

pub struct DiskReviewerApp {
    pub drives: Vec<DriveInfo>,
    pub scan_result: Option<Arc<DirNode>>,
    pub scan_progress: Option<ScanEvent>,
    event_receiver: Option<Receiver<ScanEvent>>,
    pub status_message: String,
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
        }
    }

    fn start_scan(&mut self, path: PathBuf) {
        let (sender, receiver) = bounded::<ScanEvent>(256);
        self.event_receiver = Some(receiver);
        self.status_message = format!("正在扫描: {}", path.display());
        self.scan_result = None;

        // 在后台线程启动扫描（UI 线程保持响应）
        // scan_directory() 内部使用 rayon::scope() 并行遍历子目录（D-01）
        thread::spawn(move || {
            let start = std::time::Instant::now();
            match scan_directory(&path) {
                Ok(root) => {
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
                    sender.send(ScanEvent::Error {
                        path: path.clone(),
                        error: e,
                    }).ok();
                }
            }
        });
    }

    fn consume_events(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.event_receiver {
            let mut count = 0;
            loop {
                match receiver.try_recv() {
                    Ok(event) => {
                        match &event {
                            ScanEvent::Complete { root, duration, total_files, access_denied_count } => {
                                self.scan_result = Some(Arc::new(root.clone()));
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
                        self.event_receiver = None;
                        break;
                    }
                }
            }
            if count > 0 {
                ctx.request_repaint();
            }
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
