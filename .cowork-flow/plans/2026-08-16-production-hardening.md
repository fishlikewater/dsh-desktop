# DSH Desktop 生产级改造方案

> 目标：将当前原型级 Tauri 壳层打磨为**可交付使用的生产级 Windows 桌面应用**。
> 本文档为总方案：列出差距清单、阶段划分、优先级与验收标准；批准后按阶段拆分为独立实施任务（cowork-flow 任务逐项落地）。

## 0. 现状盘点

| 维度 | 现状 | 生产级要求 |
|---|---|---|
| 版本控制 | **无 git 仓库** | 版本化源码、可追溯、可回滚 |
| 测试 | 无自动化测试（仅手动/CDP 临时脚本） | 单元 + 冒烟 + CI 门禁 |
| CI/CD | 无 | Windows 构建流水线 + 产物上传 |
| 安全 | CSP=null、capabilities 偏宽、withGlobalTauri | 最小权限、严格 CSP、注入面收敛 |
| 可靠性 | eprintln 裸日志、启动时序魔法数、YAML 手写解析 | 文件日志、事件驱动、健壮解析 |
| 分发 | NSIS 未签名、无自动更新 | 签名安装包（或文档化降级）、可升级 |
| 维护性 | lib.rs 单文件 605 行、前端单文件 490 行 | 模块化、lint 门禁、规范文档 |
| 文档 | README 含机器绝对路径、无 LICENSE/CHANGELOG | 完整项目文档 + 开源许可证 |

## 1. 差距清单与阶段规划

### Phase 0 — 工程地基（P0，低风险，先行）

| # | 事项 | 说明 | 规模 |
|---|---|---|---|
| 0.1 | git 初始化 + 首次提交 | `git init`；补全 `.gitignore`（清理 `ScreenShot_*.png`、`.toolchain-test/`、`.dsh-vision-toolkit/`、`src-tauri/gen/` 等杂物）；提交基线 | 小 |
| 0.2 | 开源许可与发布文档 | LICENSE（MIT）、CHANGELOG（Keep a Changelog 格式）、SECURITY.md、README 重构（移除 `E:\` 绝对路径，补构建/发布/故障排查/安全说明） | 小 |
| 0.3 | 代码规范门禁 | rustfmt + clippy 全量清理（零警告）；前端内联脚本检查脚本入库（`scripts/check-frontend.mjs`） | 小 |
| 0.4 | 目录整理 | 根目录杂物清理；`docs/` 归位；`.cargo/config.toml` 机器特定路径处理（见 1.4） | 小 |

### Phase 1 — 可靠性与架构（P1，核心改造，行为不变为原则）

| # | 事项 | 说明 | 规模 |
|---|---|---|---|
| 1.1 | Rust 模块化重构 | lib.rs 拆分为 `config.rs`（DSH_URL/settings 路径）、`theme.rs`（解析 + watcher）、`window.rs`（创建/居中/样式守卫）、`tray.rs`、`logging.rs`；**行为不变，重构后全量冒烟** | 中 |
| 1.2 | 日志系统 | `log` + `tauri-plugin-log`：文件轮转（`%APPDATA%\com.dsh.desktop\logs\`）+ Target 隔离；替换全部 `eprintln`；关键路径（托盘、快捷键、watcher、单实例）留审计日志 | 小 |
| 1.3 | 启动时序去魔法数 | 窗口居中改为事件驱动（`WindowEvent::Moved/Resized` 或壳页 show 回调后立即居中，去掉 2500/6500ms 睡眠线程）；iframe 加载完成 → 显示窗口的 5s 兜底保留并注释 | 中 |
| 1.4 | 配置层 | 壳层自有配置（DSH_URL 等）持久化到 `%APPDATA%\com.dsh.desktop\config.json`（环境变量仍优先）；`theme_preference` 解析加固（serde_yaml 或文本解析 + 单元测试覆盖损坏文件/缺失段/未知值） | 中 |
| 1.5 | watcher 健壮性 | notify watch 失败时告警日志 + 周期性重试；事件防抖参数化 | 小 |

### Phase 2 — 安全加固（P1，发布前必须）

| # | 事项 | 说明 | 规模 |
|---|---|---|---|
| 2.1 | CSP 策略 | 评估并落地最小 CSP：`tauri.localhost` 壳页自身脚本 + `http://127.0.0.1:*` iframe 嵌入 + connect-src 限制；必要时用 `dangerousDisableAssetCspModification` 显式声明（需文档化风险） | 中 |
| 2.2 | capabilities 最小权限 | 逐条审查 default.json：移除 `allow-get-all-windows`、`core:default` 中未用项；窗口权限按壳页实际调用收敛 | 小 |
| 2.3 | 全局注入收敛 | 评估 `withGlobalTauri` → preload script（`__TAURI__` 仅壳页可见，iframe 不受影响）；如迁移，事件/命令调用面不变 | 中 |
| 2.4 | URL 校验 | DSH_URL 解析校验已有（http + host），补充端口范围与回环限制说明；第二实例/托盘唤起的聚焦路径单测 | 小 |

