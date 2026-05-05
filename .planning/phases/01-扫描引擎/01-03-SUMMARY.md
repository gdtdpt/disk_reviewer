---
phase: 01-扫描引擎
plan: 03
subsystem: async-directory-walker
tags: [scanner, walker, win32, rayon, crossbeam, async]
requires: [01-01]
provides: [scan_directory, ScanEvent, scan-thread-integration]
affects: [01-04, 02-01, 03-01]
tech-stack:
  added: []
  patterns: [FindFirstFileExW, rayon-scope, crossbeam-bounded-channel, extended-path]
key-files:
  created: []
  modified:
    - src/scanner/walker.rs
    - src/scanner/types.rs
    - src/scanner/error.rs
    - src/scanner/mod.rs
    - src/app.rs
    - src/platform/drives.rs
decisions:
  - "ScanError wraps std::io::Error in Arc to support Clone derive (required by ScanEvent)"
  - "to_extended_path() guards against double \\?\ prefix since canonicalize() already returns it on Windows"
  - "App UI uses collect-then-execute pattern for scan buttons to avoid borrow conflicts with &self"
  - "GetLogicalDrives is in Win32::Storage::FileSystem, not Win32::System::SystemInformation"
metrics:
  duration: "~20m"
  completed: "2026-05-05"
  tasks: 3
  files: 6
---

# Phase 1 Plan 03: 异步目录遍历器 (Async Directory Walker) Summary

基于 Win32 FindFirstFileExW + rayon::scope() 并行目录遍历 + crossbeam_channel 增量事件推送的异步扫描引擎核心实现。

## 完成情况

### Task 1 [RED]: 为目录遍历写失败测试
- **Commit:** `fcfaba4`
- **Status:** 完成
- **Files:** `src/scanner/walker.rs` (新增测试模块)
- **测试:** 4 个测试 (known dir, empty dir, file size accumulation, nonexistent path)
- **RED 确认:** 编译失败，`scan_directory` 函数尚未实现

