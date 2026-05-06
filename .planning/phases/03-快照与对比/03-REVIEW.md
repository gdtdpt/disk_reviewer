---
phase: 03-快照与对比
reviewed: 2026-05-06T21:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - src/snapshot/mod.rs
  - src/snapshot/storage.rs
  - src/snapshot/serialize.rs
  - src/snapshot/diff.rs
  - src/scanner/types.rs
  - src/treemap/color.rs
  - src/ui/snapshot_dialog.rs
  - src/ui/comparison.rs
  - src/ui/mod.rs
  - src/treemap/renderer.rs
  - src/app.rs
  - src/main.rs
  - Cargo.toml
findings:
  critical: 1
  warning: 4
  info: 7
  total: 12
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-05-06T21:00:00Z
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed all 13 source files created or modified during Phase 3 (快照与对比), covering snapshot storage (SQLite), DirNode serialization, tree diff algorithm, snapshot management dialog, comparison view with diff overlay rendering, and integration into app.rs/main.rs.

Overall assessment: The implementation is functionally sound with all 69 tests passing (59 without feature flag, 10 additional storage tests with `--features snapshot`). The architecture is clean with proper feature gating. However, we found 1 critical correctness bug in the diff result ordering that can cause mismatched diff-to-treemap mapping, 4 warnings around safety/design issues, and 7 info-level quality observations.

---

## Critical Issues

### CR-01: `diff_level()` result order depends on `new.children` iteration order, causing mismatched diff-to-treemap mapping in comparison view

**File:** `src/snapshot/diff.rs:44-106`
**Issue:** `diff_level()` first iterates `new.children` (producing Added/Grown/Shrunk/Unchanged entries), then appends Removed entries from `old.children`. The resulting `Vec<DiffNode>` order is determined by the arbitrary iteration order of the new tree's children.

In `src/ui/comparison.rs:197-208`, the code builds a `diff_map: HashMap<usize, &DiffNode>` by matching on `n.label == name`. If the diff result ordering doesn't match the treemap node ordering, and there are entries with duplicate names (or names that are substrings), the label-based lookup could map a DiffNode to the wrong TreemapNode. More concretely: if two children have the same display name (e.g., two `AccessDenied` entries where `file_name()` returns the same value, or an `Others` entry alongside a directory also named "Others"), the `find()` call at line 203 will always match the *first* TreemapNode with that name, producing incorrect diff overlays.

This is a **correctness bug** -- the diff overlay will be applied to the wrong treemap rectangles, confusing users with inaccurate change annotations.

**Fix:** Match DiffNodes to TreemapNodes by `entry_index` instead of by name label. DiffNode already carries an `entry` field -- the comparison view should establish the mapping via positional/index correspondence rather than string comparison:

```rust
// In comparison.rs, replace the label-based lookup:
let diff_map: HashMap<usize, &DiffNode> = diff_nodes
    .iter()
    .filter_map(|dn| {
        // Match by entry_index from the snapshot's TreemapNode
        right_nodes
            .iter()
            .find(|n| {
                // Resolve the entry's position in the snapshot's children
                // The DiffNode was produced from snapshot (old) / scan (new).
                // For entries present in the snapshot, we need to find the
                // corresponding right_node by matching the entry reference.
                crate::snapshot::entry_name(&dn.entry) == n.label
                    && dn.entry.size() == n.size  // additional disambiguation
            })
            .map(|n| (n.entry_index, dn))
    })
    .collect();
```

Alternatively and more robustly, store the `entry_index` field directly in `DiffNode` and use it for O(1) lookup.

---

## Warnings

### WR-01: `snapshot_save_snapshot` and `snapshot_dialog_ui` allow empty snapshot names

**File:** `src/snapshot/storage.rs:64` and `src/ui/snapshot_dialog.rs:114-119`
**Issue:** `save_snapshot()` accepts any `&str` name including empty strings `""`. The dialog UI at line 114 checks `is_empty()` only to decide whether to use the default timestamp, but if the user types whitespace-only text, it will be saved as a name. SQLite's `NOT NULL` constraint is satisfied by `""`, so no error occurs. Multiple snapshots with empty/whitespace names will be confusing in the snapshot list.

While not a crash bug, this degrades data quality and makes the snapshot list harder to use.

**Fix:** Add name validation in the create path:

