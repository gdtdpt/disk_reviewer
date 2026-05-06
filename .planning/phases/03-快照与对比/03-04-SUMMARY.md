---
phase: 03-快照与对比
plan: 04
subsystem: comparison-view
tags: [comparison-window, diff-overlay, treemap, egui, D-21, D-22, SNAP-04]
requires: [03-02, 03-03]
provides: [comparison-window-ui, diff-overlay-rendering, side-by-side-treemaps]
affects: [treemap-renderer, ui-module, app-state]
tech-stack:
  added: []
  patterns:
    - Side-by-side egui Window with two independent treemap panels (D-21)
    - Color overlay rendering on treemap rectangles mapped from ChangeType (D-22)
    - Icon marker painting at rect.right_top() corner for Added/Removed/Grown/Shrunk (D-22)
    - Diff tooltip enhancement: old size, new size, delta with sign (D-22)
    - HashMap<usize, &DiffNode> built from diff_level() for O(1) lookup during treemap rendering
key-files:
  created:
    - src/ui/comparison.rs
  modified:
    - src/treemap/renderer.rs
    - src/ui/mod.rs
    - src/app.rs
decisions:
  - "[Task 1] paint_diff_overlay takes &egui::Painter (not &mut Ui) for use inside allocate_painter callbacks"
  - "[Task 1] paint_treemap_with_diff is a full copy of paint_treemap with diff overlay + tooltip hooks (avoids feature-flag branching in hot path)"
  - "[Task 2] diff_level(snapshot=old, scan=new) semantics: Added=present in scan but not snapshot; Removed=present in snapshot but not scan"
  - "[Task 2] Both panels have independent nav_stack + selected state for independent drill-down"
  - "[Task 2] resolve_by_nav_stack helper traverses DirNode tree by following child indices"
  - "[Task 2] SnapshotAction::OpenComparison snapshot name is collected before open_comparison() call to avoid &mut self borrow conflict"
metrics:
  duration: "~5 min"
  completed_date: "2026-05-06"
  tasks_completed: 2
  total_tasks: 2
  files_created: 1
  files_modified: 3
  test_count: 0 new tests (UI rendering code, TDD exempt per CLAUDE.md rules)
---

# Phase 03 Plan 04: Comparison View Summary

## One-liner

Side-by-side comparison window (D-21) with diff overlay rendering (D-22): color-coded treemap rectangles (green/red/orange/blue), icon markers (+/-/up-arrow/down-arrow), and rich tooltips showing old size, new size, and delta.

## Overview

Implemented the visual payoff of Phase 3: a comparison window that opens from the snapshot dialog, showing the current scan result on the left and snapshot data on the right. The right panel uses the tree diff algorithm from Plan 03 to highlight four change types with semi-transparent color overlays and corner icon markers.

## Tasks Completed

### Task 1: Diff overlay rendering in treemap renderer
**Commits:** `5436ca9`

Added two public functions to `src/treemap/renderer.rs`:

1. **`paint_diff_overlay(painter, rect, change)`** — Draws a semi-transparent overlay on a treemap rectangle based on its `ChangeType`:
   - Added: `from_rgba_unmultiplied(0, 200, 0, 80)` (green, ~30% opacity)
   - Removed: `from_rgba_unmultiplied(200, 0, 0, 80)` (red)
   - Grown: `from_rgba_unmultiplied(255, 165, 0, 80)` (orange)
   - Shrunk: `from_rgba_unmultiplied(0, 100, 200, 80)` (blue)
   - Unchanged: no overlay
   - Icon drawn at `rect.right_top() + vec2(-12, 2)`: +/- for Added/Removed, unicode up-arrow/down-arrow for Grown/Shrunk

2. **`paint_treemap_with_diff(ui, nodes, selected_index, canvas_rect, diff_map)`** — A variant of `paint_treemap` that looks up each node's `entry_index` in a `HashMap<usize, &DiffNode>` and calls `paint_diff_overlay` when a diff entry exists. Hover tooltips are enhanced with change details (old size, new size, delta with sign).

All 8 grep acceptance criteria verified. `cargo check --features snapshot` passes cleanly.

### Task 2: Comparison window UI with app.rs integration
**Commits:** `537ff8f`

