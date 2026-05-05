# Requirements: disk_reviewer

**Defined:** 2026-05-05
**Core Value:** 直观展示磁盘空间占用，让用户一眼看出"谁占了多少空间"

## v1 Requirements

### 扫描引擎

- [ ] **SCAN-01**: 枚举 Windows 所有逻辑盘并显示盘符和总空间
- [ ] **SCAN-02**: 异步遍历指定目录的完整目录树
- [ ] **SCAN-03**: 扫描过程中增量推送结果到 UI，不阻塞界面
- [ ] **SCAN-04**: 跳过无权限访问的目录并在结果中标注
- [ ] **SCAN-05**: 大文件数量目录下，小文件自动聚合为 "Others" 条目

### Treemap 可视化

- [ ] **VIS-01**: 基于空间占比的矩形树图（Squarified Treemap 算法）
- [ ] **VIS-02**: 每个色块显示目录/文件名、大小、占比
- [ ] **VIS-03**: 点击目录块进入子目录视图
- [ ] **VIS-04**: 面包屑导航，支持返回任意上层目录
- [ ] **VIS-05**: 选中项详情面板（完整路径、大小、占比、文件/子目录数量）

### 快照与对比

- [ ] **SNAP-01**: 将当前扫描结果保存为快照（SQLite 存储）
- [ ] **SNAP-02**: 加载历史快照并在 Treemap 中展示
- [ ] **SNAP-03**: 差异检测：识别新增、删除、增长、缩小的目录
- [ ] **SNAP-04**: 差异高亮显示（颜色区分变化类型）
- [ ] **SNAP-05**: 快照管理：创建、删除、切换快照

## v2 Requirements

### 磁盘元信息

- **META-01**: 显示磁盘文件系统类型（NTFS/FAT32/exFAT）
- **META-02**: 显示簇大小、总空间、已用空间、可用空间
- **META-03**: 显示 SMART 健康状态信息

### 磁盘管理

- **MGMT-01**: 在文件资源管理器中打开选中文件/目录
- **MGMT-02**: 删除选中文件/目录（需确认）
- **MGMT-03**: 复制文件/目录路径到剪贴板

### 过滤与搜索

- **FILT-01**: 按文件大小过滤显示
- **FILT-02**: 按文件类型过滤显示
- **FILT-03**: 按修改时间过滤显示
- **FILT-04**: 搜索文件名

## Out of Scope

| Feature | Reason |
|---------|--------|
| 实时文件系统监控 | 按需扫描即可，实时监控增加复杂度 |
| 网络/远程磁盘 | 仅本地逻辑盘 |
| 文件类型分类统计 | 未来版本考虑 |
| 导出报告（HTML/JSON） | 未来版本考虑 |
| 跨平台（macOS/Linux） | 仅 Windows，依赖 Win32 API |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SCAN-01 | Phase 1 | Pending |
| SCAN-02 | Phase 1 | Pending |
| SCAN-03 | Phase 1 | Pending |
| SCAN-04 | Phase 1 | Pending |
| SCAN-05 | Phase 2 | Pending |
| VIS-01 | Phase 2 | Pending |
| VIS-02 | Phase 2 | Pending |
| VIS-03 | Phase 2 | Pending |
| VIS-04 | Phase 2 | Pending |
| VIS-05 | Phase 2 | Pending |
| SNAP-01 | Phase 3 | Pending |
| SNAP-02 | Phase 3 | Pending |
| SNAP-03 | Phase 3 | Pending |
| SNAP-04 | Phase 3 | Pending |
| SNAP-05 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0 ✓

---
*Requirements defined: 2026-05-05*
*Last updated: 2026-05-05 after initial definition*
