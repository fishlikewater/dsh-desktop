# 任务面板 Codex 化重构（抽屉式详情）实施计划

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 参考 Codex 桌面「已安排」实测设计，把任务面板重构为：全宽列表页 + 右上「创建」按钮 + 点击任务行打开右侧详情抽屉（立即运行/暂停/编辑/删除/关闭 + 配置展示 + 下次/上次触发），建议卡片带描述文字，列表行简化为只留开关。 |
| **Task Type** | Normal: 纯前端 UI/交互重构，后端无改动 |
| **Strategy** | Serial: 同文件（frontend-dist/index.html）顺序改造 |
| **Success Criteria** | AC-001 ~ AC-006 |
| **Final Verification** | `cargo test --release` -> exit 0（回归）；`cargo build` -> exit 0；`node --check` 抽取 JS -> exit 0 |
| **Primary Risk** | Low: 无后端改动，风险集中在抽屉交互状态机（查看/编辑/新建切换） |

## Goal

将 `frontend-dist/index.html` 的任务面板从"左侧列表 + 常驻右侧表单栏"重构为 Codex 式"全宽列表 + 按需右侧详情抽屉"：
- 列表页全宽展示（标题/副标题/搜索/筛选/列表/空态），右上角「创建」按钮
- 点击任务行 → 右侧滑出详情抽屉（约 360px）：状态、名称、指令/文案正文、配置项列表（计划/时间/动作/聚焦）、下次/上次触发、操作按钮组（立即运行、暂停/启用、编辑、删除、关闭）
- 抽屉内切换"编辑/新建"模式复用现有表单字段；保存后关抽屉刷新列表
- 建议卡片增加描述文字行；点击仍为"填入表单"（壳层无模型对话解析能力，表单是等效快捷方式；Codex 的对话式创建不做）

## Acceptance Criteria

- AC-001: 面板布局改为全宽列表，右上角「创建」按钮（黑色胶囊样式参考 Codex）；常驻右侧表单栏移除。
- AC-002: 点击任务行打开右侧详情抽屉（宽度约 360px，带关闭按钮；再次点击或 Esc 关闭）：显示状态徽标（已暂停灰/正常无）、任务名、指令（run）或提醒文案（notify）正文、配置列表（计划描述、动作、聚焦勾选态）、下次触发、上次触发。
- AC-003: 抽屉操作按钮：立即运行（复用 test_task）、暂停/启用（复用 toggle_task，按钮文字随状态切换）、编辑（抽屉切表单模式）、删除（confirm 后 delete_task）；操作后刷新。
- AC-004: 「创建」按钮与「编辑」共用抽屉内表单（现有字段：名称/计划/动作/文案/聚焦/指令/预览/取消）；保存成功关闭抽屉并刷新列表；编辑态 Esc 取消。
- AC-005: 列表行简化为：开关 + 时钟图标 + 名称 + 计划/动作 + 下次触发；行内 测试/编辑/删除 按钮移除（收进抽屉）；行 hover 提示"点击查看详情"。
- AC-006: 建议卡片每项增加描述文字行（一句话说明用途，参考 Codex 文案风格）；点击行为保持"填入表单"。
- AC-007: 空状态、筛选、搜索、排序行为保持现有逻辑不回归。

## Global Constraints

- 后端 `src-tauri/src/lib.rs` **零改动**（list_tasks 已返回全部字段，test_task/toggle_task/delete_task 已存在）。
- 不引入新框架/依赖；保持单文件内联 CSS/JS。
- 不新增持久化运行历史（抽屉只展示 next/last 触发时刻；Codex 的运行历史列表不做，避免数据层范围膨胀）。
- 面板覆盖右侧内容区 + 左侧 DSH 侧边栏可见的形态不变（280px 偏移约束不变）。
- 禁止漂移：不做对话式创建、不做通知策略多级选项（所有运行/仅失败）、不做任务历史持久化。

## File Boundaries

- `frontend-dist/index.html`: 任务面板 HTML/CSS/JS 全部重构点（面板容器、列表、新建抽屉、详情抽屉、建议区）。
- `.cowork-flow/plans/2026-08-15-codex-style-drawer.md`: 本计划（self）。

## Tasks

### Task 1: 全宽列表 + 「创建」按钮 + 移除常驻右栏

**Purpose**
- 面板主体改为全宽列表；右上角新增「创建」黑色胶囊按钮（点击 → 打开抽屉新建模式）；删除常驻 `tp-side` 右栏（建议区与表单移入新结构）。

**Code Fact Sources**
- Read: `frontend-dist/index.html` 现有面板结构（`#tasks-page`、`.tp-body/.tp-main/.tp-side`、`#task-form`、`#suggest-list`）

**File Boundaries**
- Modify: `frontend-dist/index.html`（面板 HTML 结构、`#tasks-page .tp-side` 相关 CSS 删除、`.tp-head` 布局、新增 `#btn-tasks-create`）

**Key Symbols**
- Change: `.tp-head` 布局（标题组 + 右侧「创建」按钮）、删除 `.tp-side` 及 `#task-form` 原位置
- Add: `#btn-tasks-create`（黑色胶囊，右上角）

**Implementation Notes**
```html
<!-- tp-head 改为 flex 两端：左侧标题+副标题，右侧 创建 按钮 -->
<!-- 建议区与表单改为"抽屉"容器（默认隐藏），不再常驻右栏 -->
```

