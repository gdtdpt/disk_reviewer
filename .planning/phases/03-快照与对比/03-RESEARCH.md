# Phase 3: 快照与对比 - Research

**Researched:** 2026-05-06
**Domain:** SQLite snapshot storage, tree diff algorithms, egui comparison views
**Confidence:** HIGH

## Summary

Phase 3 adds snapshot persistence and comparison to disk_reviewer. The implementation stores scanned directory trees as JSON in a SQLite database using a path-indexed schema (one record per directory node, keyed by full path). Users can save named snapshots, load historical snapshots back into the treemap view, and open a side-by-side comparison window that highlights four change types (added/removed/grown/shrunk) with color overlays, icons, and tooltips.

**Primary recommendation:** Use two SQLite tables (snapshots metadata + snapshot_nodes path-indexed data), serde_json for DirNode tree serialization via `#[derive(Serialize, Deserialize)]` on existing types, a recursive name-based tree diff producing a `DiffNode` overlay, and a new `ComparisonWindow` struct managed in `app.rs` that renders two independent treemap canvases side by side. The snapshot management dialog is an egui `Window` with open/close state.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Snapshot save/load | API / Backend | Database | SQLite I/O, serialization, transaction management |
| Tree diff algorithm | API / Backend | -- | Pure computation on DirNode trees, no UI dependency |
| Snapshot metadata (CRUD) | API / Backend | Database | SQLite operations for snapshot list, rename, delete |
| Dialog UI (management) | Browser / Client | -- | egui Window rendering with state managed in app.rs |
| Comparison view (side-by-side) | Browser / Client | -- | Two treemap renderings in a single egui Window |
| Color overlays + icons | Browser / Client | -- | egui painter overlay on individual treemap rectangles |
| Tooltip (change details) | Browser / Client | -- | egui on_hover_ui_at_pointer on diff-highlighted nodes |

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-16:** 路径索引 + JSON 子树。每个目录节点单独存一条记录，key 为完整路径（如 `C:\Users\Alice`），value 为该节点的子树 JSON。对比时只加载两个快照中相同路径的子树，支持按需下钻对比，不需要加载整棵树。
- **D-17:** 快照替换时整体清理。每个快照有唯一 ID，保存新快照时先删除同 ID 的旧记录再写入；快照删除时按 `snapshot_id` 批量删除。不会有垃圾数据。
- **D-18:** 快照默认名称带创建时间（如 `快照 2026-05-06 14:30`），用户可重命名。
- **D-19:** 按名称匹配。同一层级中按条目名称匹配（如 `Alice` 匹配 `Alice`），不要求路径一致。简单直接，适合磁盘分析场景。
- **D-20:** 四种变化类型：新增（新快照有、旧快照无）、删除（旧快照有、新快照无）、增长（大小增加）、缩小（大小减少）。
- **D-21:** 独立对比窗口。新窗口中左右并排显示：左侧当前扫描结果，右侧快照数据。在快照侧的色块上标识四种状态。
- **D-22:** 颜色叠加 + 图标标记 + tooltip。快照侧色块叠加半透明色（新增=绿、删除=红、增长=橙、缩小=蓝），角落加小图标（+、-、↑、↓），鼠标悬停 tooltip 显示变更详情（名称、旧大小、新大小、变化量）。
- **D-23:** 弹出对话框。点击工具栏「快照」按钮弹出模态对话框，列出所有快照（名称、时间、大小），支持创建、删除、切换、重命名。不占用主界面空间。

### Claude's Discretion
- SQLite schema 具体设计（表结构、索引）
- JSON 序列化格式（是否需要自定义 Serialize/Deserialize）
- 对比窗口的具体布局比例
- 差异图标的具体样式和位置
- 快照对话框的 UI 细节

