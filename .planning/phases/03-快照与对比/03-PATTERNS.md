# Phase 3: 快照与对比 - Pattern Map

**Mapped:** 2026-05-06
**Files analyzed:** 9 (7 new, 2 modified)
**Analogs found:** 7 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/snapshot/storage.rs` | service | CRUD (SQLite) | `src/scanner/mod.rs` + `src/platform/drives.rs` | role-match |
| `src/snapshot/serialize.rs` | utility | transform (JSON) | `src/scanner/types.rs` + `src/treemap/color.rs` | role-match |
| `src/snapshot/diff.rs` | utility | transform (pure compute) | `src/treemap/layout.rs` (pure algorithm module) | role-match |
| `src/snapshot/mod.rs` | config | barrel export | `src/scanner/mod.rs` + `src/treemap/mod.rs` | exact |
| `src/ui/comparison.rs` | component | request-response (egui) | `src/treemap/renderer.rs` + `src/app.rs` (Window pattern) | partial |
| `src/ui/snapshot_dialog.rs` | component | request-response (egui) | `src/app.rs` (show_comparison_window snippet) + `src/ui/info_panel.rs` | partial |
| `src/scanner/types.rs` | model | N/A (add derives) | self (add Serialize/Deserialize) | trivial |
| `src/treemap/color.rs` | model | N/A (add derives) | self (add Serialize/Deserialize) | trivial |
| `src/app.rs` | component | N/A (add state fields) | self (add snapshot_manager, comparison_state, dialog) | trivial |

## Pattern Assignments

---

### `src/snapshot/storage.rs` (service, CRUD)

**Analog:** `src/scanner/mod.rs` (module structure) + `src/scanner/error.rs` (error type)

This is the SQLite I/O layer. It manages the `snapshots` and `snapshot_nodes` tables. No existing SQLite code exists in the codebase, so patterns are drawn from module conventions.

**Module structure pattern** (from `src/scanner/mod.rs` lines 1-7):
```rust
pub mod types;
pub mod error;
pub mod walker;

pub use types::{AggThresholds, DirNode, Entry, FileEntry, OthersEntry, ScanError};
pub use error::ScanError;
pub use walker::scan_directory;
```

**Error type pattern** (from `src/scanner/error.rs` lines 1-23):
```rust
use std::path::PathBuf;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScanError {
    #[error("Access denied: {path}")]
    AccessDenied { path: PathBuf },

    #[error("Path not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Win32 error: {0}")]
    Win32(u32),

    #[error("IO error: {0}")]
    Io(Arc<std::io::Error>),
}
```

**SnapshotError should follow this exact pattern:**
- `#[derive(Debug, thiserror::Error)]` on enum
- Variants: `Database(rusqlite::Error)`, `Serialization(serde_json::Error)`, `NotFound { id: i64 }`, `InvalidName(String)`
- Each variant gets `#[error("...")]` with named fields

**Data model pattern** (from `src/scanner/types.rs` lines 4-8):
```rust
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
}
```

**SnapshotMeta struct should follow this pattern:**
```rust
#[derive(Debug, Clone)]
pub struct SnapshotMeta {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub root_path: String,
    pub total_size: u64,
    pub total_files: u64,
}
```

**Transaction pattern** (from RESEARCH.md Pattern 2):
```rust
fn save_snapshot(conn: &mut Connection, name: &str, root: &DirNode) -> Result<i64> {
    let tx = conn.transaction()?;
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
    insert_nodes_recursive(&tx, snapshot_id, root, None)?;
    tx.commit()?;
    Ok(snapshot_id)
}
```

**Feature gate pattern** (from `Cargo.toml` lines 33-34):
```toml
[features]
default = []
snapshot = ["rusqlite"]
```
All snapshot types in `app.rs` and callers must be gated with `#[cfg(feature = "snapshot")]`.

**Test placement** (from `src/scanner/types.rs` lines 115 and throughout):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // tests inline in the same file
}
```

---

### `src/snapshot/serialize.rs` (utility, transform)

**Analog:** `src/scanner/types.rs` (data model with serde pattern) + RESEARCH.md

Handles serialization/deserialization of `DirNode` trees to/from JSON strings.

**Imports pattern** (from `src/treemap/color.rs` lines 1-3):
```rust
use crate::scanner::{DirNode, Entry, FileEntry};
use egui::Color32;
use std::path::Path;
```

**serialize.rs imports should be:**
```rust
use crate::scanner::DirNode;
use serde::{Serialize, Deserialize};
```

**No custom Serialize/Deserialize impls are needed** -- just add derives to existing types. The `serde` derive handles `PathBuf` (as string), `#[serde(skip)]` for fields that don't serialize.

