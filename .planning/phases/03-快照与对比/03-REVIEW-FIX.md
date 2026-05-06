---
phase: 03-快照与对比
fixed_at: 2026-05-06T21:30:00Z
review_path: .planning/phases/03-快照与对比/03-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 3: Code Review Fix Report

**Fixed at:** 2026-05-06T21:30:00Z
**Source review:** .planning/phases/03-快照与对比/03-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (1 Critical + 4 Warning)
- Fixed: 5
- Skipped: 0

## Fixed Issues

### CR-01: diff_level() result order vs treemap node ordering mismatch

**Files modified:** `src/snapshot/diff.rs`, `src/ui/comparison.rs`
**Commit:** 0517875
**Applied fix:** Added `child_index: Option<usize>` field to `DiffNode` that records the entry's position within the old tree's children. Updated `diff_level()` to populate this field for all change types (Removed entries get their old index, matched entries get their matched old entry's index, Added entries get `None`). Updated `comparison.rs` to build the `diff_map` via `child_index` (O(1) direct lookup) instead of by label name lookup, which could produce incorrect matches when names collide.

### WR-01: Empty/whitespace-only snapshot names

**Files modified:** `src/ui/snapshot_dialog.rs`
**Commit:** a19998d
**Applied fix:** Changed the empty name check from `.is_empty()` to `.trim().is_empty()` and added `.trim().to_string()` when saving a user-provided name. This prevents whitespace-only strings from being saved as valid snapshot names.

### WR-02: resolve_by_nav_stack silent failure with bare error label

**Files modified:** `src/ui/comparison.rs`
**Commit:** 921ca02
**Applied fix:** Added a "<< 返回根目录" (return to root) button alongside both error labels ("无法解析当前扫描树" and "无法解析快照树"). Clicking the button clears the respective nav_stack and selected index, allowing the user to recover and navigate back to the root directory.

### WR-03: save_snapshot requires &mut self on SnapshotStorage

**Files modified:** `src/snapshot/storage.rs`
**Commit:** 2f05815
**Applied fix:** Added a doc comment on `SnapshotStorage` explaining that `save_snapshot`, `delete_snapshot`, and `rename_snapshot` require `&mut self` due to `rusqlite::Connection` API limitations. Noted this is acceptable for the current single-threaded egui model and suggested `Rc<RefCell<Connection>>` or `Mutex` for future multi-threaded needs. No code change needed for MVP.

### WR-04: Unbounded recursion in insert_nodes_recursive

**Files modified:** `src/snapshot/storage.rs`
**Commit:** bbcef69
**Applied fix:** Converted the recursive `insert_nodes_recursive` function to use an explicit stack (`Vec<(&DirNode, Option<String>)>`) with a `while let` loop. This avoids stack overflow on deeply nested directory trees (Windows paths can be up to 32,767 characters deep). The function now uses owned `String` values for parent paths to avoid lifetime issues with references to loop-local data.

## Skipped Issues

None -- all in-scope findings were fixed.

## Verification

- `cargo check --features snapshot`: passed (0 errors, 36 pre-existing warnings)
- `cargo test --features snapshot`: 69/69 tests passed

## Deferred (Info-level, not in scope)

The following Info-level findings from REVIEW.md were not fixed per scope:
- IN-01: Dead `snapshot_status` field in `DiskReviewerApp`
- IN-02: Unused `SnapshotMeta.total_files` field
- IN-03: Unused public exports in `snapshot/mod.rs`
- IN-04: `eprintln!` in `DiskReviewerApp::new()` for DB init failure
- IN-05: `paint_treemap_with_diff` is a near-copy of `paint_treemap`
- IN-06: Duplicate `entry_name` function in two modules
- IN-07: Duplicate `format_size` function in two modules

---

_Fixed: 2026-05-06T21:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
