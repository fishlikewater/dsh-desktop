//! 全局快捷键模块：从配置读取键位注册（Task 18：设置页可修改，即时生效）。
//!
//! 命令定义在此模块（不在 lib.rs 顶层）——generate_handler 生成的
//! `__cmd__` 导入与同模块定义冲突（E0255）。

use std::sync::{Mutex, OnceLock};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{config, window};

/// 当前注册的全局快捷键（设置修改后 unregister_all + 重新注册）
static CURRENT_SHORTCUT: OnceLock<Mutex<Option<Shortcut>>> = OnceLock::new();

fn current_shortcut() -> &'static Mutex<Option<Shortcut>> {
    CURRENT_SHORTCUT.get_or_init(|| Mutex::new(None))
}

/// 按 spec 注册全局快捷键（解析失败回退默认并告警；注册失败仅告警不阻塞）。
fn register_global_shortcut(app: &tauri::AppHandle, spec: &str) {
    let (mods, code) = match config::parse_shortcut(spec) {
        Some(v) => v,
        None => {
            log::warn!(
                target: "shortcut",
                "快捷键配置非法（{spec}），回退默认 {}",
                config::DEFAULT_SHORTCUT
            );
            // 默认配置本身应合法；仍校验一次（防未来默认值改动）
            let Some(v) = config::parse_shortcut(config::DEFAULT_SHORTCUT) else {
                log::error!(target: "shortcut", "默认快捷键也非法——跳过注册");
                return;
            };
            v
        }
    };
    let shortcut = Shortcut::new(Some(mods), code);
    if let Err(e) = app
        .global_shortcut()
        .on_shortcut(shortcut, |app, _sc, event| {
            if event.state() == ShortcutState::Pressed {
                log::info!(target: "shortcut", "全局快捷键 pressed -> show main window");
                window::show_main_window(app);
            }
        })
    {
        log::error!(target: "shortcut", "注册全局快捷键回调失败: {e}");
    }
    if let Err(e) = app.global_shortcut().register(shortcut) {
        // 快捷键可能被其他应用占用，此时仅告警，不影响壳层运行
        log::warn!(target: "shortcut", "注册全局快捷键 {spec} 失败: {e}");
    } else {
        log::info!(target: "shortcut", "全局快捷键 {spec} 已注册");
        *current_shortcut().lock().unwrap() = Some(shortcut);
    }
}

/// 快捷键注册入口：从配置读取键位（未设置 → 平台默认）。
pub fn setup_global_shortcut(app: &tauri::AppHandle) {
    let spec = config::get_shortcut();
    register_global_shortcut(app, &spec);
}

/// 设置页修改快捷键后调用：注销旧键位并重新注册（即时生效）。
#[tauri::command]
pub fn reapply_global_shortcut(app: tauri::AppHandle) -> Result<(), String> {
    let _ = app.global_shortcut().unregister_all();
    *current_shortcut().lock().unwrap() = None;
    register_global_shortcut(&app, &config::get_shortcut());
    Ok(())
}
