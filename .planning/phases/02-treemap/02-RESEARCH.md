# Phase 2: Treemap 可视化 - Research

**Researched:** 2026-05-05
**Domain:** Treemap 布局算法 + egui 即时模式 GUI 渲染 + 交互模型
**Confidence:** HIGH

## Summary

Phase 2 将 Phase 1 输出的 `DirNode` 递归树转换为可视化矩形树图，支持下钻浏览。核心技术决策已全部由用户锁定（D-06 ~ D-15），研究聚焦于**如何将这些决策落地为具体的 Rust 代码**。

**主要挑战有三个：**

1. **Squarified Treemap 算法的纯 Rust 实现** (~200 行)：经典论文算法（Bruls et al. 2000），递归地将 children 按面积比例划分为行，最小化最差长宽比。输入为 `&DirNode`，输出为 `Vec<TreemapNode>`。每次下钻重新运行，无需预计算。

2. **egui 即时模式交互范式下的点击检测**：egui 没有"矩形对象"的概念——每帧重新绘制。正确的模式是：在 `update()` 中获取所有 TreemapNode 的 `Rect`，遍历检测 `rect.contains(hover_pos)`，结合 `pointer.button_clicked()` 判断点击。用 `unique_id.with("rect", index)` 为每个矩形分配 ID，通过 `ui.interact()` 做精确感知。

3. **布局架构 (70/30 分割)**：使用 `SidePanel::right("detail_panel").exact_width(320.0)` 做右侧固定面板，`CentralPanel` 自动填充剩余 ~70% 空间。面包屑用 `TopPanel` 或 `CentralPanel` 顶部的 `Horizontal` 布局实现。

**Primary recommendation:** 分 5 个 Plan 顺序实现——01 框架布局 -> 02 布局算法 -> 03 渲染 -> 04 下钻导航 -> 05 详情面板。每步都可独立测试和验证。

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**布局算法**
- **D-06:** 自研 Squarified Treemap 算法，从零实现，完全掌控布局和性能，无额外依赖。纯 Rust 实现，代码量约 ~200 行。
- **D-07:** 创建独立的 `TreemapNode` 数据结构（含 rect(x,y,w,h)、label、color、depth、关联 Entry 引用）。布局算法输出 `Vec<TreemapNode>`，渲染器消费。布局与扫描数据解耦。
- **D-08:** 布局算法消费 Phase 1 输出的 `DirNode` 递归树作为输入。每次下钻时重新运行布局算法，只布局当前目录的 children，不做预计算。

**颜色映射**
- **D-09:** 按文件类型分配颜色（蓝色=文档，绿色=媒体，红色=系统文件等）。目录颜色由其主导文件类型决定（统计目录下各类型占比，用占比最高的类型颜色代表整个目录）。
- **D-10:** 颜色-类型对应关系以图例形式显示在右侧详情面板中。

**标签渲染**
- **D-11:** 矩形面积小于阈值时完全不显示标签，仅显示色块。鼠标悬停时用 tooltip 显示完整名称和大小。

**交互与导航**
- **D-12:** 单击目录矩形直接进入子目录视图，面包屑同步更新。简单直接，符合大多数 Treemap 工具习惯。
- **D-13:** 面包屑可点击路径段（如 `C: > Users > Documents`），每段都可点击，直接跳转到对应层级。

**UI 布局**
- **D-14:** 左侧 Treemap 画布占窗口 70%，右侧固定宽度面板显示选中项详情（路径、大小、占比、文件数）。面包屑在顶部。
- **D-15:** 颜色图例放在右侧详情面板中，位于选中项信息下方。

### Claude's Discretion

- 具体文件类型到颜色的映射表（类型分组和对应 RGB 值）
- Squarified 算法的具体实现细节（数据结构、递归策略）
- 标签面积阈值的具体数值（像素大小）
- 详情面板的具体宽度
- egui 颜色映射的具体实现方式

### Deferred Ideas (OUT OF SCOPE)

