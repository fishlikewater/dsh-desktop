# 侧边栏适配 + 工作区选择 + 表单增强 实施计划

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 修复用户反馈三问题：①DSH GUI 侧边栏收起（56px rail）时任务面板左侧留白——面板左侧偏移支持「展开 280px / 收起 56px」两档切换并持久化；②创建表单明确"任务标题"字段、指令文本框加大；③新增「工作区」选择（原生文件夹选择器 + 手输路径），run 任务在所选工作区执行。 |
| **Task Type** | Normal: 后端 Task 模型加字段 + 新插件依赖 + 前端面板适配 |
| **Strategy** | Serial: 后端契约 → 前端 UI → 验证打包 |
| **Success Criteria** | AC-001 ~ AC-007 |
| **Final Verification** | `cargo test --release` -> exit 0；`cargo build` -> exit 0；`npm run build` -> exit 0（重新打包） |
| **Primary Risk** | Medium: 新增 tauri-plugin-dialog 依赖需 cargo 联网下载；若离线失败则回退纯文本路径输入 |

## Goal

1. 面板侧边栏适配：`#tasks-page` 左侧偏移与 `#tasks-shield` 宽度支持「wide（280px，侧边栏展开）/ rail（56px，侧边栏收起）」两档，面板头部切换控件 + localStorage 持久化；打开面板时套用已存档位。
2. 表单增强（参考 Codex 创建体验）：
   - 「任务标题」字段加 field-label 显式标注
   - run 模式指令 textarea 加大（min-height 140px）；notify 模式提醒文案支持多行
   - run 模式新增「工作区」字段：文本输入 + 「浏览…」按钮（tauri-plugin-dialog 原生文件夹选择器），提示"留空使用 DSH 主目录"
3. 后端：Task 新增 `workspace: Option<String>`（serde default 兼容旧数据）；`run_headless_task` 的 current_dir 优先使用任务工作区，未设置回退 DSH_HOME；新增 `pick_workspace` command（dialog 插件）。
4. 详情抽屉：run 任务显示「工作区」字段。

## Acceptance Criteria

- AC-001: 面板头部提供「侧边栏：展开/收起」切换；切换后面板左侧偏移与拦截层宽度同步变化（280 ↔ 56px）；选择持久化（localStorage），重新打开面板保持。
- AC-002: 表单「任务标题」有显式字段标签；run 模式指令文本框加高（min-height 140px）；notify 模式文案支持多行输入。
- AC-003: run 模式显示「工作区」字段：可手输路径，可点「浏览…」打开原生文件夹选择器并回填；留空提示"使用 DSH 主目录"。
- AC-004: `save_task` 保存 workspace；`list_tasks` 返回；旧 tasks.json 无该字段时反序列化正常（serde default）。
- AC-005: run 任务触发时 current_dir = workspace（存在时），否则 DSH_HOME；workspace 不存在/非法时 spawn 失败并系统通知错误。
- AC-006: 详情抽屉 run 任务显示工作区路径（未设置显示"DSH 主目录"）。
- AC-007: 回归：`cargo test --release` 全绿（含新增 workspace 兼容测试）；`cargo build` 通过；JS 语法与 id 一致性通过；`npm run build` 打包成功。

## Global Constraints

- 不修改 DSH GUI；面板形态（覆盖右侧、保留侧边栏）不变。
- Task 新字段全部 `#[serde(default)]`，旧数据零破坏。
- dialog 插件仅用于 pick_workspace（原生文件夹选择器）；若 cargo 无法下载插件则回退：移除插件依赖，「浏览…」按钮改为提示手输路径（记录于完成说明）。
- 禁止漂移：不做"运行于当前聊天"（壳层无法控制 DSH GUI 会话）、不做模型/推理选择（headless 不支持）、不做通知策略多级。

## File Boundaries

- `src-tauri/Cargo.toml`: 新增 `tauri-plugin-dialog = "2"`。
- `src-tauri/src/lib.rs`: Task.workspace 字段、run_headless_task current_dir 逻辑、pick_workspace command + 插件注册 + invoke_handler。
- `frontend-dist/index.html`: 面板侧边栏模式切换、表单标题标签/大文本框/工作区字段、抽屉详情工作区行、JS 联动。
- `.cowork-flow/plans/2026-08-15-workspace-sidebar.md`: 本计划（self）。

## Tasks

### Task 1: 后端 workspace 字段 + run 执行目录 + dialog 插件

**Purpose**
- Task 支持 workspace 字段；run 执行使用任务工作区；新增 pick_workspace 原生文件夹选择 command。

**Code Fact Sources**
- Read: `src-tauri/src/lib.rs::Task`（L68-94）、`run_headless_task`（约 L250-300）、`invoke_handler`、`run()` 插件注册处（L1180-1250）

