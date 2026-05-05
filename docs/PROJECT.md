# disk_reviewer — 磁盘空间可视化审查工具

## 1. 项目目标

构建一个 Windows 桌面应用，以矩形树图（Treemap）方式可视化磁盘空间占用，支持逐层下钻浏览和历史快照对比。

### 核心功能（MVP）

| 功能 | 说明 |
|------|------|
| 逻辑盘概览 | 选择逻辑盘后，以 Treemap 展示一级目录的空间占比 |
| 逐层下钻 | 点击目录块进入子目录，展示该层级的空间占比分布 |
| 导航回溯 | 面包屑导航，支持返回任意上层目录 |
| 空间标注 | 每个色块显示目录/文件名、大小、占比 |
| 历史快照 | 保存扫描快照，与历史版本对比并高亮差异 |

### 未来扩展方向

- 磁盘元信息展示（文件系统类型、簇大小、已用/可用空间、SMART 信息等）
- 磁盘管理功能（删除、移动文件/目录）
- 过滤与搜索（按大小、类型、修改时间筛选）
- 导出报告（HTML/JSON）

## 2. 技术栈

### 推荐方案：Rust + egui/eframe

| 维度 | 选择 | 理由 |
|------|------|------|
| **后端语言** | Rust | 零成本抽象，无 GC 停顿；直接调用 Win32 API；发布为单文件无依赖 |
| **UI 框架** | egui (eframe) | 即时模式 GUI，Treemap 完全自定义绘制；跨平台；单二进制部署 |
| **扫描引擎** | Rust + winapi / windows-rs | 直接调用 `FindFirstFileExW` + `GetFileAttributesExW`，支持大目录异步遍历 |
| **快照存储** | SQLite (rusqlite) | 单文件数据库，存储扫描快照和差异对比数据 |
| **构建系统** | Cargo | Rust 原生，无需额外配置 |

### 为什么不选其他方案

| 候选方案 | 不选原因 |
|----------|----------|
| C++ / Qt | 你有 Qt 经验，但 Qt 的 Model/View 架构对 Treemap 自定义绘制支持繁琐；构建部署复杂；Rust 在内存安全和 Win32 调用上同样胜任 |
| C# / WPF / WinUI | 依赖 .NET Runtime；启动慢；对底层 Win32 调用需要 P/Invoke 封装 |
| Electron / Web 技术 | 内存占用大（100MB+）；不适合纯本地工具；与底层 API 通信需要 Node 原生模块 |
| Python + PyQt | 打包体积大；GIL 限制多线程扫描性能；部署依赖 Python 运行时 |

### 关键技术考量

1. **扫描性能**：使用 `FindFirstFileExW` + `FTS_NO_RECALL` 标志避免触发 Windows 脱机文件召回；异步线程池遍历，UI 线程不阻塞
2. **大目录处理**：增量推送扫描结果到 UI，避免一次性加载百万文件导致内存爆炸
3. **Treemap 算法**：采用 squarified treemap 算法，保证矩形长宽比接近 1:1，视觉效果好
4. **快照差异**：基于目录树结构的增删改检测，高亮新增/增长/缩小的目录

## 3. 项目结构

```
disk_reviewer/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口，eframe 应用启动
│   ├── app.rs               # 应用状态管理（当前视图、导航栈、扫描状态）
│   ├── scanner/             # 磁盘扫描引擎
│   │   ├── mod.rs           # 扫描器入口，线程池调度
│   │   ├── walker.rs        # 目录遍历（Win32 FindFirstFileExW）
│   │   └── types.rs         # 扫描结果数据结构（DirEntry, FileEntry）
│   ├── treemap/             # Treemap 渲染
│   │   ├── mod.rs           # Treemap 绘制入口
│   │   ├── layout.rs        # Squarified Treemap 布局算法
│   │   └── renderer.rs      # egui 绘制（矩形、标签、颜色映射）
│   ├── snapshot/            # 快照存储与对比
│   │   ├── mod.rs           # 快照管理
│   │   ├── store.rs         # SQLite 读写
│   │   └── diff.rs          # 差异检测算法
│   ├── ui/                  # UI 组件
│   │   ├── mod.rs
│   │   ├── drive_selector.rs    # 逻辑盘选择面板
│   │   ├── breadcrumb.rs        # 面包屑导航
│   │   ├── info_panel.rs        # 选中项详情面板
│   │   └── snapshot_dialog.rs   # 快照管理对话框
│   └── platform/            # Windows 平台相关
│       ├── mod.rs
│       ├── drives.rs        # 逻辑盘枚举（GetLogicalDrives）
│       └── metadata.rs      # 磁盘元信息（GetDiskFreeSpaceExW, DeviceIoControl）
├── docs/
│   └── PROJECT.md           # 本文件
└── CLAUDE.md
```

