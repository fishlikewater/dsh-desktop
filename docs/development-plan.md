# 开发计划（Development Plan）

> 本文档记录 DSH Desktop 的生产级改造任务分解（T0.1~T6.2），全部任务已完成并验收，作为历史与回归参照。后续优化方向（窗口状态记忆、Win11 贴靠、服务崩溃自动重启、代码签名等）已列入独立路线图计划，不在本历史文档内。

## 目标

把 MVP 壳层（窗口 + 托盘 + 快捷键）改造为可发布、可维护、可自动更新的生产级桌面应用：规范门禁、测试与 CI、安全加固、自动更新、服务自管理、设置页、错误恢复、性能基线、最终验收。

## 任务分解（已完成）

### T0 工程初始化

| 任务 | 内容 | 验收 |
|---|---|---|
| T0.1 | 初始化 git 仓库与提交基线 | 历史可追溯 |
| T0.2 | MIT 许可证、CHANGELOG、SECURITY 文档 | 文档齐备 |
| T0.3 | 代码规范门禁（`npm run check`：clippy `-D warnings` + 前端脚本/CSP hash 检查） | 门禁通过 |

### T1 架构与配置

| 任务 | 内容 | 验收 |
|---|---|---|
| T1.1 | Rust 模块化重构（config/logging/theme/tray/window） | 模块边界清晰 |
| T1.2 | 日志系统（tauri-plugin-log：文件输出 1MB 轮转 + debug stdout；关键路径审计日志） | 日志可查 |
| T1.3 | 启动时序去魔法数（居中线程 → 壳页事件驱动） | 无固定延迟 |
| T1.4 | 壳层配置层（config.json：dsh_url/dsh_home，环境变量优先）+ 主题解析/监听加固（当时基线 23 例单测；当前全库 29 例） | 单测通过 |

### T2 安全加固

| 任务 | 内容 | 验收 |
|---|---|---|
| T2.1 | 壳页 CSP（内联脚本 sha256 白名单，frame/connect 限 127.0.0.1/localhost） | CSP 生效 |
| T2.2 | capabilities 最小权限（6 项窗口写权限 + core:default） | 最小集 |
| T2.3 | 全局注入评估（iframe 能力层隔离：对象存在但 IPC 按 URL 粒度拒绝） | 冒烟验证 |

### T3 测试与 CI

| 任务 | 内容 | 验收 |
|---|---|---|
| T3.1 | Rust 单元测试（配置/主题/窗口/服务，含反例验证） | `cargo test` 绿 |
| T3.2 | 端到端冒烟脚本（`npm run smoke`，CDP 场景） | 冒烟绿 |
| T3.3 | CI 流水线（.github/workflows/ci.yml：fmt→clippy→单测→构建→NSIS 打包） | 全链路绿 |

### T4 发布与更新

| 任务 | 内容 | 验收 |
|---|---|---|
| T4.1 | 自动更新（tauri-plugin-updater + ed25519 密钥，发布物 latest.json） | 更新可达 |
| T4.2 | 发布流程（安装包核验、SHA256 校验和、本发布流程文档） | 流程可用 |
| T4.3 | GitHub Release 自动构建（release.yml：v* tag 触发，并行 macOS/Windows 打包并发布草稿，含签名 latest.json 与 SHA256SUMS） | 一键发布 |

### T5 服务自管理与恢复

| 任务 | 内容 | 验收 |
|---|---|---|
| T5.1 | 服务自管理（方案 B 纯拉起：不捆绑 sidecar；Windows taskkill /T / macOS 进程组 SIGTERM；仅停止本应用拉起的服务） | 拉起/停止冒烟通过 |
| T5.2 | 错误恢复（在线 10s / 离线 2s 自适应轮询；iframe 15s 无响应提示重试） | 恢复路径验证 |
| T5.3 | 服务不可用覆盖层（一键启动入口、状态轮询、恢复自动加载） | 覆盖层场景 |
| T5.4 | 设置页（服务地址只读、开机自启开关、检查更新 + 一键更新、打开日志目录） | 设置弹层验证 |

### T6 体验与最终验收

| 任务 | 内容 | 验收 |
|---|---|---|
| T6.1 | 原生窗口质感（无边框 + 自定义标题栏 + Mica/Acrylic 毛玻璃 + 伪最大化 + 自绘 resize + 无边框线/无缝隙） | 视觉验收 |
| T6.2 | 最终验收（见 [final-acceptance.md](final-acceptance.md)） | 全部通过 |

## 状态

- 以上 T0.1~T6.2 全部完成，验收记录见 [final-acceptance.md](final-acceptance.md)。
- 各版本迭代记录见仓库根 `CHANGELOG.md`。
- 后续优化方向（窗口状态记忆、Win11 贴靠、服务崩溃自动重启等）在独立路线图计划中维护。