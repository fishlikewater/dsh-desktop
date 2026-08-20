# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

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
