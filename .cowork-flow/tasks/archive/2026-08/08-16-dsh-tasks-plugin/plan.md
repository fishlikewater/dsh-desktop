# 定时任务提取为 DSH 插件（@fishlikewater/dsh-tasks）+ 壳层移除

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 把 dsh-desktop 壳层的定时任务功能完整提取为 DSH cordis 插件 `@fishlikewater/dsh-tasks`（开发目录 `E:\Projects\IdeaProjects\person\dash-plugin`），安装进 DSH web profile；同时从壳层移除全部定时任务代码/配置，壳层仅保留"定时任务"托盘入口（点击打开 DSH GUI）。 |
| **Task Type** | Normal: 新插件工程 + 壳层减法 |
| **Strategy** | Serial: 插件 host 端 → 插件 client 端 → 安装验证 → 壳层移除 → 打包 |
| **Success Criteria** | AC-001 ~ AC-012 |
| **Final Verification** | 插件: DSH 重启加载日志 + GUI 截图（侧边栏入口/面板 CRUD/立即运行）；壳层: cargo test --release、cargo build、npm run build、JS 检查 |

## Goal

1. **插件 `@fishlikewater/dsh-tasks`**（目录 `E:\Projects\IdeaProjects\person\dash-plugin`）：
   - Host 端 `lib/index.js`：任务 CRUD 持久化（settings 命名空间 `dsh-tasks`，落 settings.yaml）、调度循环（ctx.setInterval 1s tick，next_trigger_at/is_due 算法移植）、触发执行（仿 dsh-headless：agents.create + followup + whenIdle + sessions.flush，cwd=workspace）、提醒通知队列（未读 notices，client 轮询 ack）、webServer 同源路由 `/_dsh/dsh-tasks/*`（GET tasks+notices / POST save/delete/toggle/run/ack）。
   - Client 端 `lib/client.js`（web）：侧边栏入口（`sidebar.footer.action` slot，定时任务图标按钮）→ 全宽任务面板 overlay（列表/详情抽屉/创建编辑表单/建议/立即运行/启停/删除，DSH 主题变量），fetch 轮询（10s）+ Web Notification（授权后）展示未读提醒。
   - `cordis.patch.yml` insert 条目 + `package.json`（dsh bundle/client 声明，仿 dsh-vision-toolkit）。
2. **安装**：`~/.dsh/profiles/web/node_modules/@fishlikewater/dsh-tasks` symlink → `E:\Projects\IdeaProjects\person\dash-plugin`；web profile 的 `cordis.patch.yml` 追加 insert 条目；重启 DSH 验证加载。
3. **壳层移除**（dsh-desktop）：
   - `src-tauri/src/lib.rs`：Task/TaskView/schedule_key/migrate_task/TasksState/调度线程/run_headless_task/commands（list_tasks/save_task/delete_task/toggle_task/test_task/pick_workspace）/托盘 tasks_item/on_menu_event "tasks" 分支/相关测试 —— 全部移除。
   - 托盘「定时任务」菜单项保留：点击 = show 主窗口 + 聚焦（入口语义改为打开 GUI，GUI 侧边栏里有任务面板）。
   - `frontend-dist/index.html`：任务面板（HTML/CSS/JS）、标题栏时钟按钮 btn-tasks、open-tasks/workspace-picked 监听 —— 全部移除。
   - README 更新。

## Acceptance Criteria

- AC-001: 插件工程结构完整（package.json/cordis.patch.yml/lib/index.js/lib/client.js），`node --check` 通过。
- AC-002: DSH 重启后插件加载成功（日志无错误），设置页/配置中可见 dsh-tasks 命名空间。
- AC-003: GUI 侧边栏底部出现「定时任务」入口；点击打开任务面板（列表/创建/编辑/启停/删除/立即运行可用）。
- AC-004: 调度生效：新建"间隔 1 分钟"提醒任务后，到点产生未读通知（面板内提示 + Web Notification 授权后）。
- AC-005: run 类型任务到点执行 DSH agent（默认模型、指定工作区 cwd），执行会话在 GUI 会话历史可见。
- AC-006: 任务持久化：重启 DSH 后任务列表保留。
- AC-007: 壳层 lib.rs 无定时任务残留（编译通过、测试通过）；托盘「定时任务」项点击打开 GUI。
- AC-008: 壳层 index.html 无任务面板残留（JS 语法/无引用错误），标题栏时钟按钮移除。
- AC-009: 壳层 `cargo test --release` 全绿（保留测试通过）。
- AC-010: 壳层 `npm run build` 打包成功。
- AC-011: README 定时任务条目更新为插件形态。
- AC-012: 旧 tasks.json 数据说明（壳层已不再读取；如需迁移手动导出——本次不做自动迁移，记录于完成说明）。

