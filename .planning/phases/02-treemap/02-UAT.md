---
status: fixes_applied
phase: 02-treemap
source: [02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md, 02-04-SUMMARY.md, 02-05-SUMMARY.md]
started: 2026-05-06T00:00:00Z
updated: 2026-05-06T00:00:00Z
---

## Current Test

number: 1
name: Treemap 矩形树图渲染
expected: |
  扫描完成后，主窗口显示矩形树图。每个矩形色块大小正比于该目录/文件的磁盘空间占用。
  不同文件类型显示不同颜色（文档、图片、视频、音频、压缩包、代码、可执行文件、系统、临时文件、其他）。
awaiting: user response

## Tests

### 1. Treemap 矩形树图渲染
expected: 扫描完成后，主窗口显示矩形树图。每个矩形色块大小正比于该目录/文件的磁盘空间占用。不同文件类型显示不同颜色。
result: issue
reported: "只看到水平排列的矩形，不是那种尽量正方形的样子。而且点击矩形时响应很慢"
severity: major

### 2. 色块标签显示
expected: 每个色块显示目录/文件名、大小、占比信息。面积足够大的色块显示文字标签（白色文字在左上角）。
result: issue
reported: "色块上没有任何信息"
severity: major

### 3. 文件类型颜色映射
expected: 至少 10 种文件类型各有独特的颜色：文档、图片、视频、音频、压缩包、代码、可执行文件、系统、临时文件、其他。目录显示其主导文件类型的颜色。
result: pass

### 4. 点击下钻进入子目录
expected: 点击目录对应的矩形色块，进入该子目录的 Treemap 视图，显示该子目录下的空间分布。
result: issue
reported: "无法点击下钻，单击只会改变侧边详情内容，双击也无法下钻。建议下钻改成双击，避免与单击选中冲突"
severity: major

### 5. 面包屑导航
expected: 窗口顶部显示面包屑导航，展示当前路径（如 C:\ > Users > ...）。每个路径段可点击。
result: blocked
blocked_by: prior-phase
reason: "无法下钻导致 nav_stack 永远为空，面包屑只有根节点，无法验证多段路径和点击跳转"

### 6. 面包屑点击返回上层
expected: 点击面包屑中任意路径段，直接跳转到对应层级的 Treemap 视图。
result: blocked
blocked_by: prior-phase
reason: "同测试 5，依赖下钻功能正常工作后才能验证面包屑跳转"

### 7. 选中项高亮
expected: 点击某个矩形色块后，该色块显示白色边框高亮（2px 描边）。
result: pass

### 8. 详情面板显示
expected: 右侧详情面板（70/30 布局）显示选中项的名称、格式化大小（KB/MB/GB）、占比、类型（目录/文件）。如果是目录，还显示文件数和子目录数。
result: pass

### 9. 无选中时目录摘要
expected: 未选中任何色块时，右侧面板显示当前目录的摘要信息（名称、总大小、文件数、子目录数）。
result: pass

### 10. 颜色图例
expected: 详情面板底部显示 10 种文件类型的颜色图例，每种颜色带 16x16 色块和中文标签。
result: pass

### 11. 悬停提示
expected: 鼠标悬停在矩形色块上时，显示工具提示，包含名称、格式化大小、占比。
result: pass

### 12. 非目录点击
expected: 点击非目录（普通文件）的矩形色块，不会进入子目录，但会在详情面板显示该文件信息。
result: blocked
blocked_by: prior-phase
reason: "色块渲染位置错乱，无法可靠点击具体文件/目录色块来验证行为"

## Summary

total: 12
passed: 6
issues: 3
pending: 0
skipped: 0
blocked: 3

## Gaps

- truth: "Treemap 矩形树图应使用 squarified 算法，矩形长宽比接近 1:1"
  status: failed
  reason: "用户报告：只看到水平排列的矩形，不是那种尽量正方形的样子。且点击响应很慢"
  severity: major
  test: 1
  root_cause: "app.rs 第 254-256 行：canvas_rect 的 min 硬编码为 pos2(0.0, 0.0)，但 egui painter 的原点在当前 UI 光标位置而非 (0,0)。layout_treemap 生成的 rect 坐标基于 (0,0) 原点，导致所有矩形绘制到错误位置（屏幕外或挤在一起）。同时第 259 行每帧调用 rebuild_treemap，大目录每帧重新计算布局导致严重卡顿。"
  artifacts:
    - path: "src/app.rs"
      issue: "canvas_rect min 应为 ui.cursor().min 而非 pos2(0.0, 0.0)；rebuild_treemap 不应每帧调用"
  missing:
    - "canvas_rect 原点应使用 painter/UI 实际原点"
    - "rebuild_treemap 应在导航/下钻时才调用，而非每帧"
  debug_session: ""

- truth: "色块上应显示目录/文件名、大小、占比标签"
  status: failed
  reason: "用户报告：色块上没有任何信息"
  severity: major
  test: 2
  root_cause: "同问题 1——canvas 原点错误导致矩形绘制到屏幕外，标签自然也看不到。renderer.rs 的 paint_treemap 在 rect.area() >= 400 像素时才绘制标签，但矩形位置错误导致完全不可见。"
  artifacts:
    - path: "src/app.rs"
      issue: "canvas_rect 原点错误导致所有矩形位置偏移"
    - path: "src/treemap/renderer.rs"
      issue: "标签绘制依赖正确的 rect 位置"
  missing:
    - "修复 canvas 原点后标签应自动可见"
  debug_session: ""

- truth: "双击目录色块应下钻进入子目录视图"
  status: failed
  reason: "用户报告：单击只改变详情面板，无法下钻。建议改为双击下钻避免与单击选中冲突"
  severity: major
  test: 4
  root_cause: "app.rs 第 261-272 行：单击逻辑本身是正确的（Dir → drill_down，其他 → selected_index），但由于问题 1 导致矩形位置错误，用户实际点击的位置与矩形不匹配，所以点击被判定为非目录或未命中。另外用户建议改为双击下钻以避免与单击选中冲突，这是合理的 UX 改进。"
  artifacts:
    - path: "src/app.rs"
      issue: "单击下钻逻辑正确但依赖正确的 hit-testing；需要改为双击"
    - path: "src/treemap/renderer.rs"
      issue: "paint_treemap 使用 Sense::click()，需改为 Sense::double_click() 或同时支持"
  missing:
    - "将下钻交互从单击改为双击（double-click）"
    - "修复 canvas 原点以恢复正确的 hit-testing"
  debug_session: ""

## Improvement Notes (non-blocking user feedback)

- **下钻交互改为双击**：用户建议目录下钻用双击而非单击，避免与单击选中冲突（测试 4）
- **颜色图例改为横向排列**：10 个颜色图例垂直排列太占空间，建议改为横向排列（测试 10）
- **点击空白区域取消选中**：选中色块后点击空白区域应取消选中，目前不支持（测试 9）
- **选中高亮样式优化**：白色边框高亮视觉效果不佳，需要美化（测试 7）
