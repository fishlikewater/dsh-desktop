# 测试说明

本项目的测试体系分三层：单元测试（Rust）、静态检查（脚本）、端到端冒烟（CDP）。

## 1. 单元测试（Rust）

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

覆盖：

- `config`：ShellConfig 解析（缺失/损坏/未知字段）、DSH_URL/DSH_HOME 优先级（9 例）
- `theme`：`parse_theme_preference`（正常/引号/缺失段/未知值/损坏/空白/多段，9 例）
- `window`：`initial_window_size`（大桌面/小桌面/中等/异常回退/设计不变量，5 例）

### ⚠️ windows-gnu 测试崩溃（0xc0000139）与规避规则

本机工具链为 `stable-x86_64-pc-windows-gnu`（rustc 1.94~1.95 及 beta 均可复现）。
在该工具链下，**tauri 项目的 test 二进制**存在链接产物缺陷，进程加载即退出
（0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND）。已二分确认的触发形态（任一即崩）：

1. **lib 单测中出现 `std::process::Child` 类型的 static**
   （如 `static X: Mutex<Option<Child>>`，换成 `u32` pid 即恢复）；
2. **tauri async command**（`#[tauri::command] pub async fn ...`，
   改用同步 command + `tauri::async_runtime::block_on` 即恢复）；
3. **集成测试（tests/）引用 lib crate**（任意符号，必崩）；
4. 部分 fs/env/PathBuf API 的测试代码（temp_dir/fs 写入/PathBuf 构造等）。

**项目内规避约定**：

- 测试只用"安全"内容：纯逻辑、字符串、数学断言（现有 23 例均合规）；
- 进程句柄一律存 pid（u32），禁止 `Child` 类型 static；
- command 一律同步，异步 API 用 `block_on` 包装；
- 不写引用 lib crate 的集成测试（tests/ 目录不使用）；
- service 模块逻辑无自动化单测，以手工冒烟验证（T5.1 假 dsh 拉起/停止实测）。

新增测试时若 `cargo test --lib` 突然 0xc0000139，先按上述四条回查新增内容。

## 2. 静态检查

```bash
npm run check
```

- `check:rust`：cargo clippy `-D warnings` 全量零警告
- `check:frontend`：`scripts/check-frontend.mjs`
  - 内联 `<script>` 语法检查（node --check）
  - JS 引用的 id 必须存在于 HTML
  - **CSP script-src sha256 与内联脚本一致性**（防改 JS 忘更新 hash 导致 CSP 拦截）

## 3. 端到端冒烟（CDP）

> 依赖 debug 构建的 CDP 端口（`additional_browser_args="--remote-debugging-port=9226"`，仅 debug_assertions）。

### 前置条件

1. DSH 服务在线：`dsh --profile web`（默认 `http://127.0.0.1:3080`）
2. debug 构建：`cargo build`（src-tauri/ 下）
3. 应用已启动（可手动或由 CI 拉起）：`src-tauri/target/debug/dsh-desktop.exe`

### 运行

```bash
# 一键执行全部场景（含前置检查）
npm run smoke

# 单个场景
node scripts/smoke/theme-sync.mjs
node scripts/smoke/window-ctrl.mjs
node scripts/smoke/isolation.mjs
node scripts/smoke/window-center.mjs
```

### 场景清单

| 脚本 | 验证内容 |
|---|---|
| `theme-sync.mjs` | 修改 settings.yaml 主题偏好 → 标题栏 ≤2s 内跟随（事件驱动，实测 ~100ms）。**注意：会临时修改 `~/.dsh/settings.yaml`（首次运行时备份 `.bak-smoke`），结束后恢复 light 基线** |
| `window-ctrl.mjs` | capabilities 权限面：伪最大化（贴齐工作区）、最小化、show |
| `isolation.mjs` | 安全边界：壳页 `__TAURI__` 可用；iframe 内对象存在但 IPC 全部被拒（URL 粒度 capabilities） |
| `window-center.mjs` | 窗口事件驱动居中（位置稳定后采样，容差 60px） |

### 前置检查失败排查

- `debug 构建不存在`：先 `cargo build`（src-tauri/ 下）
- `DSH 服务不可达`：启动 `dsh --profile web`，或设置 `DSH_URL` 环境变量指向实际地址
- `CDP 端口不可达`：确认应用已启动且为 debug 构建（release 构建不带 CDP）

## 4. CI 接入（T3.3）

CI 流水线（.github/workflows/ci.yml，push main / PR / 手动触发）按顺序执行：
fmt 检查 → clippy → 单元测试 → 前端静态检查（含 CSP hash）→ debug 构建 →
release 构建 + NSIS 打包（上传安装包 artifact）。

冒烟不在 CI 中执行：CDP 端到端依赖本机 DSH 服务（dsh CLI）与 `~/.dsh` 配置，
CI runner 无 dsh 安装，由本地 `npm run smoke` 承担（见上文场景清单）。

发布（v* tag）走独立的 .github/workflows/release.yml：并行构建
macOS（Apple Silicon）与 Windows 安装包并发布 Release 草稿，见
docs/release-process.md。
