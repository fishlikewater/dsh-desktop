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
- macOS 当前为 ad-hoc 签名（无开发者证书，`codesign -`），Gatekeeper 不会完全放行；开发者证书公证已列入后续优化方向，完成后本流程补 notarization 步骤。

## 回滚

- 应用侧：最新版发现问题时，`latest.json` 指向旧版本即可控制更新（重建/上传旧版或手动回滚 Release 资产）。
- 发布侧：Release 草稿可直接删除重建；已发布的正式版可编辑删除资产，但用户已下载的安装包无法撤回——发布前务必完成核验。

## 相关文档

- [测试说明](testing.md)（发版前检查的本地等价命令）
- [安全说明](../SECURITY.md)（未签名发布版的 SmartScreen/Gatekeeper 提示说明）