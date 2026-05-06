---
phase: 03-快照与对比
plan: 02
subsystem: snapshot-ui
tags: [snapshot-dialog, egui, crud, ui, toolbar]
requires: [03-01]
provides: [snapshot-management-ui, snapshot-dialog-integration, snapshot-toolbar]
affects: [app-state, ui-module, snapshot-dialog]
tech-stack:
  added: []
  patterns:
    - Modal egui Window dialog for snapshot management (D-23)
    - Action-return enum pattern (matches info_panel::FileListAction pattern)
    - Feature-gated snapshot state in app.rs (#[cfg(feature = "snapshot")])
    - SnapshotDialog struct with selection, rename, and delete-confirmation state
    - Context-based dialog rendering (takes &egui::Context, not &mut Ui, for use outside panel closures)
key-files:
  created:
    - src/ui/snapshot_dialog.rs
  modified:
    - src/app.rs
    - src/ui/mod.rs
    - src/snapshot/diff.rs
decisions:
  - "[Deviation] Changed snapshot_dialog_ui signature from &mut Ui to &egui::Context because dialog is called outside panel Ui scope (#2924-pattern)"
  - "[Deviation] Fixed pre-existing diff.rs lifetime error in HashMap construction (entry_name returning String used as &str key)"
  - "[Deviation] Added snapshot_dialog_state field (SnapshotDialog) to DiskReviewerApp instead of a simple bool, keeping dialog state management consistent with the action-return pattern"
metrics:
  duration: "~30 min"
  completed_date: "2026-05-06"
  tasks_completed: 2
  total_tasks: 2
  files_created: 1
  files_modified: 3
  test_count: 0 new tests (UI code, TDD exempt per CLAUDE.md rules)
---

# Phase 03 Plan 02: Snapshot Management Dialog

## One-liner

Snapshot management dialog (D-23) integrated into app.rs: modal egui Window with full CRUD operations, toolbar toggle button, snapshot loading into treemap view, and action-return enum pattern.

## Overview

Created the primary UI for SNAP-05: a popup snapshot management dialog that lists all snapshots with metadata (name, time, size, root path) and supports create, delete (with confirmation), rename, load, and open-comparison operations. Integrated into `app.rs` with a toolbar button, feature-gated snapshot manager state, and save/load methods.

## Tasks Completed

### Task 1: Add snapshot management state to app.rs
**Commits:** `c78e4e8`

Added three feature-gated fields to `DiskReviewerApp`:
- `snapshot_manager: Option<SnapshotStorage>` -- initialized from `%LOCALAPPDATA%\disk_reviewer\snapshots.db`
- `snapshot_dialog_open: bool` -- toggle for dialog visibility
- `snapshot_dialog_state: SnapshotDialog` -- dialog internal state (selection, rename, etc.)
- `snapshot_status: String` -- status messages for snapshot operations

Added two methods:
- `load_snapshot_into_view(snapshot_id)` -- loads snapshot into `scan_result`, resets nav_stack
- `save_current_snapshot(name)` -- saves current scan_result via SnapshotStorage

Added "快照" toolbar button that toggles the dialog and refreshes the snapshot list on open.

### Task 2: Implement snapshot management dialog UI
**Commits:** `523b6bc`

Created `src/ui/snapshot_dialog.rs` with:
- `SnapshotAction` enum: `None | Create(String) | Delete(i64) | Rename(i64, String) | Load(i64) | OpenComparison(i64)`
- `SnapshotDialog` struct: open state, snapshot list, selection, rename buffer, delete confirmation
- `snapshot_dialog_ui(ctx, dialog, scan_available)` -- renders the modal window with:
  - New snapshot input with default name (timestamp via chrono)
  - Scrollable snapshot list with name, time, size, root path
  - Selection highlighting
  - Inline rename editing
  - Delete confirmation sub-dialog (T-03: threat mitigation)
  - Load/Compare buttons with proper enable/disable states

Wired into `src/ui/mod.rs` with `#[cfg(feature = "snapshot")]` feature gating.

## Deviations from Plan

### Auto-fixed Issues

**1. Changed snapshot_dialog_ui parameter from `&mut Ui` to `&egui::Context` (Rule 3 - Blocking)**
- **Found during:** Task 1 compilation
- **Issue:** The plan specified passing `&mut Ui` to `snapshot_dialog_ui()`, but the dialog call is placed *after* the `CentralPanel::show()` closure, where `ui` is no longer in scope
- **Fix:** Changed function signature to take `&egui::Context` instead, which is available throughout the `update()` method. The dialog uses `ctx` to display modal Windows independently of any Ui scope
- **Files modified:** `src/ui/snapshot_dialog.rs`, `src/app.rs`
- **Commit:** `523b6bc`, `c78e4e8`

**2. Fixed pre-existing diff.rs lifetime error (Rule 3 - Blocking)**
- **Found during:** Compilation verification
- **Issue:** `diff_level()` in `src/snapshot/diff.rs` tried to create `HashMap<&str, &Entry>` by borrowing temporary Strings from `entry_name()`, causing E0515 errors
- **Fix:** Pre-collect owned `Vec<String>` names first, then zip with references to build the HashMap
- **Files modified:** `src/snapshot/diff.rs`
- **Commit:** `c78e4e8`

**3. Added `snapshot_dialog_state` field instead of just `snapshot_dialog_open` (Rule 2 - Missing)**
- **Found during:** Task 1 struct design
- **Issue:** The plan specified only `snapshot_dialog_open` and `snapshot_status` fields, but the dialog needs persistent state (selection, rename buffer, delete confirmation)
- **Fix:** Added `snapshot_dialog_state: SnapshotDialog` field (which itself contains the `open` boolean), replacing the separate `snapshot_dialog_open` field
- **Files modified:** `src/app.rs`
- **Commit:** `c78e4e8`

## Test Results

```
# Without --features snapshot: 47 tests pass
# With --features snapshot:    69 tests pass

UI code is TDD-exempt per CLAUDE.md rules (UI rendering/ layout skipped).
All existing tests continue to pass.
cargo check --features snapshot -- no errors
cargo check (no features) -- no errors
```

## Key Files

| File | Status | Purpose |
|------|--------|---------|
| `src/ui/snapshot_dialog.rs` | Created | Snapshot management dialog UI with CRUD |
| `src/app.rs` | Modified | Snapshot manager state, save/load methods, toolbar button, action handling |
| `src/ui/mod.rs` | Modified | Export snapshot_dialog module (feature-gated) |
| `src/snapshot/diff.rs` | Modified | Fixed pre-existing lifetime bug |

## Self-Check

- [x] All 2 tasks executed
- [x] Each task committed individually
- [x] `cargo check --features snapshot` passes with no errors
- [x] `cargo check` (no features) passes with no errors
- [x] All 69 tests pass with snapshot feature
- [x] All grep acceptance criteria verified
- [x] No modifications to STATE.md or ROADMAP.md

## Self-Check: PASSED
