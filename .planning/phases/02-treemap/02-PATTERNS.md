# Phase 2: Treemap 可视化 - Pattern Map

**Mapped:** 2026-05-05
**Files analyzed:** 7 (4 new, 1 modified, 2 stubs)
**Analogs found:** 5 / 7

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/treemap/mod.rs` | module-entry | transform | `src/scanner/mod.rs` | exact |
| `src/treemap/layout.rs` | utility | transform | `src/scanner/types.rs` (DirNode::finish) | role-match |
| `src/treemap/renderer.rs` | component | transform | `src/app.rs` (eframe::App impl) | role-match |
| `src/ui/mod.rs` | module-entry | transform | `src/scanner/mod.rs` | exact |
| `src/ui/breadcrumb.rs` | component | request-response | `src/app.rs` (drive list buttons) | role-match |
| `src/ui/info_panel.rs` | component | request-response | `src/app.rs` (scan result preview) | role-match |
| `src/app.rs` | component | request-response | `src/app.rs` (existing) | self-modify |

## Pattern Assignments

---

### `src/treemap/mod.rs` (module-entry, transform)

**Analog:** `src/scanner/mod.rs` (lines 1-8)

This is the standard module entry pattern used throughout the project. The file already exists as a stub (`// Phase 2: Treemap 可视化模块`) and needs to be expanded.

**Module declaration pattern** (from `scanner/mod.rs` lines 1-8):
```rust
pub mod types;
pub mod error;
pub mod walker;

pub use types::{AggThresholds, DirNode, Entry, FileEntry, OthersEntry, ScanEvent};
pub use error::ScanError;
pub use walker::scan_directory;
```

**Expected structure for treemap/mod.rs:**
```rust
pub mod layout;
pub mod renderer;

pub use layout::{TreemapNode, layout_treemap};
pub use renderer::TreemapRenderer;
```

**Key conventions:**
- Declare submodules with `pub mod`
- Re-export key types with `pub use` for convenient access as `crate::treemap::TreemapNode`
- Keep implementation details private (e.g., internal helper functions not re-exported)

---

### `src/treemap/layout.rs` (utility, transform)

**Analog:** `src/scanner/types.rs` — `DirNode::finish()` method (lines 65-123)

The layout algorithm is a pure data transformation: it consumes `DirNode` and produces `Vec<TreemapNode>`. This mirrors how `DirNode::finish()` transforms data in-place.

**Data model pattern** (from `scanner/types.rs` lines 1-17):
```rust
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct DirNode {
    pub path: PathBuf,
    pub name: String,
    pub total_size: u64,
    pub file_count: u64,
    pub children: Vec<Entry>,
    pub access_denied: bool,
}
```

**Apply to TreemapNode:** Follow the same `#[derive(Debug, Clone)]` struct pattern. The `TreemapNode` struct (D-07) should contain `rect: Rect`, `label: String`, `color: Color32`, `depth: usize`, and a reference to the source `Entry`.

**Method pattern** (from `scanner/types.rs` lines 65-74):
```rust
impl DirNode {
    pub fn finish(&mut self, thresholds: &AggThresholds) {
        // 先递归处理子目录
        for child in &mut self.children {
            if let Entry::Dir(ref mut dir) = child {
                dir.finish(thresholds);
            }
        }
        // ... processing logic ...
    }
}
```

**Apply to layout function:** The layout function should follow the same recursive pattern:
```rust
pub fn layout_treemap(node: &DirNode, rect: Rect, depth: usize) -> Vec<TreemapNode> {
    // 1. Sort children by size (descending) — mirrors DirNode finish() sorting
    // 2. Run squarified algorithm on children
    // 3. Recurse into subdirectories
}
```

**Sorting pattern** (from `scanner/types.rs` line 88):
```rust
self.children.sort_by_key(|e| std::cmp::Reverse(e.size()));
```

**Size accessor pattern** (from `scanner/types.rs` lines 36-45):
```rust
impl Entry {
    pub fn size(&self) -> u64 {
        match self {
            Entry::File(f) => f.size,
            Entry::Dir(d) => d.total_size,
            Entry::Others(o) => o.size,
            _ => 0,
        }
    }
}
```

