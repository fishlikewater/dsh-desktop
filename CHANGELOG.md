# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### 工程化（生产级改造）

- 初始化 git 仓库与提交基线（T0.1）
- 引入 MIT 开源许可证、CHANGELOG、SECURITY 文档（T0.2）
- 后续按阶段补充：代码规范门禁（T0.3）、模块化重构（T1.1）、日志系统（T1.2）、
  启动时序事件化（T1.3）、配置层（T1.4）、CSP 与权限收敛（T2.x）、
  测试与 CI（T3.x）、自动更新与发布（T4.x）、DSH 服务托管（T5.x）、
  体验兜底（T6.x）

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
