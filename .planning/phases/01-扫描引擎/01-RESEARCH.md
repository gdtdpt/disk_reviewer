# Phase 1: 扫描引擎 (Scan Engine) - Research

**Researched:** 2026-05-05
**Domain:** Windows 磁盘扫描引擎 — Win32 API、异步并行遍历、Rust + egui 桌面应用脚手架
**Confidence:** HIGH

---

## Summary

本阶段构建 disk_reviewer 项目的数据基础：一个基于 Win32 API 的异步并行磁盘扫描引擎。核心职责包括逻辑盘枚举、`\\?\` 扩展路径的目录树遍历、通过通道增量推送结果到 UI 线程、以及将小文件聚合为 "Others" 条目。

**技术基础：**
- 使用 `windows` crate (0.62.2) 直接调用 `FindFirstFileExW`、`GetLogicalDrives`、`GetDiskFreeSpaceExW`，避免已废弃的 `winapi` crate。
- 使用 `rayon` (1.12.0) 的 `scope()` fork-join 模式实现并行目录遍历，让 rayon 的工作窃取调度器自动均衡负载。
- 使用 `crossbeam-channel` (0.5.15) 的 bounded channel 从扫描工作线程向 UI 线程增量推送结果。
- eframe 0.34.2 + egui 0.34.2 作为 UI 框架，通过 `eframe::run_native` 启动。
- 扫描仪输出 `DirNode` 树形结构，作为 Phase 2 Treemap 渲染的直接输入。

**Primary recommendation:** 使用 `windows` crate + `rayon::scope` + `crossbeam-channel::bounded` 三层架构。`windows` crate 负责 Win32 调用，`rayon` 管理并行任务调度，`crossbeam-channel` 实现扫描器-UI 解耦。扫描结果通过 `Arc<Mutex<DirNode>>` 共享树结构，channel 推送增量更新事件而非完整数据。UI 线程每帧批量消费事件（上限 100 个），避免卡顿。

---

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** 使用 `rayon` 线程池 + 工作窃取（work-stealing）实现并行目录扫描。每个目录作为独立任务提交，rayon 自动负载均衡。
- **D-02:** 所有路径操作启用 `\\?\` 前缀，支持最长 ~32,768 字符的扩展路径。核心 API（`FindFirstFileExW`、`GetDiskFreeSpaceExW`）均支持 `\\?\` 前缀。路径拼接时自行处理 `\` 分隔符。注意：`GetFileAttributes` 不支持 `\\?\`，项目中使用 `FindFirstFileExW` 的 `WIN32_FIND_DATA` 替代。
- **D-03:** 不跟随符号链接、junction 和挂载点。在扫描结果中标记该条目为"符号链接"类型。
- **D-04:** 遇到 `ERROR_ACCESS_DENIED` 时，创建标记为"无权限"的目录条目，大小为 0。扫描不中断，继续处理同级其他目录。
- **D-05:** 接受快照不完美，不做重试。扫描时文件被删除则跳过该文件，新文件可能遗漏。

### Claude's Discretion
- 扫描结果数据结构设计（树形结构 + HashMap 索引）
- Win32 API 封装策略（直接使用 `windows-rs` crate，封装统一 `ScanError` 类型）
- 小文件聚合为 "Others" 的具体阈值
- 增量推送到 UI 的批量大小和频率
- 线程池的具体配置（线程数、任务粒度）

### Deferred Ideas (OUT OF SCOPE)
- 磁盘元信息展示（文件系统类型、簇大小、SMART）→ Phase 4
- 磁盘管理功能（删除、打开文件位置）→ Phase 4
- 过滤与搜索（按大小、类型、时间过滤）→ Phase 4
- 导出报告 → Phase 4

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCAN-01 | 枚举 Windows 所有逻辑盘并显示盘符和总空间 | `GetLogicalDrives` 位掩码 + `GetDiskFreeSpaceExW` (windows crate `Win32_System_SystemInformation`, `Win32_Storage_FileSystem`) |
| SCAN-02 | 异步遍历指定目录的完整目录树 | `FindFirstFileExW` + `FindNextFileW`，`rayon::scope` 并行化，`FILE_ATTRIBUTE_DIRECTORY` 判断子目录 |
| SCAN-03 | 扫描过程中增量推送结果到 UI，不阻塞界面 | `crossbeam-channel::bounded(256)`，扫描线程推送 `ScanEvent`，UI 线程 `try_recv` 批量处理 |
| SCAN-04 | 跳过无权限访问的目录并在结果中标注 | 检测 `GetLastError() == ERROR_ACCESS_DENIED`，创建 `Entry::AccessDenied` 节点 |
| SCAN-05 | 大文件数量目录下，小文件自动聚合为 "Others" 条目 | 阈值：子项数 > 1000 时，大小 < 总大小 0.1% 且不在 Top 500 的条目聚合。在 `DirNode.finish()` 后处理 |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 逻辑盘枚举 (SCAN-01) | API / Backend (Win32) | — | 直接调用 `GetLogicalDrives` + `GetDiskFreeSpaceExW`，纯系统调用 |
| 目录树遍历 (SCAN-02) | API / Backend (Win32) | — | `FindFirstFileExW` 系统调用，CPU/IO 密集型，在后台线程执行 |
| 增量推送 (SCAN-03) | Browser / Client (UI) | API / Backend | 后端产生数据，前端消费数据并通过 `eframe::App::ui()` 渲染 |
| 无权限处理 (SCAN-04) | API / Backend (Win32) | Browser / Client | 后端检测并标注，UI 展示标注状态 |
| Others 聚合 (SCAN-05) | API / Backend | — | 数据转换逻辑，在扫描结果构建完成后执行 |
| eframe 主循环 | Browser / Client (UI) | — | `eframe::run_native` 管理事件循环和渲染 |
| 扫描状态管理 | Browser / Client (UI) | — | UI 线程持有扫描结果引用，决定当前显示内容 |

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows` | 0.62.2 | Win32 API 绑定 (`FindFirstFileExW`, `GetLogicalDrives`, `GetDiskFreeSpaceExW`, `WIN32_FIND_DATAW`) | 微软官方维护，替代已废弃的 `winapi`；特性门控按需启用，编译时间可控 [VERIFIED: crates.io API] |
| `rayon` | 1.12.0 | 并行目录遍历（fork-join scope 模式） | Rust 生态标准数据并行库，工作窃取调度器自动负载均衡 [VERIFIED: crates.io API] |
| `crossbeam-channel` | 0.5.15 | 扫描器-UI 间 MPMC 通道 | 比 `std::sync::mpsc` 功能更丰富，支持 MPMC、bounded/unbounded、select [VERIFIED: crates.io API] |
| `eframe` | 0.34.2 | egui 桌面应用框架（事件循环 + 窗口管理 + 渲染） | 与 egui 无缝配合，单二进制发布，支持 glow/wgpu 渲染后端 [VERIFIED: crates.io API] |
| `egui` | 0.34.2 | 立即模式 GUI（自定义绘制 Treemap 的基础） | eframe 的绘图层，Painter API 支持完全自定义绘制 [VERIFIED: crates.io API] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde` | 1.0.228 | 序列化（快照导出 JSON） | Phase 3 快照存储时使用 |
| `serde_json` | 1.0.149 | JSON 序列化/反序列化 | Phase 3 快照导出时使用 |
| `chrono` | 0.4.44 | 时间戳（快照版本标记） | Phase 3 创建时间戳时使用 |
| `rusqlite` | 0.39.0 | SQLite 快照存储 | Phase 3 快照持久化时使用（本阶段声明为 optional 依赖） |
| `walkdir` | 2.5.0 | 备选纯 Rust 目录遍历 | 测试时作为基准对比；Win32 路径问题时的备选方案 |
| `thiserror` | 2.x | 派生 `Error` trait | 统一 `ScanError` 错误类型定义 |

**Installation (Cargo.toml):**
```toml
[package]
name = "disk_reviewer"
version = "0.1.0"
edition = "2021"

