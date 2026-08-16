# GitHub Release 自动构建（macOS + Windows）实施计划

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 推送 v* tag 时 CI 自动构建 macOS(arm64) 与 Windows(x64) 安装包，创建（或更新）GitHub Release 草稿并上传产物 |
| **Task Type** | High-risk: 新增 GitHub Actions 发布工作流，涉及签名 secret 与外部平台行为，无法在本机完整验证 |
| **Strategy** | Serial：Task1 脚本 → Task2 工作流（引用脚本）→ Task3 文档 |
| **Success Criteria** | AC-001 / AC-002 / AC-003 |
| **Final Verification** | `node scripts/patch-updater-endpoint.mjs`（env 替换成功、无 env 退出非 0）；release.yml YAML 解析且语义断言通过；文档段落核验 |
| **Primary Risk** | Medium: 仓库尚无 GitHub remote 与 secret，GitHub 端行为需首次 tag push 后人工核对 |

## Goal

推送 `v*` tag（`git push origin main --tags`）后：Windows（windows-latest，x86_64-pc-windows-gnu，NSIS）与 macOS（macos-latest，aarch64-apple-darwin，dmg+app）并行构建；tauri-action 自动创建同名 Release 草稿（已存在则复用）并上传安装包；Windows 构建签名并上传 updater 清单 latest.json，另附 SHA256SUMS.txt。updater endpoint 占位 OWNER/REPO 由 CI 动态替换为真实仓库。

## Acceptance Criteria

- AC-001: 新增 `.github/workflows/release.yml`，仅 `on: push: tags: ["v*"]` 触发，不触碰现有 ci.yml 行为。
- AC-002: 新增 `scripts/patch-updater-endpoint.mjs`：设 `GITHUB_REPOSITORY=owner/repo` 时把 tauri.conf.json 中 updater endpoints 的 OWNER/REPO 替换为 owner/repo（幂等），无 env 或 JSON 损坏时退出非 0，不改变 JSON 其余字段。
- AC-003: 文档（docs/release-process.md、docs/updater.md）说明自动发布流程与首次部署前置（remote、TAURI_SIGNING_PRIVATE_KEY secret）。

## Global Constraints

- 不改动 `.github/workflows/ci.yml`；不改 npm scripts；不改 tauri.conf.json 的 bundle.targets（CI 用 CLI 参数覆盖，本地 Windows 构建流程保持原样）。
- 签名私钥只经 GitHub secret 注入，不得写入仓库或日志。
- macOS 范围仅 Apple Silicon（aarch64-apple-darwin）；不加 x86_64、不加 Linux。
- 文档语言中文；沿用 docs/release-process.md 既有 tag 约定（v<version>，版本一致性由 scripts/check-version.mjs 保障）。

## File Boundaries

- `scripts/patch-updater-endpoint.mjs`: 新建，唯一职责是替换 updater endpoints 中的 OWNER/REPO。
- `.github/workflows/release.yml`: 新建，发布构建矩阵与 tauri-action 编排。
- `docs/release-process.md`: 只改「发布到 GitHub Releases」与「已知限制」两节。
- `docs/updater.md`: 只改 endpoint 占位相关段落。

## Tasks

### Task 1: 新增 scripts/patch-updater-endpoint.mjs

**Purpose**
- 提供 CI 构建前把 updater endpoint 占位 OWNER/REPO 换成真实仓库的小工具，本地可独立验证。

**Code Fact Sources**
- Read: `scripts/check-version.mjs`（脚本风格参照：ESM、UTF-8、console.error+exit 1 错误契约）
- Read: `src-tauri/tauri.conf.json` → `plugins.updater.endpoints`

**File Boundaries**
- Create: `scripts/patch-updater-endpoint.mjs`
- 不改任何现有文件。

**Key Symbols**
- Change: `plugins.updater.endpoints[]` 中的 `OWNER/REPO` 子串
- Preserve: tauri.conf.json 其余全部字段；pubkey；CSP

**Implementation Notes**
```js
// ESM；读 env GITHUB_REPOSITORY（形如 owner/repo）
// 1) 无 GITHUB_REPOSITORY 或空 → console.error + process.exit(1)
// 2) 读 src-tauri/tauri.conf.json，JSON.parse 失败 → exit 1（输出原样错误，不静默）
// 3) 遍历 plugins.updater?.endpoints ?? []，若含 "OWNER/REPO" 则 replaceAll 为 GITHUB_REPOSITORY
// 4) JSON.stringify(conf, null, 2) + 尾换行写回（显式 utf8）
// 5) 打印 "OK: 已替换 N 处 endpoint 为 owner/repo"（N=0 也 OK，幂等）
```

