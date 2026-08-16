# Decision Anchor

## 目标（T4.3，来自 docs/development-plan.md）
版本与变更管理：版本一致性防漂移 + 标签/分支规范。

## 实施
1. `scripts/check-version.mjs`：读取 tauri.conf.json / Cargo.toml / CHANGELOG.md 最新版本，
   断言三者一致；接入 `npm run check`（版本漂移在发版前必现）。
2. 标签/分支规范文档：docs/release-process.md 补充（main 主干 + `v<version>` 标签；
   CHANGELOG 与 tag 同步）。
3. 本机打首个标签 v0.1.0（基线版本，已有 0.1.0 内容）。

## 验收标准
- [ ] check-version.mjs 通过；故意改一个版本 → check 失败（反例验证）。
- [ ] 标签 v0.1.0 已打（git tag 验证）。
- [ ] 文档更新完成。
- [ ] npm run check 全绿。

## 验证命令
- npm run check
- git tag -l
- 反例：临时改 tauri.conf.json version → check-version 失败 → 还原
