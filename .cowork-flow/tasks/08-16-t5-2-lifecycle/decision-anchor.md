# Decision Anchor

## 目标（T5.2，来自 docs/development-plan.md）
服务生命周期管理（方案 B 纯拉起）：托盘菜单提供启动/停止服务；停止仅作用于**本应用拉起的**服务进程树，不误杀用户自启的服务；壳层退出不关闭服务。

## 实施
1. service.rs：核心逻辑提为 `start_service()` / `stop_service()`（command 包装调用）：
   - `stop_service()`：仅当 SERVICE_CHILD 记录且进程存活时 `taskkill /PID <pid> /T /F`（进程树），
     否则返回提示"未找到由本应用拉起的服务"
2. tray.rs：托盘菜单新增"启动 DSH 服务"、"停止 DSH 服务"（分隔符分组），
   点击结果用系统通知反馈（tauri-plugin-notification Rust API）
3. 壳层退出：不杀服务（服务独立生命周期，重启壳层继续可用）

## 验收标准
- [ ] 托盘两项存在且可点击；结果通知可见（日志验证）。
- [ ] 假 dsh 拉起 → 托盘停止 → 进程树退出（标记/进程验证），服务进程确实被杀。
- [ ] 未拉起过服务时点停止 → 返回提示，不误杀其他 dsh 进程。
- [ ] 壳层退出后服务进程存活（不主动关闭）。
- [ ] npm run check 全绿 + 单测（stop 逻辑判定）。

## 验证命令
- 实测：假 dsh 场景（T5.1 方式）→ 托盘停止 → Get-Process 验证
- cargo test
