---
phase: 01-扫描引擎
reviewed: 2026-05-05T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - Cargo.toml
  - src/main.rs
  - src/app.rs
  - src/scanner/mod.rs
  - src/scanner/types.rs
  - src/scanner/error.rs
  - src/scanner/walker.rs
  - src/platform/mod.rs
  - src/platform/drives.rs
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 1: 扫描引擎 — 代码审查报告

**审查日期:** 2026-05-05
**审查深度:** standard
**审查文件数:** 9
**状态:** 发现问题，需修复后合并

## 总结

扫描引擎整体架构合理，D-01 ~ D-05 五项锁定决策均已落实，TDD 流程合规，15 个测试全部通过。核心 Win32 FFI 封装、rayon 并行扫描、`\\?\` 扩展路径、重解析点检测、无权限目录标注均正确实现。

发现 **1 个严重问题**（`FindClose` 在错误路径泄漏句柄）、**5 个警告**（unsafe 块隐患、错误处理缺陷、逻辑问题）、**4 个信息项**（代码质量改进建议）。

## 文件-问题对照表

| 文件 | Critical | Warning | Info |
|------|----------|---------|------|
| `src/scanner/walker.rs` | 1 | 2 | 1 |
| `src/app.rs` | 0 | 2 | 1 |
| `src/platform/drives.rs` | 0 | 1 | 0 |
| `src/scanner/types.rs` | 0 | 0 | 1 |
| `src/scanner/error.rs` | 0 | 0 | 1 |
| `Cargo.toml` | 0 | 0 | 0 |
| `src/main.rs` | 0 | 0 | 0 |
| `src/scanner/mod.rs` | 0 | 0 | 0 |
| `src/platform/mod.rs` | 0 | 0 | 0 |

---

## Critical Issues

### CR-01: `FindClose` 在 `FindFirstFileExW` 失败路径未执行 — 句柄泄漏

**文件:** `src/scanner/walker.rs:81-102`
**行号:** 81-102

**问题:**
当 `FindFirstFileExW` 返回 `Err` 时，代码通过 `match handle` 的 `Err(_)` 分支直接 `return`。此时 `FindFirstFileExW` 可能返回了无效句柄（非 `INVALID_HANDLE_VALUE`），但 `FindClose` 永远不会被调用。第 146 行的 `unsafe { FindClose(handle).ok() }` 只在正常循环结束后执行。

更严重的是：当 `GetLastError() == 5`（ACCESS_DENIED）时，代码返回 `Ok(node)` 但 `FindFirstFileExW` 可能已经成功打开了句柄（某些 Windows 配置下 `FindFirstFileExW` 对无权限目录返回有效句柄但 `GetLastError` 为 5）。即使 `FindFirstFileExW` 返回 `Err`，Windows 文档指出当函数失败时句柄值是不确定的，可能仍需关闭。

**修复:**
```rust
let handle = unsafe {
    FindFirstFileExW(
        PCWSTR(search_path.as_ptr()),
        FindExInfoBasic,
        &mut find_data as *mut _ as *mut _,
        FindExSearchNameMatch,
        None,
        FIND_FIRST_EX_LARGE_FETCH,
    )
};

// 先检查是否为 INVALID_HANDLE_VALUE，再决定是否需要 FindClose
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;

let handle = match handle {
    Ok(h) if h != INVALID_HANDLE_VALUE => h,
    _ => {
        let err = unsafe { GetLastError() };
        if err.0 == 5 {
            node.access_denied = true;
            return Ok(node);
        }
        return Err(ScanError::Win32(err.0));
    }
};
```

---

## Warnings

### WR-01: `FindFirstFileExW` 的 `WIN32_FIND_DATAW` 指针转换未验证对齐

**文件:** `src/scanner/walker.rs:82-90`
**行号:** 86

**问题:**
`&mut find_data as *mut _ as *mut _` 将 `&mut WIN32_FIND_DATAW` 强制转换为 `*mut _`。虽然当前写法在实践中有概率工作（`WIN32_FIND_DATAW` 默认对齐满足要求），但 `FindFirstFileExW` 的 `LPVOID` 参数要求调用者确保缓冲区对齐。Rust 的 `WIN32_FIND_DATAW::default()` 在栈上分配，对齐由编译器保证，但此处未显式标注。更安全的做法是使用 `std::ptr::addr_of_mut!`。

**修复:**
```rust
use std::ptr::addr_of_mut;

