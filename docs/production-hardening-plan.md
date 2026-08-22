# 生产级改造计划（Production Hardening Plan）

> 本文档记录 v0.1.x 阶段的生产级改造总体计划（问题清单 → 改造方向 → 完成状态）。
> 改造任务已全部完成（见 [development-plan.md](development-plan.md) 与 [final-acceptance.md](final-acceptance.md)）；本文件保留作为决策记录。

## 背景与问题清单

MVP 壳层（v0.1.0）已具备窗口、托盘、全局快捷键、服务检测、主题跟随等基础能力，但距离可发布、可自动更新的生产级应用仍有以下差距：

1. **无规范门禁**：代码提交无 clippy/fmt/前端检查强制约束。
2. **无自动化测试**：仅手工验证，回归风险高。
3. **无安全加固**：壳页无 CSP、capabilities 权限未收敛、远程 GUI 与本地 IPC 边界未验证。
4. **无自动更新**：用户需手动下载新版本。
5. **服务依赖手工启动**：用户需自行运行 `dsh`，服务中途崩溃无恢复引导。
6. **无设置入口**：开机自启、地址查看、日志访问无统一 UI。
7. **无发布流程**：无安装包流水线、无校验和、无 changelog 纪律。
8. **无性能基线**：启动/内存/CPU 无量化指标。

## 改造方向

| 方向 | 目标 | 落地位置 |
|---|---|---|
| 规范门禁 | `npm run check`（fmt + clippy -D warnings + 前端/CSP hash） | scripts/check-*、.github/workflows/ci.yml |
| 模块化 | config/logging/theme/tray/window/service/settings 分模块 | src-tauri/src/ |
| 日志 | 文件 1MB 轮转 + 关键路径审计 | src-tauri/src/logging.rs |
| 安全 | CSP sha256 白名单、最小 capabilities、iframe 隔离验证 | tauri.conf.json、capabilities/default.json |
| 测试 | Rust 单测（配置/主题/窗口/服务）+ CDP 冒烟 | src-tauri/src/*::tests、scripts/smoke/ |
| CI | fmt → clippy → 单测 → 构建 → NSIS 打包 | .github/workflows/ci.yml |
| 更新 | tauri-plugin-updater + ed25519 + latest.json 双平台 | tauri.conf.json plugins.updater |
| 发布 | v* tag 触发，双平台产物 + SHA256SUMS + Release 草稿 | .github/workflows/release.yml |
| 服务 | 方案 B 纯拉起（不捆绑 sidecar），进程树停止，覆盖层一键启动 | src-tauri/src/service.rs |
| 恢复 | 在线 10s/离线 2s 自适应轮询 + 15s 加载超时提示 | frontend-dist/index.html |
| 设置 | 服务地址/版本/开机自启/更新/日志目录 | frontend-dist/index.html 设置弹层 |
| 体验 | 无边框质感、Mica 毛玻璃、无缝隙、事件驱动居中、无闪烁启动 | src-tauri/src/window.rs |

## 执行记录

- 改造按 T0.1~T6.2 分解执行，全部完成；详细任务表见 [development-plan.md](development-plan.md)。
- 版本里程：T 系列改造主体在 v0.1.1 合入；v0.1.2 补设置页排版/更新按钮；v0.1.3/v0.1.4/v0.1.5 完成 macOS 适配（原生标题栏/全屏/服务自管理/图标重建）。
- 遗留（后续优化方向）：代码签名与公证、多会话窗口、Linux 评估等。

## 决策记录（被采纳/被拒）

- **服务自管理采用方案 B（纯拉起）而非方案 A（捆绑 sidecar）**：不捆绑 dsh 运行时，安装包小、与 dsh 版本解耦；代价是依赖用户已安装 DSH CLI。方案 A 作为可选能力另行评估（需解决 node 运行时来源）。
- **冒烟不进 CI**：端到端冒烟依赖本机 DSH 服务与 `~/.dsh` 配置，CI runner 无法复现，作为本地 `npm run smoke`；mock 化改造已列入后续优化方向。
- **无边框窗口弃用系统最大化**：DWM 最大化会 -8px 偏移裁掉标题栏，改壳页伪最大化（手动贴齐工作区）+ 自绘 resize 热区 + 样式守卫（WS_POPUP）。