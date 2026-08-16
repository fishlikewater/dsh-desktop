# Decision Anchor

## 目标（T4.2，来自 docs/development-plan.md）
发布流程与产物核验：交付可核验的发布物与发布说明，记录无 GitHub 限制。

## 已知限制（必须记录）
- 无 GitHub 仓库：无法上传 Releases；发布物在本地构建并核验，发布清单/流程文档交付。
- 无代码签名证书：安装包 SmartScreen 提示，SECURITY.md 已记录。

## 实施
1. **安装包核验**：NSIS 安装包（0.1.0 已构建）静默安装到临时目录 → 校验文件清单（exe/WebView2 依赖无需捆绑/图标）→ 启动安装产物冒烟 → 卸载验证。
2. **发布说明**：docs/release-process.md（发版清单：版本递增 → 构建 → 核验 → CHANGELOG → 签名 → 上传）。
3. **CHANGELOG 0.1.0 定稿**：确认条目与实际功能一致。
4. **SHA256 校验和**：为安装包生成校验和文件（发布附件规范）。

## 验收标准
- [ ] 静默安装/启动/卸载全流程通过（安装产物可运行）。
- [ ] SHA256 校验和文件生成。
- [ ] docs/release-process.md + CHANGELOG 定稿。
- [ ] npm run check 全绿。

## 验证命令
- NSIS /S 静默安装 → 启动安装版 → CDP 冒烟（主题/窗口）→ 卸载
- Get-FileHash 校验和