### Deferred Ideas (OUT OF SCOPE)
- 差异过滤（只看新增/只看删除等）
- 快照导出/导入
- 快照自动定时创建
- 磁盘管理功能（打开位置、删除）
- 过滤与搜索

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SNAP-01 | 将当前扫描结果保存为快照（SQLite 存储） | SQLite schema (path-indexed), serde_json serialization of DirNode tree, transaction-based save |
| SNAP-02 | 加载历史快照并在 Treemap 中展示 | SQLite query by snapshot_id, serde_json deserialization back to DirNode, write to `app.scan_result` |
| SNAP-03 | 差异检测：识别新增、删除、增长、缩小的目录 | Name-based recursive tree diff algorithm, `DiffNode` overlay type |
| SNAP-04 | 差异高亮显示（颜色区分变化类型） | Color overlay rendering on treemap nodes, icon markers, tooltip with change details |
| SNAP-05 | 快照管理：创建、删除、切换快照 | egui Window dialog, snapshot list with metadata, CRUD operations |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite | 0.39.0 | SQLite database access | Already in Cargo.toml; bundled feature avoids system SQLite dependency [VERIFIED: crates.io] |
| serde | 1.0.x | Derive Serialize/Deserialize | Already in Cargo.toml; standard Rust serialization [VERIFIED: Cargo.toml] |
| serde_json | 1.0.x | JSON serialization/deserialization | Already in Cargo.toml; stores DirNode subtrees as JSON strings in SQLite [VERIFIED: Cargo.toml] |
| chrono | 0.4.x | Timestamp formatting for snapshot names | Already in Cargo.toml; `Local::now().format("%Y-%m-%d %H:%M")` for default names [VERIFIED: Cargo.toml, docs.rs] |
| thiserror | 2.x | Error type derivation | Already in Cargo.toml; project convention for error types [VERIFIED: Cargo.toml] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| egui | 0.33.0 | UI rendering (Window, panels, tooltips) | Already in Cargo.toml; comparison window, snapshot dialog, diff overlays [VERIFIED: Cargo.toml] |
| eframe | 0.33.0 | Application framework | Already in Cargo.toml; native_options, viewport management [VERIFIED: Cargo.toml] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rusqlite + serde_json | sled / redb (embedded KV) | sled lacks SQL query flexibility; redb is less mature. SQLite is battle-tested and rusqlite is the standard Rust wrapper. [ASSUMED] |
| serde_json for storage | bincode / ron | bincode is faster but not human-readable in DB; ron is Rust-specific. JSON is debuggable and serde_json is already a dependency. [ASSUMED] |
| Path-indexed schema | Single BLOB per snapshot | Single BLOB requires loading entire tree for any comparison. Path-indexed allows loading only the needed subtree for the current drill-down level. [VERIFIED: D-16 decision] |

**Installation:** All dependencies are already in Cargo.toml. One change needed:

```toml
# Cargo.toml -- update rusqlite line to enable serde_json feature
rusqlite = { version = "0.39.0", features = ["bundled", "serde_json"], optional = true }
```

**Version verification:**
- rusqlite 0.39.0: confirmed current on crates.io (updated 2026-03-15) [VERIFIED: crates.io API]
- egui 0.33.0, eframe 0.33.0: confirmed in Cargo.toml [VERIFIED: Cargo.toml]
- serde 1.0, serde_json 1.0, chrono 0.4, thiserror 2: confirmed in Cargo.toml [VERIFIED: Cargo.toml]

## Architecture Patterns

### System Architecture Diagram