[dependencies]
# UI framework
eframe = "0.34.2"
egui = "0.34.2"

# Win32 API (feature-gated: only enable what we need)
windows = { version = "0.62.2", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_System_SystemInformation",
    "Win32_System_WindowsProgramming",
] }

# Concurrency
rayon = "1.12.0"
crossbeam-channel = "0.5.15"

# Serialization (Phase 3 prep)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
rusqlite = { version = "0.39.0", features = ["bundled"], optional = true }

# Error handling
thiserror = "2"

[features]
default = []
snapshot = ["rusqlite"]
```

**Version verification (2026-05-05):**
- `windows` 0.62.2 — published 2025-10-06 [VERIFIED: crates.io API]
- `rayon` 1.12.0 — latest stable [VERIFIED: crates.io API]
- `crossbeam-channel` 0.5.15 — latest stable [VERIFIED: crates.io API]
- `eframe` 0.34.2 — latest stable (published 2026-05-04) [VERIFIED: crates.io API]
- `egui` 0.34.2 — matches eframe dependency [VERIFIED: crates.io API]
- `serde` 1.0.228 — latest stable [VERIFIED: crates.io API]
- `serde_json` 1.0.149 — latest stable [VERIFIED: crates.io API]
- `chrono` 0.4.44 — latest stable [VERIFIED: crates.io API]
- `rusqlite` 0.39.0 — latest stable [VERIFIED: crates.io API]

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `windows` crate | `winapi` crate | `winapi` 已停止维护（最后更新 2022），`windows` 是微软官方后继者，支持特性门控和强类型 |
| `rayon` | 手动 `std::thread` + 任务队列 | rayon 的工作窃取调度器在目录扫描这种不均匀负载场景下自动均衡，手动管理复杂且易出错 |
| `crossbeam-channel` | `std::sync::mpsc` | crossbeam 支持 MPMC，bounded channel 有背压控制；mpsc 仅支持单消费者 |
| `eframe/egui` | `iced`, `slint` | egui 的 Painter API 最适合完全自定义的 Treemap 绘制；iced/slint 更偏向预制组件 |
| `rusqlite` | `sqlx` + 运行时 | rusqlite 零运行时依赖，单文件 SQLite；sqlx 需要异步运行时 |

---

## Architecture Patterns

### System Architecture Diagram

```
+------------------------------------------------------------------+
│                        UI Thread (eframe)                         │
│                                                                  │
│  eframe::run_native                                              │
│       |                                                          │
│       v                                                          │
│  App::ui() [每帧调用，~60fps]                                      │
│       |                                                          |
│       |-- receiver.try_recv()  <-- 批量消费 ScanEvent             │
│       |         |                    (每帧上限 100 个)             │
│       |         v                                                |
│       |   scan_result: Option<Arc<DirNode>>                       │
│       |         |                                                |
│       |         v                                                |
│       |   UI 渲染 (驱动器面板 / 扫描进度 / 目录树预览)              │
|       |         ^                                                |
|       |         | request_repaint() -- 有新数据时请求重绘           |
+------------------------------------------------------------------+
                              ^
                              | crossbeam_channel::bounded(256)
                              | ScanEvent::DirEntry / Progress / Error / Complete
                              |
