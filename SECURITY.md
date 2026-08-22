# 安全说明（Security）

DSH Desktop 是 DeepSeek Harness Web GUI 的桌面壳层。本文档说明其安全模型、已知限制与漏洞报告方式。

## 安全模型

- **壳页与远程 GUI 隔离**：主窗口加载本地 `frontend-dist/index.html`，DSH GUI 以 iframe 嵌入。本地壳页持有 Tauri IPC 权限（`withGlobalTauri`），远程 GUI 页面**没有任何 Tauri 权限**（隔离边界经 CDP 冒烟验证，见 `scripts/smoke/isolation.mjs`）。
- **capabilities 最小权限**：`src-tauri/capabilities/default.json` 仅声明 6 项窗口写权限（`core:window:allow-{minimize,close,show,start-dragging,set-position,set-size}`）+ `core:default`。新增前端能力时必须同步评估该清单。
- **CSP**：壳页启用内容安全策略——脚本仅允许内联 sha256 白名单，`frame-src`/`connect-src` 仅限 `http://127.0.0.1:*` 与 `http://localhost:*`。**自定义 `DSH_URL` 指向其他主机时，需同步扩展 `src-tauri/tauri.conf.json` 的 `security.csp`，否则 iframe 会被 CSP 拦截**。
- **拖拽放行**：`drag_and_drop(false)` 关闭窗口级拖拽拦截，把文件拖拽/剪贴板图片交给 DSH GUI 页面处理（drop 事件携带真实文件路径，已验证）。

## 已知限制（未签名发布版）

- 发布版**未做代码签名与公证**：
  - Windows：SmartScreen 可能提示"未知发布者"——选择"更多信息 → 仍要运行"；请从官方发布渠道获取安装包并核对 SHA256 校验和（见发布说明中的 `SHA256SUMS.txt`）。
  - macOS：Apple Silicon 产物为 ad-hoc 签名（`codesign -`，无开发者证书），首次打开可能需右键 → 打开；Gatekeeper 提示属预期行为。
- 签名与公证在后续优化方向中（代码签名 + macOS 公证），完成后本限制自动消解。

## 本地数据

| 数据 | 位置 |
|---|---|
| 壳层配置（服务地址等） | `%APPDATA%\com.dsh.desktop\config.json` |
| 日志（1MB 轮转，保留 5 份） | `%LOCALAPPDATA%\com.dsh.desktop\logs\dsh-desktop.log` |
| DSH 数据目录（settings.yaml 等） | `~/.dsh`（`DSH_HOME` 或 config 可覆盖） |

日志仅存本地，不上传任何遥测。配置与日志均为明文（本机用户可读）。

## 漏洞报告

请通过 GitHub 仓库 [fishlikewater/dsh-desktop](https://github.com/fishlikewater/dsh-desktop) 的 Security 板块或 Issues（标注 `security`）报告，附：

- 受影响版本与平台
- 可复现步骤
- 影响与建议修复方向（可选）

敏感漏洞建议私有披露（GitHub Security Advisory），公开前请勿在 Issue 中粘贴利用细节。

## 相关文档

- [发布流程](docs/release-process.md)（含校验和核验指引）
- [测试说明](docs/testing.md)（隔离边界冒烟）