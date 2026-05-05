---
phase: 02
slug: treemap
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-05
---

# Phase 2 — Treemap 可视化 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[cfg(test)]` |
| **Config file** | 无 — 使用 Cargo 默认配置 |
| **Quick run command** | `cargo test treemap::` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | < 10s |

---

## Sampling Rate

- **After every task commit:** `cargo test treemap::` (< 5s)
- **After every plan wave:** `cargo test` (< 30s)
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30s

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01 | 02-01 | 1 | — | build | `cargo check` | ❌ W0 | ⬜ pending |
| 02-02 | 02-02 | 1 | VIS-01 | unit | `cargo test treemap::layout::tests -x` | ❌ W0 | ⬜ pending |
| 02-03 | 02-03 | 2 | VIS-02 | unit | `cargo test treemap::color::tests -x` | ❌ W0 | ⬜ pending |
| 02-03 | 02-03 | 2 | VIS-02 | unit | `cargo test treemap::renderer::tests -x` | ❌ W0 | ⬜ pending |
| 02-04 | 02-04 | 3 | VIS-03 | integration | `cargo test ui::breadcrumb::tests -x` | ❌ W0 | ⬜ pending |
| 02-04 | 02-04 | 3 | VIS-04 | integration | `cargo test ui::breadcrumb::tests -x` | ❌ W0 | ⬜ pending |
| 02-05 | 02-05 | 3 | VIS-05 | integration | `cargo test ui::detail_panel::tests -x` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/treemap/mod.rs` — 模块导出存根
- [ ] `src/treemap/types.rs` — TreemapNode 结构体
- [ ] `src/treemap/layout.rs` — 布局算法 + 单元测试
- [ ] `src/treemap/color.rs` — 颜色映射 + 单元测试
- [ ] `src/treemap/renderer.rs` — 渲染逻辑
- [ ] `src/ui/breadcrumb.rs` — 面包屑组件
- [ ] `src/ui/detail_panel.rs` — 详情面板
- [ ] `src/app.rs` 扩展 — nav_stack, selected_index, treemap_nodes

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Treemap 矩形渲染 | VIS-01 | egui 自定义渲染无法单元测试 | 运行应用，扫描目录，确认矩形显示 |
| 单击下钻 | VIS-03 | 交互行为需手动验证 | 点击目录矩形，确认进入子目录 |
| 面包屑导航 | VIS-04 | 交互行为需手动验证 | 点击面包屑路径段，确认跳转 |
| 选中高亮 | VIS-05 | 视觉反馈需手动验证 | 点击矩形，确认高亮和详情面板更新 |
| 颜色映射 | D-09 | 视觉确认 | 检查不同类型文件/目录颜色是否正确 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
