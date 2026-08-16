# DSH Desktop 开发计划与任务分解

> 上游：`docs/production-hardening-plan.md`（生产级改造总方案）。
> 本文件将方案拆解为**可独立排期、实施、验收的开发任务**；每任务执行时在 cowork-flow 中单独建任务（planning → implement → review → archive），本文件为任务范围与验收的权威依据。
> 决策记录（用户确认，2026-08-16）：保留 windows-gnu 工具链 · MIT 许可证 · GitHub Releases + tauri updater · 无代码签名证书（文档化降级）· 范围含全部阶段 · 服务集成采用**方案 B（纯拉起系统 dsh）**。

---

## 阶段总览

| 阶段 | 主题 | 任务 | 依赖 | 优先级 |
|---|---|---|---|---|
| P0 | 工程地基 | T0.1~T0.3 | — | P0（先行） |
| P1 | 可靠架构 | T1.1~T1.4 | P0 | P1 |
| P2 | 安全加固 | T2.1~T2.3 | P0 | P1 |
| P3 | 测试与 CI | T3.1~T3.3 | P0+P1 | P1 |
| P4 | 分发与更新 | T4.1~T4.3 | P3 | P2 |
| P5 | 服务托管 | T5.1~T5.4 | P1+P3 | P2（可与 P4 并行） |
| P6 | 体验与兜底 | T6.1~T6.2 | 任意 | P3 |

共 **22 个任务**。串行推荐顺序：T0.1→T0.2→T0.3→T1.1→T1.2→T1.3→T1.4→T2.1→T2.2→T2.3→T3.1→T3.2→T3.3→T4.1→T4.2→T4.3→T5.1→T5.2→T5.3→T5.4→T6.1→T6.2。

---

## Phase 0 — 工程地基（P0）

### T0.1 git 初始化与仓库整洁
- **目标**：源码纳入版本控制，工作区无杂物。
- **范围**：
  - `git init` + 首次基线提交（含 src-tauri/、frontend-dist/、docs/、.cowork-flow/ 按策略纳入）
  - 补全 `.gitignore`：`ScreenShot_*.png`、`.toolchain-test/`、`.dsh-vision-toolkit/`、`src-tauri/gen/`、日志目录、`*.bak-*`
  - 清理根目录杂物（截图、临时验证产物），`cdp-verify.mjs` 移入 `scripts/`（见 T3.2 亦可）
- **验收**：`git status --porcelain` 干净；首次提交含全部源文件、不含生成物；`.gitignore` 生效测试（`git check-ignore` 抽查）。
- **依赖**：无。**规模**：小。

### T0.2 开源许可与项目文档
- **目标**：可交付的合规文档体系。
- **范围**：
  - `LICENSE`（MIT，版权人按用户身份填写）
  - `CHANGELOG.md`（Keep a Changelog 格式，从 v0.1.0 起，记录既有功能与本次改造）
  - `SECURITY.md`（报告渠道 + 安全说明）
  - `README.md` 重构：移除机器绝对路径（`E:\...\dash-plugin` 改为描述性文字）；补「安装/构建/发布/故障排查/安全说明」章节；版本徽章占位
- **验收**：文档齐全且无机器特定路径；README 构建步骤在干净环境可复现。
- **依赖**：无（可与 T0.1 并行）。**规模**：小。

### T0.3 规范门禁与工具链配置
- **目标**：零警告基线 + 前端检查脚本化 + 工具链可移植。
- **范围**：
  - `rustfmt.toml`（或默认）+ `cargo clippy` 全量清理（`-D warnings` 通过）
  - 新增 `scripts/check-frontend.mjs`：提取 `frontend-dist/index.html` 内联 JS 做 `node --check` + 关键 id/类名一致性断言
  - `.cargo/config.toml` 可移植化：优先移除 linker 覆写（现代 rustup gnu 工具链 self-contained 自链接）；确需保留时改用 `${env:...}` 展开并文档化
  - `package.json` scripts 补充：`lint`、`check`（聚合 cargo clippy + 前端检查）
- **验收**：`cargo clippy -- -D warnings` 零告警；`npm run check` 通过；在无该绝对路径的机器可构建（用 CI 验证或文档化前置）。
- **依赖**：无。**规模**：小。

---

## Phase 1 — 可靠架构（P1）

