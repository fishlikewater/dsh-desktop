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