## 4. 开发计划

### Phase 1：项目初始化 + 扫描引擎

**目标**：能扫描指定目录，输出目录树结构

- [ ] 初始化 Rust 项目（Cargo.toml, 依赖配置）
- [ ] 实现逻辑盘枚举（`GetLogicalDrives` / `GetLogicalDriveStringsW`）
- [ ] 实现异步目录遍历器（`FindFirstFileExW`，线程池）
- [ ] 定义扫描结果数据结构（`DirEntry`, `FileEntry`, `ScanResult`）
- [ ] 增量推送机制（扫描过程中实时更新 UI）

### Phase 2：Treemap 可视化

**目标**：扫描完成后，以矩形树图展示空间占比

- [ ] 实现 Squarified Treemap 布局算法
- [ ] 实现 egui 自定义绘制（矩形、标签、颜色映射）
- [ ] 实现点击下钻交互
- [ ] 实现面包屑导航和返回上层
- [ ] 实现选中项详情面板（路径、大小、占比、文件数量）

### Phase 3：快照与差异对比

**目标**：保存扫描快照，支持历史对比

- [ ] 设计快照数据库 schema（SQLite）
- [ ] 实现快照保存/加载
- [ ] 实现差异检测算法（新增/删除/增长/缩小）
- [ ] 差异高亮显示（颜色区分变化类型）
- [ ] 快照管理对话框（创建/删除/切换快照）

### Phase 4：磁盘元信息 + 打磨

**目标**：展示磁盘元信息，完善交互细节

- [ ] 磁盘元信息面板（文件系统、簇大小、总空间、可用空间）
- [ ] 扫描进度指示器
- [ ] 右键菜单（打开文件位置、复制路径）
- [ ] 性能优化（大目录虚拟化、LOD 细节层次）
- [ ] 打包配置（cargo-deb / MSI 安装包）

## 5. 技术风险与应对

| 风险 | 影响 | 应对策略 |
|------|------|----------|
| 大目录扫描卡顿 | UI 无响应 | 异步线程池 + 增量推送；扫描与渲染分离 |
| 系统/隐藏文件访问被拒 | 数据不完整 | 以管理员权限运行可选；跳过无权限目录并标注 |
| 文件数量过多（百万级） | 内存爆炸 | 只保留 Top N 小文件聚合为 "Others"；虚拟化渲染 |
| egui 自定义绘制性能 | 大量矩形渲染卡顿 | 使用 egui 的 `Painter` 批量绘制；视口裁剪 |

## 6. TDD 强制要求

**本项目所有开发任务必须遵循 TDD（测试驱动开发）流程。** 这是硬性约束，不可跳过。

### GSD 配置要求

初始化 GSD 工作流时，必须在 `.planning/config.json` 中设置：
```json
{
  "workflow": {
    "tdd_mode": true
  }
}
```

### 执行规则

1. 每个 Phase 的 plan-phase 中，合格任务自动标记 `type: tdd`
2. 每个 TDD 任务必须经历 RED → GREEN → REFACTOR 三阶段
3. 提交消息遵循 `test(...)` → `feat(...)` → `refactor(...)` 序列
4. 阶段结束后执行 TDD REVIEW 检查点

详细规则见 `docs/TDD_ENFORCEMENT.md`。

## 7. 依赖清单

| 依赖 | 用途 |
|------|------|
| `eframe` + `egui` | UI 框架 |
| `windows` / `winapi` | Win32 API 绑定 |
| `rusqlite` | SQLite 快照存储 |
| `rayon` | 线程池并行扫描 |
| `walkdir` | 备选目录遍历（纯 Rust 实现） |
| `serde` + `serde_json` | 序列化（快照导出） |
| `chrono` | 时间戳（快照版本） |
| `sysinfo` | 系统信息（可选，用于磁盘元信息） |
