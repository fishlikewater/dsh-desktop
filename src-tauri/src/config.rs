//! 壳层配置与常量：服务地址、路径、窗口尺寸、事件名。
//!
//! 配置优先级：环境变量（DSH_URL/DSH_HOME）> 壳层配置文件
//! （`%APPDATA%\com.dsh.desktop\config.json`）> 内置默认值。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tauri::Url;

/// 主窗口的 label
pub const MAIN_WINDOW: &str = "main";
/// DSH 服务默认地址
pub const DEFAULT_DSH_URL: &str = "http://127.0.0.1:3080";
/// 主题文件变化事件名（Rust → 壳页）
pub const THEME_FILE_CHANGED_EVENT: &str = "theme-file-changed";
/// 打开设置弹层事件名（托盘菜单 → 壳页）
pub const OPEN_SETTINGS_EVENT: &str = "open-settings";
/// 初始窗口尺寸（大桌面基准）
pub const INITIAL_WINDOW_SIZE: (u32, u32) = (1280, 840);
/// 窗口最小尺寸（与壳页 MIN_W/MIN_H 同步）
pub const MIN_WINDOW_SIZE: (u32, u32) = (860, 620);

/// 壳层配置文件路径：%APPDATA%\com.dsh.desktop\config.json
pub fn shell_config_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    Path::new(&appdata)
        .join("com.dsh.desktop")
        .join("config.json")
}

/// 窗口几何记忆：左上角坐标 + 尺寸（物理像素）与最大化态。
/// 序列化进壳层 config.json（windows 伪最大化语义；macOS 只记几何）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    #[serde(default)]
    pub maximized: bool,
}

/// 壳层配置（未知字段忽略；缺失/损坏回退默认）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShellConfig {
    /// DSH 服务地址覆盖（环境变量 DSH_URL 优先）
    #[serde(default)]
    pub dsh_url: Option<String>,
    /// DSH 数据目录覆盖（环境变量 DSH_HOME 优先）
    #[serde(default)]
    pub dsh_home: Option<String>,
    /// 上次会话的窗口几何（无记忆为 None → 启动走默认居中）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_state: Option<WindowState>,
    /// 关闭行为：true = 点击关闭直接退出；false（默认）= 隐藏到托盘常驻
    #[serde(default)]
    pub close_behavior: bool,
    /// 首次关闭引导通知已发过（仅一次，跨会话持久）
    #[serde(default)]
    pub first_close_notified: bool,
    /// 服务崩溃后自动重新拉起（默认关闭；开启后前端离线分支自动调用
    /// service::auto_start_service，节流在 service.rs 侧）
    #[serde(default)]
    pub auto_restart_enabled: bool,
}

impl ShellConfig {
    /// 从磁盘读取；缺失/损坏/非法结构回退默认并告警。
    pub fn load_from(path: &Path) -> ShellConfig {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<ShellConfig>(&text) {
                Ok(cfg) => cfg,
                Err(e) => {
                    log::warn!(target: "config", "解析 {} 失败: {e}，使用默认配置", path.display());
                    ShellConfig::default()
                }
            },
            Err(_) => ShellConfig::default(),
        }
    }

    /// 写回磁盘（原子写：临时文件 + rename，避免半截文件）。
    /// 父目录不存在时创建（%APPDATA%\com.dsh.desktop 首启可能没有）。
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)
    }
}

/// 进程内配置缓存（首次访问时从磁盘加载一次）。
static SHELL_CONFIG: OnceLock<ShellConfig> = OnceLock::new();

/// 访问壳层配置（进程内不变）。
pub fn shell_config() -> &'static ShellConfig {
    SHELL_CONFIG.get_or_init(|| {
        let path = shell_config_path();
        let cfg = ShellConfig::load_from(&path);
        log::info!(
            target: "config",
            "壳层配置已加载（{}，dsh_url={:?}，dsh_home={:?}）",
            path.display(),
            cfg.dsh_url,
            cfg.dsh_home
        );
        cfg
    })
}

