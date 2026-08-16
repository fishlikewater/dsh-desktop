# Decision Anchor

## 目标
把 dsh-desktop 壳层的定时任务功能完整提取为 DSH cordis 插件 `@fishlikewater/dsh-tasks`（开发目录 `E:\Projects\IdeaProjects\person\dash-plugin`，symlink 安装进 DSH web profile），并从壳层移除全部定时任务代码/配置（托盘「定时任务」入口保留，点击打开 DSH GUI）。

## 验收标准
- [ ] AC-001: 插件工程结构完整（package.json/cordis.patch.yml/lib/index.js/lib/client.js），node --check 通过。
- [ ] AC-002: DSH 重启后插件加载成功（日志无错误）。
- [ ] AC-003: GUI 侧边栏底部「定时任务」入口 → 面板（列表/创建/编辑/启停/删除/立即运行）。
- [ ] AC-004: 调度生效（间隔 1 分钟提醒实测 → 未读通知 + Web Notification）。
- [ ] AC-005: run 任务到点执行 DSH agent（默认模型、工作区 cwd），会话在 GUI 历史可见。
- [ ] AC-006: 重启 DSH 任务列表保留。
- [ ] AC-007: 壳层 lib.rs 无定时任务残留（编译/测试通过），托盘项点击打开 GUI。
- [ ] AC-008: 壳层 index.html 无任务面板残留，标题栏时钟按钮移除。
- [ ] AC-009: 壳层 cargo test --release 全绿。
- [ ] AC-010: 壳层 npm run build 打包成功。
- [ ] AC-011: README 更新为插件形态。
- [ ] AC-012: 旧 tasks.json 不迁移（记录说明）。

## 被拒方案
- **Tauri 插件形态（rust crate）**: 拒绝原因——用户明确要求 dsh 插件；DSH cordis 插件才能提供 GUI 内原生 UI 与进程内 agent 执行。
- **保留壳层执行+插件仅 UI**: 拒绝原因——功能仍耦合壳层，未达"去除定时任务代码"目标。
- **通知走系统 toast 通道**: 拒绝原因——DSH 插件运行在服务端无 Windows 通知通道；用户已确认用 Web 通知 + 面板内提示。
- **自动迁移旧 tasks.json**: 拒绝原因——数据量小、格式已文档化；避免引入迁移面（记录说明即可）。

## 关键假设
- DSH web profile 支持本地插件 symlink + cordis.patch.yml insert（dsh-vision-toolkit 已验证此安装模式）。
- `agents.create`/`followup`/`whenIdle`/`sessions.flush` 可复用于定时执行（dsh-headless 已验证）。
- client 端 fetch 同源路由（webServer）可行（vision-toolkit 已验证）。
- `sidebar.footer.action` slot 可用于放置任务入口（ui-cordis 已占用同 slot，order 区分）。

## 范围边界
- 范围内: 插件 host/client 端、安装配置、壳层移除、托盘入口保留（打开 GUI）、重新打包。
- 范围外: 运行历史持久化、模型选择、多级通知策略、旧数据自动迁移、壳层其他功能改动。

## 验证命令
- `node --check lib/index.js lib/client.js`
- DSH 重启 + GUI 截图验证
- `cargo test --release` / `cargo build` / `npm run build`
