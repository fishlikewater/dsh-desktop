# 测试说明（Testing）

## 测试分层

| 层 | 工具 | 位置 | 何时运行 |
|---|---|---|---|
| 静态检查 | clippy + 前端脚本 + CSP hash | `npm run check` | 每次改动 |
| Rust 单元测试 | cargo test | `src-tauri/src/*::tests` | CI / 本地 |
| 端到端冒烟 | 自定义 CDP 脚本（Node `ws`） | `scripts/smoke/` | 本地 `npm run smoke` |
| 性能基线 | PowerShell 采样 | `scripts/bench/baseline.mjs` | 手工 |

## 静态检查

```bash
npm run check
```

包含：`cargo fmt --check`、`cargo clippy -D warnings`（全 targets）、前端脚本语法与 CSP sha256 一致性检查（`check:frontend`）、版本一致性（`check:version`）。

## 单元测试（Rust）

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

覆盖范围（基线统计）：config.rs 13 例、theme.rs 9 例、window.rs 5 例、service.rs 14 例（12 平台无关纯逻辑 + 2 macOS 端到端）。

> **windows-gnu 已知限制**：此前在 `x86_64-pc-windows-gnu` 工具链下，测试二进制加载失败（`0xc0000139`），曾归因于引用 `std::process::Child` 类型的 static、fs/env/PathBuf API 或 async tauri command；实际根因是测试 exe 缺 Common Controls v6 manifest（已于 CI 修复，见 `.github/workflows/ci.yml` 单元测试步骤）。当前约束：
> - 服务进程句柄不存 static（用 `OnceLock<Mutex<Option<u32>>>` 存 pid），保持架构简洁；
> - 新增 tauri command 保持同步化（`tauri::async_runtime::block_on`），如 `update_check`/`update_install` 的先例；
> - 服务纯逻辑（路径探测 `find_dsh_in_with`、DSH_CLI 解析 `resolve_dsh_cli_with`、命令构造 `dsh_command_parts`）与 FFI 分离，测试全平台运行（CI windows-gnu 亦覆盖）。

## 端到端冒烟（CDP）

```bash
npm run smoke        # 真实模式：DSH 服务在线
npm run smoke:mock   # mock 模式：脱离真实 DSH 服务（CI 也是此模式）
```

### 前置条件

1. **debug 构建**存在：`src-tauri/target/debug/dsh-desktop`（`--remote-debugging-port=9226` 仅 debug 构建开启）。先运行 `npm run dev` 或 `cargo build`。
2. **应用已启动**（脚本不自动拉起）。
3. 真实模式：**DSH 服务在线**（默认 `http://127.0.0.1:3080`，可用 `DSH_URL` 环境变量覆盖）；
   mock 模式：无需 DSH 服务（`run-all --mock` 自动拉起 `scripts/smoke/mock-server.mjs`，默认端口 3099，DSH_URL 自动指向 mock）。

### 场景

| 场景 | 脚本 | 验证点 |
|---|---|---|
| 窗口居中 | `window-center.mjs` | 首启居中（事件驱动） |
| 主题即时同步 | `theme-sync.mjs` | `theme-file-changed` 事件驱动跟随 |
| 隔离边界 | `isolation.mjs` | iframe 内 IPC 拒绝、壳页允许 |
| 窗口控制权限 | `window-ctrl.mjs` | 伪最大化/最小化/show 的 capabilities |

> **场景顺序**：`window-ctrl` 会最小化窗口（前端无 unminimize 权限，最小化后尺寸无法经 setSize 恢复），必须放最后。

### mock 模式说明

- mock server（`scripts/smoke/mock-server.mjs`）：极简 HTTP 服务（`/` 返回 HTML 作为 iframe 目标、`/health`），只模拟"可达 + 可加载页面"，不模拟任何 DSH API 语义。
- 场景断言语义与真实模式一致：窗口几何/权限走 CDP + capabilities；主题同步监听 `<DSH_HOME>/settings.yaml`（run-all 在 mock 模式自动准备临时 DSH_HOME 与 light 基线，应用需以同一 DSH_HOME 启动）；隔离边界只依赖 iframe 页面存在 + Tauri ACL 按 origin 拒绝（mock 与真实 DSH 同为 127.0.0.1，行为一致）。
- 应用启动方式（CI 同款）：`$env:DSH_URL="http://127.0.0.1:3099"; $env:DSH_HOME="<临时目录>"; Start-Process src-tauri/target/debug/dsh-desktop.exe`，再运行 `node scripts/smoke/run-all.mjs --mock`（CDP 就绪最长等待 60s）。

### 运行机制

- `lib.mjs` 通过 CDP 端口 `http://127.0.0.1:9226/json` 发现 target，`evalShell` 在壳页上下文求值。
- 使用 `ws` 包而非 Node 全局 WebSocket：Node 24 在 Windows 上对全局 WebSocket 关闭存在 libuv 断言崩溃。
- 进程退出走自然退出（`process.exitCode`）+ 1.5s 兜底，规避 libuv 断言。

## 性能基线

```bash
# 需 release 构建（默认 30s、1s 采样间隔）
node scripts/bench/baseline.mjs [exe路径] [采样秒数] [间隔ms]
```

输出进程工作集与 CPU 时间采样，结果供版本回归对比。

## CI 与冒烟的关系

