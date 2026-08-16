# Decision Anchor

## 目标（T5.1，来自 docs/development-plan.md）
DSH 服务自管理（方案 B：纯拉起系统 dsh，不捆绑 sidecar）：
覆盖层提供"启动 DSH 服务"按钮，壳层拉起系统 dsh。

## 环境事实
- `dsh` 为 npm 全局 shim（`D:\Program Files\nvm4w\nodejs\dsh.ps1` + dsh.cmd），
  无 dsh.exe；Rust `Command::new("dsh")` 不解析 .cmd/.ps1，需按扩展名选择启动方式。

## 实施
1. `src-tauri/src/service.rs`：
   - `find_dsh()`：DSH_CLI 环境变量 > PATH 搜索（dsh.cmd / dsh.exe / dsh.ps1）
   - `#[tauri::command] service_start() -> Result<u32, String>`：3080 已可达 → 幂等 Ok(0)；
     否则拉起（.cmd → `cmd /c`；.ps1 → `powershell -NoProfile -File`），返回 pid
   - 子进程句柄 OnceLock 保存（防回收）；下次 start 前 try_wait 检测存活
2. 前端覆盖层：服务未运行时显示"启动 DSH 服务"按钮 → invoke service_start；
   成功/失败提示；2s 轮询发现服务在线后自动进入
3. lib.rs 注册 service 模块与 command

## 验收标准
- [ ] 服务在线时 service_start 幂等返回（不重复拉起）。
- [ ] 覆盖层按钮：点击 → dsh 进程被拉起 → 轮询进入 GUI（实测，用备份的服务进程验证方式）。
- [ ] 找不到 dsh 时返回可读错误（前端提示安装 dsh）。
- [ ] npm run check 全绿；单测（find_dsh 路径解析逻辑）。

## 验证命令
- 实测：临时改 DSH_URL 到未启动端口 → 覆盖层出现按钮 → 点击拉起
- cargo test（service 解析单测）
