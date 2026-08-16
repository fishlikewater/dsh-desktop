# Decision Anchor

## 目标
修复用户反馈三问题：①DSH GUI 侧边栏收起时面板左侧留白——面板支持「展开 280px / 收起 56px」两档侧边栏对齐并持久化；②创建表单明确「任务标题」字段、指令文本框加大；③run 任务支持「工作区」选择（原生文件夹选择器 + 手输路径），执行时在该工作区运行。

## 验收标准
- [ ] AC-001: 面板头部「侧边栏：展开/收起」切换；面板左侧偏移与拦截层宽度同步（280↔56px）；localStorage 持久化，重开面板保持。
- [ ] AC-002: 「任务标题」有显式标签；run 指令 textarea min-height 140px；notify 文案多行输入。
- [ ] AC-003: run 模式「工作区」字段：手输路径 + 「浏览…」原生文件夹选择器回填；留空提示用 DSH 主目录。
- [ ] AC-004: `save_task`/`list_tasks` 透传 workspace；旧 tasks.json 无该字段反序列化正常（serde default）。
- [ ] AC-005: run 触发时 current_dir = workspace（存在时）否则 DSH_HOME；路径非法时 spawn 失败并通知。
- [ ] AC-006: 详情抽屉 run 任务显示工作区（未设置显示「DSH 主目录」）。
- [ ] AC-007: `cargo test --release` 全绿；`cargo build` 通过；JS 语法/id 一致通过；`npm run build` 重新打包成功。

## 被拒方案
- **自动检测侧边栏状态**: 拒绝原因——iframe 跨域无法读取 DSH GUI 内部布局，壳层无可靠感知手段；手动两档切换 + 持久化是诚实可行的方案。
- **仅文本输入工作区（不加 dialog 插件）**: 拒绝原因——手输路径体验差；tauri-plugin-dialog 是官方标准插件。若 cargo 离线下载失败则回退该方案。
- **"运行于当前聊天" / 模型推理选择**: 拒绝原因——壳层无法控制 DSH GUI 会话；headless 不支持命令行指定模型（沿用既定结论）。

## 关键假设
- DSH GUI 侧边栏：展开默认 280px、折叠 rail 56px（源码 computeColumns 确认：sidebar 0 → 56）。
- tauri-plugin-dialog v2 可从 crates.io 正常下载（本机构建网络可用）。
- 工作区即 headless 运行目录（`dsh --profile headless` 以运行目录为 workspace 根）。

## 范围边界
- 范围内: 侧边栏两档对齐、表单标题标签/大文本框/工作区字段、后端 workspace 字段与执行目录、抽屉工作区行、重新打包。
- 范围外: 自动检测侧边栏、运行于当前聊天、模型/推理、通知策略多级、任务运行历史持久化。

## 验证命令
- `cargo test --release` → exit 0
- `cargo build` → exit 0
- `npm run build` → exit 0