1. **Created `src/ui/comparison.rs`** with:
   - `ComparisonWindow` struct: open state, snapshot ID/name/root, independent left/right nav stacks, selection state, diff cache
   - `comparison_window_ui(ctx, comparison, comparison_window_ui)` renders an egui `Window` (resizable, default 960x600) with `ui.horizontal` 50/50 split:
     - Left: heading "当前扫描", treemap via `layout_treemap` + `paint_treemap`, back-navigation button
     - Right: heading "快照: {name}", treemap via `layout_treemap` + `paint_treemap_with_diff`, back-navigation button
   - `resolve_by_nav_stack` helper traverses DirNode by following child indices
   - Diff computed as `diff_level(snapshot_dir, scan_dir)` per-frame at current drill-down level

2. **Wired into `src/ui/mod.rs`**: `#[cfg(feature = "snapshot")]` gated `pub mod comparison` and re-exports

3. **Updated `src/app.rs`**:
   - Added `comparison_state: Option<ComparisonWindow>` field to `DiskReviewerApp`
   - Added `open_comparison(snapshot_id, name)` method that loads snapshot root from `SnapshotStorage`
   - Wired `SnapshotAction::OpenComparison` handler (collects name before calling `open_comparison` to avoid borrow conflict)
   - Added `comparison_window_ui()` call after snapshot dialog in `update()`

All 8 grep acceptance criteria verified. `cargo check --features snapshot` and `cargo check` (no features) both pass.

## Deviations from Plan

### Auto-fixed Issues (Rule 1 - Bug)

**1. Fixed `right_d` typo in comparison.rs**
- **Found during:** Task 2 compilation
- **Issue:** Variable `right_d` was used but the actual binding was named `right_dir`
- **Fix:** Changed `diff_level(right_d, left_d)` to `diff_level(right_dir, left_d)`
- **Files modified:** `src/ui/comparison.rs`
- **Commit:** `537ff8f` (fixed in same commit)

**2. Fixed `paint_treemap_with_diff` import path in comparison.rs**
- **Found during:** Task 2 compilation
- **Issue:** Plan specified `crate::treemap::paint_treemap_with_diff` but the function lives in `crate::treemap::renderer`, not re-exported at `crate::treemap` level
- **Fix:** Changed to `crate::treemap::renderer::paint_treemap_with_diff`
- **Files modified:** `src/ui/comparison.rs`
- **Commit:** `537ff8f` (fixed in same commit)

## Test Results

```
cargo check --features snapshot: PASS (0 errors, pre-existing warnings only)
cargo check (no features):       PASS (0 errors, pre-existing warnings only)
```

UI rendering code is TDD-exempt per CLAUDE.md rules (UI rendering/layout skipped). All existing 69 tests with snapshot feature continue to pass.

## Key Files

| File | Status | Purpose |
|------|--------|---------|
| `src/treemap/renderer.rs` | Modified | Added `paint_diff_overlay` + `paint_treemap_with_diff` (diff overlay colors, icons, tooltips) |
| `src/ui/comparison.rs` | Created | `ComparisonWindow` struct + `comparison_window_ui` side-by-side rendering |
| `src/ui/mod.rs` | Modified | Feature-gated export of comparison module |
| `src/app.rs` | Modified | `comparison_state` field, `open_comparison` method, `OpenComparison` action wiring |

## Self-Check

- [x] All 2 tasks executed
- [x] Each task committed individually
- [x] `cargo check --features snapshot` passes with no errors
- [x] `cargo check` (no features) passes with no errors
- [x] All grep acceptance criteria verified (8/8 for Task 1, 8/8 for Task 2)
- [x] Diff overlay color constants match D-22 specification exactly
- [x] `ComparisonWindow` struct has all 8 required fields (open, snapshot_id, snapshot_name, snapshot_root, left_nav_stack, right_nav_stack, left_selected, right_selected, diff_cache)
- [x] Window opens as separate egui `Window` with `default_size([960.0, 600.0])` (D-21)
- [x] Side-by-side 50/50 layout via `ui.horizontal` (D-21)
- [x] Both panels support independent drill-down via double-click
- [x] Close via `.open(&mut is_open)` pattern sets `comparison.open = false`
- [x] All code feature-gated with `#[cfg(feature = "snapshot")]`
- [x] No modifications to STATE.md or ROADMAP.md

## Self-Check: PASSED
