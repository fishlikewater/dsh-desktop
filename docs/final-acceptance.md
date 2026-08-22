# 最终验收（Final Acceptance）

> 本文档记录 v0.1.x 生产级改造的最终验收结果（T0.1~T6.2）。只记录已完成事项；进行中或未开始的不写入本项目。

## 验收范围

生产级改造计划（[production-hardening-plan.md](production-hardening-plan.md)）中 T0~T6 的全部任务，验收对象为 v0.1.1~v0.1.5 期间的代码基线（当前 HEAD）。

## 验收结论

**T0 工程初始化：通过**

- [x] T0.1 git 仓库与提交基线（历史可追溯）
- [x] T0.2 MIT 许可证、CHANGELOG、SECURITY 文档
- [x] T0.3 规范门禁（`npm run check`：fmt + clippy -D warnings + 前端/CSP hash + 版本一致性）

**T1 架构与配置：通过**

- [x] T1.1 Rust 模块化（config/logging/theme/tray/window/service/settings）
- [x] T1.2 日志系统（文件 1MB 轮转 KeepSome(5)，debug stdout）
- [x] T1.3 启动时序事件驱动（无固定延迟线程）
- [x] T1.4 壳层配置层 + 主题解析/监听加固（单测通过）

**T2 安全加固：通过**

- [x] T2.1 壳页 CSP（内联 sha256 白名单 + frame/connect 限本机）
- [x] T2.2 capabilities 最小权限（6 项窗口写权限 + core:default）
- [x] T2.3 iframe 隔离（CDP 冒烟验证：远程 GUI 无 Tauri IPC 权限）

**T3 测试与 CI：通过**

- [x] T3.1 Rust 单测（config 13 / theme 9 / window 5 / service 2，共 29 例），含反例（非法 JSON、非法 URL、空值回退）
- [x] T3.2 CDP 冒烟（窗口居中 / 主题同步 / 隔离边界 / 窗口控制权限；另验证拖放模拟）
- [x] T3.3 CI 流水线（ci.yml：fmt → clippy → 单测 → debug/ release 构建 → NSIS 打包）

**T4 发布与更新：通过**

- [x] T4.1 自动更新（tauri-plugin-updater + ed25519；updater-check 冒烟可达）
- [x] T4.2 发布流程（本文件 + release-process.md + SHA256 校验和）
- [x] T4.3 Release 自动构建（release.yml：v* tag，双平台矩阵 + latest.json 聚合 + SHA256SUMS）

**T5 服务自管理与恢复：通过**

- [x] T5.1 服务自管理（Win：PATH 探测 + cmd/ps1/exe 拉起 + taskkill /T /F 停止；mac：目录/登录 shell 探测 + 富 PATH + 进程组 SIGTERM/SIGKILL；仅停止本应用拉起的服务）
- [x] T5.2 错误恢复（在线 10s / 离线 2s 自适应轮询；iframe 15s 超时提示重试）
- [x] T5.3 覆盖层（一键启动入口、状态轮询、恢复自动加载）
- [x] T5.4 设置页（地址只读展示、版本、开机自启开关、检查更新/一键更新、打开日志目录）

**T6 体验与最终验收：通过**

- [x] T6.1 窗口质感（无边框 + 伪最大化 + 自绘 resize + Mica/Acrylic + 无边框线/无缝隙 + 无闪烁启动 + 小桌面 clamp）
- [x] T6.2 本文档验收清单核对完毕

## 验收环境

| 项 | 值 |
|---|---|
| Windows | Windows 10/11，WebView2 运行时，workspace（本机开发机） |
| macOS | macOS（Arch64 Apple Silicon），原生全屏/红绿灯验证 |
| Rust 工具链 | stable x86_64-pc-windows-gnu（Windows）；stable（macOS） |
| DSH 服务 | 本机 dsh（--profile web，默认 3080） |

## 验收方法与证据

- 静态门禁：`npm run check` → exit 0（每次改动门禁，CI 同步执行）
- 单测：`cargo test --manifest-path src-tauri/Cargo.toml --lib` → 全绿
- 冒烟：`npm run smoke` → 场景全 PASS（需 debug 构建 + 本机 DSH）
- 发布物：CI/release 构建产物 + SHA256SUMS 核验
- 已知限制（不阻断验收，已列入后续优化方向）：代码签名/公证未完成；多会话窗口未实现；CI 冒烟未 mock 化。

## 后续

验收通过后的优化方向（窗口状态记忆、贴靠、状态指示、崩溃自动重启、代码签名等）在独立路线图计划中维护，不在本验收文档内展开。