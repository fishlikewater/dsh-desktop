# Decision Anchor

## 目标（T3.2，来自 docs/development-plan.md）
端到端冒烟脚本入库：CDP 验证正式化，从 .toolchain-test/ 迁入 scripts/smoke/。

## 迁移清单
| 来源（.toolchain-test/） | 目标（scripts/smoke/） | 说明 |
|---|---|---|
| cdp-verify.mjs | theme-sync.mjs | 主题即时同步（light↔dark） |
| cdp-window-ctrl.mjs | window-ctrl.mjs | 窗口控制权限（伪最大化/最小化/show） |
| cdp-isolation-check.mjs | isolation.mjs | 隔离边界 + iframe IPC 拒绝（并入 iframe-ipc 检查） |
| cdp-iframe-ipc.mjs | （并入 isolation.mjs） | iframe 内 IPC 全部被拒 |
| cdp-center-verify.mjs | window-center.mjs | 窗口居中（修复：等待窗口稳定后采样） |

## 新增
- `scripts/smoke/run-all.mjs`：顺序执行全部场景 + 前置检查（debug exe 存在、DSH 服务在线、CDP 可达）；失败给出可操作诊断。
- `package.json` `smoke` 脚本。
- `docs/testing.md`：运行方式、前置条件、CI 接入点说明。

## 验收标准
- [ ] 5 个场景脚本全部入库且通过；run-all 一键执行通过。
- [ ] 前置检查：服务未启动时给出明确提示而非莫名失败。
- [ ] npm run check 全绿（冒烟脚本 node --check 语法）。
- [ ] 旧 .toolchain-test/ 脚本清理（已 gitignore，删除避免双份维护）。

## 验证命令
- node scripts/smoke/run-all.mjs（启动应用 + DSH 服务在线时全过）
- npm run check
