---
phase: 02-treemap
plan: 05
subsystem: ui-treemap
tags: [ui, info-panel, color-legend, layout]
requires:
  - phase: 02-treemap
    plans: [02-03, 02-04]
provides:
  - info-panel-component
  - color-legend
  - 70-30-layout-split
  - treemap-rendering
affects:
  - src/app.rs
  - src/ui/
tech-stack:
  added: []
  patterns:
    - egui SidePanel/CentralPanel/TopBottomPanel layout
    - take-and-restore borrow pattern for egui closures
key-files:
  created:
    - src/ui/info_panel.rs
  modified:
    - src/app.rs
    - src/ui/mod.rs
requirements-completed: [VIS-05]
decisions:
  - "[Plan 02-05] TopBottomPanel borrows self -> used take-and-restore: extract nav_action outside closure, then call navigate_to_depth"
  - "[Plan 02-05] FileCategory is in treemap::color submodule, not re-exported at treemap:: level -> use crate::treemap::color::FileCategory"
metrics:
  duration: 4m
  completed: "2026-05-05"
---

# Phase 2 Plan 5: Info Panel and 70/30 Layout Summary

Right-side detail panel with selected-node info, directory summary, and 10-category color legend, integrated into app.rs via `SidePanel::right(320.0)` for a 70/30 split layout.

## Tasks

### Task 1: Info Panel Component (D-10, D-15)
**Commit:** `965f20f` — feat(02-05): implement info_panel.rs with `info_panel_ui()` function.

**What was done:**
- Created `src/ui/info_panel.rs` with the `info_panel_ui(ui, selected, current_dir)` function.
- Selected node display: name, formatted size, percentage, type (目录/文件).
- Directory sub-info: file count (from `DirNode.file_count`) and subdirectory count (filtering `Entry::Dir`).
- No selection fallback: current directory summary (name, total size, file count, children count).
- Nothing scanned: placeholder text "点击色块查看详情".
- Color legend: all 10 `FileCategory` variants with 16x16 color swatches (using `CornerRadius::same(2)`) and Chinese labels.

### Task 2: Layout Integration (D-14)
**Commit:** `c7026b4` — feat(02-05): integrate info panel and 70/30 layout into app.rs.

**What was done:**
- Replaced single `CentralPanel` with three-panel layout:
  - `TopBottomPanel::top("breadcrumb")` for breadcrumb navigation.
  - `SidePanel::right("detail_panel").exact_width(320.0)` for the info panel.
  - `CentralPanel` for the treemap canvas (~70% width).
- Breadcrumb uses take-and-restore borrow pattern: the clicked depth is captured in a local variable during the TopBottomPanel closure, then `navigate_to_depth()` is called outside the closure to avoid mutable/immutable borrow conflicts.
- Updated `src/ui/mod.rs` to declare and re-export the `info_panel` module.
- Treemap rebuild now calls `self.rebuild_treemap(canvas_rect)` instead of directly calling `layout_treemap`, ensuring nav_stack state is preserved.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Build] Fixed `emath` import path**
- **Found during:** Task 2 compilation
- **Issue:** `use emath::{pos2, vec2, Rect};` failed — `emath` is not a root crate, it's `egui::emath`.
- **Fix:** Changed to `use egui::emath::{pos2, vec2, Rect};`.
- **Files modified:** `src/app.rs`
- **Commit:** Included in `c7026b4`

**2. [Rule 3 - Build] Fixed `FileCategory` import path in info_panel.rs**
- **Found during:** Task 2 compilation
- **Issue:** `crate::treemap::FileCategory` doesn't exist — `FileCategory` is in the `color` submodule, not re-exported at the `treemap` level.
- **Fix:** Changed to `use crate::treemap::color::FileCategory;`.
- **Files modified:** `src/ui/info_panel.rs`
- **Commit:** Included in `c7026b4`

**3. [Rule 1 - Bug] Fixed borrow conflict in TopBottomPanel closure**
- **Found during:** Task 2 compilation
- **Issue:** `self.navigate_to_depth(depth)` inside the TopBottomPanel closure conflicts with the immutable borrow of `self.scan_result` used by `breadcrumb_ui`.
- **Fix:** Used take-and-restore pattern — extract the nav depth via a local `Option<usize>` inside the closure, then call `navigate_to_depth()` after the closure ends.
- **Files modified:** `src/app.rs`
- **Commit:** Included in `c7026b4`

## Verification

- `cargo check`: PASSED (18 warnings, 0 errors)
- `SidePanel::right` in app.rs: 1 occurrence
- `exact_width(320.0)` in app.rs: 1 occurrence
- `info_panel_ui` called in app.rs: 1 occurrence
- `TopBottomPanel::top` in app.rs: 1 occurrence
- `pub mod info_panel` in ui/mod.rs: 1 occurrence
- `pub fn info_panel_ui` in info_panel.rs: 1 occurrence
- 10 `FileCategory` entries in legend: confirmed

## Self-Check: PASSED

- `src/ui/info_panel.rs`: EXISTS
- `src/app.rs`: EXISTS (modified)
- `src/ui/mod.rs`: EXISTS (modified)
- Commit `965f20f`: EXISTS
- Commit `c7026b4`: EXISTS
- All acceptance criteria from both tasks verified.
