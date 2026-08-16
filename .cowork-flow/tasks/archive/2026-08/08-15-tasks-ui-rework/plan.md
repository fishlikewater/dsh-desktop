# 定时任务界面重构（提醒 + 执行）实施计划

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 把壳层定时任务从"闹钟式浮层"重构为"定时提醒 + 定时执行"面板：入口迁到标题栏/托盘，动作诚实化（通知+聚焦组合、执行 DSH headless 任务），列表展示下次触发并排序，表单带实时预览与编辑取消，修复 interval 编辑立即触发缺陷，新增立即测试。 |
| **Task Type** | High-risk: 状态模型变更（字段迁移）+ 跨层契约（Rust command ↔ 壳页 JS）同步变更 |
| **Strategy** | Serial: 前后端共享任务模型契约，一个行为链，必须按序实施 |
| **Success Criteria** | AC-001 ~ AC-009（见下） |
| **Final Verification** | `cargo test` -> exit 0；`cargo build` -> exit 0；前端 index.html 语法检查（node --check 抽取脚本不可行，改为人工清单） |
| **Primary Risk** | Medium: run 动作依赖本机 dsh launcher 探测与 headless profile 可用性，失败时仅通知错误、不影响其他功能 |

## Goal

壳层定时任务面板重构（P0 打磨 + P1 执行动作）：
- 入口从"浮动按钮覆盖 DSH GUI 侧边栏"迁到标题栏时钟按钮 + 托盘菜单项；面板仍覆盖右侧内容区、保留左侧 DSH 侧边栏可见（用户决策）。
- 任务动作模型重构：`notify`（可自定义文案）+ `focus_window` 伴随勾选；`run`（执行 DSH headless 任务，模型选择在此生效）；旧 `focus` 动作迁移为 `notify` + `focus_window=true`。
- 列表展示"下次触发/上次触发"（后端计算返回），按下次触发升序；空状态引导；删除二次确认；编辑态可取消。
- 修复 interval 任务编辑计划后"保存即触发"缺陷（计划变更时重置基线）。
- 新增 `test_task` command（立即触发一次验证效果）。
- 建议模板改为"填入表单"而非直接创建；表单计划选择实时预览下次触发。

## Acceptance Criteria

- AC-001: 定时任务可从标题栏时钟按钮开合面板；浮动按钮 `#btn-tasks-entry` 与依赖 GUI 布局的入口逻辑已移除；托盘菜单含「定时任务」项并可从托盘打开面板。
- AC-002: 任务列表每行显示：开关、名称、计划描述、动作徽标、**下次触发时间（人性化）**；按下次触发升序；空状态含创建引导。
- AC-003: 动作模型：通知（可填自定义提醒文案）+「同时聚焦窗口」勾选；执行任务（指令 + 模型选择）为独立动作；纯通知任务不再展示模型字段。
- AC-004: 旧数据兼容：旧 `focus` 任务自动迁移为 `notify`+`focus_window`；旧 `model` 字段保留但仅 `run` 任务展示；`tasks.json` 读写兼容（serde 新字段带 default）。
- AC-005: 编辑态有「取消编辑」按钮与 Esc 退出；删除需二次确认；interval 任务编辑计划后从新计划重新计时（不立即触发）；未改计划则保留基线。
- AC-006: `test_task` 可立即触发一次任务（通知/聚焦/执行），不推进 `last_fired` 语义以外状态。
- AC-007: `run` 动作到点后 spawn `dsh --profile headless "<prompt>"` 子进程（launcher 探测：env `DSH_BIN` → PATH `dsh` → 当前用户 npx 缓存 `node bin.js`；`DSH_HOME` 探测：env → `~/.dsh`）；执行完成/失败后发系统通知。
- AC-008: 面板打开时点击左侧 DSH 侧边栏区域返回 GUI（保留原交互）；280px 侧边栏对齐为已知约束，代码注释注明 DSH GUI 侧边栏默认 280px、可拖拽 264-420、可折叠 56px。
- AC-009: `cargo test` 全绿；新增单测覆盖：schedule 变更判断、迁移逻辑、launcher 探测、list 元数据计算。

## Global Constraints