### Phase 3 — 测试与 CI（P1，质量门禁）

| # | 事项 | 说明 | 规模 |
|---|---|---|---|
| 3.1 | Rust 单元测试 | theme 解析（正常/损坏/缺失/未知值）、config 解析、窗口几何计算、tray 菜单状态；`cargo test` 全绿 | 中 |
| 3.2 | 端到端冒烟脚本入库 | `.toolchain-test/cdp-verify.mjs` 规范化进 `scripts/`：主题即时同步、窗口控制、服务不可用覆盖层三组场景；文档化运行前置（debug 构建 + DSH 服务） | 小 |
| 3.3 | CI 流水线 | GitHub Actions：Windows runner（MSVC）构建 debug+release、clippy 零警告、cargo test、前端检查、NSIS 打包产物上传；PR 必过 | 中 |
| 3.4 | 工具链决策 | **保留 windows-gnu**（用户决策）：`.cargo/config.toml` 可移植化——优先尝试移除 linker 覆写（现代 rustup gnu 工具链自带 self-contained 链接器，仅本机因 PATH 中老 gcc 8.1 需要覆写）；如需保留则用 `${env:...}` 展开并文档化；CI 用 gnu runner（`rustup toolchain install stable-x86_64-pc-windows-gnu`） | 中 |

### Phase 4 — 分发与更新（P2，交付能力）

| # | 事项 | 说明 | 规模 |
|---|---|---|---|
| 4.1 | 代码签名 | **无证书，文档化降级**（用户决策）：发布说明附 SHA256 哈希 + SmartScreen"未知发布者"提示；后续获得证书再接入 signtool | 中 |
| 4.2 | 自动更新 | **GitHub Releases 通道**（用户决策）：tauri-plugin-updater——签名密钥对（`tauri signer generate`，与代码签名证书无关）生成与保管（私钥入 GitHub Secrets）、更新清单（`latest.json`）、发布流水线发布到 Releases；NSIS 升级路径验证 | 中 |
| 4.3 | 安装体验 | NSIS 配置审查：安装目录、卸载清理（配置/日志/自启项）、版本升级覆盖、多实例保护说明 | 小 |

### Phase 5 — DSH 服务托管（P2，一体化体验，方案 B：拉起系统 dsh）

> 决策：**纯拉起系统 dsh**（用户确认）。不捆绑 sidecar（零额外体积），依赖用户机器已装 Node + dsh；适合本机开发与个人分发场景。

| # | 事项 | 说明 | 规模 |
|---|---|---|---|
| 5.1 | 服务发现与启动 | 壳层启动时探测 DSH_URL 端口：未运行则按优先级查找启动器（PATH `dsh` → nvm 目录 → 配置覆盖项），spawn `dsh --profile web`（或 `node <bin.js> --profile web`），`DSH_HOME`/`DSH_URL` 环境透传 | 中 |
| 5.2 | 进程托管 | 子进程 stdout/stderr 重定向到应用日志目录（复用 1.2 日志体系）；崩溃自动重启（指数退避，防崩溃循环）；应用退出时优雅 kill；单实例下由主进程独占托管 | 中 |
| 5.3 | 状态感知 UI | 覆盖层三态：「正在启动服务…」「启动失败（显示日志路径 + 重试按钮 + 手动启动指引）」「已就绪」；托盘图标可选区分服务状态 | 中 |
| 5.4 | 配置项 | 壳层配置新增：DSH_URL、DSH_HOME、启动器路径覆盖、自动托管开关（关闭则回到纯连接模式，兼容现有手动启动工作流） | 小 |