```rust
let name = if dialog.new_name_buffer.trim().is_empty() {
    chrono::Local::now().format("快照 %Y-%m-%d %H:%M").to_string()
} else {
    dialog.new_name_buffer.trim().to_string()
};
```

### WR-02: `resolve_by_nav_stack` silently returns `None` on invalid navigation, with bare `if let` handlers hiding errors

**File:** `src/ui/comparison.rs:22-31`
**Issue:** `resolve_by_nav_stack` returns `None` if a nav_stack index points to a non-Dir child. In the comparison window UI (lines 62-122, 147-267), these `None` cases are handled with `else { ui.label(RichText::new("无法解析...").color(Color32::RED)) }`. This is acceptable for UI rendering, but the error messages "无法解析当前扫描树" and "无法解析快照树" give the user no actionable information about what went wrong or how to recover. More critically, if the snapshot and scan trees have diverged (e.g., a directory existed in one but not the other), the right panel will silently show the error label with no diff at all -- the user loses the comparison entirely for that directory level.

**Fix:** Add a status message or recovery hint. Consider resetting the nav_stack when the tree structure changes, or showing a "返回根目录" button.

### WR-03: `load_snapshot_into_view` uses `&self` immutable borrow on `SnapshotStorage`, but `save_snapshot` requires `&mut self`

**File:** `src/snapshot/storage.rs:64,89,108`
**Issue:** `save_snapshot`, `delete_snapshot`, `rename_snapshot` all take `&mut self`, while `load_snapshot`, `load_subtree`, `list_snapshots` take `&self`. The `rusqlite::Connection` requires `&mut` for `execute()` in `save_snapshot` because it may need to mutate internal state. However, this means every call site in `app.rs` must use `&mut self.snapshot_manager`, which leads the code to pattern-match `Some(manager) = &mut self.snapshot_manager` repeatedly, and the `SnapshotStorage` struct cannot be shared across threads. This isn't a bug for the current single-threaded egui model, but it's a design issue that will bite future concurrency efforts.

**Ref:** `app.rs:240-254` (load), `app.rs:259-272` (save), `app.rs:503-509` (delete), `app.rs:513-518` (rename).

**Fix (future):** Wrap the rusqlite `Connection` in `Rc<RefCell<Connection>>` or use `std::sync::Mutex` to allow shared access. For the current codebase this is low priority since everything runs on the UI thread.

### WR-04: `insert_nodes_recursive` uses unbounded recursion, risking stack overflow on deeply nested directories

**File:** `src/snapshot/storage.rs:176-198`
**Issue:** `insert_nodes_recursive` walks the entire DirNode tree recursively, calling `tx.execute()` for every directory. Windows file systems can have paths up to 32,767 characters with deep nesting. A degenerate tree with thousands of nested directories (e.g., `C:\a\b\c\d\...`) would cause a stack overflow and crash the application. The tree is already in memory (it was loaded from the scanner), so the depth could be arbitrarily large.

**Ref:** The scanner itself (`scan_directory`) uses `rayon::scope` which also has this issue, but the scanner's thread stack is typically larger.

**Fix:** Convert the recursion to an explicit stack:

```rust
fn insert_nodes_recursive(
    tx: &rusqlite::Transaction,
    snapshot_id: i64,
    root: &DirNode,
) -> Result<(), rusqlite::Error> {
    let mut stack: Vec<(&DirNode, Option<&str>)> = vec![(root, None)];
    while let Some((node, parent_path)) = stack.pop() {
        let path_str = node.path.to_string_lossy().to_string();
        let node_json = serde_json::to_string(node)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        tx.execute(
            "INSERT INTO snapshot_nodes (snapshot_id, path, parent_path, node_json)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![snapshot_id, &path_str, parent_path, &node_json],
        )?;
        for child in &node.children {
            if let Entry::Dir(dir) = child {
                stack.push((dir, Some(&path_str)));
            }
        }
    }
    Ok(())
}
```

---

## Info

### IN-01: Dead code -- `snapshot_status` field in `DiskReviewerApp` is written but never read

**File:** `src/app.rs:38` and `src/app.rs:82`
**Issue:** The `snapshot_status: String` field is initialized to `String::new()` and is pub, but no code reads it. The app uses `status_message` (the main status bar string) for all user-facing status updates. This field is dead cargo.

**Fix:** Remove the field. If it was intended for snapshot-specific status display, implement that UI element or remove the field to avoid confusion.

### IN-02: Dead code -- `SnapshotMeta.total_files` is stored in DB and populated but never read