/// 校验并解析一个服务地址字符串；合法（http + host）返回 Some。
fn parse_url(value: &str) -> Option<Url> {
    match Url::parse(value.trim()) {
        Ok(u) if u.has_host() && u.scheme() == "http" => Some(u),
        _ => None,
    }
}

/// 解析 DSH 服务地址（纯函数，便于单测）：
/// env 合法 → env；env 非法 → 默认（告警）；env 缺失 → config 合法 → config；否则默认。
fn resolve_dsh_url(env_value: Option<&str>, config_value: Option<&str>) -> Url {
    let fallback = || Url::parse(DEFAULT_DSH_URL).expect("默认 DSH 地址非法");
    if let Some(v) = env_value {
        if let Some(u) = parse_url(v) {
            return u;
        }
        log::warn!(target: "config", "DSH_URL 无效（需 http://host:port），使用默认 {DEFAULT_DSH_URL}");
        return fallback();
    }
    if let Some(v) = config_value {
        if let Some(u) = parse_url(v) {
            return u;
        }
        log::warn!(target: "config", "config.json dsh_url 无效（{v}），使用默认 {DEFAULT_DSH_URL}");
    }
    fallback()
}

/// 读取 DSH 服务地址：环境变量 DSH_URL > config.json dsh_url > 默认；非法值回退默认。
pub fn dsh_url() -> Url {
    resolve_dsh_url(
        std::env::var("DSH_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .as_deref(),
        shell_config().dsh_url.as_deref(),
    )
}

/// 解析 DSH 数据目录（纯函数，便于单测）：
/// env 非空 → env；config 非空 → config；userprofile 非空 → userprofile/.dsh；
/// 否则 ./.dsh。DSH 默认数据目录为 ~/.dsh（DSH_HOME 与 config.json dsh_home
/// 语义本就是数据目录本身，原样直用；仅 USERPROFILE 兜底需追加 .dsh）。
fn resolve_dsh_home(
    env_value: Option<&str>,
    config_value: Option<&str>,
    userprofile: Option<&str>,
) -> String {
    env_value
        .filter(|v| !v.trim().is_empty())
        .map(String::from)
        .or_else(|| {
            config_value
                .filter(|v| !v.trim().is_empty())
                .map(String::from)
        })
        .or_else(|| {
            // DSH 默认数据目录：<userprofile>/.dsh（"/" 在 Windows 上同样合法；
            // 纯字符串拼接而非 Path join：windows-gnu 下测试代码路径中的 PathBuf
            // 构造会触发 0xc0000139，见 docs/testing.md）
            userprofile
                .filter(|v| !v.trim().is_empty())
                .map(|v| format!("{v}/.dsh"))
        })
        .unwrap_or_else(|| "./.dsh".into())
}

/// 把服务地址编码为壳页 query 参数值（与 JS URLSearchParams.get 解码对称）：
/// 除 unreserved 字符（A-Z a-z 0-9 - _ . ~）外全部 percent-encode，
/// 非 ASCII 字符按 UTF-8 字节逐字节编码（Url 规范化后基本是 ASCII，防御性处理）。
/// 手写编码的动机：必须编码 #（防 fragment 截断）与 +（防被解码为空格）。
pub fn encode_query_param(url: &Url) -> String {
    url.as_str()
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// DSH 外观主题持久化文件路径：<DSH 数据目录>/settings.yaml
/// （数据目录解析优先级：DSH_HOME 环境变量 > config.json dsh_home > USERPROFILE/.dsh）。
pub fn theme_settings_path() -> PathBuf {
    let home = resolve_dsh_home(
        std::env::var("DSH_HOME").ok().as_deref(),
        shell_config().dsh_home.as_deref(),
        std::env::var("USERPROFILE").ok().as_deref(),
    );
    Path::new(&home).join("settings.yaml")
}

/// 读取持久化的窗口状态，并 clamp 进当前工作区（记忆超出新显示器/工作区时
/// 自动收缩拉回，前端拿到的即是可落地几何）。
/// 前端在 show 回调后调用（隐藏窗口上 setPosition 无效）。
#[tauri::command]
pub fn load_window_state(window: tauri::WebviewWindow) -> WindowState {
    let state = shell_config().window_state.unwrap_or_default();
    let Some(wa) = crate::window::work_area(window) else {
        return state;
    };
    clamp_window_state(state, (wa.0, wa.1, wa.2.max(0) as u32, wa.3.max(0) as u32))
}

/// 持久化窗口状态（几何 + 最大化态）。
/// 在内存缓存配置上更新 window_state 再整份写回（保留 dsh_url/dsh_home 等字段）。
#[tauri::command]
pub fn save_window_state(state: WindowState) -> Result<(), String> {
    let path = shell_config_path();
    let mut cfg = shell_config().clone();
    // 无效几何防御：w/h 为 0（窗口尚未布局完成时前端采样）不落盘
    if state.w == 0 || state.h == 0 {
        return Ok(());
    }
    cfg.window_state = Some(state);
    cfg.save_to(&path)
        .map_err(|e| format!("保存窗口状态失败（{}）: {e}", path.display()))
}

// ===== 关闭行为（Task 8：首次关闭引导 + 「关闭时退出」开关）=====

/// 读取「关闭时退出」开关（默认 false = 隐藏到托盘）。
#[tauri::command]
pub fn get_close_behavior() -> bool {
    shell_config().close_behavior
}

/// 读取当前落盘的关闭行为（不依赖进程内 OnceLock 缓存）：
/// 开关切换后立即生效（on_window_event 在每次关闭事件时读最新值）。
pub fn current_close_behavior() -> bool {
    ShellConfig::load_from(&shell_config_path()).close_behavior
}

/// 设置「关闭时退出」开关：更新内存缓存并落盘（即时生效，下次关闭即应用）。
#[tauri::command]
pub fn set_close_behavior(enabled: bool) -> Result<(), String> {
    update_config(|cfg| {
        cfg.close_behavior = enabled;
    })
}

/// 首次关闭引导通知是否已发过（跨会话持久标记，读最新落盘值）。
pub fn first_close_notified() -> bool {
    ShellConfig::load_from(&shell_config_path()).first_close_notified
}

/// 标记首次关闭引导通知已发（写盘持久化；重复调用无副作用）。
pub fn mark_first_close_notified() -> Result<(), String> {
    update_config(|cfg| {
        cfg.first_close_notified = true;
    })
}

/// 读取「服务崩溃后自动重新拉起」开关（默认关闭）。
/// 直接读最新落盘值：设置弹层切换后 tick 离线分支立即可见（不依赖进程内缓存）。
#[tauri::command]
pub fn get_auto_restart() -> bool {
    ShellConfig::load_from(&shell_config_path()).auto_restart_enabled
}

/// 设置「服务崩溃后自动重新拉起」开关（即时生效 + 落盘）。
#[tauri::command]
pub fn set_auto_restart(enabled: bool) -> Result<(), String> {
    update_config(|cfg| {
        cfg.auto_restart_enabled = enabled;
    })
}

/// 通用配置更新：克隆内存缓存 → 修改 → 原子写盘。
/// 注意：shell_config() 是 OnceLock 只读缓存，此函数只落盘不更新缓存
/// （close_behavior 每次都读盘的最新值由 on_window_event 直接 load 保证，
/// 避免引入可变全局）。
fn update_config(mutate: impl FnOnce(&mut ShellConfig)) -> Result<(), String> {
    let path = shell_config_path();
    let mut cfg = shell_config().clone();
    mutate(&mut cfg);
    cfg.save_to(&path)
        .map_err(|e| format!("保存配置失败（{}）: {e}", path.display()))
}

/// 把记忆几何夹取进当前工作区（纯函数，便于单测）：
///
/// - 尺寸：记忆超出工作区 → 缩小到工作区（保留最小约束由调用方负责）；
/// - 位置：左上角夹取到工作区内，并保证窗口不越出右/下边界。
///
/// 工作区输入为 [x, y, width, height]（与 work_area 命令一致）。
pub fn clamp_window_state(state: WindowState, wa: (i32, i32, u32, u32)) -> WindowState {
    let (wa_x, wa_y, wa_w, wa_h) = wa;
    let w = state.w.min(wa_w.max(1));
    let h = state.h.min(wa_h.max(1));
    // 位置：至少与工作区左/上沿对齐；窗口超出右/下沿时拉回
    let x = state.x.clamp(wa_x, wa_x + wa_w as i32 - w as i32);
    let y = state.y.clamp(wa_y, wa_y + wa_h as i32 - h as i32);
    WindowState {
        x,
        y,
        w,
        h,
        maximized: state.maximized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> ShellConfig {
        serde_json::from_str::<ShellConfig>(text).unwrap_or_default()
    }

    #[test]
    fn config_missing_file_returns_default() {
        let cfg = ShellConfig::load_from(Path::new("Z:\\definitely\\not\\exist.json"));
        assert!(cfg.dsh_url.is_none() && cfg.dsh_home.is_none());
    }

    #[test]
    fn config_invalid_json_returns_default() {
        let cfg = ShellConfig::load_from(Path::new("C:\\Windows\\win.ini"));
        assert!(cfg.dsh_url.is_none() && cfg.dsh_home.is_none());
    }

    #[test]
    fn config_valid_fields_parsed() {
        let cfg = parse(r#"{"dsh_url": "http://127.0.0.1:9999", "dsh_home": "D:/data"}"#);
        assert_eq!(cfg.dsh_url.as_deref(), Some("http://127.0.0.1:9999"));
        assert_eq!(cfg.dsh_home.as_deref(), Some("D:/data"));
    }

    #[test]
    fn config_unknown_fields_ignored() {
        let cfg = parse(r#"{"dsh_url": "http://x:1", "extra": [1,2], "nested": {"a":1}}"#);
        assert_eq!(cfg.dsh_url.as_deref(), Some("http://x:1"));
        assert!(cfg.dsh_home.is_none());
    }

    #[test]
    fn resolve_url_env_wins() {
        let url = resolve_dsh_url(Some("http://127.0.0.1:5000"), Some("http://cfg:6000"));
        assert_eq!(url.as_str(), "http://127.0.0.1:5000/");
    }

    #[test]
    fn resolve_url_config_when_env_missing() {
        let url = resolve_dsh_url(None, Some("http://cfg:6000"));
        assert_eq!(url.as_str(), "http://cfg:6000/");
    }

    #[test]
    fn resolve_url_invalid_env_falls_back_to_default() {
        let url = resolve_dsh_url(Some("ftp://bad"), Some("http://cfg:6000"));
        // Url::as_str 为规范形式（带尾斜杠），与默认常量比较时去除尾部斜杠
        assert_eq!(url.as_str().trim_end_matches('/'), DEFAULT_DSH_URL);
    }

    #[test]
    fn resolve_url_empty_values_fall_back() {
        let url = resolve_dsh_url(None, None);
        assert_eq!(url.as_str().trim_end_matches('/'), DEFAULT_DSH_URL);
    }

    #[test]
    fn resolve_home_precedence() {
        assert_eq!(
            resolve_dsh_home(Some("env"), Some("cfg"), Some("up")),
            "env"
        );
        assert_eq!(resolve_dsh_home(Some("  "), Some("cfg"), Some("up")), "cfg");
        assert_eq!(resolve_dsh_home(None, Some("cfg"), Some("up")), "cfg");
        // USERPROFILE 兜底必须追加默认数据目录 .dsh（主题路径解析依赖此约定）
        assert_eq!(resolve_dsh_home(None, None, Some("up")), "up/.dsh");
        assert_eq!(resolve_dsh_home(None, None, None), "./.dsh");
    }

    #[test]
    fn encode_query_param_encodes_reserved_chars() {
        // # 不编码会截断 fragment；? & = % 会让 query 结构错乱；: / 必须编码
        let url = Url::parse("http://127.0.0.1:3080/#/chat?a=1&b=2%20c").unwrap();
        let encoded = encode_query_param(&url);
        assert!(!encoded.contains('#'));
        assert!(encoded.contains("%23"), "{encoded}");
        assert!(encoded.contains("%3F"), "{encoded}");
        assert!(encoded.contains("%26"), "{encoded}");
        assert!(encoded.contains("%3D"), "{encoded}");
        assert!(encoded.contains("%25"), "{encoded}");
        assert!(encoded.contains("%3A"), "{encoded}");
        assert!(encoded.contains("%2F"), "{encoded}");
    }

    #[test]
    fn encode_query_param_encodes_plus() {
        // + 在 query 中会被 URLSearchParams 解码为空格，必须编码为 %2B
        let url = Url::parse("http://127.0.0.1:3080/a+b").unwrap();
        let encoded = encode_query_param(&url);
        assert!(encoded.contains("%2B"), "{encoded}");
        assert!(!encoded.contains("a+b"));
    }

    #[test]
    fn encode_query_param_keeps_unreserved() {
        let url = Url::parse("http://127.0.0.1:3080/ab-1_2.3~z").unwrap();
        let encoded = encode_query_param(&url);
        assert!(encoded.contains("ab-1_2.3~z"), "{encoded}");
    }

    #[test]
    fn encode_query_param_round_trips_with_url_search_params_semantics() {
        // 模拟 JS 侧 URLSearchParams.get("dsh") 的解码语义：
        // 编码值解码后必须还原原 URL 字符串（% + # 等全部往返无损）
        let url = Url::parse("http://127.0.0.1:3080/#/a+b?x=1").unwrap();
        let encoded = encode_query_param(&url);
        let decoded = percent_decode(&encoded);
        assert_eq!(decoded, url.as_str());
    }

    // ===== 窗口状态记忆 =====

    #[test]
    fn window_state_serde_round_trip() {
        let cfg = ShellConfig {
            window_state: Some(WindowState {
                x: 12,
                y: 34,
                w: 1000,
                h: 700,
                maximized: true,
            }),
            ..Default::default()
        };
        let text = serde_json::to_string(&cfg).unwrap();
        let back: ShellConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back.window_state, cfg.window_state);
    }

    #[test]
    fn window_state_missing_field_defaults() {
        // 旧 config（无 window_state 字段）解析为 None
        let cfg = parse(r#"{"dsh_url": "http://x:1"}"#);
        assert!(cfg.window_state.is_none());
        // 部分字段缺失 → 默认 0
        let cfg = parse(r#"{"window_state": {"x": 1, "y": 2, "w": 800, "h": 600}}"#);
        let s = cfg.window_state.expect("应有 window_state");
        assert_eq!(s.x, 1);
        assert!(!s.maximized, "缺失 maximized 应默认 false");
    }

    #[test]
    fn save_to_round_trip_preserves_all_fields() {
        let dir = std::env::temp_dir().join(format!("dsh-config-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let cfg = ShellConfig {
            dsh_url: Some("http://127.0.0.1:9999".into()),
            dsh_home: Some("D:/data".into()),
            window_state: Some(WindowState {
                x: 5,
                y: 6,
                w: 900,
                h: 650,
                maximized: false,
            }),
            ..Default::default()
        };
        cfg.save_to(&path).expect("save_to 应成功");
        let back = ShellConfig::load_from(&path);
        assert_eq!(back.dsh_url, cfg.dsh_url);
        assert_eq!(back.dsh_home, cfg.dsh_home);
        assert_eq!(back.window_state, cfg.window_state);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_to_creates_missing_parent_dir() {
        let dir = std::env::temp_dir().join(format!("dsh-config-nested-{}", std::process::id()));
        let path = dir.join("a").join("b").join("config.json");
        ShellConfig::default()
            .save_to(&path)
            .expect("嵌套父目录应自动创建");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clamp_keeps_state_within_work_area() {
        let state = WindowState {
            x: 100,
            y: 100,
            w: 1000,
            h: 700,
            maximized: false,
        };
        let wa = (0, 0, 1280, 840);
        let c = clamp_window_state(state, wa);
        assert_eq!(c, state); // 工作区足够 → 原样保留
    }

    #[test]
    fn clamp_shrinks_oversized_memory() {
        // 记忆 2000x1200，新工作区只有 1280x840 → 缩到工作区
        let state = WindowState {
            x: 50,
            y: 50,
            w: 2000,
            h: 1200,
            maximized: false,
        };
        let wa = (0, 0, 1280, 840);
        let c = clamp_window_state(state, wa);
        assert_eq!((c.w, c.h), (1280, 840));
    }

    #[test]
    fn clamp_pulls_off_screen_position_back_in() {
        // 位置在工作区外（负坐标 / 超出右边界）→ 拉回工作区内
        let state = WindowState {
            x: -100,
            y: -200,
            w: 800,
            h: 600,
            maximized: false,
        };
        let wa = (100, 50, 1280, 840);
        let c = clamp_window_state(state, wa);
        assert!(c.x >= 100 && c.y >= 50, "左上角应夹在工作区内: {c:?}");
        assert!(c.x + c.w as i32 <= 100 + 1280, "不越出右边界: {c:?}");
        assert!(c.y + c.h as i32 <= 50 + 840, "不越出下边界: {c:?}");
    }

    #[test]
    fn clamp_preserves_maximized_flag() {
        let state = WindowState {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
            maximized: true,
        };
        let c = clamp_window_state(state, (0, 0, 1280, 840));
        assert!(c.maximized);
    }

    // ===== 关闭行为（Task 8）=====

    #[test]
    fn close_behavior_defaults_to_hide_to_tray() {
        // 默认 false = 隐藏到托盘（既有行为不回归）
        let cfg = ShellConfig::default();
        assert!(!cfg.close_behavior);
        let parsed = parse(r#"{"dsh_url": "http://x:1"}"#);
        assert!(!parsed.close_behavior, "旧配置缺字段应默认 false");
    }

    #[test]
    fn close_behavior_serde_round_trip() {
        let cfg = ShellConfig {
            close_behavior: true,
            ..Default::default()
        };
        let text = serde_json::to_string(&cfg).unwrap();
        let back: ShellConfig = serde_json::from_str(&text).unwrap();
        assert!(back.close_behavior);
    }

    #[test]
    fn close_behavior_save_then_load_consistent() {
        let dir = std::env::temp_dir().join(format!("dsh-close-behavior-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        // 同文件多字段共存（窗口状态 + 关闭行为）
        let cfg = ShellConfig {
            window_state: Some(WindowState {
                x: 1,
                y: 2,
                w: 800,
                h: 600,
                maximized: false,
            }),
            close_behavior: true,
            ..Default::default()
        };
        cfg.save_to(&path).unwrap();
        let back = ShellConfig::load_from(&path);
        assert!(back.close_behavior);
        assert_eq!(back.window_state, cfg.window_state);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn first_close_notified_round_trip() {
        let dir = std::env::temp_dir().join(format!("dsh-first-close-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let cfg = ShellConfig {
            first_close_notified: true,
            ..Default::default()
        };
        cfg.save_to(&path).unwrap();
        let back = ShellConfig::load_from(&path);
        assert!(back.first_close_notified, "标记应持久化");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 测试用最小 percent 解码（模拟 URLSearchParams.get 语义）
    fn percent_decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
}