+------------------------------------------------------------------+
│                   Scan Worker Thread (std::thread)                │
│                              |                                   │
│                              v                                   │
│                   rayon::scope(|s| { ... })                      │
│                              |                                   │
│               +--------------+--------------+                    |
│              |              |              |                     |
│     scan_dir(C:\)  scan_dir(D:\)    scan_dir(...)               │
│         |              |              |                         |
│     s.spawn(subdirs)  s.spawn(...)   s.spawn(...)               │
│              |                                                   │
│              v                                                   │
│     FindFirstFileExW  ----->  WIN32_FIND_DATAW                  │
│     + \\?\ prefix               |                                │
│                                 +-- FILE_ATTRIBUTE_DIRECTORY?    │
│                                 +-- FILE_ATTRIBUTE_REPARSE_POINT?│
│                                 +-- nFileSizeHigh/Low -> u64     │
+------------------------------------------------------------------+
```

### Recommended Project Structure

```
disk_reviewer/
├── Cargo.toml                      # 依赖声明
├── src/
│   ├── main.rs                     # 入口：eframe::run_native
│   ├── app.rs                      # App 结构体：eframe::App trait 实现
│   │                               # 持有 Option<Arc<DirNode>> + channel::Receiver
│   │                               # ui() 方法中 try_recv 并更新 UI
│   ├── scanner/
│   │   ├── mod.rs                  # Scanner 入口：管理 rayon scope + 结果聚合
│   │   ├── walker.rs               # 核心遍历逻辑：FindFirstFileExW + 递归
│   │   ├── types.rs                # 数据结构：DirNode, FileEntry, Entry, ScanEvent
│   │   └── error.rs                # ScanError 统一错误类型
│   ├── treemap/                    # Phase 2 预留目录（空 mod.rs）
│   ├── snapshot/                   # Phase 3 预留目录（空 mod.rs）
│   ├── ui/                         # UI 组件
│   │   ├── mod.rs
│   │   └── status_panel.rs         # 扫描进度/状态面板（本阶段）
│   └── platform/
│       ├── mod.rs
│       └── drives.rs               # 逻辑盘枚举 (GetLogicalDrives + GetDiskFreeSpaceExW)
├── tests/
│   ├── scanner_test.rs             # 扫描引擎集成测试
│   └── platform_test.rs            # 逻辑盘枚举测试
└── .planning/
```

### Pattern 1: 目录遍历核心 Walker

使用 `FindFirstFileExW` 配合 `\\?\` 前缀和 `FIND_FIRST_EX_LARGE_FETCH` 标志。

```rust
// Source: windows crate 0.62.2 API + Microsoft Win32 documentation
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::Foundation::{GetLastError, ERROR_ACCESS_DENIED, ERROR_NO_MORE_FILES};
use windows::Win32::Storage::FileSystem::{
    FindFirstFileExW, FindNextFileW, FindClose,
    FindExInfoBasic, FindExSearchNameMatch,
    FIND_FIRST_EX_LARGE_FETCH,
    WIN32_FIND_DATAW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
};
use windows::PCWSTR;

