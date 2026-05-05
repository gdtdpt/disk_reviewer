# Phase 2: Treemap 可视化 - Context

**Gathered:** 2026-05-05
**Status:** Ready for planning

## Phase Boundary

扫描完成后以矩形树图（Treemap）展示磁盘空间占比，支持下钻浏览。将 Phase 1 输出的目录树数据转换为可视化矩形布局，通过颜色、标签、交互让用户一眼看出"谁占了多少空间"。

**本阶段范围**：Treemap 布局算法、egui 渲染、下钻交互、面包屑导航、详情面板、选中高亮。

## Implementation Decisions

### 布局算法
- **D-06:** 自研 Squarified Treemap 算法，从零实现，完全掌控布局和性能，无额外依赖。纯 Rust 实现，代码量约 ~200 行。
- **D-07:** 创建独立的 `TreemapNode` 数据结构（含 rect(x,y,w,h)、label、color、depth、关联 Entry 引用）。布局算法输出 `Vec<TreemapNode>`，渲染器消费。布局与扫描数据解耦。
- **D-08:** 布局算法消费 Phase 1 输出的 `DirNode` 递归树作为输入。每次下钻时重新运行布局算法，只布局当前目录的 children，不做预计算。

### 颜色映射
- **D-09:** 按文件类型分配颜色（蓝色=文档，绿色=媒体，红色=系统文件等）。目录颜色由其主导文件类型决定（统计目录下各类型占比，用占比最高的类型颜色代表整个目录）。
- **D-10:** 颜色-类型对应关系以图例形式显示在右侧详情面板中。

### 标签渲染
- **D-11:** 矩形面积小于阈值时完全不显示标签，仅显示色块。鼠标悬停时用 tooltip 显示完整名称和大小。

### 交互与导航
- **D-12:** 单击目录矩形直接进入子目录视图，面包屑同步更新。简单直接，符合大多数 Treemap 工具习惯。
- **D-13:** 面包屑可点击路径段（如 `C: > Users > Documents`），每段都可点击，直接跳转到对应层级。

### UI 布局
- **D-14:** 左侧 Treemap 画布占窗口 70%，右侧固定宽度面板显示选中项详情（路径、大小、占比、文件数）。面包屑在顶部。
- **D-15:** 颜色图例放在右侧详情面板中，位于选中项信息下方。

### Claude's Discretion
- 具体文件类型到颜色的映射表（类型分组和对应 RGB 值）
- Squarified 算法的具体实现细节（数据结构、递归策略）
- 标签面积阈值的具体数值（像素大小）
- 详情面板的具体宽度
- egui 颜色映射的具体实现方式

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 项目规范
- `.planning/PROJECT.md` — 项目目标、核心约束、技术栈
- `.planning/REQUIREMENTS.md` — VIS-01 ~ VIS-05 需求定义
- `.planning/ROADMAP.md` — Phase 2 目标和成功标准
- `.planning/phases/01-扫描引擎/01-CONTEXT.md` — Phase 1 决策（D-01~D-05），特别是数据结构设计

### 算法参考
- Squarified Treemap 算法：Bruls, Huizing, van Wijk (2000) — "Squarified Treemaps"（经典论文，布局算法核心参考）

## Existing Code Insights

### Reusable Assets
- `src/scanner/types.rs` — `DirNode`, `Entry`, `FileEntry`, `OthersEntry`：Treemap 布局算法的输入数据结构。`DirNode` 包含递归的 `children: Vec<Entry>`，`Entry` 有 `size()` 方法返回 u64。
- `src/app.rs` — `DiskReviewerApp`：已有 egui 窗口框架、驱动器列表、扫描结果存储（`scan_result: Option<Arc<DirNode>>`）、状态消息。Phase 2 在此基础上添加 Treemap 渲染。
- `src/main.rs` — eframe 入口，中文字体已配置。

### Established Patterns
- Phase 1 建立的模块结构：`scanner/` 模块输出 `DirNode` 树，`treemap/` 模块消费并布局，`ui/` 模块渲染。
- 错误处理使用 `thiserror`（见 `scanner/error.rs`）。
- 数据模型使用 `#[derive(Debug, Clone)]`。

### Integration Points
- `app.rs` 中 `scan_result: Option<Arc<DirNode>>` → Treemap 布局算法的输入
- Treemap 渲染在 eframe `update()` 的 `CentralPanel` 内，使用 egui 的 `Painter` API 绘制矩形
- 下钻状态（当前浏览路径）存储在 app.rs 中，通过 `scan_result` 的子树切换实现

## Specific Ideas

- 类似 WinDirStat / SpaceSniffer 的视觉风格：彩色矩形、可下钻、右侧详情
- 目录块用主导类型色 — 混合内容的目录统计各类型占比，用占比最高的类型颜色
- 图例始终可见于详情面板，不需要额外弹出窗口

## Deferred Ideas

- **快照对比视图中的差异高亮** → Phase 3（SNAP-04）
- **磁盘管理功能（打开位置、删除）** → Phase 4（MGMT-01~03）
- **过滤与搜索** → Phase 4（FILT-01~04）

---

*Phase: 02-Treemap 可视化*
*Context gathered: 2026-05-05*
