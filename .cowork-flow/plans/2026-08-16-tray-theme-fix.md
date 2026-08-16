# 移除托盘定时任务 + 标题栏主题跟随 实施计划

> Execution rule: this plan exists to prevent implementation drift.

| Field | Content |
|----|------|
| **Goal** | ①托盘菜单移除「定时任务」项；②标题栏/覆盖层跟随 DSH 外观主题（settings.yaml ui-theme.preference，3s 轮询，system 模式跟随系统）。 |
| **Strategy** | Serial: Rust → 前端 → 验证打包 |
| **Success Criteria** | 托盘无任务项；preference 切换实测标题栏变色；回归全绿。 |

## Tasks

### Task 1: Rust（托盘项移除 + theme_preference command）
- 删 `tasks_item` 定义/菜单挂载/on_menu_event "tasks" 分支。
- 新增 `#[tauri::command] fn theme_preference() -> String`：读 `$DSH_HOME/settings.yaml`（无 DSH_HOME 用 USERPROFILE），按段解析 ui-theme.preference（light|dark|system，默认 light）；注册进 invoke_handler。
- 验证: `cargo build` 零警告。

### Task 2: 前端（深色变量 + system media + 轮询）
- CSS：`body.theme-dark { ... }` 深色变量覆盖 + `@media (prefers-color-scheme: dark) { body:not(.theme-light):not(.theme-dark) { ... } }`（system 无类跟随系统）。
- JS：`invoke("theme_preference")` 启动即应用 + 3s 轮询（变化才重设 body 类）。
- 验证: node --check、id 一致性。

### Task 3: 验证打包
- preference=dark 实测（改 settings.yaml → 截图对比标题栏）→ 恢复 light。
- cargo test --release、npm run build。

## Integrated Verification
1. cargo build/test → exit 0。
2. 截图实测 light/dark 标题栏差异（已通过：浅色→深色、文字深→浅）。
3. npm run build → 安装包重新生成。
