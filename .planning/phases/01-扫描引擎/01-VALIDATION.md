---
phase: 1
slug: 扫描引擎
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-05-05
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `cargo test` |
| **Config file** | None — Rust 原生测试框架，无需额外配置 |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test --all` |
| **Estimated runtime** | < 5s (unit) / < 30s (full) |

---

## Sampling Rate

- **After every task commit:** `cargo test --lib` (unit tests only, < 5s)
- **After every plan wave:** `cargo test --all` (with integration tests, < 30s)
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01-01 | 1 | SCAN-01~03 | — | N/A (scaffolding) | build | `cargo check` | ✅ | ⬜ pending |
| 01-01-02 | 01-01 | 1 | SCAN-01~03 | — | N/A (scaffolding) | build | `cargo build` | ✅ | ⬜ pending |
| 01-02-01 | 01-02 | 2 | SCAN-01 | — | N/A | unit | `cargo test platform::drives::tests` | ✅ | ⬜ pending |
| 01-02-02 | 01-02 | 2 | SCAN-01 | — | N/A | unit | `cargo test platform::drives::tests` | ✅ | ⬜ pending |
| 01-02-03 | 01-02 | 2 | SCAN-01 | — | N/A | build | `cargo build` | ✅ | ⬜ pending |
| 01-03-01 | 01-03 | 2 | SCAN-02,03 | T-01-03-01 | No symlink following | unit | `cargo test scanner::walker::tests` | ✅ | ⬜ pending |
| 01-03-02 | 01-03 | 2 | SCAN-02,03 | T-01-03-01 | Reparse point detection | unit | `cargo test scanner::walker::tests` | ✅ | ⬜ pending |
| 01-03-03 | 01-03 | 2 | SCAN-03 | T-01-03-02 | Bounded channel backpressure | build | `cargo build` | ✅ | ⬜ pending |
| 01-04-01 | 01-04 | 3 | SCAN-04,05 | T-01-04-01 | Aggregation limits memory | unit | `cargo test scanner::types::tests` | ✅ | ⬜ pending |
| 01-04-02 | 01-04 | 3 | SCAN-05 | T-01-04-01 | Aggregation correctness | unit | `cargo test scanner::types::tests` | ✅ | ⬜ pending |
| 01-04-03 | 01-04 | 3 | SCAN-04,05 | T-01-04-02 | No privilege escalation | integration | `cargo test --all` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/scanner/walker.rs` — walker 单元测试模块 (`#[cfg(test)] mod tests`)
- [ ] `src/scanner/types.rs` — 聚合逻辑单元测试模块
- [ ] `src/platform/drives.rs` — 逻辑盘枚举测试模块
- [ ] Rust 工具链已就绪 (`rustc 1.90.0`, `cargo`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| UI 不卡顿 | SCAN-03 | 视觉验证，无法自动化 | `cargo run` 后点击扫描按钮，观察窗口是否响应 |
| 无权限目录标注 | SCAN-04 | 需要系统级权限差异 | 扫描 `C:\System Volume Information`，确认显示 AccessDenied |
| 符号链接不跟随 | SCAN-03 | 需要创建 junction 测试 | 创建 junction 指向父目录，确认被标记为 Symlink |
| Others 聚合显示 | SCAN-05 | 需要大量文件触发 | 扫描包含 1000+ 文件的目录，确认 Others 条目出现 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
