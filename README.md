# DSH Desktop

把 [DeepSeek Harness](https://github.com/deepseek-ai/dsh) 的 Web GUI（默认 `http://127.0.0.1:3080`）包装为 Windows 桌面应用的 Tauri 壳层。

## 功能

- **桌面窗口**：加载运行中的 DSH Web GUI（需 DSH 服务在线）
- **原生窗口质感**：无边框窗口 + 自定义标题栏（拖拽区、双击最大化、最小化/最大化/关闭按钮）+ Windows 11 Mica 毛玻璃（Win10 回退 Acrylic）
- **系统托盘**：左键单击显示/隐藏主窗口；菜单含「显示主窗口」「开机自启」「发送测试通知」「退出」
- **开机自启开关**：托盘菜单勾选项，切换 `HKCU\...\Run` 注册表项（tauri-plugin-autostart）
- **全局快捷键**：`Ctrl+Shift+D` 唤起主窗口
- **关闭最小化**：点窗口关闭按钮时隐藏到托盘，应用常驻后台
- **系统通知**：通过 `tauri-plugin-notification` 提供（`notify` command + 托盘测试项）
- **服务不可用提示页**：DSH 未启动时显示友好覆盖层并轮询检测，服务恢复后自动加载 GUI
- **定时任务**：已迁移为 DSH 插件 **@fishlikewater/dsh-tasks**（源码在 `E:\Projects\IdeaProjects\person\dash-plugin`，symlink 安装到 `~/.dsh/profiles/web/node_modules/`，profile patch 声明）。壳层不再包含任何任务逻辑与入口：入口在 DSH GUI 侧边栏底部（闹钟图标，`sidebar.footer.action` slot）。插件能力：每天/工作日/每周/间隔四种计划；触发动作——系统通知（Web 通知 + 面板内未读提示）或执行 DSH 任务（进程内 agent 会话，默认模型、可指定工作区，会话出现在 GUI 历史）；任务持久化于 `~/.dsh/settings.yaml`（`dsh-tasks` 命名空间），1s tick 调度（±30s 触发窗口）；HTTP API `/_dsh/dsh-tasks/*`。旧壳层数据 `%APPDATA%\com.dsh.desktop\tasks.json` 不再读取（如需迁移可手动重建）
- **标题栏跟随主题**：自定义标题栏与覆盖层配色跟随 DSH GUI 外观设置（`~/.dsh/settings.yaml` 的 `ui-theme.preference`：light/dark/system；Rust 侧 notify 监听该文件，变化时推送 `theme-file-changed` 事件即时同步，30s 轮询仅作兜底，system 模式跟随系统 prefers-color-scheme）
- **可配置服务地址**：环境变量 `DSH_URL` 覆盖默认 `http://127.0.0.1:3080`（不同端口或测试用）
- **文件拖拽 / 剪贴板图片粘贴**：`drag_and_drop(false)` 放行拖拽事件给页面，由 DSH GUI 原生处理（已通过 CDP 拖放模拟验证，含 iframe 场景）
- **启动无闪烁**：窗口以 `visible(false)` + 显式 `hide()` 创建，壳页 iframe 加载完成后才显示（任务栏图标只出现一次，无暗色背景闪烁）

## 开发

```bash
# 安装依赖（首次）
npm install

# 开发模式：编译并弹出桌面窗口
npm run dev

# 生成安装包（NSIS）
npm run build
```

要求：

- Rust（MSVC 工具链为官方支持；本项目亦验证了 windows-gnu 工具链）
- Node.js ≥ 18
- Windows 10/11（WebView2 运行时，一般系统自带）
- 运行中的 DSH 服务（`dsh` 命令启动，监听 3080 端口）

## 架构

```
dsh-desktop/
├── package.json              # npm 侧：Tauri CLI
├── frontend-dist/
│   └── index.html            # 本地壳页：自定义标题栏 + iframe 嵌入 DSH GUI + 服务检测覆盖层
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
- Codex 功能对照评估见 [docs/codex-features-evaluation.md](docs/codex-features-evaluation.md)

## 路线图

- [x] 壳层 MVP：窗口 + 托盘 + 全局快捷键 + 关闭最小化
- [x] 文件拖拽 / 剪贴板图片粘贴（验证 drop 事件携带文件到达页面）
- [x] 系统通知（notify command + 托盘测试项）
- [x] 开机自启开关（托盘勾选项，注册表验证通过）
- [x] DSH 服务未启动时的友好提示页（`DSH_URL` 可配置，恢复后自动加载）
- [x] 原生窗口质感（无边框 + 自定义标题栏 + Mica/Acrylic 毛玻璃 + 伪最大化）
- [x] 无边框线/无缝隙（DWM 直角 + WS_POPUP 样式 + 关闭阴影偏移 + 自绘 resize）
- [x] 单实例（防快捷键冲突）+ 无控制台窗口
- [ ] 安装包签名
- [ ] 多会话窗口 / 独立小窗模式