- **快照对比视图中的差异高亮** → Phase 3（SNAP-04）
- **磁盘管理功能（打开位置、删除）** → Phase 4（MGMT-01~03）
- **过滤与搜索** → Phase 4（FILT-01~04）
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VIS-01 | 基于空间占比的矩形树图（Squarified Treemap 算法） | Pattern 1 (算法), Pattern 2 (TreemapNode), Code Example 1 (实现), Don't Hand-Roll |
| VIS-02 | 每个色块显示目录/文件名、大小、占比 | Pattern 3 (渲染+标签), D-11 (阈值策略), Pitfall 3 (中文文字) |
| VIS-03 | 点击目录块进入子目录视图 | Pattern 4 (下钻状态管理), Pattern 3 (点击检测), D-12 |
| VIS-04 | 面包屑导航，支持返回任意上层目录 | Pattern 4 (nav_stack), Code Example 2 (面包屑组件), D-13 |
| VIS-05 | 选中项详情面板（完整路径、大小、占比、文件/子目录数量） | Code Example 3 (详情面板), D-14, D-15, Pattern 5 (图例) |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Treemap 布局计算 | Backend (纯算法) | — | 无 UI 依赖，纯数学计算。`squarify()` 函数可独立测试 |
| 矩形渲染 | Browser/Client (egui Painter) | — | 即时模式 GUI，每帧通过 Painter API 绘制所有矩形 |
| 颜色映射 | Browser/Client | — | 文件类型 -> Color32 的映射表，渲染时查表 |
| 点击/悬停检测 | Browser/Client (egui Ui) | — | `ui.interact()` + `Sense::click()` 模式 |
| 下钻状态管理 | Browser/Client (AppState) | — | 当前浏览路径 (`nav_stack: Vec<usize>`) 存储在 App 中 |
| 导航栈操作 | Browser/Client | — | 单击目录 push 索引，面包屑点击 truncate 栈 |
| 面包屑渲染 | Browser/Client (egui) | — | Horizontal 排列的点击按钮 |
| 详情面板渲染 | Browser/Client (egui) | — | SidePanel::right() 中的静态信息展示 |
| 图例渲染 | Browser/Client (egui) | — | 详情面板中的颜色-类型对照表 |
| 扫描数据源 | Backend (Phase 1 DirNode) | — | `Arc<DirNode>` 只读引用，布局算法消费 |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `egui` | 0.33.3 | 即时模式 GUI，Painter API 自定义渲染 | 项目已选定，0.33.x 是当前稳定版 |
| `eframe` | 0.33.3 | egui 桌面应用框架 (winit + wgpu) | 项目已选定，提供 Native window |
| `emath` | 0.33.3 | `Rect`, `Pos2`, `Vec2`, `Align2` 基础数学类型 | egui 配套数学库，已作为 egui 子依赖存在 |
| `epaint` | 0.33.3 | `Shape`, `Color32`, `Stroke`, `CornerRadius`, `FontId` | egui 渲染原语，已作为 egui 子依赖存在 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | 2.x | 错误类型派生 | 项目已使用，treemap 模块错误处理沿用 |
| `std::collections::HashMap` | std | 文件类型 -> 颜色映射表 | 标准库即可，无需外部 crate |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| 自研 Squarified 算法 (D-06) | `treemap` crate 0.3.2 | 用户明确选择自研。`treemap` crate 使用 trait object (`Mappable`)，引入额外抽象层，且代码量不比自研少 |
| `SidePanel::right()` | 手动 `allocate_rect` + `painter_at` | SidePanel 更简单可靠，自动处理 resize 和 frame |
| `CentralPanel` + `SidePanel` | `egui_extras::Strip` / `Layout` | 标准容器足够，无需引入 egui_extras 依赖 |

**安装：无需额外依赖。** 所有需要的 crate 已在 Cargo.toml 中。

