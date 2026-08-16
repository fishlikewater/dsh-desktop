# Decision Anchor

## 目标（T2.2，来自 docs/development-plan.md）
capabilities 最小权限：权限面收敛到壳页实际调用。

## 壳页实际调用清单（盘点自 frontend-dist/index.html）
- 写操作（需显式权限）：minimize、close、show、startDragging、setPosition、setSize
- 读操作（core:window:default 已含）：outerSize、outerPosition（getCurrentWindow 无需权限）
- 事件：event.listen（core:default 提供）
- 自定义 command：work_area（Rust 侧，无需前端权限）

## 删除清单（未调用或 core:default 冗余）
allow-maximize、allow-unmaximize、allow-toggle-maximize、allow-hide、allow-set-focus、
allow-unminimize、allow-get-all-windows、allow-is-maximized、allow-outer-position、
allow-outer-size、allow-current-monitor

## 验收标准
- [ ] default.json 仅保留：core:default + 6 项窗口写权限。
- [ ] 实测壳页功能全过：最小化/关闭(隐藏)/显示/拖拽/resize/伪最大化/主题同步。
- [ ] npm run check 全绿。
- [ ] 权限清单与调用面一一对应（decision-anchor 记录理由）。

## 验证命令
- npm run check
- 启动应用 + CDP：窗口控制调用、主题冒烟、伪最大化