- 不修改 DSH GUI（不注入 JS、不 patch DSH、不调用 api-gateway/Typert RPC）。
- 不重写调度核心：`next_trigger_at` / `is_due` / 精确睡眠线程语义保持不变。
- 不引入新 Rust crate / 新前端框架；前端保持单文件 `frontend-dist/index.html` 内联 CSS/JS。
- `tasks.json` 存储路径与格式不变；新字段全部 `#[serde(default)]` 兼容旧数据。
- 壳页 `withGlobalTauri` 权限边界不变（iframe 内 DSH GUI 无 IPC 权限）。
- 禁止漂移：不做 cron 表达式编辑器、不做一次性提醒（at 语义）、不做通知声音配置、不做多会话/独立小窗。

## File Boundaries

- `src-tauri/src/lib.rs`: 任务模型/命令/调度触发/托盘/迁移；仅改定时任务相关区域（`===== 定时任务 =====` 段、`setup_tray`、`run` 的 invoke_handler 注册）。
- `frontend-dist/index.html`: 壳页 UI/JS；改标题栏、定时任务面板整段、相关 CSS；不动窗口控制/服务检测/resize 逻辑。
- `.cowork-flow/plans/2026-08-15-tasks-ui-rework.md`: 本计划（self）。

## Tasks

### Task 1: 后端任务模型扩展 + 旧数据迁移 + 列表元数据

**Purpose**
- Task 结构支持新动作语义：`focus_window`（伴随勾选）、`notify_body`（自定义文案）、`prompt`（run 指令）；`list_tasks` 返回附带 `nextTriggerAt`/`lastTriggerAt`（epoch 秒，前端人性化）。

**Code Fact Sources**
- Read: `src-tauri/src/lib.rs::Task`（L64-81）、`Schedule`（L83-94）、`next_trigger_at`（L108-150）、`list_tasks`（L289-292）、`load_tasks`（L274-279）

**File Boundaries**
- Modify: `src-tauri/src/lib.rs`（`===== 定时任务 =====` 段：Task 结构、新增 `TaskView`/`task_view()` 计算、`list_tasks` 返回值、`load_tasks` 后迁移）

**Key Symbols**
- Change: `struct Task`（新字段）、`fn list_tasks`（返回 `Vec<TaskView>`）
- Add: `struct TaskView { #[serde(flatten)] task: Task, next_trigger_at: Option<u64>, last_trigger_at: Option<u64> }`、`fn task_view(t: &Task, now_secs: i64) -> TaskView`、`fn migrate_task(t: &mut Task)`（旧 focus → notify+focus_window）、`fn schedule_key(s: &Schedule) -> String`（比较用）
- Preserve: `next_trigger_at` / `is_due` / `weekday_of` / `Schedule` 枚举变体

**Implementation Notes**
```rust
// Task 新增字段（全部 default，兼容旧 JSON）
#[serde(default)] pub focus_window: bool,
#[serde(default)] pub notify_body: Option<String>, // None → 用 name
#[serde(default)] pub prompt: Option<String>,      // 仅 action=="run" 使用
// action 取值收敛为 "notify" | "run"（"focus" 由迁移转换）

// 迁移：load_tasks 读入后逐条 migrate_task()
// old action=="focus" → action="notify", focus_window=true

// list_tasks 返回 TaskView：next = next_trigger_at(t, t.last_fired, now, day, secs)
```

**Test Proof**
- `lib.rs::tests`：新增 `migrate_focus_task`（focus → notify+focus_window=true）、`task_view_meta`（daily 任务 next_trigger_at 正确计算）。
- Core assertion: 迁移后 action 为 "notify" 且 focus_window 为 true；TaskView 的 next_trigger_at 与纯函数计算结果一致。

**Verification Command**
```bash
cargo test
```

**Completion Conditions**
- Task 新字段带 `#[serde(default)]`；旧 tasks.json 可反序列化且 focus 任务迁移正确。
- `list_tasks` 返回 TaskView，前端可拿到 nextTriggerAt/lastTriggerAt。
- cargo test 全绿（含新增 2 例）。

**Prohibited Drift**
- 不改 `next_trigger_at`/`is_due` 计算逻辑。
- 不改 `Schedule` 枚举形状与 serde tag。
- 不在 list_tasks 里做持久化写入。