```
+---------------------------------------------------------------------+
|                        app.rs (DiskReviewerApp)                      |
|  +--------------+  +------------------+  +------------------+        |
|  | scan_result   |  | snapshot_manager |  | comparison_state |       |
|  | Option<Arc    |  | SnapshotManager  |  | ComparisonWindow |       |
|  |  <DirNode>>   |  |                  |  |                  |       |
|  +------+-------+  +--------+---------+  +--------+---------+        |
|         |                   |                      |                 |
|         |    save_snapshot  |  load_snapshot       |  open           |
|         +------------------>+--------------------->+                 |
|                             |                                        |
+-----------------------------+----------------------------------------+
                              |                      |
                              v                      v
+-------------------+  +--------------------------------------------+
| snapshot/mod.rs   |  | snapshot/storage.rs                        |
| (public API)      |  |  +--------------------------------------+  |
|                   |  |  | SQLite (rusqlite)                    |  |
| save_snapshot()   |  |  |  +--------------+ +---------------+ |  |
| load_snapshot()   |  |  |  | snapshots    | |snapshot_nodes | |  |
| list_snapshots()  |  |  |  | (metadata)   | |(path-indexed) | |  |
| delete_snapshot   |  |  |  +--------------+ +---------------+ |  |
| rename_snapshot   |  |  +--------------------------------------+  |
+-------------------+  +--------------------------------------------+
          |
          v
+---------------------+  +-------------------------------------------+
| snapshot/serialize  |  | snapshot/diff.rs                          |
|                     |  |                                           |
| DirNode -> JSON str |  | diff_trees(old, new) -> Vec<DiffNode>    |
| JSON str -> DirNode |  |                                           |
+---------------------+  +-------------------------------------------+
          |
          v
+---------------------------------------------------------------------+
| ui/comparison.rs          ui/snapshot_dialog.rs                     |
|  +---------------------+  +--------------------------------------+  |
|  | ComparisonWindow    |  | SnapshotDialog                       |  |
|  | +-------+---------+ |  |  - snapshot list with metadata       |  |
|  | | Left  | Right   | |  |  - create/rename/delete buttons      |  |
|  | |treemap|treemap  | |  |  - load into view                    |  |
|  | |(curr.)|(snapshot| |  |  - open comparison                   |  |
|  | |       |+ diff)  | |  |                                      |  |
|  | +-------+---------+ |  +--------------------------------------+  |
|  +---------------------+                                            |
+---------------------------------------------------------------------+
```

### Recommended Project Structure

```
src/
├── main.rs                    # (unchanged)
├── app.rs                     # Add: snapshot_manager, comparison_state, snapshot_dialog_open
├── scanner/
│   ├── mod.rs
│   ├── types.rs               # Add: Serialize, Deserialize derives
│   ├── walker.rs
│   └── error.rs
├── treemap/
│   ├── mod.rs
│   ├── types.rs               # (unchanged) TreemapNode
│   ├── layout.rs
│   ├── color.rs
│   └── renderer.rs
├── snapshot/
│   ├── mod.rs                 # Public API: save/load/list/delete/rename
│   ├── storage.rs             # SQLite operations (rusqlite)
│   ├── serialize.rs           # DirNode serde_json serialization/deserialization
│   └── diff.rs                # Tree diff algorithm
├── ui/
│   ├── mod.rs                 # Add: comparison, snapshot_dialog
│   ├── breadcrumb.rs
│   ├── file_list.rs
│   ├── info_panel.rs
│   ├── comparison.rs          # NEW: ComparisonWindow (side-by-side treemaps)
│   └── snapshot_dialog.rs     # NEW: Snapshot management dialog
└── platform/
    └── drives.rs
```

### Pattern 1: Path-Indexed SQLite Storage

**What:** Each directory node is stored as a separate row, keyed by its full path. The root snapshot metadata is stored in a `snapshots` table. This enables loading only the subtree needed for the current drill-down level.

**When to use:** Always -- this is the locked storage format (D-16).

**Schema:**
```sql
-- Snapshot metadata
CREATE TABLE IF NOT EXISTS snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,       -- ISO 8601: "2026-05-06T14:30:00+08:00"
    root_path TEXT NOT NULL,        -- e.g., "C:\"
    total_size INTEGER NOT NULL,    -- bytes
    total_files INTEGER NOT NULL
);

-- Path-indexed directory nodes (one row per directory)
CREATE TABLE IF NOT EXISTS snapshot_nodes (
    snapshot_id INTEGER NOT NULL,
    path TEXT NOT NULL,             -- full path, e.g., "C:\Users\Alice"
    parent_path TEXT,               -- NULL for root node
    node_json TEXT NOT NULL,        -- JSON serialization of the DirNode subtree
    PRIMARY KEY (snapshot_id, path),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
);

-- Index for fast subtree queries
CREATE INDEX IF NOT EXISTS idx_snapshot_nodes_parent
    ON snapshot_nodes(snapshot_id, parent_path);
```