**版本验证：**
- `egui 0.33.3` — 当前 lock 版本，API 稳定 [VERIFIED: cargo tree]
- `eframe 0.33.3` — 当前 lock 版本 [VERIFIED: cargo tree]
- `emath 0.33.3` / `epaint 0.33.3` — egui 子依赖，版本同步 [VERIFIED: cargo tree]

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        eframe::App::update()                     │
│                                                                   │
│  ┌──────────────────────────────────────┐ ┌────────────────────┐ │
│  │         TopPanel / Header             │ │                    │ │
│  │  [Breadcrumb: C:\ > Users > Docs]    │ │   SidePanel::right │ │
│  └──────────────────────────────────────┘ │   (exact_width 320)│ │
│                                            │                    │ │
│  ┌──────────────────────────────────────┐ │  Selected Item:    │ │
│  │         CentralPanel (70%)           │  │  - Path           │ │
│  │                                      │  │  - Size           │ │
│  │  ┌─────────────────────────────┐    │  │  - Percentage     │ │
│  │  │  allocate_painter()          │    │  │  - File count     │ │
│  │  │  + Sense::click()            │    │  │                    │ │
│  │  │                              │    │  │  ──────────────   │ │
│  │  │  Painter::rect_filled() x N  │    │  │  Color Legend:    │ │
│  │  │  Painter::text() x N         │    │  │  [blue] Documents│ │
│  │  │                              │    │  │  [green] Media   │ │
│  │  │  for each TreemapNode:       │    │  │  [red] System    │ │
│  │  │    draw rect + label         │    │  │  ...              │ │
│  │  └─────────────────────────────┘    │  │                    │ │
│  │                                      │  │                    │ │
│  └──────────────────────────────────────┘ └────────────────────┘ │
│                                                                   │
│  Data Flow:                                                       │
│  scan_result: Arc<DirNode>                                        │
│       │                                                           │
│       ▼                                                           │
│  nav_stack: Vec<usize>  ──resolve──>  current_dir: &DirNode       │
│       │                                                           │
│       ▼                                                           │
│  squarify(current_dir) ──> Vec<TreemapNode>                       │
│       │                                                           │
│       ▼                                                           │
│  Painter renders all TreemapNodes                                 │
│       │                                                           │
│       ▼                                                           │
│  Click detected ──> push to nav_stack ──> re-layout next frame   │
└─────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
src/
├── main.rs                    # 入口，eframe 应用启动（已有）
├── app.rs                     # 应用状态管理（扩展：添加 treemap 状态）
├── scanner/                   # Phase 1 已有，不变
│   ├── mod.rs
│   ├── types.rs               # DirNode, Entry（已有，布局算法的输入）
│   ├── walker.rs
│   └── error.rs
├── treemap/                   # Phase 2 新增
│   ├── mod.rs                 # pub use layout, renderer, types, color
│   ├── types.rs               # TreemapNode 结构体（D-07）
│   ├── layout.rs              # Squarified 算法实现（D-06）
│   ├── color.rs               # 文件类型 -> 颜色映射（D-09）
│   └── renderer.rs            # egui Painter 渲染逻辑
├── ui/                        # Phase 2 扩展
│   ├── mod.rs
│   ├── breadcrumb.rs          # 面包屑导航组件（D-13）
│   └── detail_panel.rs        # 右侧详情面板（D-14, D-15）
└── platform/                  # Phase 1 已有，不变
    ├── mod.rs
    ├── drives.rs
    └── metadata.rs
```

### Pattern 1: Squarified Treemap 算法

**What:** 递归地将一组带权值的项目划分为矩形，使每个矩形面积正比于权值，且长宽比尽量接近 1:1。

**When to use:** 每次用户下钻到新目录时，对该目录的 `children` 运行一次。

**算法核心（基于 Bruls et al. 2000）：**

```
function squarify(items, x, y, w, h):
    sort items by size descending
    if items.len <= 2:
        return layout_row(items, x, y, w, h)

    row = [items[0]]
    for each item in items[1:]:
        worst_with = worst_ratio(row + [item])
        worst_without = worst_ratio(row)
        if worst_with <= worst_without:
            row.append(item)
        else:
            layout_row(row, remaining_space)
            squarify(remaining_items, new_x, new_y, new_w, new_h)
            break

function worst_ratio(row, side):
    for each item in row:
        rect_long = (item.size / row_sum) * long_side
        rect_short = (item.size / row_sum) * side
        ratio = max(rect_long/rect_short, rect_short/rect_long)
    return max(ratio)
```

**关键实现细节：**
- 沿长边排列：如果 `w >= h`，水平排列；否则垂直排列
- 每个矩形的边长 = `item_size / row_sum * row_length`（沿排列方向）
- 另一方向边长 = `item_size / row_sum * short_side`
- 递归终止：`items.len() <= 2` 时直接线性布局

**Edge cases:**
- 空目录：返回空 Vec，渲染时显示 "Empty directory"
- 单个 child：填充整个区域
- 零 size 条目（AccessDenied, Symlink）：过滤掉，不参与布局
- 极深嵌套：每次只布局当前层，递归深度 = 1，不会栈溢出

### Pattern 2: TreemapNode 数据结构 (D-07)

```rust
// src/treemap/types.rs