**Deviation Conditions**
- 若需要改动 `Schedule` 枚举或序列化格式，必须停下回报（文件边界错误）。

### Task 2: interval 编辑基线修复

**Purpose**
- 编辑任务时若计划（类型或参数）变化，重置 `last_fired = now` 重新计时；计划未变则保留基线（现状行为）。

**Code Fact Sources**
- Read: `src-tauri/src/lib.rs::save_task`（L294-315）、`Schedule`（L83-94）

**File Boundaries**
- Modify: `src-tauri/src/lib.rs`（`save_task` 及 `schedule_key` 辅助）

**Key Symbols**
- Add: `fn schedule_key(s: &Schedule) -> String`（对 scheduleType+参数做确定性序列化）
- Change: `save_task` 编辑分支：比较新旧 `schedule_key`，不同则 `task.last_fired = Some(now)`

**Implementation Notes**
```rust
// schedule_key: 形如 "daily:09:30" / "weekly:0:08:00" / "weekdays:09:00" / "interval:30"
// save_task 编辑分支:
//   let changed = schedule_key(&existing.schedule) != schedule_key(&task.schedule);
//   let keep_last = existing.last_fired;
//   *existing = task;
//   existing.last_fired = if changed { Some(now) } else { keep_last };
```

**Test Proof**
- `lib.rs::tests`：`schedule_key_distinct`（四种类型互不相同、参数不同互不相同）。
- Core assertion: 同类型同参数的 schedule_key 相等，不同参数不相等。

**Verification Command**
```bash
cargo test
```

**Completion Conditions**
- `schedule_key` 覆盖 4 种计划且确定性。
- `save_task` 计划变更时重置基线；未变更时保留。
- cargo test 全绿。

**Prohibited Drift**
- 不改新任务分支（`last_fired = Some(now)` 保持）。
- 不引入时间比较之外的复杂逻辑。

### Task 3: test_task 命令 + run 动作执行（headless spawn）

**Purpose**
- 新增 `test_task` command：立即触发指定任务（走 `fire_task`），用于验证效果。
- `fire_task` 支持 `action=="run"`：探测 dsh launcher，spawn 子进程执行 `--profile headless "<prompt>"`，完成后（成功/失败）发系统通知。

**Code Fact Sources**
- Read: `src-tauri/src/lib.rs::fire_task`（L209-229）、`invoke_handler` 注册（L832-839）、`notify`（L41-45）

**File Boundaries**
- Modify: `src-tauri/src/lib.rs`（`fire_task`、新增 `run_headless_task`/`dsh_launcher`/`dsh_home_dir`、`test_task` command、invoke_handler 注册）

**Key Symbols**
- Add: `fn dsh_launcher() -> Option<Vec<String>>`（探测；返回 argv 前缀如 `["dsh"]` 或 `[node, bin.js]`）、`fn dsh_home_dir() -> Option<PathBuf>`、`fn run_headless_task(app: &AppHandle, t: &Task)`（std::thread::spawn 内执行）、`#[tauri::command] fn test_task(...)`
- Change: `fire_task` 的 `"run"` 分支调用 `run_headless_task`

**Implementation Notes**
```rust
// dsh_launcher 探测顺序（全部失败 → None，run 动作通知失败原因）：
//   1. env DSH_BIN（含参数时按空格拆分，简单处理为单路径）
//   2. PATH 中 "dsh"（which 语义：逐目录查 dsh.exe/dsh.cmd）
//   3. 当前用户 npx 缓存: %LocalAppData%\npm-cache\_npx\<任意>\node_modules\@deepseek-ai\dsh\lib\bin.js + node 可执行（env ComSpec 无关，用 env NODE 或 "node" 走 PATH）
// dsh_home_dir: env DSH_HOME → ~/.dsh
// run_headless_task:
//   let argv = launcher + ["--profile", "headless", prompt];
//   Command::new(argv[0]).args(&argv[1..]).env("DSH_HOME", home).current_dir(home 或主目录)
//     .stdout(null).stderr(null).spawn()；spawn 失败 → notify("任务执行启动失败")
//   子进程结束（wait）后 → notify("任务完成"/"任务失败（exit != 0）")
//   注意: headless 跑完打印最后一条 assistant 文本到 stdout（当前丢弃，只通知成败）
```

