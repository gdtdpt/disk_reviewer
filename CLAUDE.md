# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

disk_reviewer — Windows 磁盘空间可视化审查工具。以矩形树图（Treemap）展示磁盘空间占用，支持逐层下钻和历史快照对比。

技术栈：**Rust + egui (eframe)**，纯本地桌面应用，单二进制发布。

详细项目信息见 `docs/PROJECT.md`。

## 构建与运行

```bash
# 开发运行
cargo run

# 发布构建
cargo build --release

# 运行测试
cargo test
```

## 代码架构

```
src/
├── main.rs              # 入口，eframe 应用启动
├── app.rs               # 应用状态管理
├── scanner/             # 磁盘扫描引擎（Win32 FindFirstFileExW，异步线程池）
├── treemap/             # Treemap 布局算法 + egui 渲染
├── snapshot/            # 快照存储（SQLite）+ 差异对比
├── ui/                  # UI 组件（驱动器选择、面包屑、详情面板、快照对话框）
└── platform/            # Windows 平台层（逻辑盘枚举、磁盘元信息）
```

## ⚡ TDD 硬性约束（不可跳过）

**所有开发任务必须遵循 TDD 流程。** 详细规则见 `docs/TDD_ENFORCEMENT.md`。

### 核心规则

1. **先写测试，再写实现**：每个任务必须从 RED 阶段开始——先写失败的测试，提交后再实现
2. **无测试 = 不提交**：没有对应测试的代码不允许合并
3. **提交消息必须遵循 TDD 约定**：
   - RED: `test({phase}-{plan}): add failing test for [feature]`
   - GREEN: `feat({phase}-{plan}): implement [feature]`
   - REFACTOR: `refactor({phase}-{plan}): clean up [feature]`
4. **门控检查**：每个 TDD 任务完成后验证 RED → GREEN 提交序列存在

### Rust 测试约定

```bash
# 运行所有测试
cargo test
# 运行特定模块
cargo test scanner::
# 单测试 + 输出
cargo test test_name -- --nocapture
```

- 单元测试：`#[cfg(test)] mod tests` 写在源文件内
- 集成测试：`tests/` 目录下，每模块一个文件
- 关键路径（扫描、布局、差异检测）覆盖率须 100%

### TDD 适用性判断

| 类型 | 是否 TDD | 示例 |
|------|----------|------|
| 算法/逻辑/数据转换 | **必须** | Treemap 布局、差异检测、大小格式化 |
| API 封装/平台调用 | **必须** | Win32 调用封装（可 mock） |
| UI 渲染/布局 | 跳过 | egui 绘制代码 |
| 纯数据模型定义 | 跳过 | struct/enum 无行为 |

## 关键约束

- **扫描必须异步**：目录遍历在后台线程执行，通过通道增量推送结果到 UI 线程
- **大目录处理**：超过阈值的小文件聚合为 "Others" 条目，避免内存爆炸
- **Treemap 渲染**：使用 squarified 算法，保证矩形长宽比接近 1:1
- **快照差异**：基于目录树结构的增删改检测，高亮新增/增长/缩小的目录
