---
status: testing
phase: 03-快照与对比
source: 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md, 03-04-SUMMARY.md
started: 2026-05-06T21:30:00Z
updated: 2026-05-06T21:30:00Z
---

# Phase 3: 快照与对比 UAT

## Current Test

number: 1
name: 保存快照
expected: |
  1. 运行 `cargo run --features snapshot`
  2. 点击任意驱动器"扫描"按钮，等待扫描完成
  3. 点击工具栏"📷 快照"按钮
  4. 快照管理对话框弹出，显示"暂无快照"
  5. 点击"新建"按钮
  6. 快照以默认名称"快照 YYYY-MM-DD HH:MM"创建成功
  7. 对话框列表显示新快照（名称、时间、大小、路径）
awaiting: user response

## Tests

### 1. 保存快照
expected: 扫描完成后点击工具栏"快照"→"新建"，以默认时间戳名称成功创建快照，对话框列表显示快照元数据（名称、时间、大小、路径、文件数）
result: [pending]

### 2. 加载快照到 Treemap
expected: 在快照对话框中选中一个快照，点击"加载"，Treemap 视图切换为快照数据，nav_stack 重置，面包屑显示根路径
result: [pending]

### 3. 重命名快照
expected: 选中快照后点击"重命名"，输入新名称后确认，列表中显示新名称
result: [pending]

### 4. 删除快照（含确认）
expected: 选中快照后点击"删除"，弹出确认对话框，确认后快照从列表中消失
result: [pending]

### 5. 快照对比视图
expected: 有扫描结果时，在快照对话框选中快照点击"对比"，弹出对比窗口（960x600），左侧显示当前扫描 treemap，右侧显示快照 treemap
result: [pending]

### 6. 差异高亮颜色
expected: 对比窗口右侧面板中，新增条目显示绿色叠加(+条目显示红色叠加(-)，增长条目显示橙色叠加(↑)，缩小条目显示蓝色叠加(↓)
result: [pending]

### 7. 差异工具提示
expected: 在对比窗口右侧面板悬停在有差异的色块上，tooltip 显示名称、大小、占比以及变化详情（如"+1.5 MB (之前: 500.0 KB)"）
result: [pending]

### 8. 对比视图独立下钻
expected: 对比窗口左右面板可独立双击目录块下钻，面包屑/导航互不影响，右侧面板在每一层都显示差异叠加
result: [pending]

### 9. 空快照名校验
expected: 在快照对话框名称输入框中输入纯空格后点击"新建"，应使用时间戳默认名称而非空白名称
result: [pending]

### 10. 无扫描结果时禁用新建
expected: 未进行扫描时（scan_result 为 None），"新建"按钮应禁用，显示灰色提示文字
result: [pending]

## Summary

total: 10
passed: 0
issues: 0
pending: 10
skipped: 0
blocked: 0

## Gaps

<!-- 发现问题后在此追加 YAML 格式的 gap 记录 -->