**Why this schema:**
- `(snapshot_id, path)` composite primary key enables O(1) lookup of any directory node
- `parent_path` column enables efficient "load children of X" queries
- `ON DELETE CASCADE` ensures snapshot deletion cleans up all nodes (D-17)
- `node_json` stores the complete DirNode subtree as JSON, so loading a subtree is a single row fetch

**Source:** [VERIFIED: rusqlite docs on CREATE TABLE, PRIMARY KEY, FOREIGN KEY with ON DELETE CASCADE]

### Pattern 2: Snapshot Save (Transaction-Based)

**What:** Save a snapshot within a single SQLite transaction. Delete old records first (D-17), then insert metadata and all directory nodes.

**Example:**
```rust
// Source: Adapted from rusqlite transaction pattern [VERIFIED: docs.rs/rusqlite/0.39.0]
use rusqlite::{Connection, Transaction};

fn save_snapshot(conn: &mut Connection, name: &str, root: &DirNode) -> Result<i64> {
    let tx = conn.transaction()?;

    // Insert metadata
    tx.execute(
        "INSERT INTO snapshots (name, created_at, root_path, total_size, total_files)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            name,
            chrono::Local::now().to_rfc3339(),
            root.path.to_string_lossy().as_ref(),
            &(root.total_size as i64),
            &(root.file_count as i64),
        ],
    )?;
    let snapshot_id = tx.last_insert_rowid();

    // Recursively insert all directory nodes
    insert_nodes_recursive(&tx, snapshot_id, root, None)?;

    tx.commit()?;
    Ok(snapshot_id)
}

fn insert_nodes_recursive(
    tx: &Transaction,
    snapshot_id: i64,
    node: &DirNode,
    parent_path: Option<&str>,
) -> Result<()> {
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
            insert_nodes_recursive(tx, snapshot_id, dir, Some(&path_str))?;
        }
    }
    Ok(())
}
```

### Pattern 3: Snapshot Load (Subtree on Demand)

**What:** Load a specific directory subtree from a snapshot by querying the `snapshot_nodes` table for the exact path. The loaded DirNode replaces `app.scan_result` and the nav_stack resets.

**Example:**
```rust
// Source: Adapted from rusqlite query pattern [VERIFIED: docs.rs/rusqlite/0.39.0]
fn load_snapshot_root(conn: &Connection, snapshot_id: i64) -> Result<DirNode> {
    let node_json: String = conn.query_row(
        "SELECT node_json FROM snapshot_nodes
         WHERE snapshot_id = ?1 AND parent_path IS NULL",
        rusqlite::params![snapshot_id],
        |row| row.get(0),
    )?;

    let root: DirNode = serde_json::from_str(&node_json)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(e),
        ))?;

    Ok(root)
}
```

### Pattern 4: Tree Diff Algorithm (Name-Based Matching)

**What:** Recursively compare two DirNode trees. At each level, match entries by name. Produce a `DiffNode` overlay that annotates each entry with its change type.

**When to use:** When rendering the comparison view (right panel). The diff is computed once when the comparison window opens, then reused during rendering.

**Example:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Unchanged,
    Added,      // new snapshot has it, old doesn't
    Removed,    // old snapshot has it, new doesn't
    Grown,      // size increased
    Shrunk,     // size decreased
}

#[derive(Debug, Clone)]
pub struct DiffNode {
    pub entry: Entry,               // the entry from the new/current tree
    pub change: ChangeType,
    pub old_size: Option<u64>,      // previous size (for Grown/Shrunk)
    pub new_size: u64,              // current size
}