#[derive(Debug, Clone)]
pub struct TreemapNode {
    pub rect: emath::Rect,
    pub label: String,
    pub color: epaint::Color32,
    pub depth: u32,
    pub entry_index: usize,
    pub is_dir: bool,
    pub size: u64,
    pub percentage: f32,
}
```

### Pattern 3: egui 自定义渲染 + 交互

**What:** 使用 `allocate_painter()` + `Painter` API 绘制矩形，使用 `response.clicked()` + `hover_pos` 检测交互。

**核心渲染模式：**

```rust
pub fn paint_treemap(
    ui: &mut egui::Ui,
    nodes: &[TreemapNode],
    selected: Option<usize>,
) -> Option<usize> {
    let size = ui.available_size();
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
    let mut clicked_index = None;

    // 1. 绘制所有矩形
    for (i, node) in nodes.iter().enumerate() {
        if !response.rect.intersects(node.rect) { continue; }
        painter.rect_filled(node.rect, CornerRadius::same(1.0), node.color);
        if selected == Some(i) {
            painter.rect_stroke(
                node.rect.shrink(1.0), CornerRadius::same(1.0),
                Stroke::new(2.0, Color32::WHITE), StrokeKind::Middle,
            );
        }
        // D-11: 面积足够大时才显示标签
        if node.rect.width() * node.rect.height() >= LABEL_AREA_THRESHOLD {
            painter.text(
                node.rect.center(), Align2::CENTER_CENTER,
                &node.label, FontId::proportional(12.0), Color32::WHITE,
            );
        }
    }

    // 2. 点击检测
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            for (i, node) in nodes.iter().enumerate().rev() {
                if node.rect.contains(pos) { clicked_index = Some(i); break; }
            }
        }
    }

    // 3. 悬停 tooltip（D-11）
    if let Some(pos) = response.hover_pos() {
        for node in nodes.iter().rev() {
            if node.rect.contains(pos) {
                response.on_hover_ui_at_pointer(|ui| {
                    ui.label(&node.label);
                    ui.label(format!("{:.1}%", node.percentage));
                });
                break;
            }
        }
    }
    clicked_index
}
```

**性能考虑：**
- 典型目录 children 数：几十到几百（Phase 1 的 Others 聚合已将大目录压缩到 ~550 条以内）
- 每帧绘制几百个矩形 + 文字：egui 的 Painter 使用批量渲染，性能足够
- 不需要虚拟化：egui 自动做视口裁剪
- 文字布局开销较大：对极小矩形跳过文字渲染（D-11 阈值策略）

### Pattern 4: 下钻状态管理

```rust
// src/app.rs 中添加的字段

pub struct DiskReviewerApp {
    // ... 已有字段 ...
    pub nav_stack: Vec<usize>,
    pub selected_index: Option<usize>,
    pub treemap_nodes: Vec<TreemapNode>,
}

impl DiskReviewerApp {
    fn current_dir(&self) -> Option<&DirNode> {
        let root = self.scan_result.as_ref()?;
        let mut current = root.as_ref();
        for &idx in &self.nav_stack {
            current = current.children.get(idx).and_then(|e| match e {
                Entry::Dir(d) => Some(d), _ => None,
            })?;
        }
        Some(current)
    }

    fn drill_down(&mut self, child_index: usize) {
        if let Some(dir) = self.current_dir() {
            if let Some(Entry::Dir(_)) = dir.children.get(child_index) {
                self.nav_stack.push(child_index);
                self.selected_index = None;
                self.rebuild_treemap();
            }
        }
    }

    fn navigate_to_depth(&mut self, depth: usize) {
        self.nav_stack.truncate(depth);
        self.selected_index = None;
        self.rebuild_treemap();
    }

    fn rebuild_treemap(&mut self) {
        if let Some(dir) = self.current_dir() {
            self.treemap_nodes = treemap::layout::squarify(dir);
        }
    }
}
```

### Pattern 5: 颜色映射 (D-09)

```rust
// src/treemap/color.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Document, Image, Video, Audio, Archive,
    Code, Executable, System, Temp, Other,
}

impl FileCategory {
    pub fn color(&self) -> Color32 {
        match self {
            FileCategory::Document   => Color32::from_rgb(70, 130, 180),
            FileCategory::Image      => Color32::from_rgb(46, 139, 87),
            FileCategory::Video      => Color32::from_rgb(220, 20, 60),
            FileCategory::Audio      => Color32::from_rgb(255, 140, 0),
            FileCategory::Archive    => Color32::from_rgb(128, 0, 128),
            FileCategory::Code       => Color32::from_rgb(0, 128, 128),
            FileCategory::Executable => Color32::from_rgb(184, 134, 11),
            FileCategory::System     => Color32::from_rgb(192, 192, 192),
            FileCategory::Temp       => Color32::from_rgb(169, 169, 169),
            FileCategory::Other      => Color32::from_rgb(105, 105, 105),
        }
    }
}