**Test Proof**
- `lib.rs::tests`：`launcher_probe_order`（mock 不可行时改为纯函数 `launcher_candidates()` 返回候选列表的顺序/形状断言：env 覆盖 > PATH > npx 缓存）。
- Core assertion: 候选列表按优先级排序且 env 存在时优先。
- `test_task` 与 `fire_task` 复用同一触发路径（人工验证 + notify 冒烟）。

**Verification Command**
```bash
cargo test
cargo build
```

**Completion Conditions**
- `test_task` 注册进 invoke_handler。
- `run` 分支 spawn 逻辑存在；launcher 探测候选顺序正确。
- cargo test/build 通过。
- （手动验证项，记录到完成说明）：在装有 dsh 的机器上 `test_task` 一个 run 任务能弹出完成通知。

**Prohibited Drift**
- 不阻塞调度线程（spawn 后立即返回；子进程在线程内 wait）。
- 不解析/不展示 headless stdout 内容（仅成败通知）。
- 不引入同步阻塞 IPC。

**Deviation Conditions**
- 若确认本机无法运行 headless（profile 初始化失败等），必须停下回报，降级为"run 动作保留但标注不可用"。

### Task 4: 前端入口重构（标题栏时钟 + 托盘联动 + 移除浮动按钮）

**Purpose**
- 标题栏（最小化按钮左侧）加时钟图标按钮，点击开合任务面板；移除浮动按钮 `#btn-tasks-entry` 及依赖布局的入口样式；保留 `#tasks-shield` 作为"点击侧边栏返回"热区；托盘「定时任务」菜单项 → Rust emit 事件 → 壳页监听打开面板。

**Code Fact Sources**
- Read: `frontend-dist/index.html` L150-212（入口/拦截层 CSS+HTML）、L799-838（openTasks/backToGui/事件绑定）、标题栏结构 L441-460
- Read: `src-tauri/src/lib.rs::setup_tray`（L511-585）

**File Boundaries**
- Modify: `frontend-dist/index.html`（标题栏按钮区、删除 `#btn-tasks-entry` 样式与元素、`openTasks/backToGui` 绑定、新增 `listen("open-tasks")`）
- Modify: `src-tauri/src/lib.rs`（`setup_tray` 加「定时任务」MenuItem → `app.emit("open-tasks", ())`）

**Key Symbols**
- Change: 删除 `#btn-tasks-entry` 相关 CSS/HTML/JS；`openTasks/backToGui` 保留但由新按钮触发
- Add: `#btn-tasks`（标题栏时钟图标按钮，复用 `.win-btn` 布局但宽度自适应）、`window.__TAURI__.event.listen("open-tasks", ...)`

**Implementation Notes**
```js
// 标题栏：在 btn-min 前插入
// <button class="win-btn" id="btn-tasks" title="定时任务" style="width:auto;padding:0 12px">
//   <svg>时钟图标</svg></button>
// 点击 → openTasks()/backToGui() 切换（与旧入口相同逻辑，仅触发源变化）
// Tauri event: __TAURI__.event.listen("open-tasks", () => openTasks())
```

**Test Proof**
- 无自动化测试（壳页无测试基建）；人工验证清单记录于完成说明：标题栏按钮开合、托盘菜单项开合、浮动按钮已不存在、窗口缩放时入口不漂移。

**Verification Command**
```bash
cargo build
```

**Completion Conditions**
- 浮动按钮元素/样式/JS 全部移除；标题栏时钟按钮与托盘菜单均可开合面板。
- 入口不再依赖 DSH GUI 布局坐标。

**Prohibited Drift**
- 不改 `#tasks-shield` 的返回交互（保留"点侧边栏返回"）。
- 不改窗口控制/服务检测 JS。

### Task 5: 前端列表重构（元数据展示 + 排序 + 空态 + 删除确认 + 立即测试）

