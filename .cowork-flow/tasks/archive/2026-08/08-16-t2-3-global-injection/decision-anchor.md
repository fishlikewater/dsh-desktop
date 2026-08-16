# Decision Anchor

## 目标（T2.3，来自 docs/development-plan.md）
全局注入评估：评估 `withGlobalTauri: true` 的注入面并决定保留或收敛，记录安全边界。

## 评估结论（决策：保留 withGlobalTauri）

### 注入面分析（实测修正）
1. **`__TAURI__` 注入范围**：Tauri 2 经 WebView2 `AddScriptToExecuteOnDocumentCreated` 注入初始化脚本，**对所有 frame 生效**——iframe（DSH GUI 远程页面）内也存在 `__TAURI__` 对象（实测确认）。
2. **能力层隔离（关键）**：capabilities 权限按 **窗口 + URL 模式**匹配（`URL: local` 仅匹配壳页本地文档）。实测 iframe 内调用 `outerSize` 与 `invoke("theme_preference")` 均被拒（"not allowed on window main, webview main, URL: http://127.0.0.1:3080/ allowed on: [windows: main, URL: local]"）。即：**对象存在但零 IPC 能力**，隔离边界在能力层成立。
3. **壳页面**：能力受 capabilities 收敛（6 项窗口写权限 + core:default），CSP 脚本 sha256 白名单无动态执行面。

### preload 迁移评估（不采纳）
- 收益：可裁剪暴露的 API 面（仅暴露所需方法）；控制注入脚本内容。
- 成本：静态 HTML 无构建链，preload 需手写 API 桥接（或引入构建工具）；且 Tauri 注入机制本身无法阻止对象进入 iframe（preload 同样经 AddScriptToExecuteOnDocumentCreated）；当前能力面已由 **URL 粒度 capabilities + CSP** 双重收敛，实际风险收益比低。
- 结论：**保留 withGlobalTauri**，隔离边界 = iframe 能力层隔离（URL 粒度权限）+ capabilities 最小权限 + CSP 脚本白名单。

## 验收标准
- [x] 决策记录（本文件）+ SECURITY.md 安全边界说明（对象层注入 + 能力层隔离的准确表述）。
- [x] 实测确认：iframe 内 `__TAURI__` 存在但 IPC 调用全部被拒；壳页 `__TAURI__` 正常。
- [x] npm run check 全绿。

## 验证命令
- CDP：壳页与 iframe 双上下文检查 __TAURI__ 存在性
- npm run check