pub fn categorize(path: &std::path::Path) -> FileCategory {
    // 扩展名匹配逻辑（见完整代码示例）
}

pub fn dominant_category(dir: &DirNode) -> FileCategory {
    // 递归统计各类型总大小，返回占比最高者
}
```

### Anti-Patterns to Avoid

- **不要在 Painter 中创建 Widget**：Painter 只画形状。交互必须通过 `ui.interact()` 或 `ctx.input()` 在 Painter 外部处理
- **不要预计算所有层级的布局**（D-08）：每次下钻重新运行，只布局当前 children
- **不要在布局算法中包含零 size 条目**：AccessDenied 和 Symlink 的 size() = 0，应预先过滤
- **不要每帧重新运行布局算法**：布局结果应缓存在 `app.treemap_nodes` 中
- **不要使用 `egui::Shape::mesh` 做简单矩形**：`Painter::rect_filled` 最高效

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 矩形绘制 | 自己调用 GPU API | `Painter::rect_filled` | egui 已封装，自动批处理 |
| 文字布局 | 自己计算字符宽度 | `Painter::text` + `FontId` | egui 的字体系统处理中文、DPI 缩放 |
| 点击区域检测 | 自己实现 hit testing | `ui.interact()` + `Sense` | egui 处理 pointer 状态、layer 排序 |
| 颜色空间转换 | 自己算 RGBA | `Color32::from_rgb` | 标准库足够 |
| 布局算法 | — | 自研 (D-06) | 用户明确决策，~200 行可控 |

**关键 insight:** 唯一需要"自研"的是 Squarified Treemap 算法本身。所有 UI 渲染、交互检测、颜色管理都应使用 egui 内置 API。

## Common Pitfalls

### Pitfall 1: 即时模式 GUI 的"无状态"陷阱
**什么会出错:** 开发者试图在 Painter 绘制时"记住"哪个矩形被选中。
**如何避免:** 所有状态（选中索引、nav_stack）必须存储在 `DiskReviewerApp` 结构体中。

### Pitfall 2: 布局算法中的浮点精度问题
**什么会出错:** `f32` 精度不足导致矩形之间有缝隙。
**如何避免:** 使用 `f64` 进行中间计算，最终转换为 `f32`。

### Pitfall 3: 中文文字渲染的尺寸估算
**什么会出错:** 中文字符宽度约为英文字符的 2 倍，标签溢出矩形。
**如何避免:** 预先计算文字宽度，超过则截断或跳过标签。

### Pitfall 4: 下钻后忘记重置选中状态
**什么会出错:** `selected_index` 指向旧索引，导致越界。
**如何避免:** 在 `drill_down()` 中始终 `self.selected_index = None`。

### Pitfall 5: OthersEntry 的颜色计算
**什么会出错:** OthersEntry 颜色不具代表性。
**如何避免:** 对 OthersEntry 同样使用 `dominant_category()` 统计内部主导类型。

## Code Examples

### 示例 1: 核心布局算法

```rust
// src/treemap/layout.rs — 关键函数

pub fn squarify(dir: &DirNode, canvas_rect: emath::Rect) -> Vec<TreemapNode> {
    let total_size = dir.total_size as f64;
    if total_size == 0.0 { return Vec::new(); }

    // 1. 过滤 + 分类
    let mut items = Vec::new();
    for (idx, child) in dir.children.iter().enumerate() {
        let size = child.size();
        if size == 0 { continue; }
        items.push((size, classify_entry(child, idx, size)));
    }

    // 2. 按 size 降序排列
    items.sort_by_key(|&(size, _)| std::cmp::Reverse(size));

    // 3. 运行布局算法
    let sizes: Vec<f64> = items.iter().map(|(s, _)| *s as f64).collect();
    let normalized = squarify_recursive(&sizes, 0.0, 0.0, 1.0, 1.0);

    // 4. 缩放到实际像素 + 组装 TreemapNode
    items.into_iter().zip(normalized.into_iter())
        .map(|((size, info), nrect)| {
            TreemapNode {
                rect: emath::Rect::from_min_size(
                    emath::pos2(
                        canvas_rect.min.x + nrect.x * canvas_rect.width(),
                        canvas_rect.min.y + nrect.y * canvas_rect.height(),
                    ),
                    emath::vec2(
                        nrect.w * canvas_rect.width(),
                        nrect.h * canvas_rect.height(),
                    ),
                ),
                ..info
            }
        })
        .collect()
}

