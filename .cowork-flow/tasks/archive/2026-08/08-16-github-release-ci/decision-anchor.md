# Decision Anchor

## 目标
推送 v* tag 时，GitHub Actions 自动构建 macOS(arm64) 与 Windows(x64) 安装包，创建（或更新）GitHub Release 草稿并上传产物（含签名的 latest.json 与 SHA256SUMS.txt）。

## 验收标准
- [ ] AC-001: 新增 .github/workflows/release.yml，仅 push tag v* 触发；矩阵含 windows-latest（nsis）与 macos-latest（app,dmg）；tauri-action 建草稿 Release（releaseDraft: true）；Windows step 用 secret 签名并 includeUpdaterJson，macOS step 不含签名 env。
- [ ] AC-002: 新增 scripts/patch-updater-endpoint.mjs：GITHUB_REPOSITORY env 下替换 updater endpoints 中 OWNER/REPO（幂等），无 env / JSON 损坏时退出非 0；不触碰其余字段。
- [ ] AC-003: docs/release-process.md 与 docs/updater.md 更新为自动发布流程，含首次部署前置（remote、TAURI_SIGNING_PRIVATE_KEY secret）与草稿核验清单。

## 被拒方案
- **on: release published 触发**: 拒绝原因——与项目既定流程（docs/release-process.md：git push origin main --tags）不一致，且 Release 先于产物存在会造成空 Release；tag push + tauri-action 自动建 Release 是 Tauri 生态标准。
- **CI 内嵌 sed/PowerShell 替换 endpoint**: 拒绝原因——Windows/macOS 双平台命令不统一，独立 node 脚本本地可测、可复用。
- **macOS 同时构建 x86_64**: 拒绝原因——范围最小化（用户确认方案），Intel 覆盖留作后续扩展。
- **自写 gh release upload 编排（不用 tauri-action）**: 拒绝原因——tauri-action 已覆盖建 Release、传资产、latest.json 生成上传，自写增加维护面。

## 关键假设
- 仓库最终会推送到 GitHub 并配置 TAURI_SIGNING_PRIVATE_KEY secret（内容为本地 .tauri/dsh-desktop.key）；在此之前 release.yml 无法被真实验证。
- 签名私钥无密码，故不设 TAURI_SIGNING_PRIVATE_KEY_PASSWORD。
- frontend-dist 已入库、package-lock.json 已入库，CI 无需前端构建步骤。
- macos-latest 原生 arm64（aarch64-apple-darwin）；产物未签名/未公证（Gatekeeper 提示右键打开）。

## 范围边界
- 范围内: scripts/patch-updater-endpoint.mjs；.github/workflows/release.yml；docs/release-process.md 与 docs/updater.md 相关章节。
- 范围外: ci.yml 改动；npm scripts 改动；tauri.conf.json/Cargo.toml/CHANGELOG.md 版本改动；Linux/ubuntu；macOS x86_64；代码签名与公证；mac 自动更新。

## 验证命令
- `node scripts/patch-updater-endpoint.mjs`（临时副本 + env）→ 替换成功、幂等；无 env → exit != 0
- `python -c "import yaml,pathlib; d=yaml.safe_load(...); assert d['on']['push']['tags']==['v*']"` → YAML OK
- `node -e "…docs 关键词断言…"` → docs OK
- `node scripts/check-version.mjs` → OK（版本三处未被触碰）