### T1.1 Rust 模块化重构
- **目标**：lib.rs（605 行）拆分为职责模块，**行为完全不变**。
- **范围**：
  - `src/config.rs`：DSH_URL 解析、settings 路径、常量（窗口尺寸/端口）
  - `src/theme.rs`：`theme_preference` 解析 + `watch_settings_theme` watcher + `theme-file-changed` 事件
  - `src/window.rs`：窗口创建/居中/样式守卫/伪最大化几何/`work_area`/`work_area_ffi`
  - `src/tray.rs`：托盘构建与事件
  - `src/logging.rs`：占位（T1.2 填充）
  - `lib.rs` 仅保留 `run()` 装配
- **验收**：重构前后行为一致（冒烟脚本全过、手动窗口/托盘/主题操作对比）；cargo build/test/clippy 零警告。
- **依赖**：T0.3。**规模**：中（先建测试基线再重构，见 T3.1 前置部分）。

### T1.2 日志系统
- **目标**：全链路可观测，替换裸 `eprintln`。
- **范围**：
  - `log` + `tauri-plugin-log`：文件输出（`%APPDATA%\com.dsh.desktop\logs\dsh-desktop.log`，按大小轮转）+ 调试构建保留 stderr
  - 关键路径留审计日志：托盘事件、全局快捷键、单实例唤起、watcher 启动/事件/失败、窗口生命周期、主题切换
- **验收**：运行后日志文件生成且内容覆盖关键路径；日志不包含敏感数据（token 等）。
- **依赖**：T1.1。**规模**：小。

### T1.3 启动时序去魔法数
- **目标**：消除 2500/6500ms 居中睡眠线程与硬编码等待，事件驱动化。
- **范围**：
  - 窗口居中改为：壳页 iframe load → 壳页调用 show() 后立即居中（已有 120ms 延迟改为事件/回调链）；Rust 侧兜底线程改为「窗口首次可见事件」触发
  - 服务检测 2s 轮询保留（已事件化成本高），但初始探测提前
  - 5s 强制显示兜底保留，注释说明理由
- **验收**：冷启动居中无闪烁无跳动；连续 10 次启动位置正确；冒烟脚本覆盖。
- **依赖**：T1.1。**规模**：中。

### T1.4 配置层与主题解析加固
- **目标**：壳层自有配置 + 解析健壮性。
- **范围**：
  - 壳层配置 `%APPDATA%\com.dsh.desktop\config.json`（DSH_URL、DSH_HOME、行为开关），环境变量仍最高优先级；`serde` + 单元测试
  - `theme_preference` 解析加固：损坏 YAML / 缺失段 / 未知值 / 空文件的确定性行为（单元测试覆盖），保留轻量文本解析或迁移 `serde_yaml`
  - watcher 健壮性：watch 失败告警日志 + 周期性重试（并入本任务）
- **验收**：单测覆盖 6+ 场景；损坏 settings.yaml 不 panic 不闪退；配置读写正确。
- **依赖**：T1.1、T1.2。**规模**：中。

---

## Phase 2 — 安全加固（P1）

### T2.1 CSP 落地
- **目标**：替换 `security.csp: null`。
- **范围**：
  - 分析壳页资源面：本地脚本（tauri.localhost）、iframe 嵌入 `http://127.0.0.1:*`、connect-src（服务探测 fetch、WebSocket）、style-src（内联样式）
  - 落地最小 CSP 并实测：壳页功能（拖拽/resize/主题/窗口控制）与 iframe 加载全部正常
  - 若需放宽资产修改，用 `dangerousDisableAssetCspModification` 显式声明并在文档记录风险
- **验收**：tauri.conf.json 含非 null CSP；冒烟脚本全过；DevTools 无 CSP 违规报错。
- **依赖**：P0。**规模**：中。

### T2.2 capabilities 最小权限
- **目标**：权限面收敛到壳页实际调用。
- **范围**：
  - 逐条审查 `capabilities/default.json`：移除 `core:default` 未用子项、`allow-get-all-windows` 等宽权限
  - 按壳页 JS 实际调用清单核对（minimize/maximize/close/start-dragging/set-position/set-size/outer-size/current-monitor…）
  - 权限变更后冒烟回归