**Source:** `src/scanner/types.rs` -- add `Serialize, Deserialize` to all types:
- `FileEntry` (line 4): `#[derive(Debug, Clone, Serialize, Deserialize)]`
- `DirNode` (line 11): `#[derive(Debug, Clone, Serialize, Deserialize)]`
- `Entry` (line 22): `#[derive(Debug, Clone, Serialize, Deserialize)]` (serde default externally-tagged works for enums)
- `OthersEntry` (line 30): `#[derive(Debug, Clone, Serialize, Deserialize)]`

**From RESEARCH.md:** `FileCategory` in `color.rs` also needs `Serialize, Deserialize` (line 5):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileCategory {
```

---

### `src/snapshot/diff.rs` (utility, transform/pure compute)

**Analog:** `src/treemap/layout.rs` (pure algorithm module with inline tests)

Diff algorithm -- name-based recursive tree comparison producing `DiffNode` overlays.

**Algorithm module pattern** (from `src/treemap/layout.rs` lines 1-5):
```rust
use crate::scanner::{DirNode, Entry, FileEntry};
use crate::treemap::color::categorize_entry;
use egui::emath::{pos2, vec2, Rect};
use std::path::PathBuf;
```

**diff.rs should have minimal imports:**
```rust
use crate::scanner::{DirNode, Entry};
```

**Data model pattern** (from `src/scanner/types.rs` lines 11-19):
```rust
#[derive(Debug, Clone)]
pub struct DirNode {
    pub path: PathBuf,
    pub name: String,
    pub total_size: u64,
    pub file_count: u64,
    pub children: Vec<Entry>,
    pub access_denied: bool,
    pub dominant_cat: FileCategory,
}
```

**ChangeType enum follows scanner/types pattern:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Unchanged,
    Added,
    Removed,
    Grown,
    Shrunk,
}
```

**DiffNode struct pattern:**
```rust
#[derive(Debug, Clone)]
pub struct DiffNode {
    pub entry: Entry,
    pub change: ChangeType,
    pub old_size: Option<u64>,
    pub new_size: u64,
}
```

**Test pattern** (from `src/treemap/layout.rs` lines 210-336):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper builders at the top of the tests module
    fn make_dir(children: Vec<(String, u64)>) -> DirNode {
        // inline helper
    }

    #[test]
    fn test_diff_added_entry() { /* ... */ }

    #[test]
    fn test_diff_removed_entry() { /* ... */ }

    #[test]
    fn test_diff_grown_entry() { /* ... */ }

    #[test]
    fn test_diff_shrunk_entry() { /* ... */ }

    #[test]
    fn test_diff_unchanged() { /* ... */ }

    #[test]
    fn test_diff_recursive_tree() { /* ... */ }
}
```

---

### `src/snapshot/mod.rs` (config/route, barrel export)

**Analog:** `src/scanner/mod.rs` (exact match) + `src/treemap/mod.rs` (exact match)

**Exact pattern to copy** (from `src/scanner/mod.rs` lines 1-7):
```rust
pub mod types;
pub mod error;
pub mod walker;

pub use types::{AggThresholds, DirNode, Entry, FileEntry, OthersEntry, ScanError};
pub use error::ScanError;
pub use walker::scan_directory;
```

**snapshot/mod.rs should be:**
```rust
pub mod storage;
pub mod serialize;
pub mod diff;

