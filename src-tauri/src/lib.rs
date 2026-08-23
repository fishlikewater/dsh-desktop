//! DSH Desktop 壳层入口：装配窗口/托盘/快捷键/主题监听。

mod config;
mod logging;
mod service;
mod settings;
mod shortcut;
mod theme;
mod tray;
mod window;

use tauri::{Manager, Url, WebviewUrl, WindowEvent};

use config::{dsh_url, encode_query_param};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(logging::init().build())
        .invoke_handler(tauri::generate_handler![
            tray::notify,
            window::work_area,
            theme::theme_preference,
            service::service_start,
            service::service_stop,
            settings::app_version,
            settings::autostart_state,
            settings::autostart_toggle,
            settings::update_check,
            settings::update_install,
            settings::open_log_dir,
            settings::read_log_tail,
            config::load_window_state,
            config::save_window_state,
            config::get_close_behavior,
            config::set_close_behavior,
            config::get_auto_restart,
            config::set_auto_restart,
            config::get_address_history,
            config::set_address_history,
            config::probe_address,
            window::clamp_rect,
            window::open_session_window,
            window::list_session_windows,
            window::close_session_window,
            tray::set_tray_state,
            config::get_shortcut,
            config::set_shortcut,
            shortcut::reapply_global_shortcut,
            service::auto_start_service
        ])
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // 自动更新：校验走 ed25519 签名（公钥在 tauri.conf.json plugins.updater）。
        // 检查动作由壳页在合适时机触发（T5.4 设置页/启动后延时可自行决定，插件本身只提供能力）。
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // 单实例：托盘常驻应用，重复启动时聚焦已有窗口而不是开第二个进程
        // （否则新旧实例会争夺全局快捷键 Ctrl+Shift+D，注册失败并弹出错误）
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!(target: "app", "second instance detected -> focus existing window");
            window::show_main_window(app);
        }))
        .setup(|app| {
            log::info!(target: "app", "DSH Desktop starting, DSH_URL={}", dsh_url());
            // 壳页入口（自定义标题栏 + iframe 嵌入 DSH GUI），服务地址经 ?dsh= 传入
            // 编码必须覆盖 # / + 等全部非 unreserved 字符（见 config::encode_query_param）
            let url: Url = dsh_url();
            let encoded: String = encode_query_param(&url);
            let initial_url = WebviewUrl::App(format!("index.html?dsh={encoded}").into());

            window::create_main_window(app.handle(), initial_url)?;

            tray::setup_tray(app.handle())?;
            // 托盘图标初态：按当前服务可达性设置（tick 后续会自动校正）
            tray::set_tray_state(service::service_reachable());
            shortcut::setup_global_shortcut(app.handle());
            // 监听 DSH 主题持久化文件变化，壳页标题栏即时跟随（取代固定轮询）
            theme::watch_settings_theme(app.handle());
            Ok(())
        })
        // 关闭窗口时最小化到托盘，应用常驻后台；
        // 「关闭时退出」开关（config.close_behavior）开启后改为真正退出。
        // Task 14：会话窗口（label != main）关闭 = 真正销毁并从托盘列表移除。
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let is_main = window.label() == config::MAIN_WINDOW;
                if !is_main {
                    log::info!(target: "window", "会话窗口关闭请求 -> 销毁（{}）", window.label());
                    let app = window.app_handle().clone();
                    let label = window.label().to_string();
                    api.prevent_close();
                    // 销毁需回主线程（事件处理器内直接 destroy 有重入风险）：
                    // spawn 延迟到事件处理完成后执行，并同步刷新托盘列表。
                    tauri::async_runtime::spawn(async move {
                        tray::on_session_window_closed(&app, &label);
                        if let Some(w) = app.get_webview_window(&label) {
                            let _ = w.destroy();
                        }
                    });
                    return;
                }
                // 读最新落盘值（开关切换即时生效，不依赖进程内缓存）
                if config::current_close_behavior() {
                    log::info!(target: "window", "close requested -> exit（关闭时退出已开启）");
                    return; // 不 prevent_close：默认放行 → 窗口关闭 → 应用退出
                }
                log::info!(target: "window", "close requested -> hide to tray");
                // 首次关闭：系统通知引导（"仍在运行，点击托盘图标恢复"），仅一次
                if !config::first_close_notified() {
                    let _ = config::mark_first_close_notified();
                    log::info!(target: "window", "首次关闭：发送托盘引导通知");
                    tray::notify(
                        window.app_handle().clone(),
                        "DSH Desktop 仍在运行".into(),
                        "点击系统托盘图标即可恢复窗口（可在配置中开启「关闭时退出」）".into(),
                    );
                }
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("运行 DSH Desktop 失败");
}
