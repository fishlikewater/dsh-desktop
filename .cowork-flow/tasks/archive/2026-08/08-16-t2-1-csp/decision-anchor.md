# Decision Anchor

## 目标（T2.1，来自 docs/development-plan.md）
CSP 落地：替换 `security.csp: null` 为最小 CSP，壳页功能与 iframe 加载不受影响。

## 资源面分析
- 壳页自身：内联 `<script>`（需 sha256 hash）、内联 `<style>`（'unsafe-inline' for style）
- iframe 嵌入：`http://127.0.0.1:*`（DSH GUI，默认 3080）
- connect-src：壳页服务探测 fetch（no-cors，指向 DSH_URL）
- Tauri 注入脚本（withGlobalTauri IPC 初始化）为 WebView2 层注入，不受页面 CSP 约束（需实测确认）

## 验收标准
- [ ] tauri.conf.json `csp` 非 null：default-src 'self' + script-src（内联脚本 sha256 hash）+ style-src + frame-src/connect-src（127.0.0.1/localhost 任意端口）。
- [ ] 实测：壳页功能全过（窗口控制/主题即时同步/拖拽/resize/伪最大化）；iframe GUI 正常加载；DevTools 无 CSP 违规。
- [ ] 文档：自定义 DSH_URL host（非 127.0.0.1/localhost）需扩展 CSP 的说明（README/SECURITY）。
- [ ] npm run check 全绿。

## 范围边界
- 范围内: CSP 配置、hash 计算、实测验证、文档。
- 范围外: capabilities 权限（T2.2）、preload 迁移（T2.3）、DSH_URL 校验收紧。

## 验证命令
- npm run check
- 启动应用 + CDP：查 body 状态、__TAURI__ 可用性、iframe 加载
- DevTools console 无 CSP 报错（CDP Runtime.evaluate 检查）
