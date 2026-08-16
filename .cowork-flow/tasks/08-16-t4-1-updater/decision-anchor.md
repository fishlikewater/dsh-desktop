# Decision Anchor

## 目标（T4.1，来自 docs/development-plan.md）
自动更新：Tauri updater 集成（密钥、配置、发布物格式）。

## 已知限制（必须记录）
- 无 GitHub 仓库：endpoints 用占位 URL（owner/repo 待定），发布流程交付脚本 + 文档，实际发布在 T4.2 记录。
- 用户决策：无代码签名证书。Tauri updater 的完整性校验走 **ed25519 密钥签名**（tauri signer），不依赖代码签名证书——更新包可验签；SmartScreen 对安装包本身的提示仍存在（SECURITY.md 已记录降级方案）。

## 实施
1. 生成 updater 密钥对：`tauri signer generate`（私钥 .tauri/dsh-desktop.key，gitignore；公钥 pubkey 入 tauri.conf.json）
2. tauri.conf.json `plugins.updater`：pubkey + endpoints（GitHub Releases latest.json 占位）
3. 依赖：tauri-plugin-updater + tauri-plugin-process（更新后重启）；lib.rs 注册
4. 发布物格式：NSIS 安装包 + latest.json（签名版），脚本 scripts/release/ 交付（T4.2 完善）

## 验收标准
- [ ] 密钥对生成；私钥 gitignore（不入库）；公钥入配置。
- [ ] updater 插件注册成功（启动日志无错误）。
- [ ] npm run check 全绿（含 clippy）。
- [ ] 文档：docs/updater.md 发布流程（构建、签名、latest.json、上传）。
- [ ] 限制记录：endpoints 占位说明。

## 验证命令
- npm run check
- 启动应用看日志 updater 初始化
- 本地验证 updater 查询逻辑（endpoint 404 时静默，不崩溃）
