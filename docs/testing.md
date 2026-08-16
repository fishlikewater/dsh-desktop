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

CI 流水线将按顺序执行：fmt 检查 → clippy → 单元测试 → 静态检查 → debug 构建 →
启动应用 → 冒烟 → release 打包。冒烟步骤依赖 DSH 服务：CI 中使用 `dsh --profile web`
作为服务进程（或标记为可选步骤）。