**File Boundaries**
- Modify: `Cargo.toml`（+tauri-plugin-dialog）
- Modify: `src-tauri/src/lib.rs`（Task 字段、run_headless_task、pick_workspace、插件注册、invoke_handler）

**Key Symbols**
- Add: `Task.workspace: Option<String>`、`#[tauri::command] fn pick_workspace(window: tauri::WebviewWindow)`（dialog 回调 + emit "workspace-picked"）、`.plugin(tauri_plugin_dialog::init())`
- Change: `run_headless_task` 中 `let cwd = t.workspace 非空 ? PathBuf::from(workspace) : home`；`cmd.current_dir(cwd)`

**Implementation Notes**
```rust
// Task 新增（serde default）
#[serde(default)] pub workspace: Option<String>,

// run_headless_task 中：
let cwd = t.workspace.clone()
    .filter(|s| !s.trim().is_empty())
    .map(std::path::PathBuf::from)
    .or_else(|| home.clone());
if let Some(d) = &cwd { cmd.current_dir(d); }

// pick_workspace（回调 + 事件回传，前端 listen "workspace-picked"）
#[tauri::command]
fn pick_workspace(window: tauri::WebviewWindow) {
    use tauri_plugin_dialog::DialogExt;
    window.dialog().file().pick_folder(move |folder| {
        if let Some(p) = folder {
            let _ = window.emit("workspace-picked", p.to_string());
        }
    });
}
```

**Test Proof**
- `lib.rs::tests`：`legacy_json_deserialize` 追加断言 workspace 默认 None；新增 `workspace_serde_roundtrip`（含 workspace 的 Task 序列化/反序列化一致）。

**Verification Command**
```bash
cargo test --release
cargo build
```

**Completion Conditions**
- workspace 字段兼容旧数据；run 执行目录逻辑正确；pick_workspace 注册成功（dialog 插件编译通过）。
- 若 dialog 插件下载失败：回退方案（不注册插件，前端浏览按钮提示手输）并记录。

**Prohibited Drift**
- 不改 headless spawn 的 DSH_HOME env 传递。
- 不改调度核心。

### Task 2: 前端侧边栏模式切换 + 表单增强 + 抽屉工作区

**Purpose**
- 面板支持 wide/rail 两档侧边栏对齐（持久化）；表单标题标签、大文本框、工作区字段（输入 + 浏览）；抽屉显示工作区。

**Code Fact Sources**
- Read: `frontend-dist/index.html` 面板 CSS/HTML/JS（tasks-page/tasks-shield/drawer/form）

**File Boundaries**
- Modify: `frontend-dist/index.html`

**Key Symbols**
- Add: `#btn-sidebar-mode`（头部切换钮）、`#task-workspace` 输入 + `#btn-workspace-browse`、`#dw-workspace` 抽屉字段、`taskWorkspace` 收集
- Change: `applySidebarMode()`（left/width 按 localStorage）、openTasks 时应用；run 表单显示工作区行；drawer 详情加工作区字段行（run 时显示）

**Implementation Notes**
```js
// 侧边栏模式（DSH GUI 侧边栏默认 280px，折叠成 56px rail）
const SB_MODE = localStorage.getItem("taskSidebarMode") || "wide";
function applySidebarMode() {
  const w = SB_MODE === "rail" ? 56 : 280;
  tasksPage.style.left = w + "px";
  document.getElementById("tasks-shield").style.width = w + "px";
  // 切换按钮文字：侧边栏 展开|收起
}
// 切换按钮点击 → 存 localStorage → applySidebarMode()

// 工作区字段（仅 run 动作显示）：
// <input id="task-workspace" placeholder="留空使用 DSH 主目录">
// <button id="btn-workspace-browse">浏览…</button>
// 浏览 → invoke("pick_workspace")；listen("workspace-picked", p => input.value = p)
// addTask 组装 workspace: value.trim() || null
```

**Test Proof**
- 无自动化；渲染预览验证（rail 模式布局、表单工作区字段）；人工清单。

**Verification Command**
```bash
node --check <抽取JS>
```

**Completion Conditions**
- 侧边栏切换生效且持久化；表单字段齐全；抽屉显示工作区；无残留。

**Prohibited Drift**
- 不改后端契约外字段。

### Task 3: 验证 + 重新打包

**Purpose**
- 全量回归 + 重新打包（用户诉求：修改后给新版本）。

**File Boundaries**
- Verify: 全部

**Verification Command**
```bash
cargo test --release
cargo build
npm run build   # 重新打包（先停旧实例）
```

**Completion Conditions**
- 全部 AC 通过；安装包重新生成；旧实例处理说明记录。

## Integrated Verification

1. `cargo test --release` → exit 0（12+ 例）。
2. `cargo build` → exit 0。
3. `node --check` 抽取内联 JS → exit 0；id 一致性检查。
4. 渲染预览：rail 模式面板、表单工作区字段、抽屉工作区行。
5. `npm run build` → 安装包重新生成（需先退出运行中实例）。
