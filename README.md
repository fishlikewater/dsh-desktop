# DSH Desktop

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

把 [DeepSeek Harness](https://github.com/deepseek-ai/dsh) 的 Web GUI（默认 `http://127.0.0.1:3080`）包装为 Windows 桌面应用的 Tauri 壳层。

> 项目处于活跃开发阶段（v0.1.x）。生产级改造计划见 docs/production-hardening-plan.md，任务分解见 docs/development-plan.md。

## 功能

- **桌面窗口**：加载运行中的 DSH Web GUI（需 DSH 服务在线）
- **原生窗口质感**：无边框窗口 + 自定义标题栏（拖拽区、双击最大化、最小化/最大化/关闭按钮）+ Windows 11 Mica 毛玻璃（Win10 回退 Acrylic）
- **系统托盘**：左键单击显示/隐藏主窗口；菜单含「显示主窗口」「开机自启」「启动/停止 DSH 服务」「设置…」「发送测试通知」「退出」
- **服务自管理（方案 B 纯拉起）**：服务未运行时覆盖层一键「启动 DSH 服务」或托盘菜单拉起系统 dsh（`--profile web`，不捆绑 sidecar）；托盘「停止」仅结束**本应用拉起的**服务进程树，不误杀用户自启的服务；服务在线时幂等返回
- **错误恢复**：服务中途崩溃/恢复自动切换（在线 10s / 离线 2s 自适应轮询）；iframe 加载 15s 无响应提示重试
- **设置页**：标题栏齿轮或托盘「设置…」打开——服务地址（只读，启动时读取）、开机自启开关、检查更新（tauri-updater，无更新/失败不崩溃）、打开日志目录
- **开机自启开关**：托盘菜单勾选项，切换 `HKCU\...\Run` 注册表项（tauri-plugin-autostart）
- **全局快捷键**：`Ctrl+Shift+D` 唤起主窗口
- **关闭最小化**：点窗口关闭按钮时隐藏到托盘，应用常驻后台
- **系统通知**：通过 `tauri-plugin-notification` 提供（`notify` command + 托盘测试项）
- **服务不可用提示页**：DSH 未启动时显示友好覆盖层并轮询检测，服务恢复后自动加载 GUI
- **标题栏跟随主题**：自定义标题栏与覆盖层配色跟随 DSH GUI 外观设置（`~/.dsh/settings.yaml` 的 `ui-theme.preference`：light/dark/system；Rust 侧 notify 监听该文件，变化时推送 `theme-file-changed` 事件即时同步，30s 轮询仅作兜底，system 模式跟随系统 prefers-color-scheme）
- **可配置服务地址**：环境变量 `DSH_URL` 覆盖默认 `http://127.0.0.1:3080`（不同端口或测试用）
- **文件拖拽 / 剪贴板图片粘贴**：`drag_and_drop(false)` 放行拖拽事件给页面，由 DSH GUI 原生处理（已通过 CDP 拖放模拟验证，含 iframe 场景）
- **启动无闪烁**：窗口以 `visible(false)` + 显式 `hide()` 创建，壳页 iframe 加载完成后才显示（任务栏图标只出现一次，无暗色背景闪烁）

## 安装

### 前置要求

- **Windows 10/11**（WebView2 运行时，一般系统自带）
- **Node.js ≥ 18** 与 **Rust 工具链**（构建需要；运行时仅需已安装的 DSH CLI）
- **运行中的 DSH 服务**：`dsh` 命令启动（默认监听 3080 端口）；安装与使用见 [DeepSeek Harness](https://github.com/deepseek-ai/dsh)

### 从安装包安装

从发布页下载 NSIS 安装包（`.exe`），运行安装后从开始菜单/桌面快捷方式启动。

> 说明：当前发布版**未做代码签名**，Windows SmartScreen 可能提示"未知发布者"——选择"更多信息 → 仍要运行"即可；请从官方发布渠道获取安装包并核对 SHA256 校验和（见发布说明）。

## 开发

```bash
# 安装依赖（首次）
npm install

# 开发模式：编译并弹出桌面窗口
npm run dev

# 代码检查（clippy + 前端脚本语法 + CSP hash 一致性）
npm run check

# 单元测试（Rust）
cargo test --manifest-path src-tauri/Cargo.toml --lib

# 端到端冒烟（CDP，需 debug 构建 + DSH 服务在线；步骤见本地 docs/testing.md）
npm run smoke

# 生成安装包（NSIS）
npm run build
```

要求：

- Rust（windows-gnu 或 MSVC 工具链均可；`stable-x86_64-pc-windows-gnu` 已验证）
- Node.js ≥ 18
- Windows 10/11（WebView2 运行时，一般系统自带）
- 运行中的 DSH 服务（`dsh` 命令启动，监听 3080 端口）

## 架构

```
dsh-desktop/
├── package.json              # npm 侧：Tauri CLI
├── frontend-dist/
│   └── index.html            # 本地壳页：自定义标题栏 + iframe 嵌入 DSH GUI + 服务检测覆盖层
├── scripts/                  # 开发/验证脚本（前端检查、CDP 冒烟）
└── src-tauri/
    ├── tauri.conf.json       # 窗口/托盘/打包配置（withGlobalTauri）
    ├── Cargo.toml            # Rust 依赖
    ├── capabilities/
    │   └── default.json      # 壳页窗口控制权限（capabilities）
    └── src/
        ├── main.rs           # 入口
        └── lib.rs            # 托盘 / 全局快捷键 / Mica 毛玻璃 / 窗口生命周期 / settings.yaml 监听（主题即时同步）
```

壳层架构：**本地壳页（iframe 方案）**。主窗口加载本地 `frontend-dist/index.html`，
壳页提供自定义标题栏（拖拽、窗口控制、伪最大化）并以 iframe 嵌入远程 DSH GUI。
本地页面有 Tauri IPC 权限（`withGlobalTauri`），远程 GUI 无权限（安全隔离）。

关键设计：

- `drag_and_drop(false)`：**必须关闭** Tauri 窗口级拖拽拦截，否则 iframe 内 DSH GUI 的
  `document.addEventListener("drop")` 收不到文件（已用 CDP 验证 iframe 场景：
  drop 事件的 `dataTransfer.types` 含 `Files`、`files` 携带真实路径）
- **伪最大化**：无边框窗口的系统最大化会被 DWM 扩展到 -8px（标题栏被裁），
  因此 `maximizable(false)` + 壳页手动贴齐工作区（Rust `work_area` command 提供尺寸，
  DWM 隐藏边框用实测补偿）
- **无边框线/无缝隙**（"四方小缝隙"的三层根因与解法）：
  1. Windows 11 默认圆角 → DWM `DWMWA_WINDOW_CORNER_PREFERENCE = DONOTROUND`（直角）
  2. `WS_THICKFRAME` 触发的 1px 激活边框线 → 样式改为 `WS_POPUP`（去 WS_CAPTION/WS_SIZEBOX）
     + **样式守卫**：子类化窗口过程拦截 `WM_STYLECHANGED`，tao 每次重置样式时**立即**
     改回 WS_POPUP（事件驱动，零延迟零开销，无轮询线程）
  3. tao 无边框阴影的 8px 客户区偏移 → builder `.shadow(false)`（内容铺满整个窗口）
  边缘调整大小改由壳页自绘 8 方向热区 + JS 拖拽实现（系统 resize 需 WS_THICKFRAME，
  已移除）
- **Mica 毛玻璃**：`transparent(true)` + `window-vibrancy` 的 `apply_mica`（Win11，
  跟随系统主题），Win10 回退 `apply_acrylic`；窗口控制权限在 `capabilities/default.json` 中声明
- **单实例**：`tauri-plugin-single-instance`，重复启动时聚焦已有窗口（否则新旧实例
  争夺全局快捷键 Ctrl+Shift+D，注册失败）
- **无控制台窗口**：`#![windows_subsystem = "windows"]`（debug/release 都不弹终端）
- **窗口标题固定为 "DSH Desktop"**，不跟随页面 `document.title`
- **Codex 风格**：壳层配色对齐 Codex 桌面（深色背景 #181818/表面 #2D2D2B/珊瑚橙强调 #D9645A，
  浅色背景 #FFFFFF/侧边栏 #F2F3F3）；标题栏为**不透明纯色**（修复 Mica/backdrop 导致的顶部
  透明-暗色闪烁）；应用图标为 DSH 鲸鱼 logo（浅灰圆角底 + 黑色鲸鱼，`src-tauri/icons/app-icon.svg`）
- Codex 功能对照评估见 docs/codex-features-evaluation.md

## 持续集成（CI）

`.github/workflows/ci.yml` 定义全链路流水线：fmt 检查 → clippy → 单测 → 前端检查 →
debug 构建 → release 构建 + NSIS 打包（上传安装包 artifact）。

> **当前状态**：仓库尚无 GitHub 远程，workflow 未实际触发过。推送到 GitHub 后
> 即自动启用；本地等价验证命令序列见 docs/testing.md。
> 端到端冒烟依赖本机 DSH 服务与 `~/.dsh` 配置，不作为 CI 步骤（本地 `npm run smoke` 承担）。

## 故障排查

| 现象 | 处理 |
|---|---|
| 窗口显示"DSH 服务未运行" | 确认 `dsh` 服务已启动（`dsh --profile web`）；或设置 `DSH_URL` 指向实际地址后重启应用 |
| 标题栏主题不跟随 | 检查 `~/.dsh/settings.yaml` 的 `ui-theme.preference` 值（light/dark/system）；应用日志（见下文）确认 watcher 状态 |
| 全局快捷键无效 | 快捷键可能被其他应用占用；查看启动日志中的注册失败告警 |
| 安装包被 SmartScreen 拦截 | 未签名发布版的已知限制（SHA256 核验指引见 SECURITY.md） |

日志位置：`%LOCALAPPDATA%\com.dsh.desktop\logs\dsh-desktop.log`（1MB 大小轮转，保留最近 5 份；设置页「打开日志目录」可直达）。

## 安全

- 壳页与远程 GUI 隔离：iframe 无 Tauri 权限
- capabilities 最小权限声明
- **CSP**：壳页启用内容安全策略（内联脚本 sha256 白名单；iframe/连接仅允许 `http://127.0.0.1:*` 与 `http://localhost:*`）。自定义 `DSH_URL` 指向其他主机时，需同步扩展 `src-tauri/tauri.conf.json` 的 `security.csp`（`frame-src`/`connect-src`）
- 安全说明与漏洞报告见 SECURITY.md

## 路线图

- [x] 壳层 MVP：窗口 + 托盘 + 全局快捷键 + 关闭最小化
- [x] 文件拖拽 / 剪贴板图片粘贴（验证 drop 事件携带文件到达页面）
- [x] 系统通知（notify command + 托盘测试项）
- [x] 开机自启开关（托盘勾选项，注册表验证通过）
- [x] DSH 服务未启动时的友好提示页（`DSH_URL` 可配置，恢复后自动加载）
- [x] 原生窗口质感（无边框 + 自定义标题栏 + Mica/Acrylic 毛玻璃 + 伪最大化）
- [x] 无边框线/无缝隙（DWM 直角 + WS_POPUP 样式 + 关闭阴影偏移 + 自绘 resize）
- [x] 单实例（防快捷键冲突）+ 无控制台窗口
- [x] 生产级改造（见 docs/development-plan.md）：规范门禁、测试与 CI、安全加固、自动更新、服务自管理、设置页、错误恢复、性能基线、最终验收（T0.1~T6.2 全部完成，见 docs/final-acceptance.md）
- [ ] 安装包签名（无证书，见 SECURITY.md 的 SHA256 核验指引）
- [ ] 多会话窗口 / 独立小窗模式

## 许可

[MIT](LICENSE) © 2026 zhangxiang