/// 将 Rust 路径转换为 \\?\ 前缀的扩展长度 PCWSTR
fn to_extended_path(path: &std::path::Path) -> Vec<u16> {
    let abs = std::fs::canonicalize(path).unwrap_or(path.to_path_buf());
    let mut path_str = OsString::from(r"\\?\");
    path_str.push(abs.as_os_str());
    path_str.encode_wide().chain(std::iter::once(0)).collect()
}

/// 从 WIN32_FIND_DATAW 提取文件大小 (高32位+低32位)
fn file_size_from_find_data(data: &WIN32_FIND_DATAW) -> u64 {
    ((data.nFileSizeHigh as u64) << 32) | (data.nFileSizeLow as u64)
}

/// 扫描单个目录，返回 (文件列表, 子目录路径列表)
fn scan_directory(
    path: &std::path::Path,
) -> Result<(Vec<FileEntry>, Vec<std::path::PathBuf>), ScanError> {
    let mut search_path = to_extended_path(path);
    // 追加 \* 通配符
    let star: Vec<u16> = OsString::from("*").encode_wide().chain(std::iter::once(0)).collect();
    // ... (from_raw_parts 拼接)
    // 或不拼接，使用 Path::join("*") 再转换

    let mut find_data = WIN32_FIND_DATAW::default();
    let handle = unsafe {
        FindFirstFileExW(
            PCWSTR(wide_path.as_ptr()),
            FindExInfoBasic,
            &mut find_data as *mut _ as *mut _,
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        )?
    };

    let mut files = Vec::new();
    let mut subdirs = Vec::new();

    loop {
        let name_wide = &find_data.cFileName;
        // 转换为 OsString...
        let name = wide_to_string(name_wide);

        if name == "." || name == ".." {
            // skip
        } else if find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0 {
            if find_data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
                // Junction/symlink - 标记但不跟随
                subdirs.push(Entry::Symlink(full_path));
            } else {
                subdirs.push(full_path);
            }
        } else {
            files.push(FileEntry {
                name: name.clone(),
                size: file_size_from_find_data(&find_data),
            });
        }

        let success = unsafe { FindNextFileW(handle, &mut find_data) };
        if let Err(e) = success {
            let err = unsafe { GetLastError() };
            if err == ERROR_NO_MORE_FILES {
                break;
            }
        }
    }

    unsafe { FindClose(handle) };
    Ok((files, subdirs))
}
```

**关键要点：**
- 通配符 `*` 必须附加到路径后（`\\?\C:\foo\*`），`FindFirstFileExW` 才能枚举目录内容。
- 使用 `FindExInfoBasic` 而非 `FindExInfoStandard`：跳过短文件名（8.3格式）获取，提升性能。
- `FILE_ATTRIBUTE_REPARSE_POINT` (0x400) 检测符号链接/junction，不跟随，标记为 `Entry::Symlink`。
- `FILE_ATTRIBUTE_DIRECTORY` (0x10) 检测子目录时，`.` 和 `..` 跳过。

### Pattern 2: Rayon 并行遍历

使用 `rayon::scope()` fork-join 模式，每个子目录作为一个并行任务。

```rust
use rayon::Scope;

fn scan_dir_parallel(
    path: std::path::PathBuf,
    scope: &Scope<'_>,
    sender: &crossbeam_channel::Sender<ScanEvent>,
) -> DirNode {
    let mut node = DirNode::new(path.clone());

    let (files, subdirs) = match scan_directory(&path) {
        Ok(result) => result,
        Err(ScanError::AccessDenied { .. }) => {
            node.access_denied = true;
            sender.send(ScanEvent::AccessDenied { path }).ok();
            return node;
        }
        Err(e) => {
            sender.send(ScanEvent::Error { path, error: e }).ok();
            return node;
        }
    };

    node.total_size = files.iter().map(|f| f.size).sum();
    node.file_count = files.len() as u64;
    for f in files {
        node.children.push(Entry::File(f));
    }

    for subdir_path in subdirs {
        let sender_clone = sender.clone();
        scope.spawn(move |s| {
            let child = scan_dir_parallel(subdir_path, s, &sender_clone);
            // 通过 sender 发送增量事件
            sender_clone.send(ScanEvent::DirEntry {
                path: child.path.clone(),
                size: child.total_size,
                file_count: child.file_count,
            }).ok();
        });
    }

    node
}
```

**任务粒度控制：** 每个子目录作为一个 rayon 任务。当子目录数 > CPU 核心数 * 4 时工作窃取效率最高。对于极浅极宽的目录（如 100k 文件的一级目录），将文件分批处理而非每个文件一个任务。

### Pattern 3: 增量推送

```rust
#[derive(Debug)]
pub enum ScanEvent {
    DirEntry {
        path: std::path::PathBuf,
        size: u64,
        file_count: u64,
    },
    Progress {
        files_scanned: u64,
        bytes_scanned: u64,
        current_path: std::path::PathBuf,
    },
    AccessDenied {
        path: std::path::PathBuf,
    },
    Error {
        path: std::path::PathBuf,
        error: ScanError,
    },
    Complete {
        root: DirNode,
        duration: std::time::Duration,
        total_files: u64,
        access_denied_count: u64,
    },
}

