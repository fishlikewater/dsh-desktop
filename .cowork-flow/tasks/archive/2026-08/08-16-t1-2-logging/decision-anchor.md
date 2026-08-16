# Decision Anchor

## 目标（T1.2，来自 docs/development-plan.md）
日志系统：全链路可观测，替换裸 `eprintln`。

## 验收标准
- [ ] `log` + `tauri-plugin-log`：文件输出（`%APPDATA%\com.dsh.desktop\logs\`，1MB 轮转）+ 调试构建保留 stdout。
- [ ] 全部 `eprintln` 替换为 log 宏（config/tray/lib）。
- [ ] 关键路径审计日志：托盘事件、全局快捷键、单实例唤起、watcher 启动/事件、窗口生命周期（创建/关闭请求）。
- [ ] 日志不包含敏感数据（token 等）。
- [ ] npm run check 全绿；运行后日志文件生成且内容覆盖关键路径。

## 范围边界
- 范围内: 日志插件接入、eprintln 替换、关键路径审计日志。
- 范围外: 日志格式/轮转策略的 UI 配置、远程日志、配置层（T1.4）。

## 验证命令
- npm run check
- 启动应用 → 检查 %APPDATA%\com.dsh.desktop\logs\ 下日志内容（含托盘/快捷键/watcher 条目）
