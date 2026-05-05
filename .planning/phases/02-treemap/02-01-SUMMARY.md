---
phase: 02-treemap
plan: 01
subsystem: ui
tags: [treemap, egui, data-structures, scaffolding]

# Dependency graph
requires:
  - phase: 01-扫描引擎
    provides: DirNode, Entry, ScanEvent types from scanner module
provides:
  - TreemapNode struct definition (9 fields, Debug + Clone)
  - treemap module entry (mod.rs with types submodule)
  - DiskReviewerApp extended with nav_stack, selected_index, treemap_nodes
  - current_dir() helper for nav_stack-based directory resolution
  - rebuild_treemap() placeholder for plan 02-02 layout algorithm
affects:
  - 02-02 (layout algorithm consumes TreemapNode)
  - 02-03 (renderer consumes TreemapNode)
  - 02-04 (drill-down uses nav_stack)
  - 02-05 (detail panel uses selected_index)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Module entry pattern: pub mod + pub use re-exports"
    - "Data model pattern: #[derive(Debug, Clone)] with public fields"
    - "Nav stack pattern: Vec<usize> for tree navigation path"

key-files:
  created:
    - src/treemap/types.rs
  modified:
    - src/treemap/mod.rs
    - src/app.rs

key-decisions:
  - "TreemapNode has 9 fields: rect, label, color, depth, entry_index, is_dir, size, percentage"
  - "nav_stack is empty Vec at root level (not [0])"
  - "rebuild_treemap() is a placeholder until plan 02-02 implements layout_treemap"
  - "consume_events() uses take-and-restore pattern to avoid borrow conflicts with rebuild_treemap()"

patterns-established:
  - "Module structure: types.rs for data, mod.rs for re-exports"
  - "App state extension: add fields to struct, init in new(), use in event handlers"

requirements-completed: [VIS-01]

# Metrics
duration: 5min
completed: 2026-05-05
---

# Phase 2 Plan 1: Treemap 模块脚手架与数据结构定义

**TreemapNode 结构体定义、treemap 模块入口、app.rs 状态字段扩展，建立 Phase 2 编译和类型依赖基础**

## Performance

- **Duration:** 5 min
- **Started:** 2026-05-05T14:55:54Z
- **Completed:** 2026-05-05T15:00:41Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Created `TreemapNode` struct with all 9 fields (rect, label, color, depth, entry_index, is_dir, size, percentage) and Debug + Clone derives
- Established `treemap/` module structure with types.rs and mod.rs re-export pattern
- Extended `DiskReviewerApp` with nav_stack, selected_index, treemap_nodes fields and current_dir()/rebuild_treemap() helper methods
- Scan completion event now initializes nav_stack to empty (root level) and triggers treemap rebuild

## Task Commits

Each task was committed atomically:

1. **Task 1: Create TreemapNode struct** - `0093cf9` (feat)
2. **Task 2: Extend app.rs state fields** - `810183f` (feat)

**Plan metadata:** (final commit below)

## Files Created/Modified
- `src/treemap/types.rs` - TreemapNode struct definition (9 fields, Debug + Clone)
- `src/treemap/mod.rs` - Module entry, declares types submodule, re-exports TreemapNode
- `src/app.rs` - Extended DiskReviewerApp with Phase 2 state fields and helper methods

## Decisions Made
- **nav_stack semantics**: Empty Vec = root level (displays scan_result's children). This matches RESEARCH.md Pattern 4.
- **rebuild_treemap() placeholder**: Does nothing until plan 02-02 implements layout_treemap. Uses `let _ = dir` to suppress unused warning.
- **consume_events() borrow pattern**: Used `self.event_receiver.take()` to temporarily take ownership, avoiding immutable borrow conflict when calling `rebuild_treemap()` (which needs mutable self). Receiver is restored unless disconnected.
- **emath import**: Used `egui::emath::Rect` instead of bare `emath::Rect` (the latter is not a standalone crate dependency).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed emath import path**
- **Found during:** Task 1 (TreemapNode struct creation)
- **Issue:** `use emath::{Pos2, Rect, Vec2}` failed — `emath` is not a standalone crate, it is re-exported under `egui::emath`
- **Fix:** Changed to `use egui::emath::Rect` and removed unused Pos2/Vec2 imports
- **Files modified:** src/treemap/types.rs
- **Verification:** `cargo check` passes
- **Committed in:** `0093cf9` (Task 1 commit)

**2. [Rule 1 - Bug] Fixed borrow checker conflict in consume_events()**
- **Found during:** Task 2 (app.rs state extension)
- **Issue:** Calling `self.rebuild_treemap()` inside the `if let Some(receiver) = &self.event_receiver` scope caused E0502 — cannot borrow `*self` as mutable while `self.event_receiver` is immutably borrowed
- **Fix:** Restructured consume_events() to use `self.event_receiver.take()` pattern — temporarily takes ownership of the receiver, processes events with a `needs_rebuild` flag, restores receiver if not disconnected, then calls `rebuild_treemap()` after the borrow scope ends
- **Files modified:** src/app.rs
- **Verification:** `cargo check` passes
- **Committed in:** `810183f` (Task 2 commit)

**3. [Rule 3 - Blocking] Added Entry import to app.rs**
- **Found during:** Task 2 (current_dir() method)
- **Issue:** `current_dir()` uses `Entry::Dir(d)` pattern match but `Entry` was not imported in app.rs
- **Fix:** Added `Entry` to the existing `use crate::scanner::...` import
- **Files modified:** src/app.rs
- **Verification:** `cargo check` passes
- **Committed in:** `810183f` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking)
**Impact on plan:** All auto-fixes necessary for compilation. No scope creep.

## Issues Encountered
- Rust borrow checker required restructuring consume_events() to separate the receiver borrow from the mutable self access needed by rebuild_treemap(). The take-and-restore pattern is a clean solution that also handles the disconnected case naturally.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- treemap module structure established with TreemapNode type ready for layout algorithm consumption
- app.rs state fields (nav_stack, selected_index, treemap_nodes) ready for plan 02-02 layout_treemap implementation
- rebuild_treemap() placeholder ready to be wired to layout_treemap() once implemented
- All Phase 2 downstream plans (02-02 through 02-05) can now import TreemapNode from crate::treemap

---
*Phase: 02-treemap*
*Completed: 2026-05-05*
