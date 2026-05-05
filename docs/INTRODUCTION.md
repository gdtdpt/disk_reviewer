# disk_reviewer — Project Introduction / 项目介绍

---

## 中文

### 项目概述

**disk_reviewer** 是一款 Windows 桌面磁盘空间可视化审查工具。它以矩形树图（Treemap）的形式直观展示磁盘空间占用分布，让用户一眼看出"谁占了多少空间"。

选择逻辑盘后，应用会异步扫描目录树，将每个目录和文件转换为面积正比于其大小的彩色矩形块。用户可以点击目录块逐层下钻，通过面包屑导航快速返回任意上层，并在右侧详情面板中查看选中项的完整信息。应用还支持将扫描结果保存为历史快照，与之前的数据进行对比，高亮显示新增、删除、增长和缩小的目录。

### 核心功能

| 功能 | 说明 |
|------|------|
| **逻辑盘概览** | 枚举所有 Windows 逻辑盘，显示总空间和可用空间，一键启动扫描 |
| **异步扫描** | 后台线程池遍历目录树，增量推送结果到 UI，扫描过程界面不卡顿 |
| **矩形树图** | 基于 Squarified Treemap 算法（Bruls et al. 2000），矩形长宽比接近 1:1，视觉效果最优 |
| **文件类型着色** | 按 10 种文件类型自动着色：文档、图片、视频、音频、压缩包、代码、可执行文件、系统文件、临时文件、其他 |
| **逐层下钻** | 点击目录矩形进入子目录视图，实时重新布局 |
| **面包屑导航** | 水平滚动路径栏，点击任意路径段快速跳转到对应层级 |
| **详情面板** | 选中色块后显示名称、大小、占比、类型；选中目录时额外显示文件数和子目录数；未选中时显示当前目录摘要 |
| **颜色图例** | 右侧面板始终显示全部 10 种文件类型的颜色方块和中文标签 |
| **历史快照** | 将扫描结果保存到 SQLite 单文件数据库，支持加载历史版本 |
| **差异对比** | 基于目录树结构检测新增/删除/增长/缩小的条目，颜色区分变化类型 |
| **大目录优化** | 小文件自动聚合为 "Others" 条目，避免内存爆炸和渲染卡顿 |

### 技术架构

```
技术栈：Rust + egui (eframe 0.33)
部署方式：单文件 .exe，无运行时依赖
平台 API：Win32（FindFirstFileExW、GetLogicalDrives、GetDiskFreeSpaceExW）
快照存储：SQLite 单文件数据库
扫描策略：异步线程池 + 增量推送
开发规范：TDD（测试驱动开发），关键路径 100% 覆盖率
```

### 代码结构

```
src/
├── main.rs              # 入口，eframe 应用启动
├── app.rs               # 应用状态管理（扫描结果、导航栈、选中状态、Treemap 节点）
├── scanner/             # 磁盘扫描引擎
│   ├── mod.rs           # 扫描协调器
│   ├── types.rs         # DirNode / Entry / FileEntry 数据结构
│   ├── walker.rs        # Win32 FindFirstFileExW 异步遍历
│   └── error.rs         # 错误类型
├── treemap/             # Treemap 布局 + 渲染
│   ├── types.rs         # TreemapNode 结构体（9 字段）
│   ├── layout.rs        # Squarified 布局算法
│   ├── color.rs         # FileCategory 枚举 + 颜色映射 + 分类函数
│   └── renderer.rs      # egui Painter 渲染（矩形、标签、选中高亮、悬停 tooltip）
├── ui/                  # UI 组件
│   ├── breadcrumb.rs    # 面包屑导航
│   └── info_panel.rs    # 右侧详情面板 + 颜色图例
├── snapshot/            # 快照存储（SQLite）+ 差异对比
└── platform/            # Windows 平台层
    └── drives.rs        # 逻辑盘枚举、磁盘元信息
```

### 开发进度

| 阶段 | 状态 | 说明 |
|------|------|------|
| Phase 1: 扫描引擎 | ✅ 完成 | 异步目录遍历、权限处理、Others 聚合 |
| Phase 2: Treemap 可视化 | ✅ 完成 | 布局算法、颜色映射、渲染器、下钻交互、面包屑、详情面板 |
| Phase 3: 快照与对比 | ⏳ 待开始 | SQLite 存储、差异检测、高亮显示 |

### 设计决策

| 决策 | 理由 |
|------|------|
| Rust + egui 而非 C++/Qt | 单二进制部署、内存安全、Win32 调用同样直接、无 GC 停顿 |
| 即时模式 GUI (egui) | Treemap 需要完全自定义绘制，即时模式比保留模式更灵活 |
| SQLite 存储快照 | 单文件、零配置、适合本地数据存储 |
| Squarified Treemap 算法 | 矩形长宽比接近 1:1，视觉效果最优 |
| 异步扫描 + 增量推送 | 避免大目录扫描时 UI 冻结 |

### 不在范围内

