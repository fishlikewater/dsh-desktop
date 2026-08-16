# Decision Anchor

## 目标（T0.1，来自 docs/development-plan.md）
git 初始化与仓库整洁：源码纳入版本控制，工作区无杂物，首次基线提交。

## 验收标准
- [ ] `git init`（main 分支）+ 首次基线提交（src-tauri/、frontend-dist/、docs/、.cowork-flow/、.agents/ 纳入）。
- [ ] `.gitignore` 补全：`ScreenShot_*.png`、`.toolchain-test/`、`.dsh-vision-toolkit/`、`src-tauri/gen/`、`.cowork-flow/.runtime/`、日志目录、`*.bak-*`；`git check-ignore` 抽查生效。
- [ ] 根目录杂物清理：截图移入已忽略目录（不删用户数据）、临时验证产物归位。
- [ ] `git status --porcelain` 干净；提交不含生成物（node_modules/target）。
- [ ] git 身份配置（user.name/user.email）。

## 范围边界
- 范围内: git init、.gitignore、杂物清理、首次提交、git 身份。
- 范围外: LICENSE/CHANGELOG（T0.2）、lint/工具链（T0.3）、任何代码行为变更。

## 验证命令
- git status --porcelain（干净）
- git log --oneline（首次提交存在）
- git check-ignore 抽查