### Task 2 [GREEN]: 实现 scan_directory() — FindFirstFileExW 遍历
- **Commit:** `25ae306`
- **Status:** 完成
- **Files:** `src/scanner/walker.rs`, `src/platform/drives.rs`
- **实现:**
  - `scan_directory()` 完整实现，使用 FindFirstFileExW + FindNextFileW + FindClose
  - `to_extended_path()` 添加 `\\?\` 前缀，含双重前缀保护 (D-02)
  - `rayon::scope()` 并行扫描子目录，每个子目录一个 rayon 任务 (D-01)
  - `FILE_ATTRIBUTE_REPARSE_POINT` 检测符号链接/junction，标记为 Entry::Symlink (D-03)
  - `ERROR_ACCESS_DENIED` 处理，标记为 Entry::AccessDenied，不中断扫描 (D-04)
  - FindNextFileW 错误跳过，接受不完美快照 (D-05)
  - 使用 `FindExInfoBasic` + `FIND_FIRST_EX_LARGE_FETCH` 标志
- **验证:** 4 个 walker 测试全部通过，所有 9 个测试无回归

### Task 3 [GREEN]: 添加 ScanEvent + 集成扫描线程到 app.rs
- **Commit:** `4fafcc7`
- **Status:** 完成
- **Files:** `src/scanner/types.rs`, `src/scanner/mod.rs`, `src/scanner/error.rs`, `src/app.rs`
- **实现:**
  - `ScanEvent` 枚举（DirEntry, Progress, AccessDenied, Error, Complete）
  - `ScanError` 增加 `Clone` derive（用 `Arc<std::io::Error>` 绕过 io::Error 非 Clone 限制）
  - `DiskReviewerApp` 结构体：drives, scan_result, scan_progress, event_receiver, status_message
  - `start_scan()` 方法：`std::thread::spawn` + `crossbeam_channel::bounded(256)`
  - `consume_events()` 方法：`try_recv()` 批量消费，每帧上限 100
  - `count_access_denied()` 递归计数函数
  - UI: 驱动器列表 + 扫描按钮 + 状态消息 + 结果预览
- **验证:** `cargo build` 通过，所有测试通过

## 验证结果

| 检查项 | 结果 |
|--------|------|
| `cargo test scanner::walker::tests` 全部通过 | PASS (4/4) |
| `cargo build` 通过 | PASS |
| `cargo test` 全部通过 | PASS (9/9, 无回归) |
| RED commit 存在 (`test(01-03): ...`) | PASS (`fcfaba4`) |
| GREEN commit 存在 (`feat(01-03): ...`) | PASS (`25ae306`, `4fafcc7`) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复 \\?\ 双重前缀导致的 ERROR_INVALID_NAME (123)**
- **Found during:** Task 2 GREEN 验证
- **Issue:** `std::fs::canonicalize()` 在 Windows 上已返回 `\\?\` 前缀路径，`to_extended_path()` 再次添加导致 `\\?\C:\...\...` 变成 `\\\\?\C:\...\...`，FindFirstFileExW 返回错误码 123 (ERROR_INVALID_NAME)
- **Fix:** 在 `to_extended_path()` 中检查路径是否已以 `\\?\` 开头，若是则跳过添加前缀
- **Files modified:** `src/scanner/walker.rs`
- **Commit:** `25ae306`

**2. [Rule 1 - Bug] 修复 PCWSTR 导入路径差异**
- **Found during:** Task 2 编译
- **Issue:** `windows::PCWSTR` 在 windows crate 0.62.2 中不存在，正确路径为 `windows::core::PCWSTR`。同样影响 pre-existing `drives.rs`。
- **Fix:** 将 `use windows::PCWSTR` 改为 `use windows::core::PCWSTR`
- **Files modified:** `src/scanner/walker.rs`, `src/platform/drives.rs`
- **Commit:** `25ae306`

**3. [Rule 1 - Bug] 修复 GetLogicalDrives 模块位置**
- **Found during:** Task 2 编译
- **Issue:** `GetLogicalDrives` 在 windows crate 0.62.2 中位于 `Win32::Storage::FileSystem`，而非 `Win32::System::SystemInformation`
- **Fix:** 修正 `drives.rs` 中的模块路径
- **Files modified:** `src/platform/drives.rs`
- **Commit:** `25ae306`

**4. [Rule 2 - Missing Critical Functionality] 为 ScanError 实现 Clone**
- **Found during:** Task 3 编译
- **Issue:** `ScanEvent` 需要 derive `Clone`，但 `ScanError` 包含 `std::io::Error` 不实现 `Clone`
- **Fix:** 将 `ScanError::Io` 变体从 `Io(std::io::Error)` 改为 `Io(Arc<std::io::Error>)`，并提供 `From<std::io::Error>` 手动实现
- **Files modified:** `src/scanner/error.rs`
- **Commit:** `4fafcc7`

**5. [Rule 1 - Bug] 修复 app.rs 中的借用冲突**
- **Found during:** Task 3 编译
- **Issue:** `for drive in &self.drives` 借用 `self` 不可变，但闭包内调用 `self.start_scan()` 需要可变借用，Rust 借用检查器拒绝
- **Fix:** 将 UI 逻辑改为 collect-then-execute 模式：先收集被点击的扫描路径，循环结束后再依次调用 `start_scan()`
- **Files modified:** `src/app.rs`
- **Commit:** `4fafcc7`

## 已知警告

编译产生多个预期内的 warning：
- `dead_code`: FileEntry, DirNode 字段，Entry 变体，ScanError 变体，ScanEvent 变体尚未被外部代码使用（Phase 2/3 将自然消除）
- `unused_imports`: mod.rs 中的 FileEntry, ScanError 重新导出尚未被其他模块引用
- `unused_variables`: app.rs 中的 `cc` 参数（eframe CreationContext）

## Threat Flags

无新增安全威胁。所有 threat model 中的缓解措施均已实现：
- T-01-03-01 (Symlink following): 已通过 D-03 缓解
- T-01-03-02 (DoS): bounded channel(256) + 每帧消费上限 100
- T-01-03-03 (Information Disclosure): 纯本地工具，接受

## TDD Gate Compliance

| 门控 | Commit | 状态 |
|------|--------|------|
| RED: `test(01-03): add failing test for directory walker` | `fcfaba4` | PASS |
| GREEN: `feat(01-03): implement directory walker with FindFirstFileExW` | `25ae306` | PASS |
| GREEN: `feat(01-03): integrate scan thread with crossbeam channel into app` | `4fafcc7` | PASS |

REFACTOR 门控：无（本计划无需独立重构步骤）。

## Self-Check: PASSED

- `fcfaba4` 存在 (RED commit)
- `25ae306` 存在 (GREEN commit - walker)
- `4fafcc7` 存在 (GREEN commit - app integration)
- `src/scanner/walker.rs` 存在且包含 scan_directory 实现
- `src/scanner/types.rs` 存在且包含 ScanEvent 枚举
- `src/scanner/mod.rs` 存在且导出所有必要符号
- `src/app.rs` 存在且包含 DiskReviewerApp 完整实现
- 所有 9 个测试通过
- cargo build 通过