- 磁盘管理功能（删除/移动文件）— 当前版本只做可视化浏览
- 实时刷新 — 按需扫描，不做文件系统监控
- 网络/远程磁盘 — 仅本地逻辑盘
- 文件类型分类统计 — 未来版本考虑
- 导出报告 — 未来版本考虑

---

## English

### Project Overview

**disk_reviewer** is a Windows desktop disk space visualization tool. It uses a Treemap to provide an intuitive, at-a-glance view of disk space distribution — answering the question "what's taking up all my space?"

After selecting a logical drive, the application asynchronously scans the directory tree and converts each directory and file into colored rectangles whose areas are proportional to their sizes. Users can click directory blocks to drill down level by level, use breadcrumb navigation to quickly return to any parent level, and view complete information about the selected item in the right-side detail panel. The application also supports saving scan results as historical snapshots, comparing them with previous data, and highlighting added, deleted, grown, and shrunk directories.

### Core Features

| Feature | Description |
|---------|-------------|
| **Drive Overview** | Enumerates all Windows logical drives, shows total and free space, one-click scan start |
| **Async Scanning** | Background thread pool traverses the directory tree, incrementally pushing results to the UI without freezing the interface |
| **Treemap Visualization** | Based on the Squarified Treemap algorithm (Bruls et al. 2000), producing rectangles with near 1:1 aspect ratios for optimal visual clarity |
| **File Type Coloring** | Automatically assigns colors based on 10 file categories: Document, Image, Video, Audio, Archive, Code, Executable, System, Temp, Other |
| **Drill-Down Navigation** | Click a directory rectangle to descend into its subdirectory view, with real-time re-layout |
| **Breadcrumb Navigation** | Horizontal scrollable path bar; click any segment to jump directly to that level |
| **Detail Panel** | Shows name, size, percentage, and type for the selected block; for directories, additionally shows file count and subdirectory count; when nothing is selected, shows current directory summary |
| **Color Legend** | Right-side panel always displays color swatches with Chinese labels for all 10 file categories |
| **Historical Snapshots** | Saves scan results to a single-file SQLite database; supports loading previous versions |
| **Diff Comparison** | Detects added/deleted/grown/shrunk entries based on directory tree structure; color-codes change types |
| **Large Directory Optimization** | Small files are automatically aggregated into "Others" entries to prevent memory exhaustion and rendering lag |

### Technical Architecture

```
Tech Stack:    Rust + egui (eframe 0.33)
Deployment:    Single-file .exe, no runtime dependencies
Platform API:  Win32 (FindFirstFileExW, GetLogicalDrives, GetDiskFreeSpaceExW)
Snapshot DB:   SQLite single-file database
Scan Strategy: Async thread pool + incremental push
Dev Discipline: TDD (Test-Driven Development), 100% coverage on critical paths
```

### Code Structure

```
src/
├── main.rs              # Entry point, eframe app startup
├── app.rs               # Application state (scan results, nav stack, selection, treemap nodes)
├── scanner/             # Disk scanning engine
│   ├── mod.rs           # Scan coordinator
│   ├── types.rs         # DirNode / Entry / FileEntry data structures
│   ├── walker.rs        # Win32 FindFirstFileExW async traversal
│   └── error.rs         # Error types
├── treemap/             # Treemap layout + rendering
│   ├── types.rs         # TreemapNode struct (9 fields)
│   ├── layout.rs        # Squarified layout algorithm
│   ├── color.rs         # FileCategory enum + color mapping + categorization functions
│   └── renderer.rs      # egui Painter rendering (rectangles, labels, selection highlight, hover tooltip)
├── ui/                  # UI components
│   ├── breadcrumb.rs    # Breadcrumb navigation
│   └── info_panel.rs    # Right-side detail panel + color legend
├── snapshot/            # Snapshot storage (SQLite) + diff comparison
└── platform/            # Windows platform layer
    └── drives.rs        # Logical drive enumeration, disk metadata
```

### Development Progress

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1: Scan Engine | ✅ Complete | Async directory traversal, permission handling, Others aggregation |
| Phase 2: Treemap Visualization | ✅ Complete | Layout algorithm, color mapping, renderer, drill-down, breadcrumb, detail panel |
| Phase 3: Snapshots & Comparison | ⏳ Pending | SQLite storage, diff detection, highlight display |

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Rust + egui instead of C++/Qt | Single-binary deployment, memory safety, equally direct Win32 calls, no GC pauses |
| Immediate-mode GUI (egui) | Treemap requires fully custom rendering; immediate mode is more flexible than retained mode |
| SQLite for snapshot storage | Single-file, zero-configuration, ideal for local data storage |
| Squarified Treemap algorithm | Produces rectangles with near 1:1 aspect ratios for optimal visual clarity |
| Async scanning + incremental push | Prevents UI freeze during large directory scans |

### Out of Scope

- Disk management (delete / move files) — visualization only in the current version
- Real-time refresh — on-demand scanning, no filesystem monitoring
- Network / remote disks — local logical drives only
- File type statistics — future version consideration
- Report export — future version consideration
