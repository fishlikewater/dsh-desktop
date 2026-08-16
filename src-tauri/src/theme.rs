//! 主题模块：读取 DSH 外观偏好（settings.yaml）并监听变化即时通知壳页。

use tauri::{AppHandle, Emitter};

use crate::config::{theme_settings_path, THEME_FILE_CHANGED_EVENT};

/// 读取 DSH 外观主题偏好（~/.dsh/settings.yaml 的 ui-theme.preference：
/// light | dark | system；由 DSH GUI 外观设置持久化）。
/// 壳页用它同步自定义标题栏配色（system 时前端跟随系统 prefers-color-scheme）。
#[tauri::command]
pub fn theme_preference() -> String {
    let path = theme_settings_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
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

/// 监听 DSH 主题持久化文件（~/.dsh/settings.yaml）变化，变化时向壳页 emit
/// "theme-file-changed" 信号，壳页收到后立即重新读取主题——取代固定轮询的
/// 0~3s 随机延迟（"慢一拍"）。
///
/// 注意：DSH 的 writeFileAtomic 是临时文件 + rename 覆盖（dsh-atomic-write），
/// rename 会替换 inode，直接 watch 文件本身在 Windows 上会丢后续事件，
/// 因此 watch 父目录、按文件名过滤；rename 提交后立即读文件即拿到新内容。
/// writeFileAtomic 一次写入会产生多个事件，用 100ms 防抖合并为一次 emit。
pub fn watch_settings_theme(app: &AppHandle) {
    use notify::{RecursiveMode, Watcher};

    let path = theme_settings_path();
    let Some(dir) = path.parent() else { return };
    let Some(target) = path.file_name() else { return };
    let target_name = target.to_string_lossy().into_owned();

    // 应用运行期间 watcher 必须常驻：泄漏进静态区（生命周期与进程一致），
    // 避免 Drop 时停止监听线程。
    static WATCHER: std::sync::OnceLock<notify::RecommendedWatcher> =
        std::sync::OnceLock::new();

    let app = app.clone();
    let mut last_emit = std::time::Instant::now() - std::time::Duration::from_secs(1);
    let Ok(mut watcher) = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        // rename 事件的 paths 是最终路径（settings.yaml），临时文件 .tmp 不匹配
        let hit = event.paths.iter().any(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy() == target_name.as_str())
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
        let _ = app.emit(THEME_FILE_CHANGED_EVENT, ());
    }) else {
        return;
    };
    let Ok(_) = watcher.watch(dir, RecursiveMode::NonRecursive) else {
        return;
    };
    // 事件回调运行在 notify 独立线程，app 克隆可跨线程使用
    let _ = WATCHER.set(watcher);
}
