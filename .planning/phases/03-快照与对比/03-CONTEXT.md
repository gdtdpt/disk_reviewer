# Phase 3: 快照与对比 - Context

**Gathered:** 2026-05-06
**Status:** Ready for planning

## Phase Boundary

保存扫描快照到 SQLite，支持历史对比并高亮差异。用户可以将当前扫描结果保存为命名快照，加载历史快照，并在对比视图中查看两个快照之间的差异（新增/删除/增长/缩小）。

**本阶段范围**：SQLite 快照存储、快照序列化/反序列化、差异检测算法、对比视图渲染、快照管理对话框。

## Implementation Decisions

### 快照存储格式
- **D-16:** 路径索引 + JSON 子树。每个目录节点单独存一条记录，key 为完整路径（如 `C:\Users\Alice`），value 为该节点的子树 JSON。对比时只加载两个快照中相同路径的子树，支持按需下钻对比，不需要加载整棵树。
- **D-17:** 快照替换时整体清理。每个快照有唯一 ID，保存新快照时先删除同 ID 的旧记录再写入；快照删除时按 `snapshot_id` 批量删除。不会有垃圾数据。
- **D-18:** 快照默认名称带创建时间（如 `快照 2026-05-06 14:30`），用户可重命名。

### 差异检测策略
- **D-19:** 按名称匹配。同一层级中按条目名称匹配（如 `Alice` 匹配 `Alice`），不要求路径一致。简单直接，适合磁盘分析场景。
- **D-20:** 四种变化类型：新增（新快照有、旧快照无）、删除（旧快照有、新快照无）、增长（大小增加）、缩小（大小减少）。

### 差异高亮方式
- **D-21:** 独立对比窗口。新窗口中左右并排显示：左侧当前扫描结果，右侧快照数据。在快照侧的色块上标识四种状态。
- **D-22:** 颜色叠加 + 图标标记 + tooltip。快照侧色块叠加半透明色（新增=绿、删除=红、增长=橙、缩小=蓝），角落加小图标（+、-、↑、↓），鼠标悬停 tooltip 显示变更详情（名称、旧大小、新大小、变化量）。

### 快照管理 UI
- **D-23:** 弹出对话框。点击工具栏「快照」按钮弹出模态对话框，列出所有快照（名称、时间、大小），支持创建、删除、切换、重命名。不占用主界面空间。

### Claude's Discretion
- SQLite schema 具体设计（表结构、索引）
- JSON 序列化格式（是否需要自定义 Serialize/Deserialize）
- 对比窗口的具体布局比例
- 差异图标的具体样式和位置
- 快照对话框的 UI 细节

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 项目规范
- `.planning/PROJECT.md` — 项目目标、核心约束、技术栈决策
- `.planning/REQUIREMENTS.md` — SNAP-01 ~ SNAP-05 需求定义
- `.planning/ROADMAP.md` — Phase 3 目标和成功标准
- `.planning/config.json` — `workflow.tdd_mode: true`，TDD 强制执行
- `docs/TDD_ENFORCEMENT.md` — TDD 执行规范、适用性矩阵、提交约定
- `docs/PROJECT.md` — 完整技术方案文档

### 先验阶段上下文
- `.planning/phases/01-扫描引擎/01-CONTEXT.md` — Phase 1 决策（D-01~D-05），特别是 DirNode/Entry 数据结构设计
- `.planning/phases/02-treemap/02-CONTEXT.md` — Phase 2 决策（D-06~D-15），特别是 TreemapNode 结构和渲染方式

## Existing Code Insights

### Reusable Assets
- `src/scanner/types.rs` — `DirNode`, `Entry`, `FileEntry`, `OthersEntry`：快照序列化的源数据结构。`DirNode` 包含递归的 `children: Vec<Entry>`，`Entry` 有 `size()` 方法。
- `src/snapshot/mod.rs` — 快照模块脚手架（当前为空，Phase 3 在此实现）
- `Cargo.toml` — `rusqlite` (bundled) + `serde` + `serde_json` 已配置，`snapshot` feature flag 已定义
- `src/treemap/types.rs` — `TreemapNode`：对比窗口可复用的渲染数据结构
- `src/treemap/renderer.rs` — `paint_treemap`：对比窗口可参考的渲染逻辑
- `src/app.rs` — `DiskReviewerApp`：已有 `scan_result: Option<Arc<DirNode>>`，快照加载后也存入此字段

### Established Patterns
- 模块结构：`scanner/` → `treemap/` → `snapshot/`，每模块独立
- 错误处理使用 `thiserror`
- 数据模型使用 `#[derive(Debug, Clone)]`
- TDD 流程：RED → GREEN → REFACTOR

### Integration Points
- `app.rs` 中 `scan_result` → 快照保存的数据来源
- 快照加载后写入 `app.rs` 的 `scan_result`，Treemap 自动渲染
- 对比窗口从 `app.rs` 获取当前树和快照树，并排渲染
- 快照对话框通过 `app.rs` 触发快照的创建/删除/切换/重命名

## Specific Ideas

- 类似 WinDirStat 的快照对比体验：左右并排，差异一目了然
- 对比窗口中支持下钻同步：左侧进入子目录，右侧同步进入对应目录
- tooltip 显示变更详情：名称、旧大小、新大小、变化量（如 `+1.2 GB`）

## Deferred Ideas

- **差异过滤**（只看新增/只看删除等）→ 后续迭代
- **快照导出/导入** → Phase 4
- **快照自动定时创建** → Phase 4
- **磁盘管理功能（打开位置、删除）** → Phase 4（MGMT-01~03）
- **过滤与搜索** → Phase 4（FILT-01~04）

---

*Phase: 03-快照与对比*
*Context gathered: 2026-05-06*
