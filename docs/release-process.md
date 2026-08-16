# 发布流程（Release Process）

本文档定义从代码到发布物的完整发版清单与核验步骤。

## 分支与标签规范

- 主干：`main`（所有发版均从 main 出；日常开发直接提交 main，无需长期分支）
- 标签：每次发版打 `v<version>`（如 `v0.1.0`），指向发版 commit
- 版本一致性：发版前 `npm run check` 强制校验 tauri.conf.json / Cargo.toml /
  CHANGELOG.md 最新版本三者一致（scripts/check-version.mjs）
- 变更记录：CHANGELOG.md 遵循 Keep a Changelog；Unreleased 段落随任务推进更新，
  发版时转为版本段落

```bash
# 发版收尾（示例）
git tag v0.1.1
git push origin main --tags
```

## 发版清单

```bash
# 0. 前置
#    - 主分支代码已合并、npm run check / cargo test / npm run smoke 全绿
#    - docs/development-plan.md 当前阶段任务已归档

# 1. 版本递增
#    - src-tauri/tauri.conf.json 的 version（如 0.1.0 → 0.1.1）
#    - src-tauri/Cargo.toml 的 version 同步
#    - CHANGELOG.md 将 Unreleased 段落标为版本号 + 日期

# 2. 构建 + 打包（release 优化 + NSIS 安装包 + updater latest.json）
npm run build

# 产物核验
#    - src-tauri/target/release/bundle/nsis/DSH Desktop_<version>_x64-setup.exe
#    - src-tauri/target/release/bundle/nsis/latest.json（签名清单，见 docs/updater.md）
```

## 安装包核验（每次发版必做）

```powershell
$installDir = "$env:LOCALAPPDATA\Temp\dsh-install-test"
# 静默安装
Start-Process "$PWD\src-tauri\target\release\bundle\nsis\DSH Desktop_<version>_x64-setup.exe" `
  -ArgumentList '/S', "/D=$installDir" -Wait
# 文件清单：dsh-desktop.exe + uninstall.exe
Get-ChildItem $installDir -Recurse -File
# 启动冒烟（release 无 CDP，仅验证进程存活；E2E 冒烟用 debug 构建）
Start-Process "$installDir\dsh-desktop.exe"; Start-Sleep 10
Get-Process dsh-desktop
# 卸载（静默）并确认目录清理
Start-Process "$installDir\uninstall.exe" -ArgumentList '/S' -Wait
```

## 校验和

```powershell
Get-ChildItem "$PWD\src-tauri\target\release\bundle\nsis\*.exe" | ForEach-Object {
  "$((Get-FileHash $_.FullName -Algorithm SHA256).Hash)  $($_.Name)"
} | Set-Content "$PWD\src-tauri\target\release\bundle\nsis\SHA256SUMS.txt" -Encoding UTF8
```

校验和随发布附件提供（未签名场景下用户以此核对完整性）。

## 发布到 GitHub Releases（自动）

推送 tag 即触发 CI（.github/workflows/release.yml，仅 `v*` tag）：

```bash
git push origin main --tags
```

CI 并行构建 Windows（NSIS，windows-gnu 工具链）与 macOS（dmg/app，Apple Silicon）
安装包，自动创建（或更新）同名 **Release 草稿**并上传产物：
`DSH Desktop_<version>_x64-setup.exe`、`latest.json`（Windows 签名更新清单）、
`SHA256SUMS.txt`、macOS dmg/app。updater endpoint 占位 `OWNER/REPO` 由 CI
构建前以 `GITHUB_REPOSITORY` 动态替换，无需手工改配置。

人工核验草稿后手动 Publish：

- 资产齐全：exe / latest.json / SHA256SUMS.txt / dmg / app 压缩包
- latest.json 的 url 指向本仓库且 signature 非空（见 docs/updater.md）
- 本地安装冒烟照旧（见「安装包核验」）
- Release 说明引用 CHANGELOG 对应段落

### 首次部署前置（一次性）

1. 配置 GitHub 远程并推送（首次发版后 CI 即可生效）：

   ```bash
   git remote add origin https://github.com/<owner>/<repo>.git
   git push -u origin main --tags
   ```

2. 配置签名私钥 secret：仓库 Settings → Secrets and variables → Actions，
   新建 `TAURI_SIGNING_PRIVATE_KEY`，值为本地 `.tauri/dsh-desktop.key` 全文
   （勿提交仓库）。未配置时 Windows 构建仍完成，但 latest.json 无签名、
   updater 校验失败（不影响安装包本身）。
3. 发布后核对：`https://github.com/<owner>/<repo>/releases/latest/download/latest.json` 可达

## 已知限制

- CI 发布需仓库已推送 GitHub 且配置 `TAURI_SIGNING_PRIVATE_KEY` secret 后生效；
  在此之前 GitHub 端行为无法验证（本地流程不受影响）
- 安装包未签名：Windows SmartScreen 提示，用户按 SHA256 核对
- macOS 产物未签名/未公证：Gatekeeper 提示，右键打开（Apple Silicon 专用，无 Intel 构建）
- updater 密钥/私钥管理见 docs/updater.md（私钥丢失 = 无法发更新）
