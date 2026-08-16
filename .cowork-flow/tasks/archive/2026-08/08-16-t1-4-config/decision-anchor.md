# Decision Anchor

## 目标（T1.4，来自 docs/development-plan.md）
配置层与主题解析加固：壳层自有配置（config.json）+ 解析健壮性 + watcher 重试。

## 验收标准
- [ ] 壳层配置 `%APPDATA%\com.dsh.desktop\config.json`：`dsh_url`、`dsh_home` 字段；缺失/损坏回退默认；环境变量仍最高优先级（DSH_URL/DSH_HOME）。
- [ ] 配置读取通过静态缓存（OnceLock），`dsh_url()`/`theme_settings_path()` 签名不变（调用点零改动）。
- [ ] `theme_preference` 解析提取为可测函数 `parse_theme_preference(content)`：正常/损坏 YAML/缺失段/未知值/空白 6+ 用例单测。
- [ ] 配置解析单测：默认/覆盖/非法值/优先级 4+ 用例。
- [ ] watcher 失败时告警 + 30s 周期重试，成功即停。
- [ ] npm run check（含 cargo test）全绿。

## 范围边界
- 范围内: config.json 配置层、解析提取与单测、watcher 重试。
- 范围外: GUI 设置页、auto_manage_service 等行为开关（T5.4）、日志配置。

## 验证命令
- npm run check（clippy + test + 前端）
- cargo test 单测全绿
- 手动:改 config.json 的 dsh_url → 启动验证日志 DSH_URL 生效
