# Decision Anchor

## 目标
①移除托盘菜单「定时任务」菜单项（定时任务已迁移为 DSH 插件，壳层入口完全清除）；②标题栏/覆盖层配色跟随 DSH 外观主题设置（~/.dsh/settings.yaml 的 ui-theme.preference：light|dark|system，3s 轮询同步，system 跟随系统 prefers-color-scheme）。

## 验收标准
- [ ] 托盘菜单无「定时任务」项（「显示主窗口/开机自启/发送测试通知/退出」）。
- [ ] 壳层新增 theme_preference command（读取 ui-theme.preference，段内解析）。
- [ ] 前端深色主题变量（body.theme-dark）+ system 模式 media query + 3s 轮询同步。
- [ ] 实测：preference=dark 时标题栏变深色、文字变浅色（截图验证通过）；恢复 light。
- [ ] cargo build/test、JS 语法、id 一致性、npm run build 打包通过。

## 被拒方案
- **iframe 内监听 DSH 主题变化事件**: 拒绝——跨域无法监听；settings.yaml 轮询是 DSH 官方持久化路径。
- **CSS 仅跟随系统**: 拒绝——用户手动在 DSH 里选 dark/light（不跟随系统）时标题栏会不同步；需读持久化偏好。

## 范围边界
- 范围内: 托盘项移除、theme_preference command、前端主题变量与轮询、README、打包。
- 范围外: DSH GUI 自身主题逻辑、其他壳层 UI 改动。

## 验证命令
- cargo build / cargo test --release
- node --check、id 一致性
- 实测 preference 切换 + 窗口截图对比
- npm run build