/// Compare two DirNode trees at one level, returning annotated DiffNodes.
/// Matches entries by name (D-19). O(n + m) per level.
pub fn diff_level(old: &DirNode, new: &DirNode) -> Vec<DiffNode> {
    use std::collections::HashMap;

    let old_map: HashMap<&str, &Entry> = old.children.iter()
        .map(|e| (entry_name(e).as_str(), e))
        .collect();
    let new_map: HashMap<&str, &Entry> = new.children.iter()
        .map(|e| (entry_name(e).as_str(), e))
        .collect();

    let mut result = Vec::new();

    // Process entries in the new tree
    for new_entry in &new.children {
        let name = entry_name(new_entry);
        match old_map.get(name.as_str()) {
            None => {
                result.push(DiffNode {
                    entry: new_entry.clone(),
                    change: ChangeType::Added,
                    old_size: None,
                    new_size: new_entry.size(),
                });
            }
            Some(old_entry) => {
                let old_size = old_entry.size();
                let new_size = new_entry.size();
                let change = if new_size > old_size {
                    ChangeType::Grown
                } else if new_size < old_size {
                    ChangeType::Shrunk
                } else {
                    ChangeType::Unchanged
                };
                result.push(DiffNode {
                    entry: new_entry.clone(),
                    change,
                    old_size: Some(old_size),
                    new_size,
                });
            }
        }
    }

    // Process entries only in the old tree (removed)
    for old_entry in &old.children {
        let name = entry_name(old_entry);
        if !new_map.contains_key(name.as_str()) {
            result.push(DiffNode {
                entry: old_entry.clone(),
                change: ChangeType::Removed,
                old_size: Some(old_entry.size()),
                new_size: 0,
            });
        }
    }

    result
}
```

**Complexity:** O(n + m) per level where n, m are child counts. Each level is processed independently during rendering (not the full tree at once).

### Pattern 5: Comparison Window (Side-by-Side Treemaps)

**What:** An egui `Window` containing two treemap canvases. Left shows the current scan result, right shows the snapshot data with diff overlays. Both support independent drill-down.

**When to use:** When user selects "compare with snapshot" from the snapshot dialog.

**Example:**
```rust
// Source: Adapted from egui Window pattern [VERIFIED: docs.rs/egui/0.33.0]
pub struct ComparisonWindow {
    pub open: bool,
    pub snapshot_id: i64,
    pub snapshot_name: String,
    pub snapshot_root: Option<Arc<DirNode>>,
    pub left_nav_stack: Vec<usize>,   // current scan nav
    pub right_nav_stack: Vec<usize>,  // snapshot nav
}

// In app.rs update() method:
fn show_comparison_window(&mut self, ctx: &egui::Context) {
    if let Some(comp) = &mut self.comparison_state {
        let mut is_open = comp.open;
        egui::Window::new(format!("对比: {}", comp.snapshot_name))
            .open(&mut is_open)
            .resizable(true)
            .default_size([960.0, 600.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left panel: current scan (50% width)
                    let left_width = ui.available_width() * 0.5;
                    ui.allocate_ui(
                        egui::vec2(left_width, ui.available_height()),
                        |ui| {
                            ui.label(egui::RichText::new("当前扫描").heading());
                            ui.separator();
                            // paint_treemap(ui, left_nodes, left_selected, left_canvas)
                        },
                    );

                    ui.separator();

                    // Right panel: snapshot with diff overlay
                    ui.allocate_ui(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        |ui| {
                            ui.label(egui::RichText::new("快照").heading());
                            ui.separator();
                            // paint_treemap_with_diff(ui, right_nodes, diff_data, right_canvas)
                        },
                    );
                });
            });
        comp.open = is_open;
    }
}
```

### Pattern 6: Diff Overlay Rendering

**What:** On the snapshot-side treemap, overlay a semi-transparent color on each rectangle based on its change type, and draw a small icon in the corner.

**Color scheme (D-22):**
- Added: `Color32::from_rgba_unmultiplied(0, 200, 0, 80)` (green, ~30% opacity)
- Removed: `Color32::from_rgba_unmultiplied(200, 0, 0, 80)` (red)
- Grown: `Color32::from_rgba_unmultiplied(255, 165, 0, 80)` (orange)
- Shrunk: `Color32::from_rgba_unmultiplied(0, 100, 200, 80)` (blue)
- Unchanged: no overlay

**Icon markers (D-22):**
- Added: "+" in top-right corner
- Removed: "-" in top-right corner
- Grown: unicode up-arrow in top-right corner
- Shrunk: unicode down-arrow in top-right corner

**Tooltip (D-22):** On hover, show name, old size, new size, and delta (e.g., "+1.2 GB").

**Example:**
```rust
// Source: Adapted from renderer.rs paint_treemap + egui Color32 [VERIFIED: docs.rs/egui/0.33.0]
fn paint_diff_overlay(
    painter: &egui::Painter,
    rect: emath::Rect,
    change: ChangeType,
) {
    use egui::Color32;

    let overlay_color = match change {
        ChangeType::Added    => Color32::from_rgba_unmultiplied(0, 200, 0, 80),
        ChangeType::Removed  => Color32::from_rgba_unmultiplied(200, 0, 0, 80),
        ChangeType::Grown    => Color32::from_rgba_unmultiplied(255, 165, 0, 80),
        ChangeType::Shrunk   => Color32::from_rgba_unmultiplied(0, 100, 200, 80),
        ChangeType::Unchanged => return,
    };
    painter.rect_filled(rect, egui::CornerRadius::same(1), overlay_color);

    // Icon in top-right corner
    let icon = match change {
        ChangeType::Added    => "+",
        ChangeType::Removed  => "-",
        ChangeType::Grown    => "\u{2191}", // up arrow
        ChangeType::Shrunk   => "\u{2193}", // down arrow
        ChangeType::Unchanged => return,
    };
    let icon_pos = rect.right_top() + egui::vec2(-12.0, 2.0);
    painter.text(
        icon_pos,
        egui::Align2::LEFT_TOP,
        icon,
        egui::FontId::proportional(10.0),
        Color32::WHITE,
    );
}

