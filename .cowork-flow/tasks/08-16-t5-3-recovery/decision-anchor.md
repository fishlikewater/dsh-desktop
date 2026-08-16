# Decision Anchor

## 目标（T5.3，来自 docs/development-plan.md）
错误恢复与降级：服务中途崩溃/重启的自动恢复；覆盖层与轮询兜底加固。

## 现状梳理
- 服务检测：壳页 2s 轮询（fetch no-cors），离线显示覆盖层，在线自动加载 iframe ✓
- 服务中途崩溃：覆盖层立即出现，恢复后自动加载 ✓（已有机制）
- iframe 加载失败（如 DSH 服务 404/异常页）：frame 的 load 事件仍触发 → 窗口显示但内容异常。
  需要：iframe 加载失败提示 + 重试。
- 主题同步失败降级：watcher 30s 重试 ✓（T1.4）；壳页 30s 兜底轮询 ✓

## 实施
1. iframe 错误检测：`frame.onerror`（WebView2 对 http 错误页会触发 load 而非 error；
   改为检查 load 后 frame 内容是否可达？务实方案：iframe load 后 3s 内探测
   `contentDocument`（跨源为 null 无法判断）→ 用 CDP 不可行（运行时）——
   改为：覆盖层在"服务在线但页面加载失败"时提供"重新加载"按钮（已有 retry 逻辑复用），
   load 事件超时（15s 未 load → 显示重试提示））
2. 轮询自适应：服务在线时轮询间隔从 2s 放宽到 10s（省资源），离线时回到 2s
3. 进程内错误恢复：Rust 侧 panic 不导致静默退出（tauri 默认 panic 退出？）。
   不引入 panic 捕获（Tauri 2 无内置）；改为：setup 失败时日志 error + 提示。

## 验收标准
- [ ] 轮询自适应：在线 10s / 离线 2s（CDP 观察 tick 行为或代码审查 + 实测计时）。
- [ ] iframe load 超时（15s）→ 覆盖层提示"页面加载超时" + 重试按钮可重新加载。
- [ ] 服务中途停止 → 覆盖层出现；恢复 → 自动进入（实测）。
- [ ] npm run check 全绿。

## 验证命令
- 实测：服务在线 → 停止服务（杀 dsh 进程？用假服务方式：config 指向 5999 不可达 → 覆盖层；
  再启动假 dsh 于 5999？假 dsh 是 cmd 不监听端口。改为：用 node 起临时 http 服务于 5999
  模拟恢复）→ 自动进入
- CDP 验证轮询间隔