**Purpose**
- 列表行展示：开关、名称、计划+动作徽标、下次触发（人性化，如"今天 18:00"/"3 分钟后"）、上次触发（次要信息）；按 nextTriggerAt 升序；空状态带创建引导；删除二次确认；行内「立即测试」按钮。

**Code Fact Sources**
- Read: `frontend-dist/index.html` L882-989（schedText/loadTasks/行渲染）

**File Boundaries**
- Modify: `frontend-dist/index.html`（`loadTasks` 渲染逻辑、`schedText`、新增 `humanizeTime`、空状态、删除确认、测试按钮）

**Key Symbols**
- Add: `fn humanizeTime(epochSecs)`、行内测试按钮（`invoke("test_task", {id})`）
- Change: `loadTasks` 排序（`nextTriggerAt` 升序，无值排最后）、删除用 `confirm()`、空态渲染引导按钮（focus 表单）

**Implementation Notes**
```js
// humanizeTime(secs): 与 Date.now()/1000 比较
//   <60s → "即将触发"；当天 → "今天 HH:mm"；明天 → "明天 HH:mm"；7 天内 → "周X HH:mm"；否则 "M月D日 HH:mm"
// 排序: tasks.sort((a,b) => (a.nextTriggerAt ?? 1e15) - (b.nextTriggerAt ?? 1e15))
// 行结构: [switch] [name / sched+徽标] [下次触发] [测试] [编辑] [删除]
// 删除: if (!confirm(`删除任务「${t.name}」？`)) return;
```

**Test Proof**
- 无自动化测试；人工清单：排序正确、人性化文案正确（含跨天）、删除有确认、测试按钮触发系统通知。

**Verification Command**
```bash
cargo build
```

**Completion Conditions**
- 列表展示下次/上次触发；默认按下次触发升序；空态有引导；删除确认；测试按钮调用 `test_task`。

**Prohibited Drift**
- 不改 `toggle_task` 交互（行内开关）。
- 不在前端重算下次触发（一律用后端返回的 nextTriggerAt）。

### Task 6: 前端表单重构（动作诚实化 + 实时预览 + 编辑取消）

**Purpose**
- 表单字段：名称 → 计划（4 选 1 + 条件输入）→ 触发行为（通知：文案可选 + 聚焦勾选；执行：指令 + 模型）→ 创建/保存；计划选择即时预览下次触发（用后端 nextTriggerAt 计算——新增 `preview_task` 场景：提交临时任务给后端算？改为前端调用新 command 或复用 save 前预览。**简化决策：预览用后端 `list_tasks` 同款计算不可行（任务未保存），改为前端本地粗算**：daily/weekly/weekdays 用 Date 推算；interval 用 last_fired+minutes。粗算仅展示）。
- 建议模板改为"填入表单"（不再直接创建）；编辑态显示「取消编辑」按钮 + Esc 退出。

**Code Fact Sources**
- Read: `frontend-dist/index.html` L367-406（表单 CSS）、L532-571（表单 HTML）、L841-880（resetForm/类型切换）、L991-1048（建议/提交逻辑）

**File Boundaries**
- Modify: `frontend-dist/index.html`（表单 HTML/CSS/JS、建议区、resetForm、addTask）

**Key Symbols**
- Add: `#task-notify-body`（提醒文案，placeholder 默认用任务名）、`#task-focus-win`（勾选）、`#task-prompt`（指令 textarea，run 时显示）、`#task-cancel-edit`（取消按钮）、`#task-preview`（下次触发预览行）、`estimateNext(form)` 本地推算函数
- Change: `task-type` 切换逻辑（interval 行、weekday 行、run 表单行联动）、`addTask` 字段组装（focus_window/notify_body/prompt）、建议按钮点击 → 填充表单 + 聚焦名称输入

**Implementation Notes**
```js
// 动作单选: notify / run（radio 或 select）。notify → 显示文案输入 + 聚焦勾选；run → 显示指令 textarea + 模型 select
// estimateNext(form)（仅预览，纯前端 Date 推算）:
//   daily/weekdays/weekly: 构造 Date(now) 对齐 HH:mm（weekly 找目标 weekday；weekdays 跳过周末），已过则 +1 天/周
//   interval: now + minutes*60_000
// 预览行文案: "下次触发：明天 09:00"
// 建议模板 data 属性改为填充表单: fillForm(name, type, time, ...) + 不调用 save_task
// 编辑态: editingId 非空时显示 #task-cancel-edit（点击 → resetForm）；document keydown Esc → resetForm
```

