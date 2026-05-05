---
status: testing
phase: 01-扫描引擎
source:
  - 01-01-SUMMARY.md
  - 01-02-SUMMARY.md
  - 01-03-SUMMARY.md
  - 01-04-SUMMARY.md
started: 2026-05-05T00:00:00Z
updated: 2026-05-05T00:00:00Z
---

## Current Test

number: 1
name: 应用启动 — 窗口显示驱动器列表
expected: |
  运行 `cargo run` 后，应用窗口弹出，标题为 "Disk Reviewer"。
  窗口中央显示 "Disk Reviewer" 标题和 "逻辑盘:" 标签。
  至少显示 C: 驱动器，包含盘符、总空间（GB）、可用空间（GB）。
  每个驱动器旁边有 "扫描" 按钮。
awaiting: user response

## Tests

### 1. 应用启动 — 窗口显示驱动器列表
expected: |
  运行 `cargo run` 后，应用窗口弹出，标题为 "Disk Reviewer"。
  窗口显示逻辑盘列表，至少 C: 盘，包含盘符、总空间、可用空间（GB）。
  每个驱动器有 "扫描" 按钮。
result: pending

### 2. 扫描功能 — 点击扫描按钮后 UI 不卡顿
expected: |
  点击某个驱动器的 "扫描" 按钮后，窗口不会冻结/白屏。
  状态消息更新为 "正在扫描: X:\"。
  扫描过程中窗口保持可交互（可以移动窗口、点击其他按钮）。
result: pending

### 3. 扫描完成 — 结果显示摘要
expected: |
  扫描完成后，状态消息显示 "扫描完成: N 个文件, 耗时 X.Xs, X 个目录无权限"。
  显示根目录路径、总大小（MB）、文件数。
result: pending

### 4. 无权限目录处理 — 不中断扫描
expected: |
  扫描包含系统目录（如 C:\）时，遇到无权限目录不会导致扫描崩溃或中断。
  扫描正常完成，access_denied_count >= 1（因为 C:\System Volume Information 等目录无权限）。
result: pending

### 5. 连续扫描 — 取消前一次扫描
expected: |
  点击一个驱动器的 "扫描" 按钮，在扫描过程中点击另一个驱动器的 "扫描" 按钮。
  前一次扫描被取消，新扫描正常开始。
  不会出现多个扫描线程并发导致结果混乱。
result: pending

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0

## Gaps

[none yet]
