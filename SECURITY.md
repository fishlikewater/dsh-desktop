# 安全说明（Security Policy）

## 支持的版本

| 版本 | 支持状态 |
|---|---|
| 0.1.x（开发阶段） | ✅ 活跃开发，安全修复随版本发布 |

## 报告漏洞

本项目处于个人开发阶段。发现安全问题时，请通过以下渠道报告（**请勿公开披露未修复漏洞**）：

- 在 GitHub 仓库创建 **Private vulnerability report**（如已公开托管）
- 或通过项目维护者直接联系

请提供：

1. 受影响版本
2. 漏洞描述与复现步骤
3. 影响评估（如可）

## 安全设计说明

- **隔离边界**：远程 DSH GUI 以 iframe 嵌入，无 Tauri IPC 权限；壳页本地页面独占 `__TAURI__` 能力
  - `__TAURI__` 对象因 WebView2 注入机制会出现在 iframe 文档中（对象层），但 capabilities 的权限按 **URL 粒度**匹配（`URL: local` 仅匹配壳页本地文档）：实测 iframe 内 `outerSize`、`invoke` 等全部 IPC 调用被拒（"not allowed on ... URL: http://127.0.0.1:3080/"）——隔离边界在能力层成立
  - 壳页能力面由 capabilities 收敛（6 项窗口写权限 + core:default）+ CSP 脚本 sha256 白名单（无 eval/动态脚本）双重约束；即便壳页被注入，也不具备文件/命令/网络能力
- **能力最小化**：`capabilities/default.json` 仅声明窗口控制所需权限
- **数据位置**：用户数据位于 `~/.dsh`（DSH_HOME）；壳层配置/日志位于 `%APPDATA%\com.dsh.desktop`
- **主题解析**：`settings.yaml` 解析仅读取 `ui-theme.preference`，损坏/未知内容回退默认值，不执行任何内容
- **CSP**：壳页启用 CSP（`src-tauri/tauri.conf.json` `security.csp`）：
  - `script-src`：内联脚本 sha256 白名单（修改 `frontend-dist/index.html` 内联 JS 后须同步更新 hash）
  - `frame-src`/`connect-src`：仅 `http://127.0.0.1:*` 与 `http://localhost:*`（DSH GUI 嵌入与服务探测）；`DSH_URL` 指向其他主机时需同步扩展
  - `style-src`：允许内联样式（壳页 `<style>`，无动态注入面）

## 已知限制

- 发布版安装包**未做代码签名**：Windows SmartScreen 会提示"未知发布者"。请通过官方渠道获取安装包并核对 SHA256 校验和（见发布说明）
- DSH 服务为本地环回（127.0.0.1）监听，未对外暴露