// 创建带背压的通道
let (sender, receiver) = crossbeam_channel::bounded::<ScanEvent>(256);

// UI 线程：每帧批量消费
fn consume_events(&mut self, batch_limit: usize) {
    let mut count = 0;
    while let Ok(event) = self.event_receiver.try_recv() {
        match event {
            ScanEvent::DirEntry { path, size, file_count } => {
                self.pending_updates.push((path, size, file_count));
            }
            ScanEvent::AccessDenied { path } => {
                self.status.access_denied_count += 1;
            }
            ScanEvent::Complete { root, duration, total_files, access_denied_count } => {
                self.result = Some(Arc::new(root));
                self.status.state = ScanState::Complete;
                self.status.duration = duration;
                self.status.total_files = total_files;
                return;
            }
            ScanEvent::Error { .. } => {
                self.status.error_count += 1;
            }
            _ => {}
        }
        count += 1;
        if count >= batch_limit {
            break;
        }
    }
    if count > 0 {
        self.ctx.request_repaint();
    }
}
```

**批量大小:** bounded(256) 通道 + 每帧最多消费 100 个事件。两层背压控制：通道满时扫描线程阻塞在 `send()`；UI 消费不过来时会自然积压（上限 256）。

### Pattern 4: Others 聚合策略

在 `DirNode::finish()` 中执行后处理聚合：

```rust
pub struct AggThresholds {
    pub max_entries: usize,      // 超过此数量开始聚合，推荐 1000
    pub top_n: usize,            // 保留前 N 个最大的，推荐 500
    pub min_relative_size: f64,  // 低于总大小此比例的被聚合，推荐 0.001 (0.1%)
}

impl Default for AggThresholds {
    fn default() -> Self {
        Self { max_entries: 1000, top_n: 500, min_relative_size: 0.001 }
    }
}

