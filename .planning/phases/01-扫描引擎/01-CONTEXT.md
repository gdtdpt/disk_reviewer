# Phase 1: 扫描引擎 - Context

**Gathered:** 2026-05-05
**Status:** Ready for planning

## Phase Boundary

构建磁盘扫描引擎：异步遍历 Windows 目录树，输出带空间信息的目录结构。这是整个应用的数据基础——后续的 Treemap 渲染和快照对比都依赖扫描结果。

**本阶段范围**：项目脚手架、逻辑盘枚举、异步目录遍历、扫描结果数据结构、错误处理策略。

## Implementation Decisions

### 并发策略
- **D-01:** 使用 `rayon` 线程池 + 工作窃取（work-stealing）实现并行目录扫描。每个目录作为独立任务提交，rayon 自动负载均衡。相比手动管理线程池，实现更简单，Rust 生态成熟。

### 路径长度
- **D-02:** 所有路径操作启用 `\\?\` 前缀，支持最长 ~32,768 字符的扩展路径。
  - 原因：深层嵌套目录（如 `node_modules`）很容易超过 260 字符的 MAX_PATH 限制
  - 核心 API（`FindFirstFileExW`、`CreateFileW`、`GetDiskFreeSpaceExW`）均支持 `\\?\` 前缀
  - 注意：`GetFileAttributes` 不支持 `\\?\`，项目中不使用它（用 `FindFirstFileExW` 的 `WIN32_FIND_DATA` 替代）
  - 路径拼接时自行处理 `\` 分隔符，不使用 `/`

### 错误处理 — 符号链接 / Junction / 挂载点
- **D-03:** 不跟随符号链接、junction 和挂载点。
  - 原因：应用本质是查看当前磁盘的实际占用，符号链接本身只占用极小空间（目录条目），跟随会导致重复计算和循环引用风险
  - 在扫描结果中标记该条目为"符号链接"类型，UI 上以图标或标签区分

### 错误处理 — 无权限目录
- **D-04:** 记录并显示，不弹窗打断扫描。
  - 遇到 `ERROR_ACCESS_DENIED` 时，创建一个标记为"无权限"的目录条目，大小为 0
  - 扫描结束后汇总显示"X 个目录因权限不足被跳过"
  - 扫描不中断，继续处理同级其他目录

### 错误处理 — 扫描过程中的文件变更
- **D-05:** 接受快照不完美，不做重试。
  - 扫描时文件被删除 → `FindNextFile` 返回错误，跳过该文件
  - 扫描时新文件创建 → 可能遗漏，不影响整体结果
  - 扫描结果代表"扫描开始时刻"的快照，不保证完全精确

### Claude's Discretion
- 扫描结果数据结构设计（树形结构 + HashMap 索引）
- Win32 API 封装策略（直接使用 `windows-rs` crate，封装统一 `ScanError` 类型）
- 小文件聚合为 "Others" 的具体阈值
- 增量推送到 UI 的批量大小和频率
- 线程池的具体配置（线程数、任务粒度）

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 项目规范
- `.planning/PROJECT.md` — 项目目标、核心约束、技术栈决策、TDD 要求
- `.planning/REQUIREMENTS.md` — SCAN-01 ~ SCAN-04 需求定义
- `.planning/config.json` — `workflow.tdd_mode: true`，TDD 强制执行
- `docs/TDD_ENFORCEMENT.md` — TDD 执行规范、适用性矩阵、提交约定
- `docs/PROJECT.md` — 完整技术方案文档

### Windows API 参考
- `FindFirstFileExW` — 目录遍历核心 API，支持 `FIND_FIRST_EX_LARGE_FETCH` 标志和 `\\?\` 前缀
- `GetLogicalDrives` / `GetLogicalDriveStringsW` — 逻辑盘枚举
- `GetDiskFreeSpaceExW` — 获取磁盘空间信息
- `DeviceIoControl` + `FSCTL_GET_NTFS_VOLUME_DATA` — NTFS 卷信息（Phase 4 预留）

## Existing Code Insights

### Reusable Assets
- 无（全新项目）

### Established Patterns
- 无（全新项目，Phase 1 建立基础模式）

### Integration Points
- 扫描引擎输出（目录树数据结构）→ Phase 2 Treemap 渲染的输入
- 扫描结果序列化格式 → Phase 3 快照存储的数据来源

## Specific Ideas

- `FindFirstFileExW` 使用 `FIND_FIRST_EX_LARGE_FETCH` 标志减少内核态切换，提升大目录扫描性能
- 使用 `WIN32_FIND_DATA` 中的 `dwFileAttributes` 判断文件属性（目录、只读、隐藏、系统、重解析点/符号链接）
- 文件大小从 `WIN32_FIND_DATA` 的 `nFileSizeHigh` + `nFileSizeLow` 组合为 u64

## Deferred Ideas

- **磁盘元信息展示**（文件系统类型、簇大小、SMART）→ Phase 4
- **磁盘管理功能**（删除、打开文件位置）→ Phase 4
- **过滤与搜索**（按大小、类型、时间过滤）→ Phase 4
- **导出报告** → Phase 4

---

*Phase: 01-扫描引擎*
*Context gathered: 2026-05-05*