**可行性依据（已调研）**：DSH 为 MIT 许可 npm 包（`@deepseek-ai/dsh`），入口 `dsh --profile web`；原生依赖（sharp/koffi/node-pty 等）全部带 win32-x64 预编译二进制，无需本地编译；用户数据在 `~/.dsh` 与程序位置解耦。风险：DSH 仍 rc 阶段接口可能变动；与手动启动并存时共享 `~/.dsh`，托管与手动互斥（端口占用检测天然规避双实例）。

### Phase 6 — 体验与兜底（P2/P3，迭代打磨）

| # | 事项 | 说明 | 规模 |
|---|---|---|---|
| 6.1 | 崩溃兜底 | panic hook + 崩溃日志落盘；窗口异常退出恢复（启动时检测上次会话） | 小 |
| 6.2 | 启动性能 | 测量 WebView2 冷启动；评估预加载/延迟加载策略；ifarme 加载时序优化 | 小 |
| 6.3 | 托盘与细节 | 托盘图标状态（服务在线/离线）、通知去重、主题即时反馈完善 | 小 |

## 2. 优先级与依赖关系

```
Phase 0 (git/文档/lint)  ← Phase 1 (模块化/日志/配置)  ← Phase 2 (安全)
                                ↑                            ↓
Phase 3 (测试/CI) 需要 Phase 0+1  ←—— Phase 4 (签名/更新) 需要 CI
Phase 5 (服务托管) 需要 Phase 1 日志 + Phase 3 冒烟  ←—— 可与 Phase 4 并行
Phase 6 (体验) 穿插迭代
```

- **阻断项**：无 git、无 LICENSE、clippy 未清、CSP=null —— 任一存在都不可视为"可交付"。
- **工具链**：windows-gnu（用户决策），CI 用 gnu runner；`.cargo/config.toml` 可移植化。
- **每阶段验收**：cargo build/test/clippy 零警告零失败；冒烟脚本全过；文档同步（README/CHANGELOG/spec）。

## 3. 工作量与排期预估（单人多轮迭代）

| 阶段 | 预估任务数 | 说明 |
|---|---|---|
| Phase 0 | 2~3 个任务 | 每任务 0.5~1 轮 |
| Phase 1 | 3~4 个任务 | 模块化重构独立成任务，先测试后重构 |
| Phase 2 | 2~3 个任务 | CSP 与权限可合并 |
| Phase 3 | 2~3 个任务 | 测试入库 + CI 流水线 |
| Phase 4 | 2~3 个任务 | updater 密钥 + Releases 流水线（签名降级文档化） |
| Phase 5 | 3~4 个任务 | 服务发现/托管/状态 UI/配置 |
| Phase 6 | 2 个任务 | 低优先 |

总计约 17~22 个实施任务；每任务独立走 cowork-flow（planning → implement → review → archive），方案批准后按阶段批量排程。

## 4. 决策记录（2026-08-16 用户确认）

| 决策项 | 结论 |
|---|---|
| 工具链 | **保留 windows-gnu**，config 可移植化，CI 用 gnu runner |
| 许可证 | **MIT** |
| 发布通道 | **GitHub Releases** + tauri updater 自动更新 |
| 代码签名 | **无证书**，文档化降级（SHA256 哈希 + SmartScreen 说明） |
| 范围 | **含全部阶段**（P0~P6，含体验项） |
| 服务集成 | **方案 B：纯拉起系统 dsh**（不捆绑 sidecar，零额外体积；壳层托管生命周期 + 状态感知 UI） |

## 5. 成功标准（整体）

- [ ] 源码入库 git，PR 门禁（clippy/test/build）全绿
- [ ] Windows 安装包（NSIS）可重复构建，卸载/升级干净
- [ ] 安全基线：CSP 落地、权限最小化、注入面收敛
- [ ] 运行可靠性：文件日志、崩溃兜底、事件驱动时序
- [ ] 一体化体验：应用自动拉起/托管 DSH 服务，服务不可用时给出日志与恢复指引
- [ ] 文档完备：README/CHANGELOG/LICENSE/SECURITY/发布流程