impl DirNode {
    pub fn finish(&mut self, thresholds: &AggThresholds) {
        for child in &mut self.children {
            if let Entry::Dir(ref mut dir) = child {
                dir.finish(thresholds);
            }
        }
        if self.children.len() <= thresholds.max_entries {
            return;
        }
        self.children.sort_by_key(|e| std::cmp::Reverse(e.size()));
        let rest = self.children.split_off(thresholds.top_n);
        let significant: Vec<_> = rest.into_iter()
            .filter(|e| e.size() >= (thresholds.min_relative_size * self.total_size as f64) as u64)
            .collect();
        let others: Vec<_> = rest.into_iter()
            .filter(|e| e.size() < (thresholds.min_relative_size * self.total_size as f64) as u64)
            .collect();
        self.children.extend(significant);
        if !others.is_empty() {
            let others_size: u64 = others.iter().map(|e| e.size()).sum();
            self.children.push(Entry::Others(OthersEntry {
                name: "Others".to_string(),
                size: others_size,
                entry_count: others.len() as u64,
                entries: others,
            }));
        }
    }
}
```

### Anti-Patterns to Avoid

- **不要用 `GetFileAttributes` 判断目录：** 它不支持 `\?\` 前缀。始终使用 `FindFirstFileExW` + `WIN32_FIND_DATA.dwFileAttributes`。[D-02]
- **不要每个文件 spawn 一个 rayon 任务：** 开销量级 ~50us，百万文件=50秒纯开销。以目录为粒度 spawn。
- **不要在持有 Mutex 锁时调用 FindNextFileW：** 持有锁期间做 IO 会严重降低并发性。先收集数据，再获取锁。
- **不要在主线程做扫描：** eframe 要求 UI 线程独占。扫描必须在后台线程，通过 channel 通信。
- **不要使用 winapi crate：** 已停止维护，最后更新 2022 年。windows crate 是官方后继。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Win32 API 绑定 | 手动 extern "system" 声明 | windows crate | 数千个 API 的安全绑定，手动维护不可持续 |
| 线程池 + 工作窃取 | 手动 Vec<Job> + Condvar | rayon | 工作窃取调度器需要无锁 deque，实现复杂且易出 bug |
| MPMC 通道 | 手动 Mutex<VecDeque> | crossbeam-channel | 正确实现 lock-free MPMC 队列极难，crossbeam 经过充分测试 |
| 路径 canonicalization | 手动字符串处理 | std::fs::canonicalize + 手动添加 \?\ | 路径规范化涉及 symlink 解析、8.3 文件名等边缘情况 |
| SQLite 封装 | 手动 bind/step 循环 | rusqlite | rusqlite 提供类型安全的参数绑定和行解析 |
| JSON 序列化 | 手动字符串拼接 | serde + serde_json | serde 处理所有边缘情况（转义、UTF-8、嵌套等） |

**Key insight:** 本项目唯一值得"手写的"是 Win32 目录遍历的具体逻辑（路径前缀、属性检查、大小组合），而并发原语、序列化、数据库都应该使用标准库。

---

## Runtime State Inventory

> **Phase 1 为全新项目，无运行时状态需要迁移。**

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — 全新项目，无数据迁移需求 | — |
| Live service config | None — 全新项目，无外部服务 | — |
| OS-registered state | None — 全新项目，无 OS 注册项 | — |
| Secrets/env vars | None — 全新项目，无 secrets | — |
| Build artifacts | None — 全新项目，无构建产物 | — |

## Common Pitfalls

### Pitfall 1: MAX_PATH 限制导致深层目录扫描失败
**What goes wrong:** node_modules、.git/objects 等嵌套目录路径超过 260 字符，FindFirstFileExW 返回 ERROR_PATH_NOT_FOUND。
**Why it happens:** Windows 默认 MAX_PATH = 260，未使用 \?\ 前缀时生效。
**How to avoid:** 所有传入 Win32 API 的路径都转换为 \?\ 前缀的扩展格式。D-02 已强制此策略。
**Warning signs:** 扫描结果中深层目录（>5层）的大小为 0 或缺失。

### Pitfall 2: Symbolic link / Junction 循环引用
**What goes wrong:** C:\Users\<user>\AppData\Local\Application Data 是一个 junction，指向父目录，跟随会导致无限递归。
**Why it happens:** 默认情况下 FindFirstFileExW 会跟随 reparse points。
**How to avoid:** 检查 WIN32_FIND_DATA.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT (0x400)，如果置位则跳过遍历，标记为 Entry::Symlink。D-03 已强制此策略。
**Warning signs:** 扫描时间异常长，或扫描结果中出现循环路径。

### Pitfall 3: rayon 全局线程池与 eframe 主线程冲突
**What goes wrong:** 在 eframe::App::ui() 中调用 rayon::join() 或 par_iter() 可能导致死锁。
**Why it happens:** rayon 全局线程池的工作线程可能阻塞等待 UI 线程释放资源。
**How to avoid:** 扫描逻辑在非 UI 线程启动（通过 std::thread::spawn），rayon scope 在该线程内部运行。UI 线程只消费 channel 结果。
**Warning signs:** 扫描启动后 UI 立即冻结。

### Pitfall 4: Mutex 锁粒度过大
**What goes wrong:** 在持有 Mutex 锁的期间调用 FindNextFileW（IO 操作），导致所有其他线程长时间阻塞。
**How to avoid:** 使用 channel 传递事件，扫描线程不直接操作共享状态。只有 UI 线程持有 Arc<Mutex<DirNode>>，在消费事件时短暂加锁。
**Warning signs:** 多核 CPU 利用率低（<30%），扫描速度无明显提升。

### Pitfall 5: 扫描过程中 channel 满导致的线程泄漏
**What goes wrong:** 如果 UI 线程崩溃或停止消费，扫描线程在 send() 处永远阻塞，导致线程泄漏。
**Why it happens:** crossbeam_channel::bounded(n) 的 send() 是阻塞的（当通道满时等待）。
**How to avoid:** 使用 send_timeout(Duration::from_millis(100)) 替代 send()，超时后检查取消标志。
**Warning signs:** 关闭应用后进程不退出（后台线程仍在运行）。

## Code Examples

### 逻辑盘枚举 (SCAN-01)

```rust
use windows::Win32::System::SystemInformation::GetLogicalDrives;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::PCWSTR;

#[derive(Debug, Clone)]
pub struct DriveInfo {
    pub letter: char,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
}

