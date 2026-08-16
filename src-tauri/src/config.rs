//! 壳层配置与常量：服务地址、路径、窗口尺寸、事件名。

use tauri::Url;

/// 主窗口的 label
pub const MAIN_WINDOW: &str = "main";
/// DSH 服务默认地址
pub const DEFAULT_DSH_URL: &str = "http://127.0.0.1:3080";
/// 主题文件变化事件名（Rust → 壳页）
pub const THEME_FILE_CHANGED_EVENT: &str = "theme-file-changed";
/// 初始窗口尺寸（大桌面基准）
pub const INITIAL_WINDOW_SIZE: (u32, u32) = (1280, 840);
/// 窗口最小尺寸（与壳页 MIN_W/MIN_H 同步）
pub const MIN_WINDOW_SIZE: (u32, u32) = (860, 620);

/// 读取 DSH 服务地址：支持环境变量 DSH_URL 覆盖（便于不同端口/测试），非法值回退默认。
pub fn dsh_url() -> Url {
    let fallback = || Url::parse(DEFAULT_DSH_URL).expect("默认 DSH 地址非法");
    match std::env::var("DSH_URL") {
        Ok(v) if !v.trim().is_empty() => match Url::parse(v.trim()) {
            Ok(u) if u.has_host() && u.scheme() == "http" => u,
            _ => {
                eprintln!("DSH_URL 无效（需 http://host:port），使用默认 {DEFAULT_DSH_URL}");
                fallback()
            }
        },
        _ => fallback(),
    }
}

/// DSH 外观主题持久化文件路径：$DSH_HOME/settings.yaml（无 DSH_HOME 用 USERPROFILE）。
pub fn theme_settings_path() -> std::path::PathBuf {
    let home = std::env::var("DSH_HOME").unwrap_or_else(|_| {
        std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into())
    });
    std::path::Path::new(&home).join("settings.yaml")
}
