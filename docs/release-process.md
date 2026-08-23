# 发布流程（Release Process）

> 本文档描述 DSH Desktop 的版本发布流程：版本规划、发布步骤、产物核验与回滚。发布由 GitHub Actions 自动完成大部分环节（[release.yml](../.github/workflows/release.yml)），人工负责发版决策与核验。

## 发布前检查

1. **代码就绪**：`npm run check` 通过（fmt / clippy / 前端与 CSP hash / 版本一致性）。
2. **单元测试**：`cargo test --manifest-path src-tauri/Cargo.toml --lib` 全绿。
3. **冒烟**（有条件时）：`npm run smoke` 全 PASS（需 debug 构建 + 本机 DSH 服务）。
4. **CHANGELOG 更新**：按 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 记录本次版本变更（Added/Fixed/工程）。
5. **版本号更新**：`src-tauri/tauri.conf.json` 的 `version`（tag 必须与 `v<version>` 完全一致，release.yml 有校验步骤，不一致会失败）。

## 发版步骤

```bash
# 1. 提交并推送版本变更（CHANGELOG + version）
git add CHANGELOG.md src-tauri/tauri.conf.json
git commit -m "chore(release): bump <新版本>"
git push origin main

# 2. 打 tag 触发 release.yml
git tag v<新版本>
git push origin v<新版本>
```

触发后 GitHub Actions release.yml 自动执行（并行）：

- **Windows**：windows-gnu 工具链构建 NSIS 安装包；tauri-action 建/更 Release 草稿并上传签名 `latest.json`；上传 `.exe.sig` 与 `SHA256SUMS.txt`。
- **macOS**：aarch64-apple-darwin 构建 app/dmg；对该 .app 做 ad-hoc 签名（`codesign -`）并重建 updater 包/dmg 后覆盖上传。
- **聚合 job**：双平台构建完成后从 Release 资产组装双平台 `latest.json` 并上传（覆盖）。

## 发布后核验

1. 检查 Release 草稿资产完整：Windows `.exe` + `.exe.sig`、macOS `.app.tar.gz` + `.sig` + `.dmg`、`SHA256SUMS.txt`、`latest.json`。
2. **SHA256 核验**（Windows 安装包）：

```powershell
Get-FileHash .\dsh-desktop-setup.exe -Algorithm SHA256 | Format-List
# 与 Release 资产中的 SHA256SUMS.txt 对应行比对
```

3. **发布为正式版**：确认产物完整后，将 Release 草稿发布（Publish）。无需人工重建产物。
4. **应用内更新验证**：应用设置页「检查更新」应发现新版本并一键更新（依赖 ed25519 签名：`TAURI_SIGNING_PRIVATE_KEY` secret 已配置时 latest.json 才有效）。

## 签名与密钥约定

- updater 签名公钥内置于 `src-tauri/tauri.conf.json`（`plugins.updater.pubkey`），发布版不可更换。
- 私钥位于本地 `.tauri/dsh-desktop.key`，**禁止提交仓库**（.gitignore 已排除）；CI 通过 secret `TAURI_SIGNING_PRIVATE_KEY` 注入（Windows 构建 + 聚合 job 必需；macOS ad-hoc 签名同样注入用于重签 .sig）。
- macOS 当前为 ad-hoc 签名（无开发者证书，`codesign -`），Gatekeeper 不会完全放行；开发者证书公证步骤已就绪（见下），凭据注入即启用。

## 代码签名与公证（Task 13）

> 状态：**前置工程已就绪，证书未采购 → blocked**。release.yml 已含条件化签名/公证步骤与自动验证门禁（secrets 缺席时跳过并输出 SKIP 说明，不产生假签名产物）。以下为证书到位后的启用手册。

### 需要的 secrets（GitHub → Settings → Secrets and variables → Actions）

| secret | 用途 | 获取路径 |
|---|---|---|
| `WINDOWS_CODESIGN_CERT_B64` | Windows 代码签名证书（.pfx 的 base64） | EV/OV 代码签名证书（如 DigiCert/GlobalSign），导出 pfx 后 `base64 -i cert.pfx` |
| `WINDOWS_CODESIGN_PASSWORD` | pfx 私钥口令 | 导出 pfx 时设置 |
| `APPLE_ID` | Apple 开发者账号邮箱 | developer.apple.com |
| `APPLE_APP_SPECIFIC_PASSWORD` | App Store Connect 专用密码（**非**账号密码） | appleid.apple.com → 登录与安全 → App 专用密码 |
| `APPLE_TEAM_ID` | Team ID（10 位） | developer.apple.com → Membership 详情 |
| `MACOS_SIGNING_IDENTITY`（可选） | Developer ID Application 证书 identity，默认 `Developer ID Application` | Keychain 中证书通用名 |

### 启用后自动执行的验证门禁（失败即发布失败）

- **Windows**：`signtool sign /fd SHA256 /td SHA256 /tr http://timestamp.digicert.com`（RFC3161 时间戳）→ `signtool verify /pa` → 重签 updater `.exe.sig`（signtool 改变 exe 字节，原 sig 失效）→ SHA256SUMS（签名后哈希）。
- **macOS**：Developer ID + `--options runtime` 签名（覆盖 ad-hoc）→ `notarytool submit --wait`（Apple 公证）→ `stapler staple`（票据内嵌）→ 重建 updater tar.gz/.sig 与 dmg 并覆盖上传 → `codesign --verify --deep --strict` + `spctl --assess` 验证。

### 注意事项

- macOS 签名身份需能访问 keychain 中的证书（GitHub Actions 可在 runner 上安装证书；也可改用云端签名服务，命令按实际方案适配）。
- 证书到期/撤销后发布流程自动失败（验证门禁），不会静默产出未签名产物。
- 启用后 SECURITY.md「已知限制」段的 SmartScreen/Gatekeeper 提示不再适用，发布说明同步更新。

## 回滚

- 应用侧：最新版发现问题时，`latest.json` 指向旧版本即可控制更新（重建/上传旧版或手动回滚 Release 资产）。
- 发布侧：Release 草稿可直接删除重建；已发布的正式版可编辑删除资产，但用户已下载的安装包无法撤回——发布前务必完成核验。

## 相关文档

- [测试说明](testing.md)（发版前检查的本地等价命令）
- [安全说明](../SECURITY.md)（未签名发布版的 SmartScreen/Gatekeeper 提示说明）