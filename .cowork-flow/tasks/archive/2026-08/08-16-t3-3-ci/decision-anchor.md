# Decision Anchor

## 目标（T3.3，来自 docs/development-plan.md）
CI 流水线：GitHub Actions 全链路（静态检查 → 单测 → 构建 → 冒烟 → 打包）。

## 已知限制（必须记录）
- 仓库无 GitHub 远程（无 repo），workflow 无法实际触发运行；交付本地可验证产物（yaml 语法校验 + 步骤审查 + 文档说明），注册限制。
- DSH 服务（dsh CLI）在 CI runner 上不可用（无 dsh 安装）：冒烟步骤标记 `continue-on-error`，失败不阻塞主链路；本地 npm run smoke 覆盖验证。

## CI 设计
- runner: windows-latest（自带 WebView2、NSIS 下载缓存）
- 工具链：rustup stable + x86_64-pc-windows-gnu（rust-mingw 自带 gcc）；node 24
- 步骤：checkout → rust toolchain → npm ci → setup 链接器（复用 setup-gnu-linker.mjs，幂等）→
  fmt 检查 → clippy -D warnings → cargo test → npm run check → debug 构建 →
  冒烟（continue-on-error，前置检查会跳过 DSH 不可达场景）→ release 构建 + NSIS 打包 → 上传 artifact
- 缓存：cargo registry/target、npm 缓存

## 验收标准
- [ ] .github/workflows/ci.yml 完整（上述步骤）；yaml 语法校验通过。
- [ ] 本地等价验证：workflow 步骤对应的命令序列在本机全绿（npm run check、cargo test、cargo build debug、cargo build release 已验证/待验证）。
- [ ] 限制记录：README/docs 说明 CI 未实际运行的原因与启用步骤。
- [ ] 无远程仓库的限制在 decision-anchor 与 docs 中记录。

## 验证命令
- npx yaml-lint（或等效）校验 ci.yml
- 本地顺序执行 workflow 各步骤命令