**Testing pattern** (from `scanner/types.rs` lines 125-233):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_dir_with_entries(n: usize, base_size: u64) -> DirNode {
        // ... test helper ...
    }

    #[test]
    fn test_layout_basic() {
        // Test that layout produces correct rectangles
    }

    #[test]
    fn test_layout_single_entry() {
        // Edge case: single child fills entire rect
    }

    #[test]
    fn test_layout_preserves_total_size() {
        // Sum of child rect areas should equal parent area
    }
}
```

---

### `src/treemap/renderer.rs` (component, transform)

**Analog:** `src/app.rs` — `eframe::App` impl (lines 124-173)

The renderer consumes `Vec<TreemapNode>` and draws via egui's `Painter`. The pattern is the same as the existing `update()` method which uses `CentralPanel` and egui widgets.

**egui drawing context pattern** (from `app.rs` lines 129-131):
```rust
egui::CentralPanel::default().show(ctx, |ui| {
    ui.heading("Disk Reviewer");
    ui.separator();
});
```

**Apply to renderer:** The renderer should expose a `draw(&self, ui: &mut Ui, nodes: &[TreemapNode])` method that uses `ui.painter()` to draw rectangles. The renderer is called from `app.rs`'s `update()` method.

**Painter pattern for drawing rectangles:**
```rust
// Inside a egui UI context:
let painter = ui.painter();
for node in nodes {
    // Draw filled rectangle
    painter.rect_filled(node.rect, 0.0, node.color);
    // Draw border
    painter.rect_stroke(node.rect, 0.0, Stroke::new(1.0, Color32::BLACK));
    // Draw label if area is large enough (D-11)
    if node.rect.area() > LABEL_AREA_THRESHOLD {
        painter.text(
            node.rect.left_top() + vec2(2.0, 2.0),
            egui::Align2::LEFT_TOP,
            &node.label,
            FontId::proportional(12.0),
            Color32::WHITE,
        );
    }
}
```

**Interaction pattern** (from `app.rs` lines 135-153 — drive button handling):
```rust
ui.horizontal(|ui| {
    ui.label(format!("{}: ...", drive.letter, ...));
    if ui.button("扫描").clicked() {
        clicked = true;
    }
});
```

**Apply to treemap node clicking:** Use `egui::Sense::click()` on node rects:
```rust
for node in nodes {
    let response = ui.interact(node.rect, Id::new(&node.label), Sense::click());
    if response.clicked() {
        // Trigger drill-down (D-12)
    }
    if response.hovered() {
        // Show tooltip with name and size (D-11)
        response.on_hover_text(format!("{}\n{}", node.label, format_size(node.size)));
    }
}
```

**Status message pattern** (from `app.rs` line 27, 160):
```rust
pub status_message: String;
// ...
ui.label(&self.status_message);
```

---

### `src/ui/mod.rs` (module-entry, transform)

**Analog:** `src/scanner/mod.rs` (lines 1-8)

Same module entry pattern as `treemap/mod.rs`. The file already exists as a stub (`// Phase 1+: UI 组件（状态面板等）`).

**Expected structure:**
```rust
pub mod breadcrumb;
pub mod info_panel;

pub use breadcrumb::Breadcrumb;
pub use info_panel::InfoPanel;
```

---

### `src/ui/breadcrumb.rs` (component, request-response)

**Analog:** `src/app.rs` — drive list with clickable buttons (lines 135-157)

Breadcrumb renders clickable path segments. The pattern is a horizontal sequence of clickable labels, matching the drive list button pattern.

**Clickable item pattern** (from `app.rs` lines 136-153):
```rust
ui.horizontal(|ui| {
    ui.label("逻辑盘:");
    let scan_requests: Vec<PathBuf> = self.drives.iter().filter_map(|drive| {
        let mut clicked = false;
        ui.horizontal(|ui| {
            ui.label(format!("{}: ...", drive.letter, ...));
            if ui.button("扫描").clicked() {
                clicked = true;
            }
        });
        if clicked {
            Some(PathBuf::from(format!(r"{}:\", drive.letter)))
        } else {
            None
        }
    }).collect();
    for path in scan_requests {
        self.start_scan(path);
    }
});
```

**Apply to breadcrumb:** Each path segment is a clickable label. Collect click events and navigate:
```rust
pub fn show_breadcrumb(ui: &mut Ui, path: &[PathBuf]) -> Option<usize> {
    let mut clicked_index = None;
    ui.horizontal(|ui| {
        for (i, segment) in path.iter().enumerate() {
            if i > 0 {
                ui.label(" > ");
            }
            if ui.button(segment.to_string_lossy()).clicked() {
                clicked_index = Some(i);
            }
        }
    });
    clicked_index
}
```