**Test Proof**
- 临时副本 + env 运行：副本 endpoints 中 OWNER/REPO 全部变为 owner/repo，JSON.parse 通过，其余字段逐字节不变（除 endpoints）；再跑一次无变化（幂等）。
- 无 env 运行：退出码非 0，stderr 含提示。

**Verification Command**
```bash
node -e "…临时副本断言脚本…"   # 由 implementer 用 child_process 内联执行，不改仓库文件
```

**Completion Conditions**
- 脚本存在且可被 node 直接执行；env 替换、幂等、无 env 报错三条断言全过。
- tauri.conf.json 工作副本未被脚本污染（原文件内容不变）。

**Prohibited Drift**
- 不替换 endpoint 以外的字段（尤其 pubkey、CSP、version）。
- 不引入第三方依赖；不把脚本挂进 npm scripts。
- 不修改 tauri.conf.json 原文件。

**Deviation Conditions**
- 若 CI 之外还有别的配置来源需要知道仓库名，停止并回报（file boundary 需重议）。

### Task 2: 新增 .github/workflows/release.yml

**Purpose**
- tag push 时并行构建两平台产物并发布到 GitHub Release 草稿。

**Code Fact Sources**
- Read: `.github/workflows/ci.yml`（步骤风格与 rust 工具链配置复制源）
- Read: `scripts/setup-gnu-linker.mjs`、`.cargo/config.toml`（Windows gnu 链接器前置，仅 Windows 需要）
- Read: `src-tauri/tauri.conf.json`（version 供 tagName；updater pubkey 供签名）

**File Boundaries**
- Create: `.github/workflows/release.yml`
- 不改任何现有文件。

**Key Symbols**
- Change: `on.push.tags = ["v*"]`；`permissions.contents = write`；matrix（windows-latest/macos-latest）
- Preserve: ci.yml 的 windows-gnu 三件套顺序（rustup default → setup-gnu-linker.mjs → rust-cache）；npm ci；frontend-dist 直接入库（无构建步骤）

**Implementation Notes**
```yaml
# 单 job + 矩阵，fail-fast: false
# matrix.include:
#   - platform: windows-latest, rust_target: x86_64-pc-windows-gnu, bundles: nsis
#   - platform: macos-latest,   rust_target: aarch64-apple-darwin,    bundles: app,dmg
# steps:
# 1 checkout@v4 → setup-node@v4(24, cache npm) → dtolnay/rust-toolchain@stable(targets: matrix.rust_target)
# 2 if: runner.os == 'Windows': rustup default stable-x86_64-pc-windows-gnu；node scripts/setup-gnu-linker.mjs
# 3 Swatinem/rust-cache@v2(workspaces: src-tauri)；npm ci
# 4 校验 tag：node -e "tag=process.env.GITHUB_REF_NAME; v=…conf.version; tag!=='v'+v && exit(1)"（env: GITHUB_REF_NAME）
# 5 node scripts/patch-updater-endpoint.mjs（env: GITHUB_REPOSITORY: ${{ github.repository }}）
# 6 tauri-apps/tauri-action@v0 拆成两个 step 以隔离签名 env：
#    - if: runner.os == 'Windows'：env GITHUB_TOKEN + TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
#      with: tagName: v__VERSION__, releaseName: "DSH Desktop v__VERSION__", releaseDraft: true,
#            prerelease: false, args: --bundles nsis, includeUpdaterJson: true
#    - if: runner.os == 'macOS'：env 仅 GITHUB_TOKEN；同上但 args: --bundles app,dmg, includeUpdaterJson: false
# 7 if: runner.os == 'Windows'：pwsh 生成 SHA256SUMS.txt（Get-FileHash，Set-Content -Encoding UTF8）
#    → continue-on-error: true 的 gh release upload ${{ github.ref_name }} SHA256SUMS.txt --clobber（env GH_TOKEN）
```

**Test Proof**
- YAML 解析 + 语义断言：on.push.tags 恰为 ["v*"]；矩阵含两平台；两个 tauri-action step 的 args/includeUpdaterJson 与平台匹配；签名 env 只出现在 Windows step；SHA256 步骤带 continue-on-error。
- GitHub 端真实行为无法本机验证：首次 tag push 后人工核对 Release 草稿与资产（写入 Task3 文档的核验清单）。

