# TDD 强制执行方案

## 概述

在 disk_reviewer 项目的整个开发周期中，TDD（测试驱动开发）不是可选项，而是硬性约束。本文档说明如何通过 GSD 工作流 + 项目配置 + CLAUDE.md 指令三重机制确保 TDD 被严格执行。

## 执行链路

TDD 约束通过三个层级生效：

```
┌─────────────────────────────────────────────────────┐
│  第1层：GSD 全局配置 (workflow.tdd_mode = true)       │
│  → plan-phase: 自动将合格任务标记为 type: tdd         │
│  → execute-phase: 强制执行 RED/GREEN/REFACTOR 门控    │
│  → 阶段结束后: TDD REVIEW 检查点                      │
├─────────────────────────────────────────────────────┤
│  第2层：项目配置 (.planning/config.json)               │
│  → 项目级 tdd_mode 覆盖，确保新项目也生效              │
├─────────────────────────────────────────────────────┤
│  第3层：CLAUDE.md 指令                                │
│  → 每个任务开始前必须先写测试                           │
│  → 无测试的 commit 不被接受                            │
│  → 提交消息必须遵循 TDD 约定                           │
└─────────────────────────────────────────────────────┘
```

## TDD 执行规则

### 何时必须使用 TDD

| 场景 | 是否 TDD | 理由 |
|------|----------|------|
| 扫描引擎（目录遍历、文件信息获取） | ✅ | 有明确的输入/输出契约 |
| Treemap 布局算法 | ✅ | 纯算法，可精确验证 |
| 快照差异检测 | ✅ | 树结构对比，行为确定 |
| 快照序列化/反序列化 | ✅ | 数据转换，输入输出明确 |
| 磁盘大小格式化（bytes → MB/GB） | ✅ | 纯函数 |
| UI 布局/渲染代码 | ❌ | 视觉验证，不适合单元测试 |
| 第三方 API 调用封装 | ✅ | 可 mock 测试 |
| 简单的数据模型定义 | ❌ | 无行为逻辑 |

### 提交消息规范

```
# RED 阶段 — 写失败测试
test(01-01): add failing test for directory walker

# GREEN 阶段 — 实现使测试通过
feat(01-01): implement directory walker with FindFirstFileExW

# REFACTOR 阶段 — 清理（可选，仅在有变更时）
refactor(01-01): extract file filter predicate
```

### 门控检查点

每个 TDD 任务完成后，执行器会验证：

1. **RED 门控**：是否存在 `test(...)` 提交？测试在实现前是否确实失败？
2. **GREEN 门控**：是否存在 `feat(...)` 提交？测试现在是否通过？
3. **REFACTOR 门控**（可选）：如果存在 `refactor(...)` 提交，测试是否仍然通过？

缺失 RED 或 GREEN 门控 = TDD 违规，记录在 SUMMARY.md 中。

## Rust 项目 TDD 约定

### 测试文件位置

```
src/
├── scanner/
│   ├── mod.rs
│   ├── walker.rs          # 实现
│   └── walker_tests.rs    # 集成测试（Rust 惯例：同目录 tests/ 子目录或 #[cfg(test)] 内联）
```

对于 Rust 项目，遵循标准惯例：
- **单元测试**：使用 `#[cfg(test)] mod tests { ... }` 写在源文件内部
- **集成测试**：放在 `tests/` 目录下，每个模块一个文件

### 测试命令

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test scanner::

# 运行单个测试
cargo test scanner::walker::tests::test_walk_empty_directory

# 显示测试输出（包括 println!）
cargo test -- --nocapture
```

### 测试覆盖要求

- 每个 TDD 任务的测试覆盖率 ≥ 90%（语句覆盖）
- 关键路径（扫描、布局、差异检测）必须 100% 覆盖
- 使用 `cargo tarpaulin` 或 `cargo llvm-cov` 生成覆盖率报告
