# disk_reviewer — Disk Space Visual Reviewer

🌐 **中文**: [README.md](README.md)

---

## Project Overview

**disk_reviewer** is a Windows desktop disk space visualization tool. It uses a Treemap to provide an intuitive, at-a-glance view of disk space distribution — answering the question "what's taking up all my space?"

After selecting a logical drive, the application asynchronously scans the directory tree and converts each directory and file into colored rectangles whose areas are proportional to their sizes. Users can click directory blocks to drill down level by level, use breadcrumb navigation to quickly return to any parent level, and view complete information about the selected item in the right-side detail panel. The application also supports saving scan results as historical snapshots, comparing them with previous data, and highlighting added, deleted, grown, and shrunk directories.

## Core Features

| Feature | Description |
|---------|-------------|
| **Drive Overview** | Enumerates all Windows logical drives, shows total and free space, one-click scan start |
| **Async Scanning** | Background thread pool traverses the directory tree, incrementally pushing results to the UI without freezing the interface |
| **Treemap Visualization** | Based on the Squarified Treemap algorithm (Bruls et al. 2000), producing rectangles with near 1:1 aspect ratios for optimal visual clarity |
| **File Type Coloring** | Automatically assigns colors based on 10 file categories: Document, Image, Video, Audio, Archive, Code, Executable, System, Temp, Other |
| **Drill-Down Navigation** | Click a directory rectangle to descend into its subdirectory view, with real-time re-layout |
| **Breadcrumb Navigation** | Horizontal scrollable path bar; click any segment to jump directly to that level |
| **Detail Panel** | Shows name, size, percentage, and type for the selected block; for directories, additionally shows file count and subdirectory count; when nothing is selected, shows current directory summary |
| **Color Legend** | Right-side panel always displays color swatches with labels for all 10 file categories |
| **Historical Snapshots** | Saves scan results to a single-file SQLite database; supports loading previous versions |
| **Diff Comparison** | Detects added/deleted/grown/shrunk entries based on directory tree structure; color-codes change types |
| **Large Directory Optimization** | Small files are automatically aggregated into "Others" entries to prevent memory exhaustion and rendering lag |

## Technical Architecture

```
Tech Stack:    Rust + egui (eframe 0.33)
Deployment:    Single-file .exe, no runtime dependencies
Platform API:  Win32 (FindFirstFileExW, GetLogicalDrives, GetDiskFreeSpaceExW)
Snapshot DB:   SQLite single-file database
Scan Strategy: Async thread pool + incremental push
Dev Discipline: TDD (Test-Driven Development), 100% coverage on critical paths
```

## Code Structure

```
src/
├── main.rs              # Entry point, eframe app startup
├── app.rs               # Application state (scan results, nav stack, selection, treemap nodes)
├── scanner/             # Disk scanning engine
│   ├── mod.rs           # Scan coordinator
│   ├── types.rs         # DirNode / Entry / FileEntry data structures
│   ├── walker.rs        # Win32 FindFirstFileExW async traversal
│   └── error.rs         # Error types
├── treemap/             # Treemap layout + rendering
│   ├── types.rs         # TreemapNode struct (9 fields)
│   ├── layout.rs        # Squarified layout algorithm
│   ├── color.rs         # FileCategory enum + color mapping + categorization functions
│   └── renderer.rs      # egui Painter rendering (rectangles, labels, selection highlight, hover tooltip)
├── ui/                  # UI components
│   ├── breadcrumb.rs    # Breadcrumb navigation
│   └── info_panel.rs    # Right-side detail panel + color legend
├── snapshot/            # Snapshot storage (SQLite) + diff comparison
└── platform/            # Windows platform layer
    └── drives.rs        # Logical drive enumeration, disk metadata
```

## Development Progress

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1: Scan Engine | ✅ Complete | Async directory traversal, permission handling, Others aggregation |
| Phase 2: Treemap Visualization | ✅ Complete | Layout algorithm, color mapping, renderer, drill-down, breadcrumb, detail panel |
| Phase 3: Snapshots & Comparison | ⏳ Pending | SQLite storage, diff detection, highlight display |

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Rust + egui instead of C++/Qt | Single-binary deployment, memory safety, equally direct Win32 calls, no GC pauses |
| Immediate-mode GUI (egui) | Treemap requires fully custom rendering; immediate mode is more flexible than retained mode |
| SQLite for snapshot storage | Single-file, zero-configuration, ideal for local data storage |
| Squarified Treemap algorithm | Produces rectangles with near 1:1 aspect ratios for optimal visual clarity |
| Async scanning + incremental push | Prevents UI freeze during large directory scans |

## Build & Run

```bash
# Development
cargo run

# Release build
cargo build --release

# Run tests
cargo test
```

## Out of Scope

- Disk management (delete / move files) — visualization only in the current version
- Real-time refresh — on-demand scanning, no filesystem monitoring
- Network / remote disks — local logical drives only
- File type statistics — future version consideration
- Report export — future version consideration