#[cfg(feature = "snapshot")]
pub use storage::{SnapshotManager, SnapshotMeta, SnapshotError};
#[cfg(feature = "snapshot")]
pub use diff::{ChangeType, DiffNode, diff_level};
```

---

### `src/ui/comparison.rs` (component, request-response/egui)

**Analog:** `src/treemap/renderer.rs` (egui painter patterns) + RESEARCH.md Pattern 5 & 6

Side-by-side comparison window. Two treemap canvases rendered in one egui `Window`.

**Window pattern** (from RESEARCH.md Pattern 5 lines 1-45):
```rust
pub struct ComparisonWindow {
    pub open: bool,
    pub snapshot_id: i64,
    pub snapshot_name: String,
    pub snapshot_root: Option<Arc<DirNode>>,
    pub left_nav_stack: Vec<usize>,
    pub right_nav_stack: Vec<usize>,
}
```

**egui painter pattern** (from `src/treemap/renderer.rs` lines 29-34):
```rust
pub fn paint_treemap(
    ui: &mut Ui,
    nodes: &[TreemapNode],
    selected_index: Option<usize>,
    canvas_rect: emath::Rect,
) -> Option<TreemapAction> {
    let (response, painter) = ui.allocate_painter(canvas_rect.size(), Sense::hover());
```

**Overlay painting pattern** (from `src/treemap/renderer.rs` lines 56-82):
```rust
for (i, node) in nodes.iter().enumerate() {
    let rect = node.rect.translate(offset);
    if !response_rect.intersects(rect) { continue; }
    // ... gradient mesh, strokes, labels
    painter.rect_stroke(rect, CornerRadius::same(1), Stroke::new(0.5, ...));
    if selected_index == Some(i) {
        painter.rect_stroke(rect.shrink(1.0), ...);
    }
}
```

**Diff overlay function** (from RESEARCH.md Pattern 6 lines 477-508):
```rust
fn paint_diff_overlay(
    painter: &egui::Painter,
    rect: emath::Rect,
    change: ChangeType,
) {
    let overlay_color = match change {
        ChangeType::Added   => Color32::from_rgba_unmultiplied(0, 200, 0, 80),
        ChangeType::Removed => Color32::from_rgba_unmultiplied(200, 0, 0, 80),
        ChangeType::Grown   => Color32::from_rgba_unmultiplied(255, 165, 0, 80),
        ChangeType::Shrunk  => Color32::from_rgba_unmultiplied(0, 100, 200, 80),
        ChangeType::Unchanged => return,
    };
    painter.rect_filled(rect, CornerRadius::same(1), overlay_color);
    // icon in top-right corner
    let icon = match change { ... };
    let icon_pos = rect.right_top() + egui::vec2(-12.0, 2.0);
    painter.text(icon_pos, egui::Align2::LEFT_TOP, icon,
        egui::FontId::proportional(10.0), Color32::WHITE);
}
```

**Tooltip pattern** (from `src/treemap/renderer.rs` lines 136-154):
```rust
if let Some(pos) = response.hover_pos() {
    for node in nodes.iter().rev() {
        if node.rect.translate(offset).contains(pos) {
            response.on_hover_ui_at_pointer(|ui| {
                ui.set_min_width(200.0);
                ui.label(egui::RichText::new(&node.label).size(14.0).strong());
                // diff-specific: show old_size, new_size, delta
            });
            break;
        }
    }
}
```

---

### `src/ui/snapshot_dialog.rs` (component, request-response/egui)

**Analog:** `src/app.rs` (Window management pattern) + `src/ui/info_panel.rs` (UI component pattern)

Modal dialog for snapshot management (list, create, delete, rename, load, compare).

**UI component pattern** (from `src/ui/info_panel.rs` lines 6-24):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileListAction {
    None,
    Select(usize),
    Drill(usize),
}
```

**SnapshotDialogAction should follow this pattern:**
```rust
#[derive(Debug, Clone)]
pub enum SnapshotDialogAction {
    None,
    Create(String),       // name
    Delete(i64),          // snapshot_id
    Rename(i64, String),  // id, new_name
    Load(i64),            // snapshot_id
    Compare(i64),         // snapshot_id
}
```

**Dialog struct pattern** (from RESEARCH.md Pattern 7):
```rust
pub struct SnapshotDialog {
    pub open: bool,
    pub snapshots: Vec<SnapshotMeta>,
    pub selected_id: Option<i64>,
    pub rename_buffer: String,
}
```

**Window rendering pattern** (from `src/app.rs` lines 198-371 -- the `update` method):
```rust
impl eframe::App for DiskReviewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ...
        egui::Window::new("title")
            .open(&mut is_open)
            .resizable(true)
            .default_size([w, h])
            .show(ctx, |ui| {
                // content
            });
    }
}
```

**SnapshotDialog render function pattern:**
```rust
pub fn snapshot_dialog_ui(
    ui: &mut Ui,
    dialog: &mut SnapshotDialog,
) -> SnapshotDialogAction {
    let mut action = SnapshotDialogAction::None;
    // list snapshots with metadata
    // create / delete / rename / load / compare buttons
    action
}
```

---

### `src/scanner/types.rs` (model, N/A -- add derives)

**Analog:** Self-modification. Add `Serialize, Deserialize` derives.

**Current derives** (line 4):
```rust
#[derive(Debug, Clone)]
pub struct FileEntry {
```

**Should become:**
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
```

Apply to: `FileEntry` (line 4), `DirNode` (line 11), `Entry` (line 22), `OthersEntry` (line 30).

---

### `src/treemap/color.rs` (model, N/A -- add derives)

**Analog:** Self-modification. Add `Serialize, Deserialize` derives.

**Current derives** (line 5):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
```

**Should become:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FileCategory {
```

---

### `src/app.rs` (component, N/A -- add state fields)

**Analog:** Self-modification. Add snapshot-related state fields.

**Current struct** (lines 13-27):
```rust
pub struct DiskReviewerApp {
    pub drives: Vec<DriveInfo>,
    pub scan_result: Option<Arc<DirNode>>,
    pub scan_progress: Option<ScanEvent>,
    event_receiver: Option<Receiver<ScanEvent>>,
    pub status_message: String,
    cancel_token: Option<Arc<AtomicBool>>,
    pub nav_stack: Vec<usize>,
    pub selected_index: Option<usize>,
    pub treemap_nodes: Vec<crate::treemap::TreemapNode>,
    needs_rebuild: bool,
    last_canvas_rect: Option<Rect>,
    pending_resize: Option<Rect>,
}
```

**Add after `pending_resize`:**
```rust
    // Phase 3: Snapshot state
    #[cfg(feature = "snapshot")]
    pub snapshot_manager: Option<crate::snapshot::SnapshotManager>,
    #[cfg(feature = "snapshot")]
    pub comparison_state: Option<crate::ui::comparison::ComparisonWindow>,
    #[cfg(feature = "snapshot")]
    pub snapshot_dialog: crate::ui::snapshot_dialog::SnapshotDialog,
```

**Initialization in `new()`** (lines 30-46):
```rust
Self {
    drives,
    scan_result: None,
    // ...
    #[cfg(feature = "snapshot")]
    snapshot_manager: None,
    #[cfg(feature = "snapshot")]
    comparison_state: None,
    #[cfg(feature = "snapshot")]
    snapshot_dialog: crate::ui::snapshot_dialog::SnapshotDialog {
        open: false,
        snapshots: Vec::new(),
        selected_id: None,
        rename_buffer: String::new(),
    },
}
```

---

### `src/ui/mod.rs` (config, barrel export)

**Analog:** Self-modification. Add new submodules.

**Current** (lines 1-7):
```rust
pub mod breadcrumb;
pub mod file_list;
pub mod info_panel;

pub use breadcrumb::breadcrumb_ui;
pub use file_list::file_list_ui;
pub use info_panel::info_panel_ui;
```

**Add:**
```rust
#[cfg(feature = "snapshot")]
pub mod comparison;
#[cfg(feature = "snapshot")]
pub mod snapshot_dialog;
```

---

## Shared Patterns

### Feature Flag Gating
**Source:** `Cargo.toml` lines 33-34
**Apply to:** All snapshot-related code in `app.rs`, `ui/mod.rs`, `main.rs`
```toml
[features]
default = []
snapshot = ["rusqlite"]
```
Every use of snapshot types outside `src/snapshot/` must be gated:
```rust
#[cfg(feature = "snapshot")]
use crate::snapshot::SnapshotManager;
```

### Error Type Convention
**Source:** `src/scanner/error.rs` lines 1-23
**Apply to:** `src/snapshot/storage.rs` (SnapshotError)
```rust
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Snapshot not found: {id}")]
    NotFound { id: i64 },
}
```

### Data Model Convention
**Source:** `src/scanner/types.rs` lines 4-8
**Apply to:** All new structs in snapshot/diff modules
```rust
#[derive(Debug, Clone)]
pub struct SnapshotMeta {
    pub id: i64,
    pub name: String,
    // ...
}
```

### Test Module Convention
**Source:** `src/scanner/types.rs` line 115, `src/treemap/layout.rs` line 210, `src/treemap/color.rs` line 116
**Apply to:** `src/snapshot/storage.rs`, `src/snapshot/diff.rs`, `src/snapshot/serialize.rs`
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // inline tests
}
```

### Module Re-export Convention
**Source:** `src/scanner/mod.rs` lines 1-7, `src/treemap/mod.rs` lines 1-9
**Apply to:** `src/snapshot/mod.rs`, `src/ui/mod.rs`
```rust
pub mod storage;
pub mod serialize;
pub mod diff;

pub use storage::{SnapshotManager, SnapshotMeta, SnapshotError};
pub use diff::{ChangeType, DiffNode, diff_level};
```

### egui Window Pattern
**Source:** RESEARCH.md Pattern 5
**Apply to:** `src/ui/comparison.rs`, `src/ui/snapshot_dialog.rs`
```rust
egui::Window::new("title")
    .open(&mut is_open)
    .resizable(true)
    .default_size([w, h])
    .show(ctx, |ui| { /* content */ });
```

### egui Painter Pattern
**Source:** `src/treemap/renderer.rs` lines 29-111
**Apply to:** `src/ui/comparison.rs` (diff overlay rendering)
```rust
let (response, painter) = ui.allocate_painter(canvas_rect.size(), Sense::hover());
// painter.rect_filled(rect, CornerRadius::same(1), color);
// painter.text(pos, align, text, font, color);
```

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/ui/comparison.rs` | component | request-response | No existing side-by-side window exists; closest analog is `renderer.rs` (single treemap) + `app.rs` (Window management) |
| `src/ui/snapshot_dialog.rs` | component | request-response | No existing modal dialog exists; closest analog is `info_panel.rs` (panel UI) + `app.rs` (Window pattern) |

## Metadata

**Analog search scope:** `src/scanner/`, `src/treemap/`, `src/ui/`, `src/platform/`, `src/app.rs`, `src/main.rs`, `Cargo.toml`
**Files scanned:** 14 source files + Cargo.toml
**Pattern extraction date:** 2026-05-06
