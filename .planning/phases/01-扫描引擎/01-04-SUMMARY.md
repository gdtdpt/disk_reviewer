---
phase: 01-扫描引擎
plan: 04
subsystem: others-aggregation-access-denied
tags: [scanner, aggregation, others, access-denied, finish, treemap-prep]
requires: [01-03]
provides: [DirNode.finish, AggThresholds, OthersEntry, access-denied-count]
affects: [02-01, 03-01]
tech-stack:
  added: []
  patterns: [others-aggregation, recursive-post-processing, access-denied-struct-variant]
key-files:
  created: []
  modified:
    - src/scanner/types.rs
    - src/scanner/mod.rs
    - src/app.rs
decisions:
  - "AggThresholds 默认值: max_entries=1000, top_n=500, min_relative_size=0.001 (SCAN-05)"
  - "DirNode.finish() 在扫描完成后、Complete 事件发送前调用（后处理聚合）"
  - "AccessDenied 使用结构体变体 Entry::AccessDenied { path: PathBuf }，与 walker.rs 一致"
  - "OthersEntry 保留被聚合的原始 entries 向量，支持后续展开查看"
  - "walker.rs AccessDenied 处理已在 01-03 中正确实现，本计划仅确认"
metrics:
  duration: "~15m"
  completed: "2026-05-05"
  tasks: 3
  files: 3
---

# Phase 1 Plan 04: Others 聚合 + AccessDenied 完善 Summary

实现 DirNode.finish() 后处理方法（SCAN-05 小文件聚合为 Others），确认 walker.rs 的 AccessDenied 处理（SCAN-04），并在 app.rs 扫描管道中集成 finish() 调用。Phase 1 所有 5 个需求（SCAN-01 ~ SCAN-05）全部交付。

## 完成情况

### Task 1 [RED]: 为 Others 聚合 + AccessDenied 行为写失败测试
- **Commit:** `5123124`
- **Status:** 完成
- **Files:** `src/scanner/types.rs` (新增 #[cfg(test)] mod tests)
- **测试:** 6 个测试
  - `test_others_aggregation_above_threshold` — 1500 条目超过阈值，验证 Others 产生且条目数减少
  - `test_others_size_correct` — Others.size > 0，finish() 不修改 total_size
  - `test_no_aggregation_below_threshold` — 500 条目低于阈值，不产生 Others
  - `test_others_entry_count` — Others.entry_count > 0，内部条目大小之和等于 Others.size
  - `test_access_denied_entry_size_is_zero` — AccessDenied 条目 size() == 0
  - `test_symlink_entry_size_is_zero` — Symlink 条目 size() == 0
- **RED 确认:** 编译错误，`AggThresholds` 类型和 `DirNode.finish()` 方法不存在

### Task 2 [GREEN]: 实现 DirNode.finish() + AggThresholds
- **Commit:** `7569efa`
- **Status:** 完成
- **Files:** `src/scanner/types.rs`, `src/scanner/mod.rs`
- **实现:**
  - `AggThresholds` 结构体：max_entries=1000, top_n=500, min_relative_size=0.001
  - `DirNode.finish()` 递归后处理方法：
    1. 先递归处理所有子目录
    2. children.len() <= max_entries 时不聚合
    3. 按 size 降序排序，保留 top_n 个
    4. 剩余中 size < total_size * min_relative_size 的聚合为 Others
    5. 剩余中 size >= 阈值的保留
  - `OthersEntry` 结构体已存在（Phase 1-01 脚手架）
  - `Entry::AccessDenied { path: PathBuf }` 结构体变体已存在
  - `scanner/mod.rs` 导出新增的 `AggThresholds` 和 `OthersEntry`
- **验证:** 6 个聚合测试全部通过

### Task 3 [GREEN]: 集成 finish() 到扫描管道 + 确认 AccessDenied 处理
- **Commit:** `27fb62a`
- **Status:** 完成
- **Files:** `src/app.rs`
- **实现:**
  - 在 `start_scan` 闭包中，`scan_directory` 返回 `Ok(mut root)` 后调用 `root.finish(&AggThresholds::default())`
  - finish() 在 Complete 事件发送前执行，确保 UI 端收到的是已聚合的树
  - walker.rs AccessDenied 处理确认正确（01-03 已实现）：
    - 子目录 ScanError::AccessDenied 捕获后插入 Entry::AccessDenied { path }
    - 扫描不中断，继续处理同级其他目录
  - `count_access_denied()` 已正确处理 `Entry::AccessDenied { .. }` 结构体变体
- **验证:** 全部 15 个测试通过，cargo build 通过

## 验证结果

| 检查项 | 结果 |
|--------|------|
| `cargo test scanner::types::tests` 全部通过 | PASS (6/6) |
| `cargo test` 全部通过 | PASS (15/15, 无回归) |
| `cargo build` 通过 | PASS |
| RED commit 存在 (`test(01-04): ...`) | PASS (`5123124`) |
| GREEN commit 存在 (`feat(01-04): ...`) | PASS (`7569efa`, `27fb62a`) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 计划写 "7 tests" 但实际只有 6 个 #[test] 函数**
- **Found during:** Task 1 RED 编写测试
- **Issue:** PLAN.md 头部描述写 "7 tests"，但实际列出的测试函数只有 6 个
- **Fix:** 按实际 6 个测试函数实现，全部通过
- **Files modified:** `src/scanner/types.rs`
- **Commit:** `5123124`

无其他偏差。walker.rs 的 AccessDenied 处理在 01-03 中已正确实现，app.rs 的 count_access_denied() 也已正确处理结构体变体，无需额外修复。

## 已知警告

编译产生多个预期内的 warning（均为预存，非本计划新增）：
- `dead_code`: FileEntry, DirNode 字段，Entry 变体，ScanError 变体，ScanEvent 变体尚未被外部代码使用（Phase 2/3 将自然消除）
- `unused_imports`: mod.rs 中的 FileEntry, OthersEntry, ScanError 重新导出尚未被其他模块引用
- `unused_variables`: app.rs 中的 `cc` 参数（eframe CreationContext）

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: mitigate | src/scanner/types.rs | T-01-04-01: Others 聚合限制内存增长（max_entries=1000 阈值） |
| threat_flag: accept | src/scanner/types.rs | T-01-04-02: AccessDenied 仅标记，不尝试提升权限 |
| threat_flag: accept | src/scanner/types.rs | T-01-04-03: finish() 只排序+聚合，不修改磁盘数据 |

## TDD Gate Compliance

| 门控 | Commit | 状态 |
|------|--------|------|
| RED: `test(01-04): add failing test for Others aggregation and AccessDenied handling` | `5123124` | PASS |
| GREEN: `feat(01-04): implement DirNode.finish() with Others aggregation` | `7569efa` | PASS |
| GREEN: `feat(01-04): integrate finish() into scan pipeline and fix AccessDenied handling` | `27fb62a` | PASS |

REFACTOR 门控：无（本计划无需独立重构步骤）。

## Self-Check: PASSED

- `5123124` 存在 (RED commit)
- `7569efa` 存在 (GREEN commit - AggThresholds + finish())
- `27fb62a` 存在 (GREEN commit - app.rs integration)
- `src/scanner/types.rs` 存在且包含 AggThresholds, DirNode.finish(), 测试模块
- `src/scanner/mod.rs` 存在且导出 AggThresholds, OthersEntry
- `src/app.rs` 存在且包含 root.finish() 调用
- 全部 15 个测试通过
- cargo build 通过