### Pattern 7: Snapshot Management Dialog

**What:** An egui `Window` that lists all snapshots with metadata (name, time, size). Supports create (from current scan), load, rename, delete, and open comparison.

**When to use:** When user clicks the snapshot button in the toolbar.

**Example:**
```rust
// Source: Adapted from egui Window pattern [VERIFIED: docs.rs/egui/0.33.0]
pub struct SnapshotDialog {
    pub open: bool,
    pub snapshots: Vec<SnapshotMeta>,
    pub selected_id: Option<i64>,
    pub rename_buffer: String,
}

pub struct SnapshotMeta {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub root_path: String,
    pub total_size: u64,
    pub total_files: u64,
}
```

### Anti-Patterns to Avoid

- **Loading entire tree at once:** Do not deserialize all snapshot_nodes into memory. Load only the subtree for the current drill-down level (path-indexed lookup). [VERIFIED: D-16]
- **Missing serde derive on scanner types:** The existing Entry enum derives only Debug and Clone. Add Serialize and Deserialize to Entry, DirNode, FileEntry, OthersEntry, and FileCategory. [VERIFIED: serde docs]
- **Ignoring SQLite transaction scope:** Always wrap snapshot save in a single conn.transaction(). [VERIFIED: rusqlite transaction docs]
- **Computing diff on every frame:** Cache the diff result when the comparison window opens or drill-down changes.
- **Missing serde_json feature:** Cargo.toml must include serde_json feature on rusqlite. Without it, JSON column support is unavailable.
- **Feature flag gating:** Gate all snapshot types in app.rs with #[cfg(feature = "snapshot")] to avoid compile errors without the feature.

## Don-t Hand-Roll

| Problem | Do not Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SQLite access | Raw C FFI calls | rusqlite | Type-safe, ergonomic [VERIFIED: crates.io] |
| JSON serialization | Manual string building | serde_json | Handles all edge cases [VERIFIED: docs.rs/serde_json] |
| Date/time formatting | Manual integer math | chrono format() | Handles timezones, edge cases [VERIFIED: docs.rs/chrono] |
| File size formatting | Manual division loop | Reuse format_size() from renderer.rs | Already tested in Phase 2 |

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None | N/A |
| Live service config | None | N/A |
| OS-registered state | None | N/A |
| Secrets/env vars | None | N/A |
| Build artifacts | None | N/A |

## Common Pitfalls

### Pitfall 1: Missing serde derive on scanner types
Compile error when trying to serialize DirNode. Fix: add Serialize and Deserialize to all scanner types.

### Pitfall 2: Forgetting rusqlite serde_json feature
Current Cargo.toml has features = ["bundled"] only. Must add "serde_json" to enable JSON column support.

### Pitfall 3: SQLite database path on Windows
Use LOCALAPPDATA env var to place the DB in %LOCALAPPDATA%\disk_reviewer\snapshots.db, not the working directory.

