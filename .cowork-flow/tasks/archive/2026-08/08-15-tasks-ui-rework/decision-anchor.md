# Decision Anchor

## 目标
把壳层定时任务从"闹钟式浮层"重构为"定时提醒 + 定时执行"面板：入口迁到标题栏/托盘，动作模型诚实化（通知+聚焦组合、执行 DSH headless 任务），列表展示下次触发并排序，表单实时预览与编辑取消，修复 interval 编辑立即触发缺陷，新增立即测试。

## 验收标准
- [ ] AC-001: 定时任务可从标题栏时钟按钮开合面板；浮动按钮 `#btn-tasks-entry` 与依赖 GUI 布局的入口逻辑已移除；托盘菜单含「定时任务」项并可从托盘打开面板。
- [ ] AC-002: 任务列表每行显示：开关、名称、计划描述、动作徽标、下次触发时间（人性化）；按下次触发升序；空状态含创建引导。
- [ ] AC-003: 动作模型：通知（可填自定义提醒文案）+「同时聚焦窗口」勾选；执行任务（指令 + 模型选择）为独立动作；纯通知任务不再展示模型字段。
- [ ] AC-004: 旧数据兼容：旧 `focus` 任务自动迁移为 `notify`+`focus_window`；旧 `model` 字段保留但仅 `run` 任务展示；`tasks.json` 读写兼容（serde 新字段带 default）。
- [ ] AC-005: 编辑态有「取消编辑」按钮与 Esc 退出；删除需二次确认；interval 任务编辑计划后从新计划重新计时（不立即触发）；未改计划则保留基线。
- [ ] AC-006: `test_task` 可立即触发一次任务（通知/聚焦/执行），不推进 `last_fired` 语义以外状态。
- [ ] AC-007: `run` 动作到点后 spawn `dsh --profile headless "<prompt>"` 子进程（launcher 探测：env `DSH_BIN` → PATH `dsh` → 当前用户 npx 缓存 `node bin.js`；`DSH_HOME` 探测：env → `~/.dsh`）；执行完成/失败后发系统通知。
- [ ] AC-008: 面板打开时点击左侧 DSH 侧边栏区域返回 GUI（保留原交互）；280px 侧边栏对齐为已知约束，代码注释注明 DSH GUI 侧边栏默认 280px、可拖拽 264-420、可折叠 56px。
- [ ] AC-009: `cargo test` 全绿；新增单测覆盖：schedule 变更判断、迁移逻辑、launcher 探测候选、list 元数据计算。

## 被拒方案
- **浮动按钮继续作为唯一入口**: 拒绝原因——硬编码坐标依赖 DSH GUI 布局（280px 侧边栏假设），GUI 改版/折叠/缩放即错位遮挡，且入口按钮遮挡 GUI 原生功能。
- **调用 DSH api-gateway（Typert RPC）执行任务**: 拒绝原因——Cordis 内部协议，需严格描述符与 Connection 封装，壳层接入成本高且非公开 API；`dsh --profile headless` 是官方一次性执行路径，复用同一 DSH_HOME 配置。
- **全屏覆盖面板（不保留侧边栏）**: 拒绝原因——用户决策保留左侧 DSH 侧边栏可见；面板改为覆盖右侧内容区。
- **前端本地计算下次触发作为列表展示**: 部分拒绝——列表展示一律用后端返回的 `nextTriggerAt`（唯一权威）；仅表单"计划预览"允许前端粗算（任务未保存，后端无对象可算）。

## 关键假设
- DSH GUI 侧边栏默认 280px（已读 DSH 源码确认：`panels.sidebar === 0 ? 280 : panels.sidebar`），用户可拖拽 264-420 或折叠成 56px rail；面板左侧 280px 对齐为已知约束，错位仅视觉差异、功能不受影响。
- 本机 dsh launcher 可用（已实测 `node bin.js --version` → `0.1.0-rc.6`）；headless profile 首次运行自动从随附模板初始化。
- 旧 tasks.json 无 `focus_window`/`notify_body`/`prompt` 字段，需 serde default + 加载时迁移。
- 用户机器 DSH_HOME = `~/.dsh`（restart 脚本证据），配置/凭证与 web profile 共享。

## 范围边界
- 范围内: 入口重构（标题栏/托盘）、动作模型迁移、列表元数据与排序、表单重构（预览/取消/模板填入）、`test_task`、`run` 执行（headless spawn）、interval 基线修复、文案与 README 同步。
- 范围外: cron 表达式编辑器、一次性提醒（at 语义）、通知声音/重复配置、独立小窗/多会话、DSH GUI 任何修改、api-gateway/Typert 接入、调度核心逻辑重写。

## 验证命令
- `cargo test` → exit 0
- `cargo build` → exit 0
