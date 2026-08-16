# macOS 构建适配 + 既有功能打磨（P0/P1/P2 一致性）实施计划

> Execution rule: this plan exists to prevent implementation drift. The implementer must work through tasks in order.
> If the plan conflicts with current code facts, a plan-external file becomes necessary, or a test fails for an unplanned reason, stop and report back instead of expanding scope.

| Field | Content |
|----|------|
| **Goal** | 修复 macOS 平台编译失败（P0）与 5 项真实缺陷（P1），清理过时/重复注释与文档不一致（P2 一致性项） |
| **Task Type** | High-risk: 平台条件编译与 Windows FFI 隔离，macOS 侧无法在本机链接级验证（尽力用 cargo check --target 交叉检查） |
| **Strategy** | Serial：Task1 window.rs → Task2 service/settings → Task3 编码/日志/壳页 → Task4 文档 |
| **Success Criteria** | AC-001~AC-004 |
| **Final Verification** | \`cargo clippy --all-targets -- -D warnings\`；\`cargo test --lib\`；\`npm run check\`；\`cargo check --target x86_64-apple-darwin\`（尽力） |
| **Primary Risk** | Medium: macOS 编译依赖条件编译正确性，本机无 macOS 链接器；交叉 cargo check 失败时以 CI 为最终验证并如实记录 |

## Goal

1. macOS（aarch64-apple-darwin）上 \`cargo check\` 可过：Windows-only 的 FFI/进程 API（hwnd、SPI_GETWORKAREA、creation_flags、taskkill/cmd/powershell 分支）全部按 target_os 隔离；非 Windows 提供服务管理 stub（返回可读错误，不扩功能）。
2. P1 五项缺陷修复：服务探活 panic、URL 参数编码丢字符（#/+/空格）、壳页 innerHTML 注入面、停止服务失败丢 pid、日志无限增长。
3. P2 一致性：window.rs 过时/重复注释清理与双问号可读性、DETACHED_PROCESS 魔法数字命名、docs/testing.md 与 README.md 过时描述修正、CHANGELOG Unreleased 补条目。

## Acceptance Criteria

- AC-001: \`cargo clippy --all-targets -- -D warnings\`（x86_64-pc-windows-gnu）零告警；\`cargo test --lib\` 全绿（现有 23 例 + 新增编码函数用例）。
- AC-002: 所有 Windows-only 调用点有 \`#[cfg(target_os = "windows")]\` 或等价隔离；\`cargo check --target x86_64-apple-darwin\` 通过（若本机交叉 check 因环境限制失败，如实记录并注明靠 CI 验证，不伪造通过）。
- AC-003: P1 五项修复有行为验证：socket_addrs 空结果不 panic（返回不可达）；编码函数对 #、+、空格、非 ASCII 单测覆盖；壳页 target-line 用 DOM 构建（无 innerHTML 拼接 URL）且 \`npm run check\`（CSP hash 同步后）通过；stop_service 失败时 pid 记录保留；logging 轮转改为 KeepSome(5)。
- AC-004: docs/testing.md CI 段落与实际 ci.yml 一致；README 故障排查段落更新；CHANGELOG Unreleased 新增发布自动化与本轮修复条目。

## Global Constraints

- 不改 ci.yml；不改 npm scripts；不新增依赖（crate 或 npm）。
- 不扩功能：macOS 服务管理为 stub（返回可读错误），不实现 mac 端拉起/停止；macOS 窗口不加 vibrancy 效果。
- 行为不变约束（P2 项）：注释清理、常量命名、可读性改写不得改变 Windows 运行行为；日志轮转 KeepSome(5) 是 P1 第 5 项的既定行为变更。
- windows-gnu 测试规避规则继续生效（docs/testing.md：纯逻辑单测、无 Child static、无 async command、不写 tests/ 集成测试）。
- 文档语言中文；UTF-8 显式读写。

## File Boundaries

- \`src-tauri/src/window.rs\`: 平台隔离 + 注释清理（Task1）
- \`src-tauri/src/settings.rs\`、\`src-tauri/src/service.rs\`: 平台隔离、panic 修复、pid 时序、魔法数字（Task2）
- \`src-tauri/src/config.rs\`、\`src-tauri/src/lib.rs\`、\`src-tauri/src/logging.rs\`、\`frontend-dist/index.html\`、\`src-tauri/tauri.conf.json\`（仅 CSP hash）: Task3
- \`docs/testing.md\`、\`README.md\`、\`CHANGELOG.md\`: Task4

## Tasks

### Task 1: window.rs 平台隔离与注释清理

**Purpose**
- macOS 编译恢复（cfg 隔离），注释与实现一致。

**Code Fact Sources**
- Read: \`src-tauri/src/window.rs\` L44-109（create_main_window）、L162-201（work_area_ffi）、L203-315（popup 样式与 style_guard）
- Read: \`src-tauri/src/config.rs\`（INITIAL_WINDOW_SIZE/MIN_WINDOW_SIZE 常量）

**File Boundaries**
- Modify: \`src-tauri/src/window.rs\`（仅 create_main_window 区域与注释；不动初始尺寸算法与单测）

**Key Symbols**
- Change: \`create_main_window()\` 内工作区获取与窗口样式段
- Preserve: \`initial_window_size()\`、\`work_area()\` command、\`show_main_window()\`

**Implementation Notes**
\`\`\`rust
// 1) 工作区获取改为跨平台 helper：
//    #[cfg(target_os = "windows")]
//    fn primary_work_area() -> (u32, u32) { 由 work_area_ffi 包装（max(0) 收敛） }
//    #[cfg(not(target_os = "windows"))]
//    fn primary_work_area(app: &tauri::AppHandle) -> (u32, u32) {
//        app.primary_monitor().ok().flatten() 的 work_area 尺寸（max(1)），
//        失败回退 INITIAL_WINDOW_SIZE
//    }
// 2) 窗口样式段整体包裹：
//    #[cfg(target_os = "windows")]
//    {
//        apply_window_effect(&window);
//        disable_window_rounding(&window);
//        if let Ok(hwnd) = window.hwnd() { make_window_popup_style(...); style_guard::install(...); }
//    }
// 3) 注释：删除 L72-75/L89-91 重复块（保留一处）；L208 过时注释改为
//    “tao 会重置样式，由 WM_STYLECHANGED 样式守卫即时修正（无轮询线程）”
// 4) L20 current_monitor().ok()?? 改写为显式 let-else（行为不变）
\`\`\`

**Test Proof**
- \`cargo test --lib\`：window 模块 5 例不回归；\`cargo clippy -D warnings\` 零告警。
- \`cargo check --target x86_64-apple-darwin\`：window.rs 平台错误清零（本任务后其余模块仍可能报错，属 Task2 范围）。

**Verification Command**
\`\`\`bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --lib
\`\`\`

**Completion Conditions**
- create_main_window 内无裸调用 Windows-only 符号；注释与实现一致且无重复。
- clippy/test 全绿。

**Prohibited Drift**
- 不改初始尺寸算法、伪最大化逻辑；不在 macOS 加 vibrancy；不动 capabilities。

**Deviation Conditions**
- 若 primary_monitor API 签名与预期不符（tauri v2 版本差异），停止并回报。

### Task 2: service.rs / settings.rs 平台隔离与缺陷修复

**Purpose**
- macOS 编译恢复；服务探活不 panic；停止失败保留 pid；魔法数字命名。

**Code Fact Sources**
- Read: \`src-tauri/src/service.rs\`（全文）、\`src-tauri/src/settings.rs\`（open_log_dir）、\`src-tauri/src/lib.rs\` L42-52（invoke_handler 注册面）
- Read: \`docs/testing.md\` 0xc0000139 规避规则（改动后单测仍须合规）

**File Boundaries**
- Modify: \`src-tauri/src/service.rs\`、\`src-tauri/src/settings.rs\`

**Key Symbols**
- Change: \`service_reachable()\`、\`stop_service()\`、Windows 实现函数 cfg 标注、\`open_log_dir()\`
- Preserve: \`service_start/service_stop\` command 签名与注册、SERVICE_PID 用 u32 pid（非 Child）约定

**Implementation Notes**
\`\`\`rust
// service.rs：
// 1) pub(crate) const DETACHED_PROCESS: u32 = 0x0000_0008;  // 注释统一：
//    DETACHED_PROCESS(0x8)；不弹黑窗效果与 CREATE_NO_WINDOW 相近
// 2) #[cfg(target_os = "windows")] 标注：SERVICE_PID、service_pid、find_dsh_in、
//    find_dsh_in_with、find_dsh、service_reachable、spawn_dsh；
//    start_service/stop_service 拆为两版：
//    - windows 版：现有逻辑（service_reachable 用 let-else 防空 vec panic；
//      stop_service 先拷贝 pid、taskkill 成功后再 *guard = None）
//    - not(windows) 版：start → Err("当前平台暂不支持从壳层拉起 DSH 服务，请手动启动 dsh")；
//      stop → Err("当前平台暂不支持从壳层停止 DSH 服务")
// 3) service_reachable：
//    let Ok(addrs) = url.socket_addrs(|| None) else { return false };
//    let Some(addr) = addrs.first() else { return false };
// 4) creation_flags 替换为 DETACHED_PROCESS 常量
// settings.rs：
// 5) open_log_dir 内 explorer+creation_flags 段 #[cfg(target_os = "windows")]；
//    #[cfg(not(target_os = "windows"))] 用 Command::new("open").arg(&dir).spawn()
\`\`\`

**Test Proof**
- \`cargo test --lib\` 全绿（现有 23 例，本任务不新增测试——service 逻辑受 0xc0000139 规避约束，以手工冒烟为主）。
- clippy 零告警；Windows debug 构建通过（行为等价验证）。
- \`cargo check --target x86_64-apple-darwin\`：service/settings 平台错误清零。

**Verification Command**
\`\`\`bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo build --manifest-path src-tauri/Cargo.toml
\`\`\`

**Completion Conditions**
- 平台错误清零；Windows 行为不变（构建+测试通过）；常量命名落地且注释语义一致。

**Prohibited Drift**
- 不实现 mac 端真实服务管理；不引入 kill 库；不改 command 返回类型。

**Deviation Conditions**
- 若非 Windows stub 导致 invoke_handler 注册报错（命令必须在 handler 中），停止并回报（需调注册面）。

### Task 3: URL 编码 / 日志轮转 / 壳页注入面

**Purpose**
- 编码完整化 + 单测；日志封顶；innerHTML 注入面消除。

**Code Fact Sources**
- Read: \`src-tauri/src/lib.rs\` L70-84（手写编码）、\`src-tauri/src/config.rs\`（dsh_url/常量与测试风格）、\`src-tauri/src/logging.rs\`、\`frontend-dist/index.html\` L364-370（target-line 渲染）
- Read: \`scripts/check-frontend.mjs\`（CSP hash 校验机制——改内联 JS 后必须同步 hash）

**File Boundaries**
- Modify: \`src-tauri/src/config.rs\`（新增 encode_query_param + 单测）、\`src-tauri/src/lib.rs\`（改调 helper）、\`src-tauri/src/logging.rs\`（KeepSome(5)）、\`frontend-dist/index.html\`（target-line DOM 构建）、\`src-tauri/tauri.conf.json\`（仅 CSP script-src hash）

**Key Symbols**
- Change: \`encode_query_param(url: &Url) -> String\`（config.rs 新增 pub(crate)）；\`RotationStrategy::KeepSome(5)\`；target-line 渲染方式
- Preserve: \`dsh_url()\` 语义；CSP 其余指令；壳页其余 JS

**Implementation Notes**
\`\`\`rust
// encode_query_param：percent-encode 除 unreserved（A-Z a-z 0-9 - _ . ~）外全部字符，
// 非 ASCII 按 UTF-8 字节逐字节 %XX（url crate 规范化后基本是 ASCII，防御性处理）。
// 单测：'#'→%23、'+'→%2B、' '→%20、中文→UTF-8 字节序列、'~' 保留、
//   ':' '/' '?' '&' '=' '%' 均编码（与 JS URLSearchParams.get 解码对称）
\`\`\`
\`\`\`js
// index.html：target-line 改为 replaceChildren + createTextNode/createElement("code")，
// 不再 innerHTML 拼接 DSH_URL；视觉与文案不变
\`\`\`

**Test Proof**
- \`cargo test --lib\`：新增编码用例全绿（含反例：#/+ 空格不丢失、round-trip 与 URLSearchParams 语义一致）。
- \`npm run check\`：CSP hash 更新后通过（check-frontend 验证 hash 一致性）。
- \`cargo clippy -D warnings\` 零告警。

**Verification Command**
\`\`\`bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run check
\`\`\`

**Completion Conditions**
- 编码单测通过；npm run check 通过（CSP hash 已同步）；KeepSome(5) 生效（编译验证）；target-line 无 innerHTML 拼接 URL。

**Prohibited Drift**
- 不引入 percent-encoding 等新 crate；不改 index.html 视觉/文案；不动 CSP 其他指令。

**Deviation Conditions**
- 若 KeepSome 与 tauri-plugin-log 2.9.0 实际 API 不符（以 registry 源码为准：KeepSome(usize) 已确认），停止并回报。

### Task 4: 文档一致性修正

**Purpose**
- 文档与实际实现/CI 一致；CHANGELOG 补齐。

**Code Fact Sources**
- Read: \`docs/testing.md\` L90-94、\`README.md\` L119-137、\`CHANGELOG.md\`、\`.github/workflows/ci.yml\`（冒烟不在 CI 的事实）

**File Boundaries**
- Modify: \`docs/testing.md\`（仅 CI 段落）、\`README.md\`（仅故障排查日志段落）、\`CHANGELOG.md\`（仅 Unreleased）

**Key Symbols**
- Change: testing.md「CI 接入」段落改写为与实际 ci.yml 一致（fmt→clippy→单测→前端检查→debug 构建→release+NSIS，冒烟不在 CI、由本地承担）；README 日志描述改为文件日志；CHANGELOG Unreleased 增两条（发布自动化、macOS 适配与缺陷修复）
- Preserve: 其余章节原文

**Implementation Notes**
- CHANGELOG 新条目：\`T4.4 GitHub Release 自动构建（release.yml：v* tag 触发，macOS/Windows 安装包自动发布草稿 Release）\`；\`修复：macOS 构建适配（平台隔离）、服务探活 panic、URL 参数编码丢字符、停止服务失败丢 pid、日志轮转封顶\`

**Test Proof**
- 无行为代码；grep 断言：testing.md 不含「CI 中使用 dsh --profile web」过时表述；README 不含「暂以调试输出为主」；CHANGELOG 含新条目关键词。

**Verification Command**
\`\`\`bash
node -e "…三文件关键词断言…"
\`\`\`

**Completion Conditions**
- 三处文档断言全过；无新增过时表述。

**Prohibited Drift**
- 不改发版清单/测试说明其余部分；不在 CHANGELOG 提前宣布未完成功能。

**Deviation Conditions**
- 若发现其他文档与实现矛盾（超出本清单），记录并回报，不顺手改。

## Integrated Verification

1. \`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings\` → 0 告警
2. \`cargo test --manifest-path src-tauri/Cargo.toml --lib\` → 全绿（23 + 新增）
3. \`npm run check\` → 全绿（CSP hash 已同步）
4. \`cargo build --manifest-path src-tauri/Cargo.toml\`（debug）→ 通过
5. \`rustup target add x86_64-apple-darwin\` + \`cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-apple-darwin\` → 通过（尽力；失败则记录原因，以 CI 为最终验证）
6. 文档断言全绿；git 改动范围 = 计划内文件

## Completion Checks

- 所有任务完成条件满足；无 Prohibited Drift 越界。
- 交付说明：改动清单、macOS 验证边界（交叉 check 结果）、剩余未纳入项（开机自启逻辑抽取、HEAD 探测、resize pointer capture、theme 手写 YAML 解析——记录为后续可选）。
