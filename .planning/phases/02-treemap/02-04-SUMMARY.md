---
phase: 02-treemap
plan: 04
subsystem: treemap
tags: [drill-down, breadcrumb, navigation, egui-components]
requires:
  - 02-01  # TreemapNode struct + nav_stack field
  - 02-02  # layout_treemap
  - 02-03  # paint_treemap + color mapping + renderer
provides:
  - drill-down     # drill_down() + navigate_to_depth() methods
  - breadcrumb-nav # breadcrumb component with horizontal scroll
affects:
  - VIS-03  # 点击进入子目录，面包屑导航返回上层
  - VIS-04  # 面包屑显示完整路径，每段可点击
tech-stack:
  added: []
  patterns: [egui-scrollarea, nav-stack, egui-buttons]
key-files:
  created:
    - src/ui/breadcrumb.rs
  modified:
    - src/ui/mod.rs
    - src/app.rs
requirements: [VIS-03, VIS-04]
decisions:
  - "breadcrumb_ui returns Option<usize> (clicked depth) for loose coupling"
  - "rebuild_treemap accepts Rect canvas param for drill-down/nav consistency"
metrics:
  duration: 4m
  completed: 2026-05-05
---

# Phase 2 Plan 04: Drill-Down Navigation and Breadcrumb Summary

Implemented treemap drill-down interaction (VIS-03) and breadcrumb navigation (VIS-04). Users can now click directory rectangles to navigate deeper, and use the breadcrumb bar to jump back to any parent level.

## What Was Built

### Breadcrumb Component (src/ui/breadcrumb.rs)
- `breadcrumb_ui()` renders a horizontal, scrollable breadcrumb bar
- Takes `&DirNode` (scan root) and `&[usize]` (nav_stack) as inputs
- Returns `Option<usize>` indicating which depth level was clicked (None = no click)
- Root segment uses `scan_result.name`, child segments use `dir.name` from nav_stack traversal
- Each segment is a clickable `egui::Button`, separated by `>` labels
- Wrapped in `ScrollArea::horizontal` to handle long paths without overflow

### Drill-Down Methods (src/app.rs)
- `drill_down(child_index)`: Validates that the clicked entry is a `Dir`, pushes index to `nav_stack`, resets `selected_index`, rebuilds treemap
- `navigate_to_depth(depth)`: Truncates `nav_stack` to the given depth, resets `selected_index`, rebuilds treemap
- `rebuild_treemap(canvas)`: Updated to accept `emath::Rect` parameter instead of using hardcoded 1x1 placeholder

### app.rs Integration
- Breadcrumb rendered at top of `CentralPanel`, between heading and drive list
- When breadcrumb returns a depth, `navigate_to_depth()` is called
- Treemap click detection distinguishes Dir vs non-Dir entries:
  - Dir clicked -> calls `drill_down()` which navigates into the subdirectory
  - Non-Dir clicked -> sets `selected_index` for detail inspection
- Navigation or drill-down resets `selected_index = None` to prevent stale references

## Tasks Executed

| # | Task | Type | Commit | Status |
|---|------|------|--------|--------|
| 1 | Breadcrumb component | auto | 0d1ed46 | PASS - cargo check |
| 2 | Drill-down logic + integration | auto | 1588c84 | PASS - cargo check |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed emath namespace resolution in app.rs**
- **Found during:** Task 2 cargo check
- **Issue:** `emath::Rect`, `emath::pos2`, `emath::vec2` used with `emath` as bare module name, but the import is `use egui::emath::{pos2, vec2, Rect};` — direct names are in scope, not `emath::` prefix
- **Fix:** Changed all `emath::Rect::from_min_size(emath::pos2(...), emath::vec2(...))` to `Rect::from_min_size(pos2(...), vec2(...))` and `emath::Rect` parameter to `Rect`
- **Files modified:** src/app.rs
- **Commit:** 1588c84

## Self-Check

### File Verification
- [x] FOUND: src/ui/breadcrumb.rs
- [x] FOUND: src/ui/mod.rs (updated with breadcrumb module export)
- [x] FOUND: src/app.rs (updated with drill_down, navigate_to_depth, breadcrumb integration)

### Commit Verification
- [x] FOUND: 0d1ed46 (feat breadcrumb component)
- [x] FOUND: 1588c84 (feat drill-down + integration)

### Build Verification
- [x] `cargo check` compiles cleanly (only pre-existing warnings)

## Self-Check: PASSED

## Known Stubs

None. All functionality is fully wired.

## Threat Surface Scan

No new security-relevant surface introduced. This plan only adds UI navigation logic — input comes from egui click events on treemap rectangles, no new network endpoints, auth paths, or file access patterns.