**State management pattern** (from `app.rs` lines 10-17):
```rust
pub struct DiskReviewerApp {
    pub drives: Vec<DriveInfo>,
    pub scan_result: Option<Arc<DirNode>>,
    // ...
}
```

**Apply to breadcrumb state:** The breadcrumb path should be stored in `app.rs` as `current_path: Vec<PathBuf>` or similar, and the breadcrumb component reads it and returns which index was clicked.

---

### `src/ui/info_panel.rs` (component, request-response)

**Analog:** `src/app.rs` — scan result preview display (lines 163-170)

The info panel shows selected item details using `ui.label()` calls, matching the scan result preview pattern.

**Detail display pattern** (from `app.rs` lines 163-170):
```rust
if let Some(result) = &self.scan_result {
    ui.label(format!(
        "根目录: {}  总大小: {:.1} MB  文件数: {}",
        result.path.display(),
        result.total_size as f64 / 1e6,
        result.file_count
    ));
}
```

**Apply to info panel:** Show selected node details with the same `ui.label(format!(...))` pattern:
```rust
pub fn show_info_panel(ui: &mut Ui, selected: Option<&TreemapNode>, total_size: u64) {
    match selected {
        Some(node) => {
            ui.label(format!("路径: {}", node.label));
            ui.label(format!("大小: {:.1} MB", node.size as f64 / 1e6));
            ui.label(format!("占比: {:.1}%", node.size as f64 / total_size as f64 * 100.0));
            // D-10: Color legend below
            ui.separator();
            ui.label("图例:");
            // ... color legend entries ...
        }
        None => {
            ui.label("点击色块查看详情");
        }
    }
}
```

**Conditional display pattern** (from `app.rs` line 163):
```rust
if let Some(result) = &self.scan_result {
    // display details
}
```

---

### `src/app.rs` (component, request-response) — MODIFICATION

**Analog:** `src/app.rs` (self — extending existing structure)

This is a modification to the existing `DiskReviewerApp`. The patterns to follow are already established in the file.

**Struct extension pattern** — Add new fields to the existing struct (lines 10-17):
```rust
pub struct DiskReviewerApp {
    pub drives: Vec<DriveInfo>,
    pub scan_result: Option<Arc<DirNode>>,
    pub scan_progress: Option<ScanEvent>,
    event_receiver: Option<Receiver<ScanEvent>>,
    pub status_message: String,
    cancel_token: Option<Arc<AtomicBool>>,
    // NEW for Phase 2:
    pub current_node: Option<Arc<DirNode>>,       // Currently viewed subtree
    pub breadcrumb_path: Vec<PathBuf>,             // Navigation path
    pub selected_node: Option<String>,             // Selected TreemapNode label
    pub treemap_nodes: Vec<TreemapNode>,           // Cached layout output
}
```

**Constructor extension pattern** — Add initialization in `new()` (lines 20-30):
```rust
pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    let drives = drives::enumerate_drives();
    Self {
        drives,
        scan_result: None,
        scan_progress: None,
        event_receiver: None,
        status_message: "就绪".to_string(),
        cancel_token: None,
        // NEW:
        current_node: None,
        breadcrumb_path: Vec::new(),
        selected_node: None,
        treemap_nodes: Vec::new(),
    }
}
```

**Event handling extension** — In `consume_events()`, set `current_node` when scan completes (lines 88-103):
```rust
ScanEvent::Complete { root, duration, total_files, access_denied_count } => {
    self.scan_result = Some(Arc::new(root.clone()));
    self.current_node = Some(Arc::new(root.clone()));  // NEW: start at root
    self.breadcrumb_path = vec![root.path.clone()];      // NEW: init breadcrumb
    // ... existing status message ...
}
```

**UI layout pattern** — The existing `update()` method uses `CentralPanel`. Phase 2 needs to change this to a left/right split (D-14):
```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    self.consume_events(ctx);

    // Breadcrumb at top
    egui::TopBottomPanel::top("breadcrumb").show(ctx, |ui| {
        // Breadcrumb rendering
    });

    // Main area: 70% treemap, 30% info panel
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.columns(2, |columns| {
            // Left column (70%): Treemap
            columns[0].with_layout(Layout::top_down(Align::Min), |ui| {
                // Treemap rendering
            });
            // Right column (30%): Info panel
            columns[1].with_layout(Layout::top_down(Align::Min), |ui| {
                // Info panel rendering
            });
        });
    });
}
```

**Import pattern** — Add new imports at top of file:
```rust
use crate::treemap::{layout_treemap, TreemapNode};
use crate::ui::{Breadcrumb, InfoPanel};
```

