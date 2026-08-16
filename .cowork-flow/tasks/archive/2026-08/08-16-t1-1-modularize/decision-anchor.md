# Decision Anchor

## 目标（T1.1，来自 docs/development-plan.md）
Rust 模块化重构：lib.rs（605 行）拆分为职责模块，**行为完全不变**。

## 拆分方案
- `src/config.rs`：DSH_URL 解析、settings 路径、常量（窗口尺寸/端口/事件名）
- `src/theme.rs`：`theme_preference` 解析 + `watch_settings_theme` watcher + `theme-file-changed` 事件
- `src/window.rs`：窗口创建/居中/样式守卫/伪最大化几何/`work_area`/`work_area_ffi`/`disable_window_rounding`/`make_window_popup_style`/`style_guard`
- `src/tray.rs`：托盘构建与事件
- `src/logging.rs`：占位（T1.2 填充，本次仅建模块骨架）
- `lib.rs`：仅保留 `run()` 装配 + 命令注册

## 验收标准
- [ ] cargo build/test/clippy 零警告零失败。
- [ ] 重构后行为一致：CDP 冒烟（主题即时同步）通过；手动窗口/托盘操作正常。
- [ ] 公开面不变：command 名称（theme_preference/work_area/notify）、事件名（theme-file-changed）、窗口 label（main）保持不变。

## 范围边界
- 范围内: 纯拆分与移动，公共函数/常量提取。
- 范围外: 任何行为变更、日志实现（T1.2）、配置层（T1.4）、时序调整（T1.3）。

## 验证命令
- npm run check（clippy + 前端）
- cargo build
- CDP 冒烟：主题即时同步场景
