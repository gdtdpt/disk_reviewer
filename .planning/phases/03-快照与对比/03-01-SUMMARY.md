---
phase: 03-快照与对比
plan: 01
subsystem: snapshot-storage
tags: [sqlite, serialization, snapshot, storage, rusqlite, serde]
requires: []
provides: [snapshot-storage-layer, dirnode-serialization, sqlite-crud]
affects: [scanner-types, treemap-color, cargo-toml]
tech-stack:
  added:
    - rusqlite serde_json feature
    - serde::Serialize + serde::Deserialize on scanner types
    - serde::Serialize + serde::Deserialize on FileCategory
    - chrono for default name timestamps
  patterns:
    - Path-indexed SQLite storage (one row per directory node, keyed by full path)
    - Transaction-based batch save with ON DELETE CASCADE
    - Internally-tagged serde enum for Entry (externally-tagged by default)
key-files:
  created:
    - src/snapshot/storage.rs
    - src/snapshot/serialize.rs
  modified:
    - src/scanner/types.rs
    - src/treemap/color.rs
    - src/snapshot/mod.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - "[Task 1] Added PartialEq alongside serde derives on scanner types to support assert_eq! in tests"
  - "[Task 1] ScanEvent excluded from serde derives due to Arc<std::io::Error> non-serializability; not needed for snapshot storage"
  - "[Task 2] Storage module gated behind #[cfg(feature = \"snapshot\")] to avoid rusqlite dependency without feature flag"
  - "[Task 2] Default name format uses chrono::Local::now().format(\"快照 %Y-%m-%d %H:%M\") per D-18"
  - "[Task 3] serialize module ungated (no rusqlite dependency); only DirNode serde_json wrapper functions"
metrics:
  duration: "~45 min"
  completed_date: "2026-05-06"
  tasks_completed: 3
  total_tasks: 3
  files_created: 2
  files_modified: 5
  test_count: 23 new tests (6 + 10 + 7)
---

# Phase 03 Plan 01: Snapshot Storage Layer

## One-liner

SQLite-backed snapshot storage with path-indexed schema, serde-derivable scanner types, and verified JSON round-trip serialization for DirNode trees.

## Overview

Created the foundation for Phase 3 snapshot functionality: scanner types can now serialize to/from JSON, and the SQLite storage layer supports full CRUD operations with path-indexed directory nodes.

## Tasks Completed

### Task 1: Serde Serialize/Deserialize on scanner types
**Commits:** `3d3405e` (RED), `91dbc34` (GREEN)

Added `serde::Serialize`, `serde::Deserialize`, and `PartialEq` derives to `FileEntry`, `DirNode`, `Entry`, `OthersEntry` in `src/scanner/types.rs` and `FileCategory` in `src/treemap/color.rs`. `ScanEvent` intentionally excluded (contains `Arc<std::io::Error>`). Verified with 6 JSON round-trip tests.

### Task 2: SQLite path-indexed storage with CRUD operations
**Commits:** `691159c` (RED), `e242a39` (GREEN), `5f6cf25` (fix)

Created `src/snapshot/storage.rs` with `SnapshotStorage` struct managing a rusqlite connection. Schema uses two tables: `snapshots` (metadata) and `snapshot_nodes` (path-indexed JSON subtrees). Features:
- WAL mode + foreign_keys enabled
- Transaction-based save with recursive node insertion
- ON DELETE CASCADE for clean snapshot removal (D-17)
- `idx_snapshot_nodes_parent` index for subtree queries
- Default name format: "快照 YYYY-MM-DD HH:MM" (D-18)
- Module gated behind `#[cfg(feature = "snapshot")]`

Verified with 10 tests covering schema creation, save/load, list, delete cascade, rename, default name, nonexistent load, and subtree load.

### Task 3: DirNode JSON serialization module
**Commits:** `5c07e66` (RED + GREEN)

Created `src/snapshot/serialize.rs` with 4 public wrapper functions: `serialize_tree`, `deserialize_tree`, `serialize_subtree`, `deserialize_subtree`. Verified with 7 tests covering basic round-trip, 3-level deep nesting, OthersEntry, AccessDenied, Symlink, and 100-child trees.

## Deviations from Plan

### Auto-fixed Issues (Rule 1 - Bug)

**1. Added PartialEq derives alongside serde derives**
- **Found during:** Task 1 implementation
- **Issue:** Tests use `assert_eq!` on `Entry` and nested structs, requiring `PartialEq` trait which is not auto-derived with serde
- **Fix:** Added `PartialEq` to `FileEntry`, `DirNode`, `OthersEntry`, `Entry` derives
- **Files modified:** `src/scanner/types.rs`
- **Commit:** `91dbc34`

**2. Feature gate for storage module**
- **Found during:** Overall verification
- **Issue:** `cargo test` without `--features snapshot` fails to compile because `storage.rs` unconditionally imports `rusqlite`
- **Fix:** Added `#[cfg(feature = "snapshot")]` to `mod storage` and `pub use storage::*` in `src/snapshot/mod.rs`
- **Files modified:** `src/snapshot/mod.rs`
- **Commit:** `5f6cf25`

**3. ScanEvent excluded from serde derives**
- **Found during:** Task 1 GREEN phase
- **Issue:** `ScanEvent` contains `Arc<std::io::Error>` which does not implement `Serialize`/`Deserialize`
- **Fix:** Intentionally did not add serde derives to `ScanEvent`; not needed for snapshot storage
- **Files modified:** `src/scanner/types.rs`
- **Commit:** `91dbc34`

### Plan Deviations

**4. Combined RED/GREEN for serialize module**
- **Found during:** Task 3 TDD cycle
- **Issue:** The serialize functions (`serialize_tree`, `deserialize_tree`) are trivial 1-line wrappers around `serde_json`. Since tests and implementations reference each other within the same module, they were committed together.
- **Fix:** Both RED tests and GREEN implementation in commit `5c07e66`; no functional impact
- **Files modified:** `src/snapshot/serialize.rs`

## Test Results

```
# Without --features snapshot: 47 tests pass
# With --features snapshot:    57 tests pass (47 + 10 storage + 0 overlap)

Task 1 (scanner::types):     6 new serde round-trip tests -- PASS
Task 2 (snapshot::storage):  10 CRUD tests -- PASS
Task 3 (snapshot::serialize): 7 JSON round-trip tests -- PASS
```

## Key Files

| File | Status | Purpose |
|------|--------|---------|
| `src/scanner/types.rs` | Modified | Added serde + PartialEq derives |
| `src/treemap/color.rs` | Modified | Added serde derives to FileCategory |
| `src/snapshot/mod.rs` | Modified | Export storage + serialize modules |
| `src/snapshot/storage.rs` | Created | SQLite CRUD operations |
| `src/snapshot/serialize.rs` | Created | DirNode JSON serialization |
| `Cargo.toml` | Modified | Added serde_json feature to rusqlite |

## Self-Check

- [x] All 3 tasks executed
- [x] Each task committed individually
- [x] TDD RED -> GREEN sequence verified (commits: `3d3405e` -> `91dbc34`, `691159c` -> `e242a39`, `5c07e66`)
- [x] All 23 new tests pass
- [x] `cargo check --features snapshot` passes with no errors
- [x] `grep serde_json Cargo.toml` returns match
- [x] No modifications to STATE.md or ROADMAP.md
