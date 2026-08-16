# Decision Anchor

## 目标
无 DSH_HOME 环境变量且无 config.json 覆盖时，主题文件路径解析到默认 DSH 数据目录 %USERPROFILE%\.dsh，恢复标题栏主题即时同步。

## 验收标准
- [ ] AC-001: resolve_dsh_home 的 USERPROFILE 兜底返回 <userprofile>/.dsh、最终兜底返回 ./.dsh；DSH_HOME env / config.json dsh_home 分支语义不变（原样直用）。
- [ ] AC-002: cargo test --lib（含更新用例）与 clippy 全绿；无 DSH_HOME 环境下 debug 应用端到端：改 settings.yaml 主题 ≤2s 跟随（CDP 冒烟 PASS）。

## 关键假设
- DSH_HOME env 与 config.json dsh_home 的语义本就是"DSH 数据目录"（DSH 侧约定 ~/.dsh），只有 USERPROFILE 兜底需要追加 .dsh。
- 端到端验证需要先关闭正在运行的旧 release 进程（单实例插件拦截新实例），验证后编译 release 交还用户。

## 范围边界
- 范围内: config.rs 的 resolve_dsh_home 与单测、theme.rs 路径注释、CHANGELOG.md；端到端验证与 release 重建。
- 范围外: parse_theme_preference、watcher 机制、冒烟脚本、DSH_URL 解析。

## 验证命令
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` → 全绿
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` → 0 告警
- `node scripts/smoke/theme-sync.mjs dark` → PASS（≤2s 跟随）