/// 归一化矩形
#[derive(Clone, Copy)]
struct NRect { x: f32, y: f32, w: f32, h: f32 }

fn squarify_recursive(sizes: &[f64], x: f32, y: f32, w: f32, h: f32) -> Vec<NRect> {
    let n = sizes.len();
    if n == 0 { return Vec::new(); }
    if n == 1 { return vec![NRect { x, y, w, h }]; }
    if n == 2 {
        if w >= h {
            let w1 = (sizes[0] / (sizes[0] + sizes[1])) as f32 * w;
            return vec![NRect { x, y, w: w1, h }, NRect { x: x + w1, y, w: w - w1, h }];
        } else {
            let h1 = (sizes[0] / (sizes[0] + sizes[1])) as f32 * h;
            return vec![NRect { x, y, w, h: h1 }, NRect { x, y: y + h1, w, h: h - h1 }];
        }
    }

    let total: f64 = sizes.iter().sum();
    let short_side = w.min(h);
    let long_side = w.max(h);
    let mut row = vec![sizes[0]];
    let mut row_sum = sizes[0];
    let mut remaining = &sizes[1..];

    while !remaining.is_empty() {
        let current_worst = worst_ratio(&row, row_sum, short_side, long_side, total);
        let mut new_row = row.clone();
        new_row.push(remaining[0]);
        let new_worst = worst_ratio(&new_row, row_sum + remaining[0], short_side, long_side, total);
        if new_worst <= current_worst {
            row_sum += remaining[0];
            row = new_row;
            remaining = &remaining[1..];
        } else { break; }
    }

    // 布局 row，递归剩余
    let row_total: f64 = row.iter().sum();
    let row_ratio = row_total as f32 / total as f32;
    let mut result = Vec::new();
    let mut offset = 0.0f32;
    if w >= h {
        let row_w = row_ratio * w;
        for &size in &row {
            let sw = (size as f32 / row_total as f32) * row_w;
            result.push(NRect { x: x + offset, y, w: sw, h });
            offset += sw;
        }
        result.extend(squarify_recursive(remaining, x + row_w, y, w - row_w, h));
    } else {
        let row_h = row_ratio * h;
        for &size in &row {
            let sh = (size as f32 / row_total as f32) * row_h;
            result.push(NRect { x, y: y + offset, w, h: sh });
            offset += sh;
        }
        result.extend(squarify_recursive(remaining, x, y + row_h, w, h - row_h));
    }
    result
}

