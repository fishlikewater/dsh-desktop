//! 主题模块：读取 DSH 外观偏好（settings.yaml）并监听变化即时通知壳页。

use tauri::{AppHandle, Emitter};

use crate::config::{theme_settings_path, THEME_FILE_CHANGED_EVENT};

/// 解析 settings.yaml 文本中的 ui-theme.preference（light | dark | system，默认 light）。
/// 纯函数便于单测：损坏 YAML、缺失段、未知值、空白一律返回 "light"。
pub fn parse_theme_preference(content: &str) -> String {
    // 定位 ui-theme 段（顶层键顶格、段内字段缩进），读取其 preference 字段
    let mut in_theme = false;
    for line in content.lines() {
        let t = line.trim();
        if line.starts_with("ui-theme:") {
            in_theme = true;
            continue;
        }
        if in_theme {
            // 遇到下一个顶格键 → 离开 ui-theme 段
            if !line.starts_with(' ') && !line.starts_with('\t') && t.ends_with(':') {
                break;
            }
            if let Some(v) = t.strip_prefix("preference:") {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if matches!(v, "light" | "dark" | "system") {
                    return v.into();
                }
                break;
            }
        }
    }
    "light".into()
}

/// 读取 DSH 外观主题偏好（<DSH 数据目录>/settings.yaml 的 ui-theme.preference：
/// light | dark | system；数据目录默认 ~/.dsh，由 DSH GUI 外观设置持久化）。
/// 壳页用它同步自定义标题栏配色（system 时前端跟随系统 prefers-color-scheme）。
#[tauri::command]
pub fn theme_preference() -> String {
    let path = theme_settings_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    parse_theme_preference(&content)
}

/// 监听 DSH 主题持久化文件（~/.dsh/settings.yaml）变化，变化时向壳页 emit
/// "theme-file-changed" 信号，壳页收到后立即重新读取主题——取代固定轮询的
/// 0~3s 随机延迟（"慢一拍"）。
///
/// 注意：DSH 的 writeFileAtomic 是临时文件 + rename 覆盖（dsh-atomic-write），
/// rename 会替换 inode，直接 watch 文件本身在 Windows 上会丢后续事件，
/// 因此 watch 父目录、按文件名过滤；rename 提交后立即读文件即拿到新内容。
/// writeFileAtomic 一次写入会产生多个事件，用 100ms 防抖合并为一次 emit。
pub fn watch_settings_theme(app: &AppHandle) {
    if try_start_theme_watcher(app) {
        return;
    }
    // 首次启动失败（文件路径异常/权限等）：后台线程每 30s 重试，成功即停。
    // 重试期间壳页 30s 兜底轮询仍在工作，主题同步不会永久失效。
    log::warn!(target: "theme", "主题文件监听启动失败，30s 后重试（期间依赖壳页兜底轮询）");
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        if try_start_theme_watcher(&app) {
            log::info!(target: "theme", "主题文件监听重试成功");
            break;
        }
    });
}

/// 尝试启动主题文件监听；成功返回 true（含已启动的情况）。
fn try_start_theme_watcher(app: &AppHandle) -> bool {
    use notify::{RecursiveMode, Watcher};

    let path = theme_settings_path();
    let Some(dir) = path.parent() else {
        return false;
    };
    let Some(target) = path.file_name() else {
        return false;
    };
    let target_name = target.to_string_lossy().into_owned();
    // 闭包内按文件名过滤；外层日志复用同名变量（闭包 move 后不可再借用）
    let filter_name = target_name.clone();

    // 应用运行期间 watcher 必须常驻：泄漏进静态区（生命周期与进程一致），
    // 避免 Drop 时停止监听线程。
    static WATCHER: std::sync::OnceLock<notify::RecommendedWatcher> = std::sync::OnceLock::new();
    if WATCHER.get().is_some() {
        return true; // 已在监听
    }

    let app = app.clone();
    let mut last_emit = std::time::Instant::now() - std::time::Duration::from_secs(1);
    let Ok(mut watcher) = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        // rename 事件的 paths 是最终路径（settings.yaml），临时文件 .tmp 不匹配
        let hit = event.paths.iter().any(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy() == filter_name.as_str())
                .unwrap_or(false)
        });
        if !hit {
            return;
        }
        let now = std::time::Instant::now();
        if now.duration_since(last_emit) < std::time::Duration::from_millis(100) {
            return;
        }
        last_emit = now;
        log::debug!(target: "theme", "settings.yaml 变化 -> emit {THEME_FILE_CHANGED_EVENT}");
        let _ = app.emit(THEME_FILE_CHANGED_EVENT, ());
    }) else {
        log::warn!(target: "theme", "创建 settings.yaml watcher 失败，主题同步降级为壳页 30s 兜底轮询");
        return false;
    };
    if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
        log::warn!(target: "theme", "监听 {} 失败: {e}，主题同步降级为壳页 30s 兜底轮询", path.display());
        return false;
    }
    // 事件回调运行在 notify 独立线程，app 克隆可跨线程使用
    let _ = WATCHER.set(watcher);
    log::info!(target: "theme", "已监听主题文件 {}（文件名过滤 {target_name}）", path.display());
    true
}

#[cfg(test)]
mod tests {
    use super::parse_theme_preference;

    #[test]
    fn parses_dark() {
        assert_eq!(
            parse_theme_preference("ui-theme:\n  preference: dark\n"),
            "dark"
        );
    }

    #[test]
    fn parses_quoted_value() {
        assert_eq!(
            parse_theme_preference("ui-theme:\n  preference: \"system\"\n"),
            "system"
        );
    }

    #[test]
    fn defaults_light_when_section_missing() {
        assert_eq!(
            parse_theme_preference("agent-default-model:\n  model: x\n"),
            "light"
        );
    }

    #[test]
    fn defaults_light_when_field_missing() {
        assert_eq!(parse_theme_preference("ui-theme:\n  other: 1\n"), "light");
    }

    #[test]
    fn defaults_light_on_unknown_value() {
        assert_eq!(
            parse_theme_preference("ui-theme:\n  preference: blue\n"),
            "light"
        );
    }

    #[test]
    fn defaults_light_on_garbage_content() {
        assert_eq!(parse_theme_preference("{{{ not yaml ]]]"), "light");
    }

    #[test]
    fn defaults_light_on_empty() {
        assert_eq!(parse_theme_preference(""), "light");
        assert_eq!(parse_theme_preference("   \n\n"), "light");
    }

    #[test]
    fn ignores_other_sections_before_ui_theme() {
        let content = "mcp:\n  servers: []\nui-theme:\n  preference: light\n";
        assert_eq!(parse_theme_preference(content), "light");
    }

    #[test]
    fn stops_at_next_top_level_key() {
        let content = "ui-theme:\n  preference: dark\ndsh-tasks:\n  tasks: []\n";
        assert_eq!(parse_theme_preference(content), "dark");
    }
}