- **验收**：权限清单与调用面一一对应；移除权限后壳页功能不受影响；文档说明每条保留权限的理由。
- **依赖**：P0。**规模**：小。

### T2.3 全局注入评估与收敛
- **目标**：评估并（如可行）收敛 `withGlobalTauri`。
- **范围**：
  - 现状：`withGlobalTauri: true` 仅壳页需要（iframe 是远程页面天然隔离）
  - 评估迁移 preload script：`__TAURI__` 仅注入壳页文档、移除全局注入；事件/命令调用面不变
  - 若迁移风险高（无构建链的静态 HTML + preload 兼容性），则保留并在文档记录理由与隔离边界
- **验收**：无论迁移与否，壳页功能回归全过；文档记录最终决策与安全边界。
- **依赖**：P0。**规模**：中。

---

## Phase 3 — 测试与 CI（P1）

### T3.1 Rust 单元测试
- **目标**：核心逻辑可回归。
- **范围**：
  - `theme` 解析：正常/损坏/缺失段/未知值/空白 5+ 用例
  - `config` 解析：默认/覆盖/非法值/优先级
  - 窗口几何：居中计算、伪最大化判定、尺寸补偿
  - 测试随模块化同步建立（T1.1 前先立测试基线）
- **验收**：`cargo test` 全绿；关键逻辑破坏时测试必失败（反例验证）。
- **依赖**：T1.1、T1.4。**规模**：中。

### T3.2 端到端冒烟脚本入库
- **目标**：CDP 验证正式化。
- **范围**：
  - `.toolchain-test/cdp-verify.mjs` 规范化迁移 `scripts/smoke/`：场景分组——主题即时同步、窗口控制（最小化/伪最大化/还原）、服务不可用覆盖层、拖拽区域
  - 前置条件检测（debug 构建存在、DSH 服务、CDP 端口）与失败信息友好化
  - 文档：`docs/testing.md` 说明运行方式与 CI 接入点
- **验收**：全部场景在本地可重复通过；失败时给出可操作诊断。
- **依赖**：T3.1（可选）。**规模**：小。

### T3.3 CI 流水线
- **目标**：PR 门禁 + 构建产物。
- **范围**：
  - GitHub Actions：`windows-latest` + `rustup toolchain install stable-x86_64-pc-windows-gnu`
  - 步骤：cargo fmt 检查 → clippy `-D warnings` → cargo test → `npm run check` → `npm run build`（debug）→ NSIS release 打包上传 artifact
  - 触发器：push（main）+ PR；release 分支触发发布流水线（接入 T4.2）
- **验收**：流水线在无本地环境的干净 runner 全绿；产物 artifact 可下载安装。
- **依赖**：T0.3、T3.1、T3.2。**规模**：中。

---

## Phase 4 — 分发与更新（P2）

### T4.1 updater 密钥与基础配置
- **目标**：自动更新基础设施就绪。
- **范围**：
  - `tauri signer generate` 生成密钥对；公钥入 `tauri.conf.json`；私钥保管方案（本机加密存储 + GitHub Secrets 占位）
  - `tauri-plugin-updater` 接入：前端检查更新（托盘菜单「检查更新」+ 启动静默检查）、下载与安装流程
  - 更新清单格式与签名校验落地
- **验收**：本地模拟更新清单可触发下载/安装；未签名/篡改清单被拒绝。
- **依赖**：T3.3（CI 产物）。**规模**：中。

### T4.2 GitHub Releases 发布流水线
- **目标**：一键发布。
- **范围**：
  - Actions 发布工作流：tag（`v*`）触发 → 构建 → 生成 NSIS 安装包 + `latest.json` 更新清单 → 发布到 GitHub Releases（草稿 → 手动确认）
  - 发布说明模板：变更摘要 + **SHA256 校验和 + SmartScreen「未知发布者」说明**（无证书降级方案落地）
- **验收**：打 tag 后产物自动构建为 draft release；校验和与说明齐全。
- **依赖**：T4.1、T3.3。**规模**：中。

### T4.3 NSIS 安装体验与签名降级文档
- **目标**：安装/卸载/升级干净。
- **范围**：
  - NSIS 配置审查：安装目录、快捷方式、卸载清理（配置/日志/自启注册表项）、版本升级覆盖旧版
  - `docs/publishing.md`：发布流程、校验和验证步骤、SmartScreen 说明、证书获取指引（未来签名接入点）
