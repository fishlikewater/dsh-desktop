# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [0.1.6] - 2026-08-23

### Added

- **多会话窗口**：托盘「新建会话…」打开独立会话窗口（独立检测服务、可分别切地址），托盘菜单动态列出会话项并可唤起/关闭；窗口状态记忆按 label 分域（主窗口记忆，会话窗口居中）；会话窗口独立关闭语义
- **全局快捷键可配置**：设置页可修改唤起主窗口的快捷键（默认 Windows Ctrl+Shift+D / macOS Cmd+Shift+D，支持 ctrl/cmd/alt/shift + a-z/0-9/f1-f12），修改即时生效；注册失败仅告警
- **应用内日志查看器**：设置页「查看」内嵌读尾面板（末 500 行，等宽可滚动，刷新/空态/错误态），「打开目录」保留
- **About 弹层**：展示壳版本 + DSH CLI 版本（`dsh --version` 探测，未安装可读提示）+ 服务地址
- **服务地址可编辑 + 历史切换**：地址行下拉 + 输入 + 切换（TCP 探测可达性，非本机地址 UI 提示告知限制），历史最新在前、上限 5 条
- **DPI/显示器适配**：跨屏缩放/热插拔时窗口尺寸与位置夹取到工作区（clamp_rect + 缩放监听兜底）
- **托盘状态图标**：托盘图标区分服务在线/离线（白描边圆点双态，随壳页检测同步）
- **覆盖层增强**：安装指引折叠块（npm 安装 + DSH_CLI 说明）、「打开设置」入口、页面加载进度条（15s 黑洞期顶部细条，每秒更新）
- **代码签名/公证前置工程**：release.yml 条件化 Windows signtool 签名 + macOS notarytool 公证步骤与自动验证门禁（signtool verify /pa、codesign --verify + spctl --assess），secrets 注入即启用（启用手册见 docs/release-process.md）

### Fixed

- **package.json 版本漂移**：同步至与 tauri.conf.json / Cargo.toml / CHANGELOG 一致（0.1.6），并纳入版本一致性门禁（check-version.mjs 检查四处）
- **会话窗口创建死锁**：窗口创建/销毁移出 IPC 同步命令线程（spawn 异步执行），避免 Windows 上与主线程互等导致的命令超时

### 工程

- 冒烟测试扩展至 8 场景（多会话窗口场景排最后，断言走 Rust 命令 + CDP 列表计数；evalShell 逐 target 探测主窗口，evalIn 两阶段超时兜底）
- 单元测试 70 → 80 例（快捷键解析 5 例、日志读尾 5 例）
- 评估结论记录：dsh sidecar 放弃捆绑（npm shim 无独立可执行产物）、dsh:// 深链接不实施（GUI 无路由）、Linux 服务自管理不实施（systemd 语义冲突）——详见 docs/testing.md

## [0.1.5] - 2026-08-22

> 首个正式发布的 macOS 完整适配版本：v0.1.4 构建为发布草稿、未对外发布，其全部内容并入本版（详见 [0.1.4]）。

- macOS 原生窗框与红绿灯标题栏（Overlay，VS Code 同款）+ 原生全屏
- 应用图标恢复黑鲸鱼设计（含 macOS Dock 尺寸修复）
- macOS 服务自管理（覆盖层/托盘一键启动与停止 DSH 服务）
- 打包与 CI 修复：mac 图标进包（bundle.icon 引用 icns）、CSP sha256 换行符一致性（`.gitattributes` eol=lf）
- 详细条目见 [0.1.4]

## [0.1.4] - 2026-08-22

### Added