**Test Proof**
- 无自动化测试；人工清单：四类计划预览正确（跨天/跨周）、run 表单联动、建议填入后可修改再建、编辑取消恢复新建态。

**Verification Command**
```bash
cargo build
```

**Completion Conditions**
- 表单字段与动作语义一致（通知无模型、run 有指令+模型）；预览行存在且推算正确；建议=填入表单；编辑可取消（按钮+Esc）。

**Prohibited Drift**
- 预览推算不写回后端；创建/保存仍走 `save_task` 全量对象。
- 不改变 `save_task` 的字段契约（前端提交的 JSON 与 Rust Task 对齐）。

### Task 7: 托盘菜单项 + 文案统一 + 集成验证

**Purpose**
- 托盘菜单加「定时任务」项（Task 4 已含，此项做收尾与验证）；面板全部文案与真实能力一致（"定时任务"标题 + 副标题说明提醒/执行能力）；README 更新功能描述；整体验证。

**Code Fact Sources**
- Read: `src-tauri/src/lib.rs::setup_tray`、`README.md` L15、`frontend-dist/index.html` 全部文案

**File Boundaries**
- Modify: `frontend-dist/index.html`（文案）
- Modify: `README.md`（L15 功能描述 + 架构说明）
- Verify: `src-tauri/src/lib.rs`、`frontend-dist/index.html`

**Key Symbols**
- Change: 面板标题/副标题/空态/建议模板文案；README 定时任务条目

**Implementation Notes**
- 文案要点：面板标题「定时任务」；副标题「到点提醒或执行 DSH 任务」；空态「还没有定时任务，创建第一个提醒或自动化任务」；建议模板改为：喝水提醒（工作日 14:00 通知）、久坐提醒（每 60 分钟通知+聚焦）、代码提交检查（工作日 17:00 执行任务，填入表单）。
- README 定时任务条目：入口（标题栏时钟/托盘）、动作（通知+聚焦、执行 DSH 任务）、下次触发展示、test_task。

**Test Proof**
- `cargo test` 全绿；`cargo build` 通过；人工清单走查。

**Verification Command**
```bash
cargo test
cargo build
```

**Completion Conditions**
- 文案与能力一致；README 同步；托盘菜单含「定时任务」；全部 AC 走查通过。

**Prohibited Drift**
- 不改调度核心语义；不扩新功能。

## Integrated Verification

1. `cargo test` → exit 0（全部单测，含新增：migrate、task_view、schedule_key、launcher 候选）。
2. `cargo build` → exit 0（debug 编译通过，前端为静态文件无构建）。
3. 人工走查清单（写入任务完成说明）：
   - 标题栏时钟按钮开合面板；托盘「定时任务」开合面板；浮动按钮已消失。
   - 列表显示下次触发并按时间排序；空态引导；删除确认；行内立即测试弹出系统通知。
   - 创建通知任务（自定义文案+聚焦勾选）→ 立即测试验证；创建 run 任务（指令+模型）→ 立即测试验证 headless 执行与完成通知。
   - 编辑任务：计划变更后不立即触发（观察 last_fired 重置）；取消编辑恢复新建态。
   - 旧 tasks.json（含 focus 任务）加载后正常显示且 focus 任务已迁移。
   - 窗口缩放、GUI 侧边栏折叠时入口不漂移；面板左侧 280px 对齐在默认侧边栏下正确。

## Risks & Mitigations

- run 动作依赖本机 dsh launcher：探测失败或 headless 执行失败 → 系统通知错误信息，任务本身不崩溃；launcher 探测支持 env `DSH_BIN` 兜底。
- headless 首次运行自动初始化 profile，可能耗时数秒：子进程在独立线程 wait，不阻塞调度。
- 280px 侧边栏对齐约束：DSH GUI 侧边栏默认 280px（源码确认），用户可拖拽 264-420 或折叠 56px——面板左侧露出/遮挡为已知视觉差异，功能不受影响；代码注释说明。