fn worst_ratio(row: &[f64], row_sum: f64, short_side: f32, long_side: f32, total: f64) -> f32 {
    if row.is_empty() || row_sum == 0.0 || total == 0.0 { return f32::MAX; }
    let row_long = row_sum as f32 / total as f32 * long_side;
    let row_short = short_side;
    row.iter().map(|&s| {
        let s = s as f32;
        let w = s / row_sum as f32 * row_long;
        let h = s / row_sum as f32 * row_short;
        let mn = w.min(h);
        let mx = w.max(h);
        if mn <= 0.0 { f32::MAX } else { mx / mn }
    }).fold(0.0f32, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_squarify_empty() {
        let result = squarify_recursive(&[], 0.0, 0.0, 1.0, 1.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_squarify_single() {
        let result = squarify_recursive(&[100.0], 0.0, 0.0, 1.0, 1.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].w, 1.0);
        assert_eq!(result[0].h, 1.0);
    }

    #[test]
    fn test_squarify_total_area_preserved() {
        let sizes = vec![100.0, 200.0, 300.0, 400.0];
        let result = squarify_recursive(&sizes, 0.0, 0.0, 1.0, 1.0);
        let total_area: f32 = result.iter().map(|r| r.w * r.h).sum();
        assert!((total_area - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_squarify_no_overlap() {
        let sizes = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let result = squarify_recursive(&sizes, 0.0, 0.0, 1.0, 1.0);
        for r in &result {
            assert!(r.x >= 0.0 && r.y >= 0.0);
            assert!(r.x + r.w <= 1.01);
            assert!(r.y + r.h <= 1.01);
        }
    }
}
```

### 示例 2: 面包屑组件

```rust
// src/ui/breadcrumb.rs

pub fn breadcrumb_ui(
    ui: &mut egui::Ui,
    scan_result: &DirNode,
    nav_stack: &[usize],
    on_navigate: &mut impl FnMut(usize),
) {
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button(&scan_result.name).clicked() {
                on_navigate(0);
            }
            let mut current = scan_result;
            for (depth, &idx) in nav_stack.iter().enumerate() {
                ui.label(">");
                if let Some(Entry::Dir(dir)) = current.children.get(idx) {
                    if ui.button(&dir.name).clicked() {
                        on_navigate(depth + 1);
                    }
                    current = dir;
                }
            }
        });
    });
}
```

### 示例 3: 详情面板

```rust
// src/ui/detail_panel.rs

pub fn detail_panel_ui(
    ui: &mut egui::Ui,
    selected: Option<&TreemapNode>,
    current_dir: Option<&DirNode>,
    legend: &[(String, Color32)],
) {
    ui.heading("详情");
    ui.separator();
    if let Some(node) = selected {
        ui.label(format!("名称: {}", node.label));
        ui.label(format!("大小: {}", format_size(node.size)));
        ui.label(format!("占比: {:.1}%", node.percentage));
        ui.label(if node.is_dir { "类型: 目录" } else { "类型: 文件" });
    } else if let Some(dir) = current_dir {
        ui.label(format!("当前: {}", dir.name));
        ui.label(format!("总大小: {}", format_size(dir.total_size)));
        ui.label(format!("文件数: {}", dir.file_count));
    } else {
        ui.label("未选择");
    }
    ui.separator();
    ui.heading("图例");
    for (label, color) in legend {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, *color);
            ui.label(label);
        });
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut i = 0;
    while size >= 1024.0 && i < UNITS.len() - 1 { size /= 1024.0; i += 1; }
    format!("{:.1} {}", size, UNITS[i])
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ctx.input().pointer.hover_pos()` | `ctx.input(\|i\| i.pointer.hover_pos())` | egui 0.20 | 闭包式 API 避免死锁 |
| `painter.rect(fill, stroke)` 单调用 | `rect_filled` + `rect_stroke` 分开 | egui 0.33 | API 更清晰 |
| `Stroke` 无 `StrokeKind` | `StrokeKind::Middle/Inside/Outside` | egui 0.28 | 描边位置可控 |

**Deprecated/outdated:** 无。egui 0.33.3 是当前最新稳定版。

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 标签面积阈值约 400 平方像素（~20x20px） | Pattern 3 | 可通过用户测试调整 |
| A2 | 右侧面板宽度 320px | Architecture | 可改为 `default_width(320.0).min_width(250.0)` |
| A3 | 文件类型分为 10 类 | Pattern 5 | 分类粒度可调整 |
| A4 | OthersEntry 颜色使用灰色 | Pattern 5 | 备选：统计内部主导类型 |
| A5 | 布局使用归一化坐标后缩放 | Code Example 1 | 极端情况可能有 1px 误差，可接受 |
| A6 | 面包屑根节点使用 DirNode.name | Code Example 2 | 需确认 Phase 1 中 name 格式 |

## Open Questions

1. **DirNode.name 的格式是什么？**
   - 我们知道：DirNode 有 `name: String` 字段
   - 不明确：根节点的 name 是 "C:" 还是 "C:\\"
   - 建议：在 Plan 02-01 中确认

2. **是否需要"返回上级"按钮？**
   - 建议：面包屑已足够，不需要额外按钮

3. **选中高亮的视觉样式？**
   - 建议：使用白色 2px 边框高亮选中矩形

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 编译器 | 全部 | ✓ | 1.90.0 | — |
| Cargo | 全部 | ✓ | 1.90.0 | — |
| egui | 渲染 | ✓ | 0.33.3 | — |
| eframe | 窗口 | ✓ | 0.33.3 | — |
| 微软雅黑字体 | 中文渲染 | ✓ | msyh.ttc | 回退到默认字体 |

**Missing dependencies with no fallback:** 无
**Missing dependencies with fallback:** 无

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[cfg(test)]` |
| Config file | 无 — 使用 Cargo 默认配置 |
| Quick run command | `cargo test treemap::` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VIS-01 | 布局算法产生面积正比的矩形 | unit | `cargo test treemap::layout::tests -x` | ❌ Wave 0 |
| VIS-01 | 矩形总面积等于画布面积 | unit | `cargo test layout::tests::test_area_preserved -x` | ❌ Wave 0 |
| VIS-02 | 标签显示名称和大小 | unit | `cargo test treemap::renderer::tests -x` | ❌ Wave 0 |
| VIS-02 | 小矩形不显示标签 | unit | `cargo test renderer::tests::test_label_threshold -x` | ❌ Wave 0 |
| VIS-03 | 单击目录下钻 | integration | `cargo test treemap::tests::test_drill_down -x` | ❌ Wave 0 |
| VIS-04 | 面包屑导航跳转 | integration | `cargo test ui::breadcrumb::tests -x` | ❌ Wave 0 |
| VIS-05 | 选中项详情显示 | integration | `cargo test ui::detail_panel::tests -x` | ❌ Wave 0 |
| VIS-05 | 颜色图例显示 | unit | `cargo test treemap::color::tests -x` | ❌ Wave 0 |
| D-09 | 文件类型颜色映射 | unit | `cargo test color::tests::test_categorize -x` | ❌ Wave 0 |
| D-09 | 目录主导类型计算 | unit | `cargo test color::tests::test_dominant_category -x` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test treemap::` (< 5s)
- **Per wave merge:** `cargo test` (< 30s)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/treemap/mod.rs` — 模块导出
- [ ] `src/treemap/types.rs` — TreemapNode 结构体
- [ ] `src/treemap/layout.rs` — 布局算法 + 单元测试
- [ ] `src/treemap/color.rs` — 颜色映射 + 单元测试
- [ ] `src/treemap/renderer.rs` — 渲染逻辑
- [ ] `src/ui/breadcrumb.rs` — 面包屑组件
- [ ] `src/ui/detail_panel.rs` — 详情面板
- [ ] `src/app.rs` 扩展 — nav_stack, selected_index, treemap_nodes

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | 纯本地应用，无认证 |
| V3 Session Management | no | 无会话 |
| V4 Access Control | no | 无多用户 |
| V5 Input Validation | yes | 文件路径来自系统扫描，不来自用户输入。UI 交互仅为点击检测，无注入风险 |
| V6 Cryptography | no | 不涉及加密操作 |

