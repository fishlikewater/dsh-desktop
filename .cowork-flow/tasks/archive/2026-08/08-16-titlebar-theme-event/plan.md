# 标题栏主题同步改事件驱动 实施计划

> Execution rule: this plan exists to prevent implementation drift.

| Field | Content |
|----|------|
| **Goal** | 壳页标题栏主题由固定 3s 轮询改为事件驱动：Rust 侧 notify 监听 `~/.dsh/settings.yaml`，变化时 emit Tauri 事件，前端收到后立即刷新；轮询降为低频兜底（30s）。 |
| **Strategy** | Serial: Rust → 前端 → 验证 |
| **Success Criteria** | 修改 settings.yaml 的 ui-theme.preference 后标题栏 1s 内变色；cargo build 零警告；node --check 通过。 |

## 背景结论（已调研）

- 慢一拍根因：壳页 `setInterval(syncTheme, 3000)` 固定轮询，与 GUI 内即时切换存在 0~3s 随机相位差。
- DSH 写盘链路（setTheme → RPC mutate → settings-file）无防抖、毫秒级，写盘不是瓶颈。
- `writeFileAtomic`（dsh-atomic-write）为 **临时文件 + rename 覆盖**：notify 必须 watch **父目录**并按文件名过滤，直接 watch 文件会在 rename 替换 inode 后丢事件。

## Tasks

### Task 1: Rust（notify 监听 + 事件 emit）
- `Cargo.toml` 加 `notify` 依赖。
- `lib.rs`：启动时解析 `$DSH_HOME/settings.yaml`（与 theme_preference 同一路径逻辑）的父目录，`RecommendedWatcher` 监听；事件中文件名（或 rename 目标）等于 `settings.yaml` 时，100ms 防抖合并后 `app.emit("theme-file-changed", ())`。
- 验证: `cargo build` 零警告（debug 即可）。

### Task 2: 前端（事件监听即时刷新 + 低频兜底）
- `frontend-dist/index.html`：`window.__TAURI__.event.listen("theme-file-changed", ...)` → 立即 `syncTheme()`（lastTheme 去重已有）。
- 轮询 `setInterval(syncTheme, 3000)` 改为 `30000` 兜底（防止文件监听失效后永久失步）。
- 验证: `node --check`（提取内联 script 语法检查）、id/变量名一致性。

### Task 3: 验证
- 改 `~/.dsh/settings.yaml` ui-theme.preference light↔dark，实测标题栏即时变色（截图对比）。
- 恢复原值；确认无回归。

## Integrated Verification
1. cargo build → exit 0。
2. 修改 settings.yaml → 标题栏 1s 内变色（事件驱动生效）。
3. node --check 通过。