**Test Proof**
- 无自动化；人工清单：面板打开后为全宽列表、右上「创建」按钮存在、无右侧常驻栏。

**Verification Command**
```bash
cargo build
node --check <抽取的JS>
```

**Completion Conditions**
- 面板全宽列表；「创建」按钮存在；`.tp-side` 常驻栏移除。

**Prohibited Drift**
- 不改后端 command。
- 不破坏窗口控制/服务检测逻辑。

### Task 2: 详情抽屉（查看模式）

**Purpose**
- 点击任务行打开右侧详情抽屉：状态、名称、指令/文案正文、配置列表、下次/上次触发、按钮组（立即运行/暂停/编辑/删除/关闭）。

**Code Fact Sources**
- Read: `frontend-dist/index.html` 现有 `loadTasks` 行渲染、`humanizeTime`/`lastText`/`schedText`

**File Boundaries**
- Modify: `frontend-dist/index.html`（新增抽屉 HTML/CSS/JS、`loadTasks` 行点击绑定）

**Key Symbols**
- Add: `#task-drawer`（固定右侧 360px 抽屉）、`openDrawer(task)`/`closeDrawer()`、`renderDrawerView(t)`
- Change: 任务行 li 增加点击（排除开关点击区域）；行内 测试/编辑/删除 按钮移除

**Implementation Notes**
```js
// 抽屉结构：
// [关闭X]  [状态徽标]  [任务名]  [指令/文案正文]
// 计划：工作日 17:30   动作：通知 + 聚焦   通知文案：xxx（如非默认）
// 下次触发：今天 17:30   上次触发：08-15 17:30
// [立即运行] [暂停/启用] [编辑] [删除]
// 立即运行 → invoke("test_task"); 暂停 → invoke("toggle_task"); 删除 → confirm
```

**Test Proof**
- 无自动化；人工清单：点击行开抽屉、Esc/关闭按钮关抽屉、按钮行为正确。

**Verification Command**
```bash
cargo build
```

**Completion Conditions**
- 抽屉打开显示全部信息与按钮；行内按钮移除；点击开关不触发行点击。

**Prohibited Drift**
- 不新增后端 command（复用 test_task/toggle_task/delete_task）。

### Task 3: 抽屉表单模式（新建/编辑）+ 建议卡片描述

**Purpose**
- 「创建」与「编辑」在抽屉内切换表单模式（复用现有表单字段与联动逻辑）；保存成功关抽屉刷新；建议卡片加描述行、点击填入表单。

**Code Fact Sources**
- Read: `frontend-dist/index.html` 现有 `resetForm`/`addTask`/表单联动/建议绑定

**File Boundaries**
- Modify: `frontend-dist/index.html`（抽屉内表单 HTML、`resetForm`/`addTask` 适配抽屉、建议 `data-desc`）

**Key Symbols**
- Change: 表单移入抽屉、`resetForm` 增加关抽屉语义（查看模式切换）、建议按钮绑定改为"填入表单 + 打开抽屉"
- Add: `renderDrawerForm(mode, task?)`、建议卡片 `<i>` 描述行

**Implementation Notes**
```js
// 抽屉三态：closed / view / form(new|edit)
// openCreate() → form(new)；行内"编辑" → form(edit)；保存成功 → view 刷新（或关抽屉）
// 建议点击 → openCreate() + fillForm(btn.dataset)（保留现有填入逻辑）
```

**Test Proof**
- 无自动化；人工清单：新建/编辑/取消全流程、建议填入、预览行、Esc。

**Verification Command**
```bash
cargo build
```

**Completion Conditions**
- 创建/编辑共用抽屉表单；保存后刷新；建议卡片有描述且可填入；无残留旧右栏引用。

**Prohibited Drift**
- 不改 `save_task` 提交字段契约。

### Task 4: 集成验证 + 文案

**Purpose**
- 面板标题/副标题/空态文案与 Codex 风格对齐；全量验证。

**Code Fact Sources**
- Read: `frontend-dist/index.html` 全部文案

**File Boundaries**
- Modify: `frontend-dist/index.html`（文案）
- Verify: 全量命令

**Implementation Notes**
- 副标题参考 Codex："安排提醒，或到点执行 DSH 任务"（保持诚实，不写"监测更新"之类做不到的）。
- 空态：图标 + "还没有定时任务" + 引导按钮（点击 → 打开创建抽屉）。

**Test Proof**
- `cargo test --release`（回归 12 例）；`cargo build`；`node --check`。

**Verification Command**
```bash
cargo test --release
cargo build
node --check <抽取的JS>
```

**Completion Conditions**
- 全部 AC 走查通过；文案诚实一致；构建与测试全绿。

**Prohibited Drift**
- 不新增功能范围外内容。

## Integrated Verification

1. `cargo test --release` → exit 0（12 例回归）。
2. `cargo build` → exit 0。
3. `node --check` 抽取内联 JS → exit 0。
4. DOM id 一致性检查：JS 引用的 id 全部存在于 HTML。
5. 人工走查清单（写入完成说明）：面板全宽列表 + 创建按钮；点击行开详情抽屉（信息与按钮完整）；立即运行弹通知；暂停/启用切换；编辑进入表单模式并保存；删除有确认；新建流程完整；建议卡片带描述且可填入；空态引导；Esc/关闭行为；开关不触发行点击。
