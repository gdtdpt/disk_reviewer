# Roadmap: disk_reviewer

## Overview

构建 Windows 磁盘空间可视化工具。从底层扫描引擎开始，逐步构建 Treemap 渲染、快照对比，最终交付一个完整的磁盘空间分析工具。

## Milestones

- 📋 **v1.0 MVP** — Phases 1-3（扫描 + 可视化 + 快照对比）

## Phases

- [x] **Phase 1: 扫描引擎** — 异步目录遍历，输出目录树结构 (completed 2026-05-05)
- [ ] **Phase 2: Treemap 可视化** — 矩形树图渲染 + 下钻交互
- [ ] **Phase 3: 快照与对比** — 快照存储 + 差异检测 + 高亮显示

## Phase Details

### Phase 1: 扫描引擎
**Goal**: 能扫描指定目录，异步输出完整的目录树结构
**Depends on**: Nothing (first phase)
**Requirements**: SCAN-01, SCAN-02, SCAN-03, SCAN-04, SCAN-05
**Success Criteria** (what must be TRUE):
	  1. 应用启动后能列出所有逻辑盘（盘符、总空间、可用空间）
	  2. 选择任意目录后开始扫描，UI 不卡顿
	  3. 扫描过程中实时显示已发现的目录和文件
	  4. 无权限目录被跳过并标注，扫描不中断
**Plans**: 4 plans

Plans:
- [x] 01-01-PLAN.md — 项目初始化 + Rust 工程脚手架 [SCAN-01, SCAN-02, SCAN-03]
- [x] 01-02-PLAN.md — 逻辑盘枚举（GetLogicalDrives + GetDiskFreeSpaceExW）[SCAN-01]
- [x] 01-03-PLAN.md — 异步目录遍历器（FindFirstFileExW + 增量推送 + channel）[SCAN-02, SCAN-03]
- [x] 01-04-PLAN.md — Others 聚合 + AccessDenied 处理完善 [SCAN-04, SCAN-05]

### Phase 2: Treemap 可视化
**Goal**: 扫描完成后以矩形树图展示空间占比，支持下钻浏览
**Depends on**: Phase 1
**Requirements**: SCAN-05, VIS-01, VIS-02, VIS-03, VIS-04, VIS-05
**Success Criteria** (what must be TRUE):
	  1. 扫描完成后显示 Treemap 矩形树图，每个色块大小正比于空间占用
	  2. 色块显示目录/文件名、大小、占比
	  3. 点击目录块进入子目录视图
	  4. 面包屑导航显示当前路径，可点击返回任意上层
	  5. 选中色块后在详情面板显示完整信息
**Plans**: 5 plans

Plans:
- [ ] 02-01-PLAN.md — egui 应用框架 + 主窗口布局
- [ ] 02-02-PLAN.md — Squarified Treemap 布局算法
- [ ] 02-03-PLAN.md — Treemap egui 渲染（矩形、标签、颜色映射）
- [ ] 02-04-PLAN.md — 下钻交互 + 面包屑导航
- [ ] 02-05-PLAN.md — 详情面板 + 选中高亮

### Phase 3: 快照与对比
**Goal**: 保存扫描快照，支持历史对比并高亮差异
**Depends on**: Phase 2
**Requirements**: SNAP-01, SNAP-02, SNAP-03, SNAP-04, SNAP-05
**Success Criteria** (what must be TRUE):
	  1. 用户可以将当前扫描结果保存为命名快照
	  2. 可以加载历史快照并在 Treemap 中展示
	  3. 选择两个快照后自动检测差异（新增/删除/增长/缩小）
	  4. 差异在 Treemap 中以不同颜色高亮显示
	  5. 快照管理对话框支持创建、删除、切换快照
**Plans**: 4 plans

Plans:
- [ ] 03-01-PLAN.md — SQLite 快照存储（schema 设计 + 读写）
- [ ] 03-02-PLAN.md — 快照序列化/反序列化 + 快照管理 UI
- [ ] 03-03-PLAN.md — 差异检测算法（树结构对比）
- [ ] 03-04-PLAN.md — 差异高亮渲染 + 快照对比视图

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. 扫描引擎 | v1.0 | 4/4 | Complete   | 2026-05-05 |
| 2. Treemap 可视化 | v1.0 | 0/5 | Not started | - |
| 3. 快照与对比 | v1.0 | 0/4 | Not started | - |
