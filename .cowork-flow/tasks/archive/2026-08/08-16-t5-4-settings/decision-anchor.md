# Decision Anchor

## 目标（T5.4，来自 docs/development-plan.md）
壳层设置页：托盘菜单"设置"入口 + 设置弹层（服务地址、开机自启、检查更新、日志目录）。

## 实施
1. 壳页新增设置弹层（`#settings-modal`）：从标题栏齿轮按钮或托盘菜单"设置"打开
   - 服务地址：只读显示当前 DSH_URL（配置层 T1.4 决定启动时读取；运行时改需重启，标注说明）
   - 开机自启：复选框（调 autostart 插件的 Rust command？前端无权限——新增 Rust command `autostart_toggle`/`autostart_state` 或复用托盘逻辑）
   - 检查更新：按钮（updater check，T4.1 已注册插件；新增 Rust command `update_check` 触发并返回结果）
   - 打开日志目录：按钮（command 打开资源管理器）
2. 托盘菜单加"设置"项 → 显示窗口 + 触发壳页打开弹层（event `open-settings`）
3. command 集：`autostart_state`、`autostart_toggle`、`update_check`、`open_log_dir`

## 验收标准
- [ ] 弹层打开/关闭正常（标题栏按钮 + 托盘菜单双入口）
- [ ] 开机自启复选框：读取真实状态、切换生效（注册表验证）
- [ ] 检查更新：endpoint 404 时返回"无可用更新/检查失败"且不崩溃
- [ ] 打开日志目录：资源管理器打开 %LOCALAPPDATA%\com.dsh.desktop\logs
- [ ] npm run check 全绿；CDP 冒烟通过

## 验证命令
- CDP：开弹层、切换自启、update_check 调用、open_log_dir
- 注册表核对自启项