- **macOS 原生窗口装饰**：保留系统窗框与红绿灯按钮（Overlay 标题栏，内容延伸至标题栏下，VS Code 同款）；壳页标题栏缩至 28px 并与红绿灯垂直居中，左侧留出按钮区域，隐藏 Windows 风格的最小化/最大化/关闭按钮与自绘边缘 resize 热区；字体栈加入 `-apple-system`/`BlinkMacSystemFont`
- **macOS 原生全屏**：绿色按钮（红绿灯）与双击标题栏均进入系统全屏空间（此前无边框窗口 + `maximizable(false)` 没有任何全屏入口）；全屏动画与退出均由系统处理
- macOS 窗口不再启用透明背景（透明窗口在原生全屏下存在渲染问题），壳页 body 铺满主题背景色保证暗色主题无露白
- **macOS 服务自管理**：覆盖层「启动 DSH 服务」与托盘菜单服务启停现已支持 macOS（此前为 stub，需手动拉起）——`dsh` 探测覆盖 Homebrew / nvm / npm 全局目录与登录 shell（GUI 应用 PATH 精简，且 dsh 的 `env node` shebang 依赖 PATH 找到 node）；拉起时合并富 PATH、独立进程组；停止用进程组 SIGTERM（后台收割僵尸，2s 未退再 SIGKILL）；仍为方案 B（不捆绑 sidecar），仅停止本应用拉起的服务

### Fixed

- **应用图标恢复为黑鲸鱼设计**：此前安装包图标是按深蓝 "D" 设计（`icon-source.png`）生成的，黑鲸鱼设计（`app-icon.svg`：黑鲸鱼 + 浅灰圆角底）仅出现在壳页标题栏；现以鲸鱼设计全量重建各平台图标：macOS 按 Apple 图标网格规范（内容 824×824 居中 + 四周 100px 透明边距）打包 `icon.icns` 并让 `bundle.icon` 显式引用（此前 tauri-bundler 会把图标列表中的满铺 PNG 重新打包成 icns，带边距的 icns 不会被使用）；Windows 图标保持满铺规范

### 工程

- tauri.conf.json CSP script-src sha256 随壳页脚本变更同步更新（check:frontend 门禁通过）
- 新增 `scripts/regen-icons.swift`：解析 `app-icon.svg` 栅格化 → 全量生成各平台 PNG/ICNS/ICO（改图后重跑即可）
- 新增 `.gitattributes`：`frontend-dist/index.html` 固定 `eol=lf`——CSP sha256 基于实际字节，Windows 检出默认 CRLF 会导致 hash 不一致（CI 红 + 构建产物壳页脚本被 CSP 拦截）

## [0.1.3] - 2026-08-20

### Fixed

- macOS 产物 ad-hoc 代码签名：修复 Apple Silicon（M 系列）安装后「已损坏，无法打开」问题
  （CI 构建后对 .app 递归 `codesign -`，重建 dmg / updater 包并重签 .sig 后覆盖上传）

### 工程

- release.yml 支持 `workflow_dispatch` 手动触发

## [0.1.2] - 2026-08-20

### Added

- 设置页显示当前应用版本（新增 `app_version` command）
- 设置页「自动更新」：检查更新后提供独立的「更新」按钮（发现新版本时启用，点击下载安装并重启）

### Fixed

- 设置弹层排版：弹层相对整个窗口居中（此前仅在 iframe 内容区居中）、关闭按钮与标题垂直中心对齐、卡片内边距/行距统一、遮罩层级优化

### 工程

- 修复 CI：`cargo fmt --check` 门禁（settings.rs 长行断行）；Release 构建步骤注入签名密钥（createUpdaterArtifacts 强制签名，与 release.yml 对齐）

## [0.1.1] - 2026-08-20

### 新增（自动更新）

- 「检查更新」支持发现新版本后一键下载安装并重启（`update_install`：download_and_install + request_restart）
- updater `endpoints` 指向真实仓库 `fishlikewater/dsh-desktop`（不再依赖 CI 动态替换占位）
- 新增 CDP 冒烟 `scripts/smoke/updater-check.mjs`（验证 `update_check` 端到端可达）

### 工程化（生产级改造，见 docs/development-plan.md）