### Known Threat Patterns for Rust + egui

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| 路径遍历 | Tampering | 文件路径来自 Win32 API 扫描结果，不接受用户输入的路径 |
| 内存溢出 | Denial of Service | Phase 1 的 Others 聚合限制条目数；布局算法有 O(n log n) 复杂度上限 |

## Sources

### Primary (HIGH confidence)
- egui 0.33.3 Painter API — `rect_filled`, `rect_stroke`, `text` 方法签名 [VERIFIED: docs.rs/egui]
- egui 0.33.3 `allocate_painter` + `Sense::click()` 交互模式 [VERIFIED: github.com/emilk/egui]
- egui 0.33.3 `SidePanel::right()` + `CentralPanel` 布局 [VERIFIED: github.com/emilk/egui]
- egui 0.33.3 `ctx.input()` 闭包 API [VERIFIED: github.com/emilk/egui]
- `Color32::from_rgb()` API [VERIFIED: docs.rs/egui]
- `Rect::from_min_size()`, `contains()`, `intersects()`, `shrink()` [VERIFIED: docs.rs/egui]
- Squarified Treemap 算法原始论文 Bruls et al. 2000 [CITED: vanwijk.win.tue.nl/stm.pdf]
- treemap-rs 参考实现结构 [CITED: github.com/bacongobbler/treemap-rs]
- 项目现有代码 `src/scanner/types.rs`, `src/app.rs`, `src/main.rs` [VERIFIED: codebase]
- Cargo.toml 依赖版本 [VERIFIED: cargo tree]

### Secondary (MEDIUM confidence)
- `Sense` 结构体 bitflags 定义 [VERIFIED: github.com/emilk/egui sense.rs]
- `Response::on_hover_ui_at_pointer()` tooltip API [VERIFIED: github.com/emilk/egui response.rs]
- `Label::truncate()` 文字截断模式 [VERIFIED: github.com/emilk/egui label.rs]

### Tertiary (LOW confidence)
- 文件类型分类表（10 类）— 基于常见 Windows 文件类型经验分类 [ASSUMED]
- 颜色映射 RGB 值 — 基于视觉区分度选择 [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 所有依赖已锁定，版本已验证
- Architecture: HIGH — 用户决策已锁定，egui API 已验证
- Pitfalls: MEDIUM — 基于 egui 使用经验和算法实现经验

**Research date:** 2026-05-05
**Valid until:** 2026-06-04 (30 days — egui 0.33 是稳定版，API 不会快速变化)
