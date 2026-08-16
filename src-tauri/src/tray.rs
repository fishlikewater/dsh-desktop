//! 托盘模块：常驻托盘图标与菜单（显示窗口/开机自启/测试通知/退出）。

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

use crate::window::show_main_window;

/// 发送系统通知（供前端调用或托盘菜单触发）
#[tauri::command]
pub(crate) fn notify(app: tauri::AppHandle, title: String, body: String) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

/// 构建系统托盘：左键单击显示窗口，菜单含「显示主窗口」「开机自启」「测试通知」「退出」
pub fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let auto_enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let autostart_item = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自启",
        true,
        auto_enabled,
        None::<&str>,
    )?;
    let notify_item = MenuItem::with_id(app, "notify", "发送测试通知", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show_item, &autostart_item, &notify_item, &quit_item],
    )?;

    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        // 兜底：1x1 透明像素，正常情况下不会走到这里
        tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1)
    });

    // 事件闭包中需要更新勾选状态，这里克隆句柄（内部 Arc 共享）
    let autostart_handle = autostart_item.clone();

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("DSH Desktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                log::debug!(target: "tray", "menu: show");
                show_main_window(app);
            }
            "autostart" => {
                let mgr = app.autolaunch();
                let target = !mgr.is_enabled().unwrap_or(false);
                let result = if target { mgr.enable() } else { mgr.disable() };
                match result {
                    Ok(()) => {
                        log::info!(target: "tray", "开机自启已切换 -> {target}");
                        let _ = autostart_handle.set_checked(target);
                    }
                    Err(e) => log::error!(target: "tray", "切换开机自启失败: {e}"),
                }
            }
            "notify" => {
                log::debug!(target: "tray", "menu: notify");
                notify(
                    app.clone(),
                    "DSH Desktop".to_string(),
                    "通知通道工作正常 ✓".to_string(),
                );
            }
            "quit" => {
                log::info!(target: "tray", "menu: quit");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