## Global Constraints

- 不修改 DSH 官方包源码；插件为独立 npm 包。
- 通知能力回退（用户已确认）：Web 通知 + 面板内提示；无系统 toast、无聚焦窗口。
- 壳层保留「定时任务」托盘入口（用户已确认），点击打开 GUI。
- 插件命名（用户已确认）：`@fishlikewater/dsh-tasks`，目录 `dash-plugin`。
- 禁止漂移：不做运行历史列表、不做模型选择、不做多级通知策略、不做自动数据迁移。

## File Boundaries

- `E:\Projects\IdeaProjects\person\dash-plugin\`（新工程）：package.json / cordis.patch.yml / lib/index.js / lib/client.js / README.md
- `~/.dsh/profiles/web/node_modules/@fishlikewater/dsh-tasks`（symlink）
- `~/.dsh/profiles/web/cordis.patch.yml`（追加 insert 条目）
- `src-tauri/src/lib.rs` / `frontend-dist/index.html` / `README.md`（壳层移除）

## Tasks

### Task 1: 插件 host 端（调度/存储/执行/RPC）

**Purpose**
- DSH 服务端承载定时任务：持久化、调度、执行、通知队列、HTTP API。

**Code Fact Sources**
- Read: `@deepseek-ai/dsh-headless/lib/index.js`（run 驱动全文）、`@deepseek-ai/dsh-client-ui-jobs/lib/client.js`（client 形态）、`~/.dsh/profiles/web/node_modules/@dsh-external/dsh-vision-toolkit/lib/web.js`（webServer 路由）、`cordis-plugin-timer`（可选调度 API）
- Read: 壳层 `src-tauri/src/lib.rs`（next_trigger_at/is_due/调度算法移植）

**File Boundaries**
- Create: `E:\Projects\IdeaProjects\person\dash-plugin\package.json`、`cordis.patch.yml`、`lib/index.js`

**Key Symbols**
- `name = "@fishlikewater/dsh-tasks"`、`inject = ["settings", "agents", "sessions", "agentDefaultModel", "webServer", "logger"]`
- `Config`（schemastery）：`tasks: z.array(Task)`；Task: id/name/scheduleType/hour/minute/weekday/minutes/action/enabled/notifyBody/prompt/workspace/lastFired
- `ctx.settings.register("dsh-tasks", Config, { base: config, applies: "live" })`
- 调度：`ctx.setInterval(tick, 1000)`；`nextTriggerAt(task, now)` 移植壳层算法（daily/weekdays/weekly/interval）
- 触发：notify → notices 队列 push {id,taskId,name,body,at}；run → `runAgent(ctx, task)`（agents.create + followup + whenIdle + sessions.flush，cwd=workspace||undefined）
- webServer：`register({kind:"prefix", path:"/_dsh/dsh-tasks/", handler})`；GET tasks（TaskView+nextTriggerAt+notices）；POST JSON action: save/delete/toggle/run/ack

**Implementation Notes**
- tick 幂等：记录 lastCheckedAt，防止重复触发；通知队列上限 100 条，ack 后删除。
- run 动作失败：push 失败通知（body 带错误摘要）。
- settings live 变更：监听 settings 更新后重建调度视图（简单起见每次 tick 从 settings 读 tasks）。

**Verification Command**
```bash
node --check lib/index.js
```

**Completion Conditions**
- host 端逻辑完整；无依赖缺失（@deepseek-ai/schemastery 等从 profile node_modules 解析）。

### Task 2: 插件 client 端（侧边栏入口 + 任务面板）

**Purpose**
- DSH GUI 内提供定时任务 UI（侧边栏入口 + 面板）。

**Code Fact Sources**
- Read: `@deepseek-ai/dsh-client-ui-cordis/lib/client.js`（sidebar.footer.action 注册形态）、`dsh-client-ui-jobs/lib/client.js`（ModuleLoader/locale/组件形态）
- Read: 壳层 `frontend-dist/index.html`（面板 UI 移植源：列表/抽屉/表单/建议）

**File Boundaries**
- Create: `E:\Projects\IdeaProjects\person\dash-plugin\lib\client.js`

**Key Symbols**
- `window.__ModuleLoader__.load({ id, factory })` 包装；`inject = ["slots", "locale"]`
- `apply(ctx)`: locale 注册（zh/en）+ `ctx.slots.inject("sidebar.footer.action", () => ctx.slots.register({name, id:"dsh-tasks-panel", order}, TasksButton))`
- TasksButton：图标按钮，点击切换全宽 overlay 面板（组件内部 state）
- 面板组件：列表（名称/计划/动作/下次触发/开关）、详情抽屉（指令/文案/计划/动作/工作区/下次/上次 + 立即运行/启停/编辑/删除）、创建编辑表单（标题/计划/动作卡片/文案/指令/工作区文本+showDirectoryPicker 增强）、建议卡片（带描述、填入表单）
- 数据：`fetch("/_dsh/dsh-tasks/tasks")` GET + POST；面板打开时轮询 10s；Web Notification 授权（按钮触发 Notification.requestPermission）与未读 notices 展示 + ack
- 样式：DSH 设计变量（var(--dsw-*)），CSS 注入模式仿 ui-jobs（style[data-plugin-css]）

**Verification Command**
```bash
node --check lib/client.js
```

**Completion Conditions**
- client 端无语法错误；组件引用全部自包含。

### Task 3: 安装进 DSH profile + 重启验证

**Purpose**
- 让 DSH 加载插件并验证 GUI 内效果。

**Code Fact Sources**
- Read: `~/.dsh/profiles/web/cordis.patch.yml`（追加）、`~/.dsh/restart-dsh.ps1`（重启方式）

**File Boundaries**
- Create: symlink `~/.dsh/profiles/web/node_modules/@fishlikewater/dsh-tasks` → `E:\Projects\IdeaProjects\person\dash-plugin`
- Modify: `~/.dsh/profiles/web/cordis.patch.yml`（追加 insert 条目 id=dsh-tasks）

**Implementation Notes**
- symlink 用 `New-Item -ItemType SymbolicLink`（需确认权限；失败则复制目录）。
- patch 追加：
```yaml
- insert:
    - id: dsh-tasks
      name: '@fishlikewater/dsh-tasks'
