# Phase 1: 扫描引擎 - Discussion Log

**Date:** 2026-05-05
**Participants:** User + Claude

## Gray Areas Discussed

### 1. 扫描并发策略
- **Options:** A) rayon 工作窃取 B) 固定线程池+队列 C) 单线程+yield
- **Decision:** A — rayon 线程池 + 工作窃取
- **Rationale:** 实现简单，Rust 生态成熟，自动负载均衡

### 2. 路径长度处理
- **Options:** A) 启用 `\\?\` 前缀 B) 遇到超长路径跳过
- **Decision:** A — 启用 `\\?\` 前缀
- **Rationale:** 深层目录（如 node_modules）容易超 260 字符；核心 API 均支持；唯一不支持的 `GetFileAttributes` 项目中不使用
- **Caveat:** `GetFileAttributes` 不支持 `\\?\`，需用 `FindFirstFileExW` 替代

### 3. 符号链接 / Junction / 挂载点
- **Options:** A) 跟随 B) 不跟随但标记 C) 不跟随不标记
- **Decision:** B — 不跟随，标记为符号链接类型
- **Rationale:** 应用本质是查看磁盘实际占用，符号链接本身几乎不占空间；跟随会导致重复计算和循环引用

### 4. 扫描过程中文件变更
- **Options:** A) 重试机制 B) 接受快照不完美
- **Decision:** B — 接受不完美
- **Rationale:** 扫描结果代表"扫描开始时刻"的快照，不需要完全精确

### 5. 无权限目录
- **Options:** A) 静默跳过 B) 记录并标注 C) 弹窗提示
- **Decision:** B — 记录并标注，不弹窗
- **Rationale:** 用户需要知道哪些目录被跳过了，但扫描不应被中断

## Deferred Ideas
- 磁盘元信息 → Phase 4
- 磁盘管理功能 → Phase 4
- 过滤与搜索 → Phase 4
- 导出报告 → Phase 4