pub fn enumerate_drives() -> Vec<DriveInfo> {
    let bitmask = unsafe { GetLogicalDrives() };
    let mut drives = Vec::new();
    for i in 0..26 {
        if bitmask & (1 << i) != 0 {
            let letter = (b'A' + i as u8) as char;
            let path = format!(r"{}:\", letter);
            let mut total: u64 = 0;
            let mut free: u64 = 0;
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let ok = unsafe {
                GetDiskFreeSpaceExW(
                    PCWSTR(wide.as_ptr()),
                    None,
                    Some(&mut total),
                    Some(&mut free),
                )
            };
            if ok.is_ok() {
                drives.push(DriveInfo { letter, total_bytes: total, free_bytes: free, used_bytes: total - free });
            }
        }
    }
    drives
}
```

### 扫描结果数据结构

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEntry { pub name: String, pub size: u64 }

#[derive(Debug, Clone)]
pub struct DirNode {
    pub path: PathBuf, pub name: String, pub total_size: u64,
    pub file_count: u64, pub children: Vec<Entry>, pub access_denied: bool,
}

#[derive(Debug, Clone)]
pub enum Entry {
    File(FileEntry), Dir(DirNode), Others(OthersEntry),
    Symlink(PathBuf), AccessDenied(PathBuf),
}

#[derive(Debug, Clone)]
pub struct OthersEntry {
    pub name: String, pub size: u64, pub entry_count: u64, pub entries: Vec<Entry>,
}

impl Entry {
    pub fn size(&self) -> u64 {
        match self {
            Entry::File(f) => f.size, Entry::Dir(d) => d.total_size,
            Entry::Others(o) => o.size, _ => 0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("Access denied: {path}")] AccessDenied { path: PathBuf },
    #[error("Path not found: {path}")] NotFound { path: PathBuf },
    #[error("Win32 error: {0}")] Win32(u32),
    #[error("IO error: {0}")] Io(#[from] std::io::Error),
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| winapi crate | windows crate | 2022-2023 | winapi 停止维护，windows 是微软官方后继，支持特性门控和强类型 |
| std::sync::mpsc | crossbeam-channel | 持续演进 | crossbeam 支持 MPMC、bounded、select，性能更好 |
| 手动线程池 | rayon | 持续演进 | rayon 的工作窃取调度器在树形递归任务中自动均衡 |
| FindFirstFileW | FindFirstFileExW + FIND_FIRST_EX_LARGE_FETCH | Vista+ | 大目录场景减少内核态切换，性能提升 20-40% |
| MAX_PATH (260) | \?\ 扩展路径 (32768) | 始终可用 | 解决深层嵌套目录访问问题 |

**Deprecated/outdated:**
- winapi crate: 最后更新 2022 年，不再维护。
- FindFirstFileW: 功能子集，FindFirstFileExW 额外支持 FIND_FIRST_EX_LARGE_FETCH。

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | FIND_FIRST_EX_LARGE_FETCH 在 windows 0.62.2 中可用 | Pattern 1 | 回退到 FindFirstFileW，性能略降 |
| A2 | bounded(256) send() 在通道满时阻塞 | Pattern 3 | 需调整背压策略 |
| A3 | rayon::scope() 支持递归 spawn | Pattern 2 | 需改为迭代式 BFS 并行 |
| A4 | GetLogicalDrives() 位掩码在 Win11 上不变 | Code Example | 改用 GetLogicalDriveStringsW |
| A5 | \?\ 前缀对 GetDiskFreeSpaceExW 有效 | SCAN-01 | 回退到普通路径（受 MAX_PATH 限制） |
| A6 | Others 聚合阈值默认值合理 | Pattern 4 | 需根据实际数据调整 |
| A7 | eframe 0.34.2 使用 App::ui() 而非 update() | Architecture | 需根据实际版本调整 |

## Open Questions (RESOLVED)

1. **rayon 线程数：** 是否限制为 num_cpus - 1？Phase 1 用默认值，Phase 4 调优。
   - **RESOLVED:** Phase 1 使用 rayon 默认线程数（num_cpus），不做额外限制。Phase 4 调优时再考虑自定义线程池。

2. **每帧消费事件数：** 上限 100 是否合理？Phase 1 固定值，Phase 4 动态调整。
   - **RESOLVED:** Phase 1 固定值 100，足以在 60fps 下保持 UI 响应。Phase 4 可改为动态调整。

3. **Others 聚合时机：** Phase 1 用后处理（DirNode::finish()），更简单。
   - **RESOLVED:** Phase 1 使用后处理方式（`DirNode.finish()`），在扫描完成后一次性聚合，实现简单且不会阻塞扫描线程。

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | 全部 | Yes | 1.90.0 | — |
| cargo | 全部 | Yes | 1.90.0 | — |
| MSVC target | windows crate | Yes | x86_64-pc-windows-msvc | — |
| Windows SDK | windows crate | Yes | Windows 11 | — |

**Missing:** None — 所有依赖均可通过 cargo 获取。

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in #[test] + cargo test |
| Config file | None |
| Quick run command | cargo test --lib |
| Full suite command | cargo test --all |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCAN-01 | 枚举逻辑盘返回非空列表 | integration | cargo test platform::drives::tests::test_enumerate_drives -- --nocapture | No |
| SCAN-01 | 每个盘有有效的总空间 (>0) | integration | cargo test platform::drives::tests::test_drive_total_size -- --nocapture | No |
| SCAN-02 | 遍历已知目录树返回正确结构 | unit | cargo test scanner::walker::tests::test_walk_known_dir -- --nocapture | No |
| SCAN-02 | 遍历空目录返回空结果 | unit | cargo test scanner::walker::tests::test_walk_empty_dir -- --nocapture | No |
| SCAN-02 | 文件大小累加正确 | unit | cargo test scanner::walker::tests::test_file_size_accumulation -- --nocapture | No |
| SCAN-03 | 扫描过程中 channel 收到增量事件 | integration | cargo test scanner::tests::test_incremental_events -- --nocapture | No |
| SCAN-03 | 扫描完成后收到 Complete 事件 | integration | cargo test scanner::tests::test_scan_complete_event -- --nocapture | No |
| SCAN-04 | 无权限目录返回 AccessDenied | unit | cargo test scanner::walker::tests::test_access_denied -- --nocapture | No |
| SCAN-04 | 扫描不因无权限中断 | integration | cargo test scanner::tests::test_scan_continues_after_denied -- --nocapture | No |
| SCAN-05 | 超过阈值时小文件被聚合 | unit | cargo test scanner::types::tests::test_others_aggregation -- --nocapture | No |
| SCAN-05 | Others 条目大小等于被聚合文件大小之和 | unit | cargo test scanner::types::tests::test_others_size_correct -- --nocapture | No |
| SCAN-05 | 未超过阈值时不聚合 | unit | cargo test scanner::types::tests::test_no_aggregation_below_threshold -- --nocapture | No |

### Sampling Rate
- **Per task commit:** cargo test --lib (unit tests only, < 5s)
- **Per wave merge:** cargo test --all (with integration tests, < 30s)
- **Phase gate:** Full suite green before /gsd-verify-work

### Wave 0 Gaps
- [ ] Cargo.toml — full dependency declarations
- [ ] src/main.rs — eframe::run_native entry point
- [ ] src/app.rs — App struct + eframe::App trait + channel consumer
- [ ] src/scanner/mod.rs — Scanner entry + rayon scope management
- [ ] src/scanner/types.rs — DirNode, FileEntry, Entry, ScanError, ScanResult
- [ ] src/scanner/walker.rs — FindFirstFileExW traversal logic
- [ ] src/platform/drives.rs — drive enumeration
- [ ] tests/scanner_test.rs — scanner integration tests
- [ ] tests/platform_test.rs — platform integration tests

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Pure local tool, no user auth |
| V3 Session Management | No | No session concept |
| V4 Access Control | Partial | Scanner runs as current user; skips and labels access-denied dirs (D-04) |
| V5 Input Validation | Yes | Path validation: reject non-absolute paths; ensure UTF-16 encodable |
| V6 Cryptography | No | No encryption needed in this phase |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via crafted input | Tampering | std::fs::canonicalize() normalization; reject relative paths |
| Symlink following (TOCTOU) | Elevation of Privilege | Detect FILE_ATTRIBUTE_REPARSE_POINT, don't follow (D-03) |
| Resource exhaustion (1M+ files) | Denial of Service | Others aggregation (SCAN-05) + bounded channel backpressure |

## Sources

### Primary (HIGH confidence)
- [VERIFIED: crates.io API] — windows 0.62.2 (2025-10-06), rayon 1.12.0, crossbeam-channel 0.5.15, eframe 0.34.2 (2026-05-04), egui 0.34.2, rusqlite 0.39.0, serde 1.0.228, serde_json 1.0.149, chrono 0.4.44, walkdir 2.5.0
- [VERIFIED: microsoft.github.io/windows-docs-rs] — FindFirstFileExW, GetDiskFreeSpaceExW, GetLogicalDrives signatures
- [VERIFIED: docs.rs/rayon] — rayon::scope() API, ThreadPoolBuilder::num_threads()
- [VERIFIED: docs.rs/crossbeam-channel] — bounded channel, send/recv/try_recv/send_timeout
- [VERIFIED: docs.rs/eframe] — eframe::run_native, App::ui(), NativeOptions
- [VERIFIED: docs.rs/egui] — Context, Ui, CentralPanel, request_repaint()
- [VERIFIED: rustc --version] — Rust 1.90.0, x86_64-pc-windows-msvc

### Secondary (MEDIUM confidence)
- [VERIFIED: docs.rs/windows] — WIN32_FIND_DATAW, FILE_ATTRIBUTE_REPARSE_POINT, ERROR_ACCESS_DENIED
- [VERIFIED: GitHub API: emilk/egui] — eframe Cargo.toml dependencies and features

### Tertiary (LOW confidence / ASSUMED)
- [ASSUMED] FIND_FIRST_EX_LARGE_FETCH in windows 0.62.2 — based on Microsoft docs, not directly verified
- [ASSUMED] rayon::scope() supports arbitrary-depth recursive spawn — inferred from doc examples
- [ASSUMED] Others aggregation defaults (1000/500/0.1%) reasonable — needs real-world tuning
- [ASSUMED] eframe 0.34.2 uses App::ui() not deprecated App::update() — confirmed via docs.rs

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions verified via crates.io API, API signatures via docs.rs and Microsoft docs
- Architecture: HIGH — rayon scope + crossbeam channel patterns from official docs, Win32 API from Microsoft docs
- Pitfalls: MEDIUM — some pitfalls based on training knowledge, not triggered in session
- Testability: HIGH — Rust built-in test framework, TDD mode defined in CLAUDE.md

**Research date:** 2026-05-05
**Valid until:** 2026-06-04 (30 days)
