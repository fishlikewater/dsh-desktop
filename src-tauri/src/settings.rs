//! 设置页 command 集：开机自启、检查更新、打开日志目录。
//!
//! 服务地址为启动时读取（T1.4 配置层），运行中修改需重启，设置页只读展示。

use std::path::Path;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

/// 读取开机自启状态（注册表为准）。
#[tauri::command]
pub fn autostart_state(app: tauri::AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// 切换开机自启，返回切换后的新状态。
#[tauri::command]
pub fn autostart_toggle(app: tauri::AppHandle) -> Result<bool, String> {
    let mgr = app.autolaunch();
    let target = !mgr.is_enabled().unwrap_or(false);
    let result = if target { mgr.enable() } else { mgr.disable() };
    match result {
        Ok(()) => {
            log::info!(target: "settings", "开机自启已切换 -> {target}");
            Ok(target)
        }
        Err(e) => Err(format!("切换开机自启失败: {e}")),
    }
}

/// 检查更新（tauri-plugin-updater，ed25519 签名校验由插件负责）。
/// 无可用更新 / 检查失败（endpoint 404 等）均返回可读消息，不崩溃。
/// 注意：不用 async command——windows-gnu 下 tauri async command 会使
/// test 二进制加载失败（0xc0000139），见 docs/testing.md；用 block_on 同步化。
#[tauri::command]
pub fn update_check(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater()
        .map_err(|e| format!("更新器不可用: {e}"))?;
    match tauri::async_runtime::block_on(updater.check()) {
        Ok(Some(update)) => {
            log::info!(target: "settings", "发现新版本 {}", update.version);
            Ok(format!("发现新版本 {}", update.version))
        }
        Ok(None) => Ok("当前已是最新版本".into()),
        Err(e) => {
            log::warn!(target: "settings", "检查更新失败: {e}");
            Err(format!("检查更新失败: {e}"))
        }
    }
}

/// 打开日志目录（%LOCALAPPDATA%\com.dsh.desktop\logs）到资源管理器。
#[tauri::command]
pub fn open_log_dir() -> Result<String, String> {
    let appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let dir = Path::new(&appdata).join("com.dsh.desktop").join("logs");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(format!("创建日志目录失败: {e}"));
    }
    // DETACHED_PROCESS：explorer 独立于壳层进程（同 T5.1 的 rustc 1.95 注意点）
    let child = std::process::Command::new("explorer")
        .arg(dir.to_str().unwrap_or_default())
        .creation_flags(0x0000_0008)
        .spawn();
    match child {
        Ok(_) => {
            log::info!(target: "settings", "已打开日志目录 {}", dir.display());
            Ok(format!("已打开日志目录 {}", dir.display()))
        }
        Err(e) => Err(format!("打开日志目录失败: {e}")),
    }
}