- CI（`.github/workflows/ci.yml`）覆盖静态检查 + 单测 + 构建 + 打包 + **冒烟（mock 模式）**：Debug 构建后以临时 DSH_HOME + mock server 启动应用并运行全部场景。
- 真实模式 `npm run smoke` 仍需本机 DSH 服务与 `~/.dsh` 配置，用于本地完整验证。

## 托盘双态图标（手工冒烟记录）

托盘图标切换无法在 CI 无 UI runner 下断言（`set_tray_state` 命令的可达性由冒烟场景覆盖：在线分支调用不报错）。本机手工验证步骤：

1. 启动应用（DSH 服务运行中）→ 托盘图标为**绿圆**（`tray-on.png`）；
2. 停止 DSH 服务 → 10s 内（在线轮询周期）托盘图标变为**灰圆**（`tray-off.png`）；
3. 重新启动服务 → 图标恢复绿圆；
4. 离线时左键单击托盘 → 主窗口唤起（与在线行为一致，即"优先唤起"语义）。

图标资源为 32x32 白描边圆点（深浅系统主题均可见），路径 `src-tauri/icons/tray-{on,off}.png`（编译期内嵌 `include_bytes!`）。

## Linux 支持评估结论（Task 20）

**Linux 服务自管理：不承诺完整支持（仅评估）。** 依据：

1. **启动语义冲突**：Windows/macOS 由壳层 spawn 持有 pid；Linux 桌面惯例是 systemd user service（`systemctl --user start dsh`）——SERVICE_PID 模型与 systemd 托管冲突；
2. **自动重启不适用**：`Restart=on-failure`（systemd）与壳层 Task 9 节流机制重复冲突；
3. **探测缺失**：`find_dsh`/`find_dsh_macos` 仅 windows/macos cfg，Linux 无 PATH 探测实现；
4. **连接语义可用**：壳层 tick 检测、覆盖层、地址切换、托盘、多会话等全部功能在 Linux 正常（连接 127.0.0.1:3080 即可）；仅"壳层拉起/停止"为 stub（可读错误引导手动启动或 systemd user service）。

**结论**：Linux 上以 systemd user unit 或手动启动为准；壳层服务自管理（拉起/停止）不实现，stub 错误文案已指导。

## 评估类结论汇总

- **dsh sidecar（Task 15）**：放弃捆绑（npm shim 无独立可执行产物 + node 运行时无来源）→ 纯安装引导；
- **dsh:// 深链接（Task 19）**：不实施（无依赖无 GUI 路由，仅唤起价值被快捷键覆盖）；
- **Linux 服务自管理（Task 20）**：不实施（systemd 语义冲突 + 连接语义已满足）。

## dsh:// 深链接评估结论（Task 19）

**不实施 dsh:// 协议注册。** 依据：

1. 项目无 `tauri-plugin-deep-link` 依赖、无平台协议注册配置（`Cargo.toml`/`tauri.conf.json` 均无）——实现需引入插件或手工平台注册（macOS `Info.plist` CFBundleURLTypes / Windows 注册表 + 图标），成本大于收益；
2. DSH GUI（DeepSeek Harness Web GUI，外部项目）无 `dsh://` 路由处理——协议唤起无法路由到 GUI 具体页面，仅剩"唤起主窗口"价值（全局快捷键已覆盖）；
3. 不为不可用的路由虚构协议支持。

## dsh sidecar 可行性结论（Task 15）

**结论：放弃 sidecar 捆绑（方案 A），改纯安装引导。** 依据：

1. `dsh` 是 npm 全局 shim（`dsh.cmd`/`dsh.ps1`，无 `dsh.exe`，见 `service.rs` 头注释）——`bundle.externalBin` 要求自包含可执行文件，shim 只是 node 脚本，无独立可执行产物；
2. node 运行时不随壳分发：捆绑 node.exe（约 90MB）违反项目约束（不把 node 运行时塞进 resources 冒充 sidecar），且 `dsh` 的 node_modules 依赖不在壳项目交付边界——捆绑等于重打包 DSH 本体；
3. `tauri.conf.json` 现有 `resources` 仅做普通文件拷贝（WebView2Loader.dll），不走 sidecar 通道——捆绑 shim 无意义（目标机仍需 node）；
4. 现有探测已覆盖 `DSH_CLI` 环境变量 + PATH（`.cmd > .exe > .ps1`），用户预装是最低摩擦路径。

**安装引导**：覆盖层启动失败且错误含「未找到 dsh」时，前端展示安装指引（npm 全局安装命令 + `DSH_CLI` 覆盖提示），用户可自助安装后重试。

## 多会话窗口（冒烟局限记录）

Windows WebView2 创建第二个 WebView 后，主窗口的 CDP `Runtime.evaluate` 会失效（多 WebView 共享调试 session 的平台限制），因此：

- **multi-window 场景排在 run-all 最后**：不殃及其他场景；断言走 Rust 命令（open/list/close_session_window）+ CDP 列表计数（纯 HTTP，不依赖 evaluate）；
- 第二窗口存在性的核心证据 = CDP page target 计数 ≥ 2 + `open_session_window` 返回 label；
- 隐藏会话窗口的 CDP evaluate 本身也不可靠（不响应），逐窗口注入断言不可用；
- 手工冒烟步骤（本机 Windows）：
  1. 托盘菜单「新建会话…」→ 第二个窗口出现且独立检测服务（可分别切地址）；
  2. 托盘菜单出现「会话：DSH 会话 - …」项，点击唤起对应窗口；
  3. 关闭会话窗口 → 菜单项消失，主窗口不受影响；
  4. 主窗口关闭仍隐藏到托盘（行为不变）。