```
- 重启：调用 `~/.dsh/restart-dsh.ps1`（用户已有脚本）或询问用户重启方式；验证：日志无错误 + 截图 GUI 侧边栏入口。

**Verification Command**
```bash
node -e "import('@fishlikewater/dsh-tasks').then(m => console.log('load ok', m.name))"  # 在 profile node_modules 上下文
```

**Completion Conditions**
- 插件被 DSH 加载；GUI 可见入口与面板；调度/通知/执行实测通过（AC-002~AC-006）。

### Task 4: 壳层移除（lib.rs + index.html + README）

**Purpose**
- 壳层回归纯粹容器：移除全部定时任务代码/配置，保留托盘入口（打开 GUI）。

**Code Fact Sources**
- Read: `src-tauri/src/lib.rs`（tasks 相关全量）、`frontend-dist/index.html`（面板全量）、`README.md` L15

**File Boundaries**
- Modify: `src-tauri/src/lib.rs`（删除 tasks 模块段/commands/托盘项改语义）、`frontend-dist/index.html`（删面板/时钟按钮/监听/CSS/JS）、`README.md`

**Implementation Notes**
- lib.rs：删除 Task/TaskView/migrate_task/schedule_key/next_trigger_at/is_due/TasksState/调度线程/fire_task/run_headless_task/launcher_candidates/dsh_launcher/dsh_home_dir?（dsh_home_dir 若无其他使用则删）/commands/tests 中 tasks 相关；托盘 tasks_item 保留但 on_menu_event "tasks" 分支改为 `show 主窗口 + set_focus`（不 emit open-tasks）
- index.html：删除 tasks-page/task-drawer/task-list/task-empty/suggest-* 全部 HTML/CSS/JS、btn-tasks 时钟按钮、open-tasks/workspace-picked 监听、panel 相关 CSS 变量若专用则删
- 检查残留：grep tasks/shield/drawer 等关键词 0 命中

**Verification Command**
```bash
cargo test --release && cargo build
node --check <抽取JS>
```

**Completion Conditions**
- 编译/测试/JS 全绿；残留检查通过。

### Task 5: 壳层打包 + 收尾

**Purpose**
- 重新打包壳层；更新 README；流程收尾。

**File Boundaries**
- Verify: 全部

**Verification Command**
```bash
npm run build
```

**Completion Conditions**
- 安装包重新生成；README 更新；AC 全部达成。

## Integrated Verification

1. 插件：`node --check` × 2 → exit 0。
2. DSH 重启加载 + GUI 截图验证（侧边栏入口/面板 CRUD/通知/立即运行）。
3. 壳层：`cargo test --release`（保留测试）→ exit 0；`cargo build` → exit 0；JS 检查；残留 grep 0。
4. `npm run build` → 安装包重新生成。
