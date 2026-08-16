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

/// 壳层配置（未知字段忽略；缺失/损坏回退默认）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShellConfig {
    /// DSH 服务地址覆盖（环境变量 DSH_URL 优先）
    #[serde(default)]
    pub dsh_url: Option<String>,
    /// DSH 数据目录覆盖（环境变量 DSH_HOME 优先）
    #[serde(default)]
    pub dsh_home: Option<String>,
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
/// env 非空 → env；config 非空 → config；userprofile 非空 → userprofile；否则 "."。
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
            userprofile
                .filter(|v| !v.trim().is_empty())
                .map(String::from)
        })
        .unwrap_or_else(|| ".".into())
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

/// DSH 外观主题持久化文件路径：$DSH_HOME/settings.yaml
/// （DSH_HOME 解析优先级：环境变量 > config.json dsh_home > USERPROFILE）。
pub fn theme_settings_path() -> PathBuf {
    let home = resolve_dsh_home(
        std::env::var("DSH_HOME").ok().as_deref(),
        shell_config().dsh_home.as_deref(),
        std::env::var("USERPROFILE").ok().as_deref(),
    );
    Path::new(&home).join("settings.yaml")
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
        assert_eq!(resolve_dsh_home(None, None, Some("up")), "up");
        assert_eq!(resolve_dsh_home(None, None, None), ".");
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
