# Decision Anchor

## 目标（T3.1，来自 docs/development-plan.md）
Rust 单元测试：核心逻辑可回归。

## 现状
T1.4 已建立 theme（9 例）+ config（9 例）= 18 个单测。本任务补齐：
- 窗口尺寸计算（初始/最小尺寸自适应逻辑）——提取纯函数 `initial_window_size(wa_w, wa_h) -> (init_w, init_h, min_w, min_h)`

## 验收标准
- [ ] `window.rs` 尺寸计算提取纯函数（无 FFI 依赖），3+ 用例：大桌面（1280x840 保持）、小桌面（<860x620 贴屏）、中间值（min 上限裁剪）。
- [ ] cargo test 全绿（≥21 例）。
- [ ] 反例验证：改动尺寸逻辑时测试必失败。
- [ ] npm run check 全绿。

## 范围边界
- 范围内: 窗口尺寸纯函数提取 + 单测。
- 范围外: 其他模块单测补全（冒烟在 T3.2）、CI（T3.3）。

## 验证命令
- cargo test --manifest-path src-tauri/Cargo.toml --lib
- npm run check