---

## Shared Patterns

### Module Entry Pattern
**Source:** `src/scanner/mod.rs` (lines 1-8)
**Apply to:** `src/treemap/mod.rs`, `src/ui/mod.rs`

```rust
pub mod submodule1;
pub mod submodule2;

pub use submodule1::KeyType;
pub use submodule2::AnotherType;
```

Every module follows: declare submodules, re-export key types.

### Data Model Pattern
**Source:** `src/scanner/types.rs` (lines 1-63)
**Apply to:** `src/treemap/layout.rs` (TreemapNode struct)

```rust
#[derive(Debug, Clone)]
pub struct TypeName {
    pub field: FieldType,
    // ...
}
```

All data structs use `#[derive(Debug, Clone)]` and public fields.

### Error Handling Pattern
**Source:** `src/scanner/error.rs` (lines 1-23)
**Apply to:** `src/treemap/layout.rs` (if error types needed)

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum ErrorType {
    #[error("Description: {field}")]
    Variant { field: FieldType },
}
```

Use `thiserror` with `#[error("...")]` attributes. Implement `From` for external error types.

### Test Pattern
**Source:** `src/scanner/types.rs` (lines 125-233), `src/scanner/walker.rs` (lines 187-261)
**Apply to:** All new algorithm/utility files

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper functions for test data construction
    fn make_test_data() -> DataType {
        // ...
    }

    #[test]
    fn test_basic_case() {
        // Arrange -> Act -> Assert
    }

    #[test]
    fn test_edge_case_empty() {
        // Edge case testing
    }

    #[test]
    fn test_error_case() {
        // Error path testing
    }
}
```

Tests are inline in the source file. Use descriptive test names in English (matching existing convention).

### eframe App Pattern
**Source:** `src/app.rs` (lines 10-186)
**Apply to:** `src/app.rs` modifications, `src/treemap/renderer.rs`

```rust
pub struct AppName {
    // State fields
}

impl AppName {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialization
    }
}

impl eframe::App for AppName {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Consume events / update state
        // 2. Render UI with egui panels and widgets
    }
}
```

### egui Widget Pattern
**Source:** `src/app.rs` (lines 129-172)
**Apply to:** `src/ui/breadcrumb.rs`, `src/ui/info_panel.rs`, `src/treemap/renderer.rs`

```rust
ui.horizontal(|ui| {
    ui.label("text");
    if ui.button("label").clicked() {
        // handle click
    }
});

ui.separator();

if let Some(data) = &self.optional_field {
    ui.label(format!("{}: {}", label, data));
}
```

### Cross-module Reference Pattern
**Source:** `src/app.rs` line 8, `src/scanner/walker.rs` line 14
**Apply to:** `src/treemap/layout.rs`, `src/treemap/renderer.rs`

```rust
use crate::scanner::{DirNode, Entry};
// or
use crate::scanner::error::ScanError;
```

Use `crate::` prefix for cross-module imports. Import from the module's `pub use` re-exports.

### Channel Event Pattern
**Source:** `src/app.rs` (lines 83-121)
**Apply to:** Any future async interaction (Phase 2 doesn't add new channels, but the pattern is relevant)

```rust
fn consume_events(&mut self, ctx: &egui::Context) {
    if let Some(receiver) = &self.event_receiver {
        let mut count = 0;
        loop {
            match receiver.try_recv() {
                Ok(event) => { /* handle */ count += 1; }
                Err(TryRecvError::Empty) => break;
                Err(TryRecvError::Disconnected) => { self.event_receiver = None; break; }
            }
            if count >= 100 { break; } // Per-frame limit
        }
        if count > 0 { ctx.request_repaint(); }
    }
}
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/treemap/layout.rs` | utility | transform | No existing squarified treemap implementation. Closest analog is `DirNode::finish()` for the recursive data transformation pattern, but the algorithm itself is new. Planner should reference the Bruls et al. (2000) paper from CONTEXT.md. |
| `src/treemap/renderer.rs` | component | transform | No existing egui Painter-based custom drawing code. The `eframe::App` impl in `app.rs` is the closest analog for egui context usage, but custom rectangle rendering with `Painter::rect_filled` is new. |

## Metadata

**Analog search scope:** `src/scanner/`, `src/platform/`, `src/app.rs`, `src/main.rs`
**Files scanned:** 10 Rust source files + Cargo.toml + CONTEXT.md + ROADMAP.md
**Pattern extraction date:** 2026-05-05
