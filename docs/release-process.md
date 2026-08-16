# 发布流程（Release Process）

本文档定义从代码到发布物的完整发版清单与核验步骤。

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

## 发布到 GitHub Releases

1. 创建 Release（tag `v<version>`，标题同 tag）
2. 上传附件：
   - `DSH Desktop_<version>_x64-setup.exe`
   - `latest.json`（必须命名为 `latest.json`，updater endpoints 引用）
   - `SHA256SUMS.txt`
3. Release 说明引用 CHANGELOG 对应段落
4. 发布后核对：`https://github.com/<owner>/<repo>/releases/latest/download/latest.json` 可达

## 已知限制

- 仓库尚无 GitHub 远程：Releases 无法上传，updater 不可用（endpoint 404 时静默降级，不影响使用）
- 安装包未签名：SmartScreen 提示，用户按 SHA256 核对
- updater 密钥/私钥管理见 docs/updater.md（私钥丢失 = 无法发更新）