let handle = unsafe {
    FindFirstFileExW(
        PCWSTR(search_path.as_ptr()),
        FindExInfoBasic,
        addr_of_mut!(find_data) as *mut _,
        FindExSearchNameMatch,
        None,
        FIND_FIRST_EX_LARGE_FETCH,
    )
};
```

### WR-02: `FindNextFileW` 错误被静默吞没，无法区分 "文件被删除" 和 "磁盘 I/O 错误"

**文件:** `src/scanner/walker.rs:136-144`
**行号:** 136-144

**问题:**
D-05 决策要求"接受快照不完美"，但当前实现将所有 `FindNextFileW` 错误（除 `ERROR_NO_MORE_FILES` 外）全部静默忽略。这意味着如果遇到 `ERROR_DISK_CORRUPT` (1392) 或 `ERROR_FILE_CORRUPT` (1393) 等严重错误，扫描会静默跳过并继续，用户无法得知部分数据可能已损坏。

**修复:** 至少记录一个警告级别的日志或统计错误数量：
```rust
let mut error_count: u64 = 0;
// ...
if let Err(_) = success {
    let err = unsafe { GetLastError() };
    if err == ERROR_NO_MORE_FILES {
        break;
    }
    error_count += 1;
    // 可选：超过一定阈值后终止扫描
}
```

### WR-03: `app.rs` 中 `scan_progress` 事件消费后不清除，导致 UI 每帧重复处理

**文件:** `src/app.rs:86`
**行号:** 86

**问题:**
`self.scan_progress = Some(event)` 在每次事件消费时被覆盖写入。`ScanEvent::Complete` 分支中 `self.scan_progress` 被设置为 `Some(Complete {...})`，但 `self.scan_result` 也被同时设置。后续帧中 `consume_events` 不再有新事件（channel 已空），但 `self.scan_progress` 仍持有上次的 `Complete` 事件。UI 代码（第 145 行）检查 `self.scan_result` 而非 `self.scan_progress`，所以当前不会重复渲染，但 `scan_progress` 语义上表示"当前进度"，在完成时应被清除。

**修复:** 在 `Complete` 事件处理中清除进度：
```rust
ScanEvent::Complete { root, duration, total_files, access_denied_count } => {
    self.scan_result = Some(Arc::new(root.clone()));
    self.scan_progress = None; // 清除进度
    // ...
}
```

### WR-04: `app.rs` 中 `start_scan` 未取消正在进行的扫描，连续点击导致多个扫描线程并发

**文件:** `src/app.rs:29-63`
**行号:** 29-31

**问题:**
用户连续点击多个驱动器的"扫描"按钮时，`start_scan` 会创建多个后台线程同时扫描不同目录。每个线程都通过 `sender.send()` 向同一个 channel 发送事件，`consume_events` 会混合处理来自不同扫描任务的 `Complete` 事件，导致 `self.scan_result` 被最后一个完成的扫描覆盖，用户看到的结果与预期不符。

**修复:** 在 `start_scan` 开头取消之前的扫描：
```rust
fn start_scan(&mut self, path: PathBuf) {
    // 取消之前的扫描
    self.event_receiver = None;
    self.scan_result = None;
    self.scan_progress = None;

    let (sender, receiver) = bounded::<ScanEvent>(256);
    // ...
}
```

### WR-05: `GetDiskFreeSpaceExW` 使用 `\\?\` 前缀路径 — 与 D-02 决策矛盾

**文件:** `src/platform/drives.rs:15-16`
**行号:** 15-16

**问题:**
D-02 决策文档指出 `GetDiskFreeSpaceExW` 支持 `\\?\` 前缀，但 CONTEXT.md 同时指出 `GetDiskFreeSpaceExW` 对根路径如 `C:\` 不需要 `\\?\` 前缀。当前代码传入 `C:\` 格式（无 `\\?\` 前缀），这是正确的。但 `to_extended_path()` 在 walker.rs 中使用了 `\\?\` 前缀。两者不一致：drives.rs 直接传 `C:\`，walker.rs 传 `\\?\C:\`。虽然两者都能工作，但 drives.rs 的路径未经过 `to_extended_path()` 处理，如果未来传入非标准路径（如相对路径），`GetDiskFreeSpaceExW` 可能失败。

**修复:** 统一使用 `to_extended_path` 或在 drives.rs 中显式说明为何不需要扩展路径：
```rust
// drives.rs 中路径为根路径（如 C:\），GetDiskFreeSpaceExW 不需要 \\?\ 前缀
// 这是有意为之：根路径格式固定，无需 canonicalize
```

---

## Info

### IN-01: `src/scanner/types.rs` 中 `ScanEvent` 定义在 types.rs 但逻辑上属于 app 层

**文件:** `src/scanner/types.rs:235-260`
**行号:** 235-260

**问题:**
`ScanEvent` 枚举包含 `Complete { root, duration, total_files, access_denied_count }` 等 UI 消费端概念，将其放在 `scanner` 模块的 `types.rs` 中导致 scanner 模块与 UI 层耦合。`ScanEvent` 应属于 app 层或独立的 event 模块。

**建议:** 将 `ScanEvent` 移至 `src/app.rs` 或新建 `src/event.rs`。

### IN-02: `src/scanner/error.rs` 中 `ScanError::Io` 使用 `Arc<std::io::Error>` 而非 `Box`

**文件:** `src/scanner/error.rs:16`
**行号:** 16

**问题:**
`Arc<std::io::Error>` 用于支持 `Clone` derive（`std::io::Error` 本身不实现 `Clone`）。但 `Arc` 引入原子引用计数开销，而错误路径不需要共享所有权。使用 `Box<std::io::Error>` 配合手动实现 `Clone`（通过 `new` 构造）更轻量，或者直接用 `String` 存储错误信息。

**建议:** 考虑改为 `ScanError::Io(String)` 或 `ScanError::Io(Box<std::io::Error>)` 并手动实现 Clone。

### IN-03: `src/app.rs` 中 `count_access_denied` 函数可改为 `DirNode` 的方法

**文件:** `src/app.rs:157-167`
**行号:** 157-167

**问题:**
`count_access_denied` 是一个自由函数，递归遍历 `DirNode` 的 children 统计 `AccessDenied` 条目。这个逻辑与 `DirNode` 的数据结构紧密相关，放在 `types.rs` 中作为 `DirNode` 的方法更符合 Rust 的面向对象风格。

**建议:**
```rust
impl DirNode {
    pub fn count_access_denied(&self) -> u64 {
        let mut count = 0;
        for child in &self.children {
            match child {
                Entry::AccessDenied { .. } => count += 1,
                Entry::Dir(d) => count += d.count_access_denied(),
                _ => {}
            }
        }
        count
    }
}
```

### IN-04: `src/platform/drives.rs` 中 `GetLogicalDrives` 返回值未检查为 0 的情况

**文件:** `src/platform/drives.rs:10`
**行号:** 10

**问题:**
`GetLogicalDrives()` 返回 0 表示调用失败（通过 `GetLastError` 获取原因）。当前代码将返回 0 解释为"没有逻辑盘"，返回空 `Vec`。虽然这在实践中几乎不可能发生，但按照 Windows API 最佳实践，返回 0 时应检查 `GetLastError`。

**建议:**
```rust
let bitmask = unsafe { windows::Win32::Storage::FileSystem::GetLogicalDrives() };
if bitmask == 0 {
    return Vec::new(); // 或记录警告
}
```

---

## 架构合规检查

| 决策 | 要求 | 实现 | 状态 |
|------|------|------|------|
| D-01 | rayon 线程池 + 工作窃取 | `rayon::scope()` + `s.spawn()` | 合规 |
| D-02 | `\\?\` 扩展路径 | `to_extended_path()` | 合规 |
| D-03 | 不跟随符号链接 | `FILE_ATTRIBUTE_REPARSE_POINT` -> `Entry::Symlink` | 合规 |
| D-04 | 无权限目录标注 | `Entry::AccessDenied { path }` | 合规 |
| D-05 | 接受快照不完美 | `FindNextFileW` 错误跳过 | 合规 |

## 安全审查

| 威胁 | 状态 | 说明 |
|------|------|------|
| 路径遍历 | 安全 | `canonicalize()` 规范化路径，`\\?\` 前缀阻止相对路径解析 |
| 符号链接攻击 | 安全 | `FILE_ATTRIBUTE_REPARSE_POINT` 检测，不跟随 |
| 资源耗尽 | 安全 | bounded channel(256)，Others 聚合 |
| 权限提升 | 安全 | 不尝试提升权限，ACCESS_DENIED 静默记录 |
| 句柄泄漏 | **有风险** | CR-01: `FindClose` 在错误路径未执行 |

## 总体判定

**FAIL** — CR-01（句柄泄漏）为严重问题，必须在合并前修复。WR-01 ~ WR-05 建议在 Phase 1 修复或创建 Phase 1.1 修复计划。

---
*审查时间: 2026-05-05*
*审查者: Claude (gsd-code-reviewer)*
*深度: standard*