**Verification Command**
```bash
python -c "import yaml,pathlib; d=yaml.safe_load(pathlib.Path('.github/workflows/release.yml').read_text(encoding='utf-8')); assert d['on']['push']['tags']==['v*']; print('YAML OK')"
```（若本机缺 pyyaml，先 pip install pyyaml 于用户环境，再跑断言）

**Completion Conditions**
- release.yml 存在、YAML 合法、语义断言全过。
- 与 ci.yml 无触发重叠（ci 只监听 branch main，release 只监听 tag v*）。
- 签名私钥不出现在任何 step 输出中（只引用 secret 表达式）。

**Prohibited Drift**
- 不改 ci.yml；不新增 Linux/ubuntu；不加 macos x86_64 job。
- 不把私钥内容写入仓库或 workflow 内联；不设 TAURI_SIGNING_PRIVATE_KEY_PASSWORD（当前密钥无密码）。
- 不改动 tauri.conf.json、Cargo.toml、CHANGELOG.md（版本由发版流程递增）。

**Deviation Conditions**
- 若 tauri-action 无法满足"复用已存在 Release"需求需自写 gh 上传时，停止并回报。

### Task 3: 更新发布文档

**Purpose**
- 文档与自动流程一致：发布步骤、部署前置（remote + secret）、mac 产物说明、endpoint 替换机制。

**Code Fact Sources**
- Read: `docs/release-process.md`（「发布到 GitHub Releases」「已知限制」两节）
- Read: `docs/updater.md`（endpoint 占位与发布流程段落）

**File Boundaries**
- Modify: `docs/release-process.md`（仅上述两节）
- Modify: `docs/updater.md`（仅 endpoint 占位说明段落）

**Key Symbols**
- Change: 「发布到 GitHub Releases」改为：推 tag → CI 自动构建/建草稿 → 人工核验资产后 Publish → 核对 latest.json 可达
- Change: 新增「首次部署前置」：git remote 配置；Settings→Secrets 配 TAURI_SIGNING_PRIVATE_KEY（内容为 .tauri/dsh-desktop.key 全文）
- Preserve: 本地发版清单（版本递增、npm run build、安装包核验、SHA256 手工流程）原文保留

**Implementation Notes**
- 草稿核验清单：NSIS exe、dmg、app 压缩包、latest.json、SHA256SUMS.txt 齐全；latest.json 的 url 指向真实仓库且 signature 非空；本地安装冒烟照旧。
- mac 产物未签名/未公证说明一句话（Gatekeeper 提示，右键打开）。

**Test Proof**
- 无行为代码；核验方式为 grep 关键段落存在且旧「手动上传附件」指引已被替换。

**Verification Command**
```bash
node -e "const s=require('fs').readFileSync('docs/release-process.md','utf8'); for(const k of ['TAURI_SIGNING_PRIVATE_KEY','Publish','dmg']) if(!s.includes(k)) throw new Error('missing: '+k); console.log('docs OK')"
```

**Completion Conditions**
- release-process.md 发布章节描述 tag 触发 → 草稿 → 人工 Publish 的闭环；含 secret 配置步骤。
- updater.md 说明 endpoint 由 CI 构建时以 GITHUB_REPOSITORY 动态替换，本地构建仍为占位（不影响本地调试）。
- 三处文档中不再出现「手动上传 exe/latest.json 到 Release」为唯一路径的表述。

**Prohibited Drift**
- 不改本地发版清单与安装包核验步骤；不改 CHANGELOG 规范章节。
- 不写未实现的功能承诺（如 mac 自动更新、代码签名）。

**Deviation Conditions**
- 若发现 docs/testing.md 等第三方文档与发布流程冲突，仅记录冲突、回报，不顺手改。

## Integrated Verification

1. Task1 验证命令全绿（替换/幂等/无 env 报错，原文件未污染）。
2. Task2 YAML 解析 + 语义断言全绿。
3. Task3 文档断言全绿。
4. `node scripts/check-version.mjs` 仍通过（确认未触碰版本三处）。
5. git status 确认改动仅限四个文件：两个新建 + 两个文档。

## Completion Checks

- 所有任务完成条件满足；无 Prohibited Drift 越界。
- 向用户交付：改动文件列表、GitHub 首次部署步骤（remote、secret、tag push、草稿核验）、已知限制（mac 未签名、无 remote 时 CI 无法验证）。
