# Decision Anchor

## 目标
壳页标题栏/覆盖层主题同步由固定 3s 轮询改为**事件驱动**：Rust 侧用 notify 监听 `~/.dsh/settings.yaml`（ui-theme.preference 持久化文件），变化时 emit `theme-file-changed` 事件；前端收到事件立即重新读取主题并应用，轮询降为 30s 低频兜底。

## 验收标准
- [ ] Rust 新增 settings.yaml 父目录 watcher（notify），文件名匹配时 100ms 防抖后 emit 事件；cargo build 零警告。
- [ ] 前端监听 `theme-file-changed` 事件 → 立即 syncTheme()（沿用 lastTheme 去重）。
- [ ] 轮询间隔 3000 → 30000（兜底）。
- [ ] 实测：修改 ~/.dsh/settings.yaml 的 ui-theme.preference，标题栏 1s 内变色（截图验证）；恢复原值。

## 关键决策
- **watch 父目录而非文件本身**：DSH 的 writeFileAtomic 是临时文件 + rename 覆盖，直接 watch 文件会在 inode 替换后丢事件（Windows ReadDirectoryChangesW 行为）。
- **事件只做信号、不带值**：前端收到事件后调用现有 theme_preference command 读取，复用既有解析与去重逻辑，避免 Rust 侧重复解析。
- **保留 30s 兜底轮询**：防 notify 在某些环境（网络盘/RDP）失效后永久失步，代价可忽略。

## 被拒方案
- **仅缩短轮询（500ms）**：仍非即时、持续读盘，不解决"慢一拍"的观感。
- **Rust emit 时直接携带解析后的主题值**：与 theme_preference 重复职责；事件信号 + 前端读取更简单。

## 范围边界
- 范围内: notify 依赖、lib.rs watcher + emit、前端事件监听与兜底频率、README、打包验证。
- 范围外: DSH GUI 主题逻辑、主题变量配色值本身、其他壳层改动。

## 验证命令
- cargo build（debug）
- node --check（提取内联 script）
- 实测 preference 切换 + 截图对比
