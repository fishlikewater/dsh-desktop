# 修复：标题栏主题同步失效（DSH_HOME 默认路径缺 .dsh）

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 无 DSH_HOME 环境变量且无 config.json 覆盖时，主题监听/读取路径解析到默认 DSH 数据目录 %USERPROFILE%\.dsh（而不是裸 USERPROFILE） |
| **Task Type** | Normal: 单函数语义修正 + 单测更新 + 端到端验证（CDP 冒烟） |
| **Strategy** | Serial：代码修复 → 单测/clippy → 端到端验证 → 文档 |
| **Success Criteria** | AC-001 / AC-002 |
| **Final Verification** | \`cargo test --lib\` 全绿；\`cargo clippy -D warnings\` 0 告警；CDP 冒烟：改 settings.yaml 主题 ≤2s 跟随（debug 构建实测） |
| **Primary Risk** | Low: 修复为默认路径拼接，风险面仅 theme_settings_path |

## 根因（已诊断确认）

- 应用在无 DSH_HOME 环境变量启动时（直接双击 exe），resolve_dsh_home 的 USERPROFILE 兜底返回裸 \`C:\Users\<user>\`，theme_settings_path 变为 \`C:\Users\<user>\settings.yaml\`；实际文件在 \`.dsh\settings.yaml\`。watcher 与 30s 兜底轮询全部失效。
- 证据：日志 18:45:59「已监听主题文件 C:\Users\Administrator\settings.yaml」；该文件不存在；\`.dsh\settings.yaml\` 存在（mtime 19:59 与用户改主题时间吻合）。
- 历史启动由带 DSH_HOME 的父进程拉起（18:42:59 日志监听 .dsh 路径正确），故此前未暴露。

## Acceptance Criteria

- AC-001: resolve_dsh_home 的 USERPROFILE 兜底返回 \`<userprofile>\.dsh\`，最终兜底返回 \`.\\.dsh\`；DSH_HOME env / config.json dsh_home 分支语义不变（原样直用，不追加 .dsh）。
- AC-002: cargo test --lib（含更新后的 resolve_home_precedence 用例）与 clippy 全绿；端到端：无 DSH_HOME 环境下启动 debug 应用，修改 .dsh/settings.yaml 的 ui-theme.preference 后标题栏 ≤2s 跟随（复用 scripts/smoke/theme-sync.mjs 的验证路径，CDP 断言 body class）。

## File Boundaries

- \`src-tauri/src/config.rs\`: 仅 resolve_dsh_home 的兜底分支与测试、相关注释
- \`src-tauri/src/theme.rs\`: 仅 theme_preference 与模块注释中路径说明（同步 .dsh 语义）
- \`CHANGELOG.md\`: Unreleased 修复条目

## Tasks

### Task 1: 修复 resolve_dsh_home 默认路径 + 单测

**Purpose**
- 默认 DSH 数据目录解析为 %USERPROFILE%\.dsh。

**Code Fact Sources**
- Read: \`src-tauri/src/config.rs\` L118-150（resolve_dsh_home / theme_settings_path）
- Read: \`src-tauri/src/theme.rs\` L35-43（路径注释）

**File Boundaries**
- Modify: \`src-tauri/src/config.rs\`（仅 resolve_dsh_home 实现、文档注释、resolve_home_precedence 用例）

**Key Symbols**
- Change: \`resolve_dsh_home()\` 的 userprofile 分支与最终兜底（join ".dsh"）
- Preserve: env/config 分支语义；函数签名；theme_settings_path 结构

**Implementation Notes**
\`\`\`rust
// userprofile 分支：
//   .map(|v| Path::new(v).join(".dsh").to_string_lossy().into_owned())
// 最终兜底：
//   .unwrap_or_else(|| Path::new(".").join(".dsh").to_string_lossy().into_owned())
// 文档注释同步：env 非空 → env；config 非空 → config；
//   userprofile 非空 → userprofile/.dsh；否则 ./.dsh
// 单测：resolve_dsh_home(None,None,Some("up")) == "up/.dsh"（用 Path join 构造期望，平台无关）
//       resolve_dsh_home(None,None,None) == "./.dsh"
\`\`\`

**Test Proof**
- \`cargo test --lib\`：resolve_home_precedence 更新后全绿（env/config 优先级行为不变）。
- \`cargo clippy --all-targets -- -D warnings\`：0 告警。

**Verification Command**
\`\`\`bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
\`\`\`

**Completion Conditions**
- 单测与 clippy 全绿；env/config 分支行为不变（用例覆盖）。

**Prohibited Drift**
- 不改 dsh_url 解析；不给 env/config 分支追加 .dsh；不动其他模块。

**Deviation Conditions**
- 若发现其他模块也依赖 resolve_dsh_home 的旧语义，停止并回报。

### Task 2: 端到端验证（CDP 冒烟）与文档

**Purpose**
- 真实环境证明修复有效：无 DSH_HOME 环境下主题即时跟随；CHANGELOG 补条目。

**Code Fact Sources**
- Read: \`scripts/smoke/theme-sync.mjs\`（验证路径：改 settings.yaml → CDP 断言 body class）

**File Boundaries**
- Modify: \`CHANGELOG.md\`（仅 Unreleased 修复条目）
- 验证过程会临时修改 \`~/.dsh/settings.yaml\`（冒烟脚本自带备份与 light 恢复）

**Key Symbols**
- Change: CHANGELOG Unreleased 增「修复：无 DSH_HOME 环境时主题路径解析缺 .dsh，标题栏不随主题变化」
- Preserve: 冒烟脚本原样使用

**Implementation Notes**
- 关掉当前运行的旧 release 进程（单实例插件会拦截新实例）→ cargo build（debug）→ 以无 DSH_HOME 的干净环境启动 debug 应用 → \`node scripts/smoke/theme-sync.mjs dark\` → 通过后恢复（脚本自动恢复 light）→ 编译 release 供用户重启使用。
- 若 CDP 冒烟环境不具备（端口/应用状态），改为截图对比标题栏颜色（描述图像工具）验证，并如实记录。

**Test Proof**
- theme-sync 冒烟 PASS = 修复端到端成立（≤2s 跟随）。

**Verification Command**
\`\`\`bash
node scripts/smoke/theme-sync.mjs dark
\`\`\`

**Completion Conditions**
- 冒烟通过；CHANGELOG 条目存在；release 构建产出并交还用户（提示重启）。

**Prohibited Drift**
- 不改冒烟脚本；不改主题解析逻辑（parse_theme_preference）。

**Deviation Conditions**
- 若冒烟失败但单测通过，停止并回报（说明 watcher 层另有问题）。

## Integrated Verification

1. cargo test --lib / clippy 全绿（Task1）。
2. 端到端冒烟 PASS（Task2）。
3. git 改动范围 = config.rs / theme.rs / CHANGELOG.md（+ 流程工件）。
