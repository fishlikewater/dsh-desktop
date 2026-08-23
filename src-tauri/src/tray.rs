//! 托盘模块：常驻托盘图标与菜单（显示窗口/开机自启/服务启停/配置/测试通知/退出）。

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

use crate::service;
use crate::window::show_main_window;

/// 托盘图标句柄（set_tray_state 切换双态用；None = 尚未创建托盘）。
/// OnceLock<Mutex> 模式与 service.rs::SERVICE_PID 一致。
static TRAY_ICON: std::sync::OnceLock<Mutex<Option<TrayIcon>>> = std::sync::OnceLock::new();

fn tray_icon() -> &'static Mutex<Option<TrayIcon>> {
    TRAY_ICON.get_or_init(|| Mutex::new(None))
}

/// 按状态切换托盘图标（在线绿 / 离线灰）。
/// 壳页 tick 在线/离线分支调用（检测事实源唯一在壳页，不新增 Rust 轮询）。
#[tauri::command]
pub fn set_tray_state(online: bool) {
    let current = tray_icon().lock().unwrap();
    let Some(tray) = current.as_ref() else {
        return; // 托盘未创建（理论不发生）：静默
    };
    let result = tray.set_icon(Some(tray_icon_for(online)));
    match result {
        Ok(()) => {
            log::debug!(target: "tray", "托盘图标 -> {}", if online { "在线" } else { "离线" })
        }
        Err(e) => log::warn!(target: "tray", "切换托盘图标失败: {e}"),
    }
}

/// 加载单态托盘图标（32x32 PNG；编译期内嵌，随应用打包）
fn tray_icon_for(online: bool) -> tauri::image::Image<'static> {
    let bytes = if online {
        include_bytes!("../icons/tray-on.png").as_slice()
    } else {
        include_bytes!("../icons/tray-off.png").as_slice()
    };
    match tauri::image::Image::from_bytes(bytes) {
        Ok(img) => img,
        Err(e) => {
            log::error!(target: "tray", "解析托盘图标失败: {e}，回退透明像素");
            tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1)
        }
    }
}

/// 发送系统通知（供前端调用或托盘菜单触发）
#[tauri::command]
pub(crate) fn notify(app: tauri::AppHandle, title: String, body: String) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

/// 构建系统托盘：左键单击显示窗口，菜单含「显示主窗口」「开机自启」「启动/停止 DSH 服务」
/// 「新建会话…」「会话窗口列表」「配置…」「测试通知」「退出」
pub fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let (menu, autostart_item) = build_tray_menu(app)?;

    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        // 兜底：1x1 透明像素，正常情况下不会走到这里
        tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1)
    });

    // 事件闭包中需要更新勾选状态，这里克隆句柄（内部 Arc 共享）
    let autostart_handle = autostart_item.clone();

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("DSH Desktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                log::debug!(target: "tray", "menu: show");
                show_main_window(app);
            }
            // Task 14：会话窗口项（id 前缀 win:）
            id if id.starts_with("win:") => {
                let label = &id[4..];
                log::debug!(target: "tray", "menu: 唤起窗口 {label}");
                if let Some(w) = app.get_webview_window(label) {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            // Task 14：新建会话窗口
            "new-session" => {
                log::info!(target: "tray", "menu: new-session");
                let _ = crate::window::open_session_window(app.clone(), None);
                rebuild_tray_menu(app);
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
            "svc-start" => {
                log::info!(target: "tray", "menu: svc-start");
                let result = service::start_service();
                notify_tray_result(
                    app,
                    "启动 DSH 服务",
                    match result {
                        Ok(0) => "服务已在运行".to_string(),
                        Ok(pid) => format!("已拉起 DSH 服务（pid={pid}），等待就绪…"),
                        Err(e) => e,
                    },
                );
            }
            "svc-stop" => {
                log::info!(target: "tray", "menu: svc-stop");
                let result = service::stop_service();
                notify_tray_result(
                    app,
                    "停止 DSH 服务",
                    match result {
                        Ok(msg) => msg,
                        Err(e) => e,
                    },
                );
            }
            "settings" => {
                log::info!(target: "tray", "menu: settings");
                show_main_window(app);
                // 壳页监听该事件后打开设置弹层
                let _ = app.emit(crate::config::OPEN_SETTINGS_EVENT, ());
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

    // 保存托盘句柄：壳页 set_tray_state 切换双态图标用
    *tray_icon().lock().unwrap() = Some(tray);

    Ok(())
}

/// 构建托盘菜单（Task 14 可重建）：固定项 + 「新建会话…」+ 会话窗口列表。
/// 返回 (menu, autostart_item)——autostart 句柄供事件闭包更新勾选态。
fn build_tray_menu(
    app: &tauri::AppHandle,
) -> tauri::Result<(
    tauri::menu::Menu<tauri::Wry>,
    tauri::menu::CheckMenuItem<tauri::Wry>,
)> {
    let auto_enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart_item = tauri::menu::CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自启",
        true,
        auto_enabled,
        None::<&str>,
    )?;
    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let svc_start_item = MenuItem::with_id(app, "svc-start", "启动 DSH 服务", true, None::<&str>)?;
    let svc_stop_item = MenuItem::with_id(app, "svc-stop", "停止 DSH 服务", true, None::<&str>)?;
    let new_session_item = MenuItem::with_id(app, "new-session", "新建会话…", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "配置…", true, None::<&str>)?;
    let notify_item = MenuItem::with_id(app, "notify", "发送测试通知", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    // 会话窗口列表（label != MAIN_WINDOW；关闭后重建时自动消失）
    let win_items: Vec<MenuItem<tauri::Wry>> = app
        .webview_windows()
        .iter()
        .filter(|(label, _)| label.as_str() != crate::config::MAIN_WINDOW)
        .map(|(label, w)| {
            let title = w.title().unwrap_or_else(|_| label.clone());
            MenuItem::with_id(
                app,
                format!("win:{label}"),
                format!("会话：{title}"),
                true,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = vec![
        &show_item,
        &autostart_item,
        &sep,
        &svc_start_item,
        &svc_stop_item,
        &sep,
        &new_session_item,
    ];
    for item in &win_items {
        items.push(item);
    }
    items.push(&sep);
    items.push(&settings_item);
    items.push(&notify_item);
    items.push(&quit_item);
    let menu = Menu::with_items(app, &items)?;
    Ok((menu, autostart_item))
}

/// 重建托盘菜单（会话窗口开/关后调用，列表随窗口变化更新）。
pub fn rebuild_tray_menu(app: &tauri::AppHandle) {
    let Ok((menu, _)) = build_tray_menu(app) else {
        return;
    };
    let current = tray_icon().lock().unwrap();
    if let Some(tray) = current.as_ref() {
        match tray.set_menu(Some(menu)) {
            Ok(()) => log::debug!(target: "tray", "托盘菜单已重建"),
            Err(e) => log::warn!(target: "tray", "重建托盘菜单失败: {e}"),
        }
    }
}

/// 会话窗口已关闭：更新托盘菜单（移除该窗口项）。
pub fn on_session_window_closed(app: &tauri::AppHandle, _label: &str) {
    rebuild_tray_menu(app);
}

/// 托盘操作结果通知（成功/失败统一反馈）
fn notify_tray_result(app: &tauri::AppHandle, title: &str, body: String) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}
