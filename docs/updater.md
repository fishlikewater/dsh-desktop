# 自动更新（Tauri updater）

## 架构

- 更新检查由 `tauri-plugin-updater` 提供；完整性校验使用 **ed25519 签名**（非代码签名证书），
  由 `tauri signer` 生成的密钥对负责。
- 发布物：NSIS 安装包（`.exe`）+ 签名清单 `latest.json`，托管在 GitHub Releases
  （`endpoints` 指向 `https://github.com/OWNER/REPO/releases/latest/download/latest.json`）。
- `OWNER/REPO` 为仓库占位：CI 构建前由 `scripts/patch-updater-endpoint.mjs`
  以 `GITHUB_REPOSITORY` 动态替换为真实仓库；本地构建保持占位（不影响本地调试，
  更新检查 404 时静默降级）。

## 密钥管理（重要）

| 文件 | 位置 | 处置 |
|---|---|---|
| 私钥 | `.tauri/dsh-desktop.key` | **gitignore，勿入库**；丢失后无法发布更新 |
| 公钥 | `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` | 入库 |

> 私钥可加密码（`tauri signer generate -p <password>`），签名时通过环境变量
> `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 提供。当前生成未设密码，如需加固请重新生成。

## 发布流程（每次发版）

前置：`cargo build --release` 产物 + `tauri.conf.json` 版本号已递增（如 0.1.0 → 0.1.1，
updater 仅接受高于当前安装版本的版本）。

```bash
# 1. 构建 + 打包（自动生成签名版安装包与 latest.json）
npm run build

# 产物：
#   src-tauri/target/release/bundle/nsis/DSH Desktop_<version>_x64-setup.exe
#   src-tauri/target/release/bundle/nsis/latest.json
```

> 打包时 tauri CLI 自动用 `.tauri/dsh-desktop.key` 签名更新包；
> 若私钥有密码，需设置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

```bash
# 2. 上传到 GitHub Releases：推送 tag 即由 CI 自动完成
#    （见 docs/release-process.md「发布到 GitHub Releases（自动）」）；
#    本地手工上传作为兜底（Release 名为版本号，如 v0.1.1）：
#    附件：
#      DSH Desktop_<version>_x64-setup.exe
#      latest.json
#    注意：latest.json 必须放在 Release 附件中命名为 latest.json（endpoints 引用它）
```

### latest.json 校验

发布前核对 `latest.json` 内容：

```json
{
  "version": "0.1.1",
  "notes": "更新说明",
  "pub_date": "2026-08-16T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6 ...",
      "url": "https://github.com/OWNER/REPO/releases/download/v0.1.1/DSH%20Desktop_0.1.1_x64-setup.exe"
    }
  }
}
```

- `signature` 为安装包的 ed25519 签名（tauri CLI 生成，勿手改）
- `url` 须与上传附件名一致（空格编码为 `%20`）

## 更新触发

- 插件注册即提供能力；壳页在合适时机调用检查（当前版本未接入 UI，
  计划在 T5.4 设置页提供"检查更新"入口；也可通过 `npm run tauri info` 调试）。
- Rust 侧调用示例：

```rust
use tauri_plugin_updater::UpdaterExt;
let updater = app.updater().unwrap();
let update = updater.check().await?;
if let Some(u) = update { u.download_and_install().await?; }
```

## 已知限制

- 安装包本身未做代码签名：用户首次安装仍会看到 SmartScreen 提示（见 SECURITY.md）。
- updater 只校验更新包内容（防篡改），不校验发布者身份链。
- 无 GitHub 仓库前更新不可用（endpoint 404 → 检查静默失败，不影响使用）。