**File:** `src/snapshot/storage.rs:14`
**Issue:** The `total_files` field is stored in the `snapshots` table and deserialized in `list_snapshots()`, but the snapshot dialog UI never displays it. It's only used in internal consistency checks in tests.

**Fix:** Either display it in the snapshot dialog (e.g., in the metadata line alongside size and time) or remove the field from the struct and DB schema.

### IN-03: Unused public exports in `snapshot/mod.rs`

**File:** `src/snapshot/mod.rs:7-8`
**Issue:** The module re-exports `serialize_tree`, `deserialize_tree`, `diff_level`, `diff_trees_recursive`, and `entry_name`, but none of these are used by any external caller. The storage module internally uses only `serde_json` directly. The `serialize_subtree`/`deserialize_subtree` functions in `serialize.rs` are also unused (the storage module serializes via `serde_json::to_string(node)` directly at `storage.rs:183`).

**Fix:** Remove unused re-exports. Consider whether the `serialize.rs` module is needed at all -- it's currently just a thin wrapper. If the wrappers add value (e.g., centralizing error conversion), keep but use them in storage.rs instead of raw `serde_json` calls.

### IN-04: `eprintln!` in `DiskReviewerApp::new()` for DB initialization failure

**File:** `src/app.rs:71-75`
**Issue:** If `SnapshotStorage::new()` fails (e.g., disk full, permission denied), the error is printed to stderr via `eprintln!` and silently swallowed -- the `snapshot_manager` field becomes `None`. The user has no indication in the UI that snapshot functionality is unavailable. Later calls to save/load will silently do nothing or return confusing error messages ("没有可保存的扫描结果" vs. a DB error).

**Fix:** Store the initialization error message in a field and display it in the status bar or snapshot dialog.

### IN-05: `paint_treemap_with_diff` is a near-complete copy of `paint_treemap`

**File:** `src/treemap/renderer.rs:194-337`
**Issue:** `paint_treemap_with_diff` (144 lines) is a copy of `paint_treemap` (127 lines) with only the diff overlay and enhanced tooltip added. This is a significant maintenance burden -- any bug fix or feature addition to the treemap renderer must be duplicated in both functions. The summary explicitly acknowledges this as a deliberate trade-off ("avoids feature-flag branching in hot path"), but it still creates a DRY violation.

**Fix (future refactor):** Parameterize `paint_treemap` with an optional diff map (`Option<&HashMap<usize, &DiffNode>>`) to unify the two functions. The overhead of an `Option` check per frame is negligible.

### IN-06: Duplicate `entry_name` function in two modules

**File:** `src/snapshot/diff.rs:24-40` and `src/treemap/layout.rs:196-208`
**Issue:** Both `diff.rs` and `layout.rs` define a function that extracts the display name from an `Entry`. The implementations are identical (matching on File/Dir/Others/Symlink/AccessDenied). This is code duplication that could lead to divergence.

**Fix:** Extract `entry_name` into a shared location (e.g., `scanner/types.rs` as an inherent method on `Entry`) and use it from both modules.

### IN-07: Duplicate `format_size` function in two modules

**File:** `src/treemap/renderer.rs:339-348` and `src/ui/snapshot_dialog.rs:43-52`
**Issue:** Both `renderer.rs` and `snapshot_dialog.rs` define identical `format_size` functions for human-readable byte formatting. The same function also exists in `src/treemap/layout.rs` comments as a note.

**Fix:** Extract into a shared utility module or as a free function in `scanner/types.rs`.

---

## Positive Observations

- **All 69 tests pass** with and without the `snapshot` feature flag. The TDD discipline throughout the phase is commendable -- all algorithm/diff/serialization code has tests.
- **Feature gating is clean.** The `#[cfg(feature = "snapshot")]` annotations are properly placed. The snapshot and diff modules compile without the feature; only the storage module (which depends on rusqlite) is gated. This matches the design intent.
- **SQL injection safe.** All SQL queries use parameterized statements (`rusqlite::params![]`), not string formatting. The only dynamic SQL identifiers (table/column names) are hardcoded constants.
- **The diff algorithm is solid.** Name-based matching (D-19) and the four change types (D-20) are well-implemented and thoroughly tested with 12 test cases covering all edge cases.
- **Snapshot storage design is good.** Path-indexed schema, WAL mode, foreign key cascade, and transaction-based batch save all follow best practices.

---

_Reviewed: 2026-05-06T21:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
