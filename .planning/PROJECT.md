# disk_reviewer

## What This Is

Windows 桌面应用，以矩形树图（Treemap）可视化磁盘空间占用。选择逻辑盘后展示各目录空间占比，点击目录块可逐层下钻，支持历史快照对比并高亮差异。面向需要了解磁盘空间分布的用户。

## Core Value

**直观展示磁盘空间占用，让用户一眼看出"谁占了多少空间"。** 如果所有功能都失败，这个必须工作。

## Requirements

### Validated

(None yet — ship to validate)

### Active

#### 扫描引擎
- [ ] REQ-SCAN-01: 枚举 Windows 逻辑盘（GetLogicalDrives）
- [ ] REQ-SCAN-02: 异步遍历目录树，增量推送结果到 UI
- [ ] REQ-SCAN-03: 跳过无权限目录并标注，不中断扫描
- [ ] REQ-SCAN-04: 大文件数量目录下，小文件聚合为 "Others"

#### Treemap 可视化
- [ ] REQ-VIS-01: 基于空间占比的矩形树图（Squarified Treemap 算法）
- [ ] REQ-VIS-02: 色块显示目录/文件名、大小、占比
- [ ] REQ-VIS-03: 点击进入子目录，面包屑导航返回上层
- [ ] REQ-VIS-04: 选中项详情面板（路径、大小、占比、文件数量）

#### 快照与对比
- [ ] REQ-SNAP-01: 保存扫描快照到 SQLite
- [ ] REQ-SNAP-02: 加载历史快照
- [ ] REQ-SNAP-03: 差异检测（新增/删除/增长/缩小）
- [ ] REQ-SNAP-04: 差异高亮显示（颜色区分变化类型）
- [ ] REQ-SNAP-05: 快照管理（创建/删除/切换）

### Out of Scope

- **磁盘管理功能（删除/移动文件）** — 当前版本只做可视化浏览，管理功能留给未来版本
- **实时刷新** — 按需扫描，不做文件系统监控
- **网络/远程磁盘** — 仅本地逻辑盘
- **文件类型分类统计** — 未来版本考虑
- **导出报告** — 未来版本考虑

## Context

- **目标平台**: Windows 10/11
- **技术栈**: Rust + egui (eframe)，纯本地桌面应用，单二进制发布
- **底层 API**: Win32（FindFirstFileExW、GetLogicalDrives、GetDiskFreeSpaceExW、DeviceIoControl）
- **扫描策略**: 异步线程池 + 增量推送，避免 UI 卡顿
- **快照存储**: SQLite 单文件数据库
- **TDD 约束**: 所有逻辑代码必须走 TDD 流程（RED → GREEN → REFACTOR），详见 docs/TDD_ENFORCEMENT.md
- **用户经验**: 开发者有 Qt/C++ 经验，首次使用 Rust

## Constraints

- **平台**: 仅 Windows — 需要直接调用 Win32 API
- **部署**: 单文件 .exe，无运行时依赖
- **性能**: 扫描与 UI 线程分离，百万级文件不卡顿
- **内存**: 大目录小文件聚合，避免内存爆炸
- **语言**: Rust — 需要接入底层 Win32 API

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust + egui 而非 C++/Qt | 单二进制部署、内存安全、Win32 调用同样直接、无 GC 停顿 | — Pending |
| 即时模式 GUI (egui) 而非保留模式 (Qt) | Treemap 需要完全自定义绘制，即时模式更灵活 | — Pending |
| SQLite 存储快照 | 单文件、零配置、适合本地数据存储 | — Pending |
| Squarified Treemap 算法 | 矩形长宽比接近 1:1，视觉效果最优 | — Pending |
| 异步扫描 + 增量推送 | 避免大目录扫描时 UI 冻结 | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-05 after initialization*
