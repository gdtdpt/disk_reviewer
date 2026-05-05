---
phase: 02-treemap
plan: 03
subsystem: treemap
tags: [color-mapping, egui-renderer, treemap-painting, file-categorization]
requires:
  - 02-01  # TreemapNode struct
  - 02-02  # layout_treemap
provides:
  - color-mapping     # FileCategory enum + categorize + dominant_category
  - treemap-renderer  # paint_treemap + format_size
affects:
  - VIS-02           # 色块显示目录/文件名、大小、占比
tech-stack:
  added: []
  patterns: [tdd, egui-painter, color-mapping]
key-files:
  created:
    - src/treemap/color.rs
    - src/treemap/renderer.rs
  modified:
    - src/treemap/layout.rs
    - src/treemap/mod.rs
    - src/app.rs
requirements: [VIS-02]
decisions:
  - "ts extension mapped to Code (TypeScript), not Video, to avoid unreachable pattern"
  - "CornerRadius::same() uses u8 in egui 0.33, not f32"
metrics:
  duration: 7m
  completed: 2026-05-05
---

# Phase 2 Plan 03: Treemap Color Mapping and egui Rendering Summary

Implemented file type color mapping (D-09) and egui Treemap rendering (VIS-02). The treemap now renders colored rectangles proportional to file sizes, with labels for area >= 400 sq px, hover tooltips, and selection highlighting.

## What Was Built

### Color Mapping (src/treemap/color.rs)
- `FileCategory` enum with 10 variants: Document, Image, Video, Audio, Archive, Code, Executable, System, Temp, Other
- `categorize()` maps 80+ file extensions to categories (case-insensitive)
- `categorize_entry()` dispatches on `Entry` variants (File, Dir, Others, Symlink, AccessDenied)
- `dominant_category()` recursively accumulates size-by-category, returns the dominant one for directories
- Each category has a distinct RGB color and Chinese label
- 12 unit tests covering all 10 categories, color RGB values, and dominant_category

### Layout Color Integration (src/treemap/layout.rs)
- Replaced hardcoded gray `Color32::from_rgb(150, 150, 150)` with `FileCategory`-based colors
- Items now carry `&Entry` references for runtime categorization
- Directories use `dominant_category()`, non-directories use `categorize_entry()`

### egui Renderer (src/treemap/renderer.rs)
- `paint_treemap()` takes `&[TreemapNode]` and optional `selected_index`
- Uses `allocate_painter` + `Sense::click()` for click detection (reverse iteration = topmost rect first)
- Draws filled rectangles with `CornerRadius::same(1)`
- Labels rendered when `width * height >= 400.0` sq px, white text at top-left
- Selected node gets white 2px stroke border via `StrokeKind::Middle`
- Hover tooltip via `on_hover_ui_at_pointer` showing name, formatted size, percentage
- `format_size()` utility: human-readable sizes (B, KB, MB, GB, TB)

### app.rs Integration
- Treemap rendering section added after status message in `update()`
- Canvas computed from available width and height (min 200px)
- Layout recomputed each frame from `current_dir()` before painting
- Click updates `selected_index`

## Tasks Executed

| # | Task | Type | Commit | Status |
|---|------|------|--------|--------|
| 1 | File type color mapping (RED + GREEN) | TDD | 5b9c058 (RED) + d9ded50 (GREEN) | PASS - 12/12 tests |
| 2 | Layout color integration | auto | 0920ffc | PASS - 6/6 tests |
| 3 | egui renderer + app integration | auto | 7e2e482 | PASS - cargo check |

### TDD Gate Compliance
1. RED commit: `5b9c058` - `test(02-03): add failing tests for file category color mapping`
2. GREEN commit: `d9ded50` - `feat(02-03): implement file category color mapping`
3. Gate check: PASS - RED -> GREEN sequence confirmed

## Self-Check

### File Verification
- [x] FOUND: src/treemap/color.rs
- [x] FOUND: src/treemap/renderer.rs
- [x] FOUND: src/treemap/mod.rs (updated with color, renderer modules)
- [x] FOUND: src/treemap/layout.rs (updated with color imports)
- [x] FOUND: src/app.rs (updated with paint_treemap integration)

### Commit Verification
- [x] FOUND: 5b9c058 (test RED)
- [x] FOUND: d9ded50 (feat GREEN color)
- [x] FOUND: 0920ffc (feat layout color integration)
- [x] FOUND: 7e2e482 (feat renderer + app integration)

### Test Verification
- [x] `cargo test treemap::color::tests` - 12 passed
- [x] `cargo test treemap::layout::tests` - 6 passed
- [x] `cargo test` - 33 passed total
- [x] `cargo check` - compiles cleanly

## Self-Check: PASSED

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed unreachable pattern for "ts" extension**
- **Found during:** Task 3 compilation
- **Issue:** `.ts` extension appeared in both Video and Code match arms; Video arm came first making Code unreachable
- **Fix:** Removed `"ts"` from Video arm (`.ts` is TypeScript source code, not transport stream video in typical usage)
- **Files modified:** src/treemap/color.rs
- **Commit:** 7e2e482

**2. [Rule 1 - Bug] Fixed CornerRadius::same() type mismatch**
- **Found during:** Task 3 compilation
- **Issue:** `CornerRadius::same(1.0)` passed `f32` but egui 0.33 expects `u8`
- **Fix:** Changed to `CornerRadius::same(1)` in both `rect_filled` and `rect_stroke` calls
- **Files modified:** src/treemap/renderer.rs
- **Commit:** 7e2e482

**3. [Rule 1 - Bug] Fixed emath import path in app.rs**
- **Found during:** Task 3 compilation
- **Issue:** `use emath::{...}` failed - must use full `egui::emath` path
- **Fix:** Changed to `use egui::emath::{pos2, vec2, Rect};`
- **Files modified:** src/app.rs
- **Commit:** 7e2e482

**4. [Rule 3 - Blocking] Fixed PathBuf import in test module**
- **Found during:** Task 1 RED phase compilation
- **Issue:** `PathBuf` not imported in the `#[cfg(test)] mod tests` block
- **Fix:** Added `use std::path::PathBuf;` inside the `test_dominant_category_documents` test function
- **Files modified:** src/treemap/color.rs
- **Commit:** 5b9c058 (RED)

## Known Stubs

- `FileCategory::label()` is defined but not yet used in the UI (planned for legend in plan 02-05)
- `app.rs` still uses `crate::treemap` import (line 9) which is unused after refactoring to specific imports — causes a warning, not an error. Left as-is since it does not affect functionality.

## Threat Surface Scan

No new security-relevant surface introduced. This plan only adds rendering logic and color mapping — no new network endpoints, auth paths, or file access patterns.