- T0.1 初始化 git 仓库与提交基线；T0.2 MIT 许可证、CHANGELOG、SECURITY 文档
- T0.3 代码规范门禁（`npm run check`：clippy `-D warnings` + 前端脚本/CSP hash 检查）
- T1.1 Rust 模块化重构（config/logging/theme/tray/window）
- T1.2 日志系统（tauri-plugin-log：文件输出 1MB 轮转 + debug stdout；关键路径审计日志）
- T1.3 启动时序去魔法数（居中线程 → 壳页事件驱动）
- T1.4 壳层配置层（config.json：dsh_url/dsh_home，环境变量优先）+ 主题解析/监听加固（23 例单测）
- T2.1 壳页 CSP（内联脚本 sha256 白名单，frame/connect 限 127.0.0.1/localhost）
- T2.2 capabilities 最小权限（6 项窗口写权限 + core:default）
- T2.3 全局注入评估（iframe 能力层隔离：对象存在但 IPC 按 URL 粒度拒绝）
- T3.1 Rust 单元测试（23 例，含反例验证）；T3.2 端到端冒烟脚本（`npm run smoke`，4 场景）
- T3.3 CI 流水线（.github/workflows/ci.yml：fmt→clippy→单测→构建→NSIS 打包）
- T4.1 自动更新（tauri-plugin-updater + ed25519 密钥，发布物 latest.json）
- T4.2 发布流程（安装包核验、SHA256 校验和、docs/release-process.md）
- T4.3 GitHub Release 自动构建（release.yml：v* tag 触发，并行构建 macOS/Windows 安装包并发布 Release 草稿，含签名 latest.json 与 SHA256SUMS）

### 修复（macOS 构建适配与打磨）

- macOS 构建适配：Windows-only FFI/进程 API 按平台隔离（window 样式段、工作区获取、服务自管理、打开日志目录）；非 Windows 平台服务管理为 stub（返回可读错误）
- 服务探活不再 panic：地址解析失败/空结果视为不可达
- URL 参数编码完整化：服务地址经 ?dsh= 传入时 #/+/空格等字符不再丢失
- 壳页服务提示行改 DOM 构建（消除 innerHTML 拼接注入面，CSP hash 已同步）
- 停止服务失败时保留 pid 记录（可重试）
- 日志轮转封顶（KeepSome(5)，约 6MB 上限，避免无限增长）
- 修复：无 DSH_HOME 环境变量启动时主题路径解析缺 .dsh（默认数据目录应为 ~/.dsh），标题栏不随主题变化
- 自动更新：updater endpoint 从 OWNER/REPO 占位改为真实仓库；设置弹层检查更新支持发现新版本后一键「立即更新」下载安装并重启

## [0.1.0] - 2026-08-16

### Added

- 无边框窗口 + 自定义标题栏（拖拽区、双击最大化、最小化/最大化/关闭按钮）
- Windows 11 Mica 毛玻璃（Win10 回退 Acrylic）
- 系统托盘：左键显示/隐藏窗口；菜单（显示主窗口/开机自启/发送测试通知/退出）
- 开机自启开关（tauri-plugin-autostart）
- 全局快捷键 `Ctrl+Shift+D` 唤起主窗口
- 关闭最小化到托盘，应用常驻后台
- 系统通知（`notify` command）
- 服务不可用覆盖层 + 自动重连（2s 轮询检测）
- 标题栏/覆盖层跟随 DSH 外观主题（settings.yaml ui-theme.preference；
  Rust notify 监听 + `theme-file-changed` 事件即时同步，30s 轮询兜底）
- 可配置服务地址（`DSH_URL` 环境变量，默认 `http://127.0.0.1:3080`）
- 文件拖拽 / 剪贴板图片粘贴放行（iframe 场景已验证）
- 启动无闪烁（窗口隐藏创建，内容就绪后显示）

### Fixed

- 标题栏主题切换"慢一拍"（3s 轮询 → 文件监听事件驱动，实测 ~100ms 同步）
- 无边框窗口四角圆角与 DWM 边框线（DONOTROUND + 透明边框 + WS_POPUP 样式守卫）

### Security

- 壳页与远程 GUI 隔离（iframe 无 Tauri 权限）；capabilities 最小化窗口控制权限
