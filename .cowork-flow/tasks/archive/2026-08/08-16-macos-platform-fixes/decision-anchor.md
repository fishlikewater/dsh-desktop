# Decision Anchor

## 目标
修复 macOS（aarch64-apple-darwin）构建编译失败（Windows-only FFI/进程 API 平台隔离），修复 5 项真实缺陷（服务探活 panic、URL 编码丢字符、innerHTML 注入面、停止失败丢 pid、日志无限增长），并清理过时注释与文档不一致（不扩功能）。

## 验收标准
- [ ] AC-001: cargo clippy --all-targets -- -D warnings 零告警；cargo test --lib 全绿（23 例 + 新增编码用例）。
- [ ] AC-002: Windows-only 调用点全部 cfg 隔离；cargo check --target x86_64-apple-darwin 通过（若环境受限失败则如实记录、以 CI 为最终验证，不伪造）。
- [ ] AC-003: P1 五项修复落地：socket_addrs 空结果不 panic；编码单测覆盖 #/+空格/非 ASCII；target-line 无 innerHTML 拼接且 npm run check（CSP hash 同步）通过；stop_service 失败保留 pid 记录；logging 改 KeepSome(5)。
- [ ] AC-004: docs/testing.md CI 段落与实际一致；README 日志描述更新；CHANGELOG Unreleased 补发布自动化与本轮修复条目。

## 被拒方案
- **macOS 实现真实服务管理（spawn/kill）**: 拒绝原因——超出"编译可用"目标，DSH 在 mac 的部署形态未知；stub 返回可读错误即可（不扩功能）。
- **引入 percent-encoding crate**: 拒绝原因——编码逻辑 ~15 行手写 + 单测即可，不为一处功能加依赖。
- **日志改 KeepOne**: 拒绝原因——KeepSome(5) 保留最近 5 份轮转（约 6MB 上限），排查价值与磁盘上限兼顾。
- **HEAD 探测 / resize pointer capture / 开机自启逻辑抽取**: 拒绝原因——本次范围仅 P0+P1+P2 注释/文档一致性，此三项记录为后续可选。

## 关键假设
- macOS 服务管理不做（stub）；macOS 窗口不加 vibrancy 效果（标题栏纯色可用）。
- cargo check --target x86_64-apple-darwin 在本机（windows-gnu 工具链 + 预编译 std）可执行类型检查；若构建脚本/依赖阻碍则失败并记录。
- windows-gnu 测试规避规则不变：新增单测仅纯字符串逻辑（encode_query_param），不触碰 Child/async/fs。

## 范围边界
- 范围内: window.rs、settings.rs、service.rs、config.rs、lib.rs、logging.rs、frontend-dist/index.html、tauri.conf.json（仅 CSP hash）、docs/testing.md、README.md、CHANGELOG.md。
- 范围外: ci.yml、npm scripts、capabilities、新依赖、mac 服务管理实现、mac vibrancy、HEAD 探测、pointer capture、开机自启逻辑抽取、theme 手写 YAML 换库。

## 验证命令
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` → 0 告警
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` → 全绿
- `npm run check` → 全绿
- `cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-apple-darwin` → 通过（尽力）
- 文档三文件关键词断言 → 全绿