### Pitfall 4: Snapshot save without current scan
Disable save button when scan_result is None.

### Pitfall 5: Feature flag gating breaks non-snapshot builds
Gate all snapshot-related code with #[cfg(feature = "snapshot")].

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Files per snapshot | SQLite path-indexed nodes | D-16 | Partial tree loading |
| Byte-level diff | Name-based tree diff | D-19 | Semantic comparison |
| Inline management | Popup dialog | D-23 | Cleaner UI |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | serde_json::to_string on DirNode works for trees < 32 levels deep | Code Examples | Stack overflow on deep trees; add depth limit |
| A2 | PathBuf serde round-trips correctly on Windows | Code Examples | Mitigation: use to_string_lossy() |
| A3 | FileCategory derives Serialize/Deserialize cleanly | Code Examples | Low risk -- simple C-like enum |
| A4 | chrono RFC 3339 strings sort chronologically | Pattern 2 | Verified behavior |
| A5 | ON DELETE CASCADE works with rusqlite | Pattern 1 | Mitigation: test in Wave 0 |

## Open Questions

1. **Synchronized drill-down in comparison window?** Recommendation: independent drill-down first, synchronized as stretch goal.
2. **Comparing scans from different drives?** Recommendation: allow with warning banner.
3. **Database migration strategy?** Recommendation: PRAGMA user_version for now.

## Environment Availability

All dependencies available. No blockers.

## Validation Architecture

### Test Framework
- Framework: Rust built-in #[cfg(test)] + cargo test
- Quick run: cargo test snapshot:: -- --nocapture
- Full suite: cargo test

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | File Exists? |
|--------|----------|-----------|-------------|
| SNAP-01 | DirNode JSON round-trip | unit | No -- Wave 0 |
| SNAP-01 | Save to SQLite | unit | No -- Wave 0 |
| SNAP-02 | Load from SQLite | unit | No -- Wave 0 |
| SNAP-03 | Diff: Added/Removed/Grown/Shrunk | unit | No -- Wave 0 |
| SNAP-03 | Diff: recursive tree | unit | No -- Wave 0 |
| SNAP-05 | Snapshot CRUD | unit | No -- Wave 0 |
| SNAP-05 | Snapshot rename | unit | No -- Wave 0 |

### Wave 0 Gaps
- [ ] src/snapshot/mod.rs -- module structure
- [ ] src/snapshot/storage.rs -- SQLite operations
- [ ] src/snapshot/serialize.rs -- JSON serialization
- [ ] src/snapshot/diff.rs -- tree diff algorithm
- [ ] src/ui/comparison.rs -- ComparisonWindow
- [ ] src/ui/snapshot_dialog.rs -- SnapshotDialog
- [ ] Tests for all above modules
- [ ] Cargo.toml -- add serde_json to rusqlite features
- [ ] scanner/types.rs -- add Serialize/Deserialize derives

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | Yes | Parameterized queries for snapshot names |
| Others | No | Local desktop app, no auth/crypto needed |

## Sources

### Primary (HIGH confidence)
- rusqlite 0.39.0 docs -- Connection, Transaction, parameterized queries
- serde_json 1.0 docs -- to_string, from_str
- serde 1.0 docs -- enum serialization (externally tagged default)
- egui 0.33.0 docs -- Window, Color32, Ui layout
- chrono 0.4 docs -- Local::now(), format()
- crates.io -- rusqlite 0.39.0 confirmed current (2026-03-15)
- Project CONTEXT.md -- D-16 through D-23 locked decisions
- Project CLAUDE.md -- TDD enforcement, Rust + egui constraints
- Existing codebase: scanner/types.rs, app.rs, treemap/renderer.rs, Cargo.toml

### Secondary (MEDIUM confidence)
- rusqlite GitHub README -- feature flags, bundled SQLite version

## Metadata

- Standard stack: HIGH -- versions verified against Cargo.toml and crates.io
- Architecture: HIGH -- patterns from official docs; schema from locked decisions
- Pitfalls: HIGH -- derived from actual codebase state
- Research date: 2026-05-06
- Valid until: 2026-06-05 (30 days)