- **验收**：全新安装/覆盖升级/卸载三路径实测干净；发布文档完整。
- **依赖**：T4.2。**规模**：小。

---

## Phase 5 — DSH 服务托管（P2，方案 B：拉起系统 dsh）

### T5.1 服务发现与启动
- **目标**：应用可自动拉起 DSH 服务。
- **范围**：
  - 端口探测（已有逻辑复用）→ 未运行则查找启动器：PATH `dsh` → 常见 nvm/全局 node 目录 → 配置覆盖项
  - spawn `dsh --profile web`（或 `node <dsh bin.js> --profile web`），透传 `DSH_HOME`/`DSH_URL` 环境
  - 启动结果反馈：成功（端口就绪）/失败（错误信息分类：未安装/启动崩溃/端口被占）
- **验收**：停止外部 dsh 后启动应用 → 服务自动拉起、GUI 加载；`DSH_URL` 指向自定义端口时探测正确。
- **依赖**：T1.2（日志）、T3.2（冒烟）。**规模**：中。

### T5.2 进程托管
- **目标**：子进程生命周期可控。
- **范围**：
  - stdout/stderr 重定向到日志目录（`logs/dsh-service.log`）
  - 崩溃自动重启：指数退避（1s/2s/4s…上限 60s），连续失败 N 次后停止并上报 UI
  - 应用退出时优雅终止子进程（先 SIGTERM/CTRL_BREAK，超时强制 kill）
  - 单实例：仅主进程托管；二次实例唤起不重复拉起（端口占用天然互斥）
- **验收**：kill 服务进程后自动重启；连续失败有上限；退出应用无残留进程。
- **依赖**：T5.1。**规模**：中。

### T5.3 状态感知 UI
- **目标**：服务状态可视化。
- **范围**：
  - 覆盖层三态：「正在启动服务…」「启动失败（日志路径 + 重试按钮 + 手动启动指引）」「已就绪」
  - 托盘菜单加「DSH 服务状态」只读项或图标状态区分（可选）
  - Rust → 前端事件：`service-state-changed`（starting/ready/failed/stopped）
- **验收**：三态切换正确；失败态给出可操作信息（含日志路径）；冒烟脚本覆盖。
- **依赖**：T5.1、T5.2。**规模**：中。

### T5.4 服务托管配置项
- **目标**：托管行为可配置。
- **范围**：
  - 壳层配置新增：`auto_manage_service`（默认开）、`dsh_launcher`（路径覆盖）、`dsh_home`、`service_url`
  - 关闭自动托管 = 纯连接模式（现有工作流完全兼容）
- **验收**：开关与覆盖项生效；关闭托管时行为与当前版本一致。
- **依赖**：T1.4、T5.3。**规模**：小。

---

## Phase 6 — 体验与兜底（P3）

### T6.1 崩溃兜底
- **目标**：崩溃可诊断。
- **范围**：panic hook 落盘（`logs/crash-*.log`，含版本/时间/回溯）；启动时检测上次异常退出并提示；关键状态（窗口位置/尺寸）异常恢复。
- **验收**：人为触发 panic → 日志可定位；重启无状态损坏。
- **依赖**：T1.2。**规模**：小。

### T6.2 性能与托盘细节打磨
- **目标**：启动体验与常驻细节。
- **范围**：WebView2 冷启动测量与优化（预加载/延迟加载评估）；托盘图标服务在线/离线状态；通知去重；主题即时反馈收尾。
- **验收**：启动时间记录基线并优化 ≥1 项；托盘/通知细节按清单完成。
- **依赖**：任意。**规模**：小。

---

## 执行约定

1. **任务创建**：每任务执行时用 `task next --run --title ... --slug ... --from-plan` 创建并绑定本文件对应条目；scope 变更需更新本文件。
2. **验收**：每任务完成须满足本文件「验收」列 + 项目 DoD（clippy/test/build 零警告、文档同步）。
3. **依赖**：串行推荐顺序执行；P4 与 P5 可并行，需确保无文件冲突（不同模块）。
4. **范围外**：DSH GUI 自身功能、DSH 服务侧改造（托管仅调用 `dsh --profile web` 公开入口）。
5. **变更**：本文件为权威任务清单；调整需在对应任务 review 中记录。
