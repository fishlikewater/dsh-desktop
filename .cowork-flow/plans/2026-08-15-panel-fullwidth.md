# 面板全宽化与侧边栏交互修复 实施计划

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 修复两问题：①GUI 侧边栏收起时面板左侧留白——面板默认全宽覆盖（left:0）从根上消除对侧边栏状态的依赖；②点击「收起侧边栏」误退出面板——移除 #tasks-shield 拦截层。面板头部提供三档「侧边栏」模式（全宽/展开280/收起56）localStorage 持久化。 |
| **Task Type** | Normal: 纯前端改动（frontend-dist/index.html） |
| **Strategy** | Serial: HTML/CSS → JS → 验证 → 打包 |
| **Success Criteria** | AC-001 ~ AC-005 |
| **Final Verification** | `node --check`；渲染预览三态；`cargo build`；`npm run build` |

## Goal

1. 移除 `#tasks-shield` 元素及全部相关 JS（openTasks/backToGui/点击绑定）——侧边栏可操作，不再误退。
2. 面板默认全宽：`#tasks-page` left:0（CSS 默认值改为 0；不再有 shield）。
3. 头部「侧边栏」控件由按钮改为三档 select：全宽（left:0）/ 展开（left:280）/ 收起（left:56），localStorage `taskSidebarMode`（full|wide|rail，默认 full），打开面板时套用。
4. 退出面板途径保持：✕ / Esc / 标题栏时钟 / 托盘。

## Acceptance Criteria

- AC-001: shield 移除；侧边栏收起/展开按钮点击不再退出面板。
- AC-002: 默认全宽打开，无左侧留白（任何侧边栏状态）。
- AC-003: 三档 select 切换生效且持久化。
- AC-004: 退出途径正常。
- AC-005: JS 语法、id 一致性、渲染预览三态、cargo build、npm run build 全部通过。

## File Boundaries

- `frontend-dist/index.html`: shield 移除、侧边栏 select、JS 三态逻辑。

## Tasks

### Task 1: 移除 shield + 三档侧边栏模式

**Purpose**
- 消除拦截层误退；面板默认全宽；三档切换。

**Code Fact Sources**
- Read: `frontend-dist/index.html` 中 tasks-shield 相关（HTML 注释区、openTasks/backToGui/绑定）、btn-sidebar-mode 相关（CSS/HTML/JS）

**File Boundaries**
- Modify: `frontend-dist/index.html`

**Key Symbols**
- Remove: `#tasks-shield` 元素、`document.getElementById("tasks-shield")` 两处调用、shield 点击绑定
- Change: `#tasks-page` CSS left:280px → 0（默认全宽）；`#btn-sidebar-mode` 按钮 → `#sidebar-mode` select（full/wide/rail 三档）
- Change: `sbMode` 三态逻辑 + `applySidebarMode()`（left = 0/280/56）+ localStorage 默认 "full"

**Implementation Notes**
```html
<select id="sidebar-mode" title="面板与 DSH 侧边栏的对齐方式">
  <option value="full">侧边栏：全宽</option>
  <option value="wide">侧边栏：展开</option>
  <option value="rail">侧边栏：收起</option>
</select>
```
```js
const SB_MODE_KEY = "taskSidebarMode";
let sbMode = localStorage.getItem(SB_MODE_KEY) || "full"; // full|wide|rail
function applySidebarMode() {
  const w = sbMode === "wide" ? 280 : sbMode === "rail" ? 56 : 0;
  tasksPage.style.left = w + "px";
  const sel = document.getElementById("sidebar-mode");
  if (sel.value !== sbMode) sel.value = sbMode;
}
document.getElementById("sidebar-mode").addEventListener("change", function (e) {
  sbMode = e.target.value;
  localStorage.setItem(SB_MODE_KEY, sbMode);
  applySidebarMode();
});
// openTasks() 调用 applySidebarMode()；backToGui() 不再触碰 shield
```

**Verification Command**
```bash
node --check <抽取JS>；id 一致性；渲染预览三态
```

**Completion Conditions**
- shield 无残留引用；三档切换生效；默认全宽。

### Task 2: 验证 + 重新打包

**Purpose**
- 全量回归 + 打包新版（用户诉求）。

**Verification Command**
```bash
cargo build
npm run build
```

**Completion Conditions**
- 全部 AC 通过；安装包重新生成。

## Integrated Verification

1. `node --check` → exit 0；id 一致性（无 tasks-shield 残留引用）。
2. 渲染预览：full/wide/rail 三态截图 + 视觉分析。
3. `cargo build` → exit 0。
4. `npm run build` → 安装包重新生成（先退出运行中实例）。
