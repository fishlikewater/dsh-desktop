use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, Url, WebviewUrl, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// 主窗口的 label
const MAIN_WINDOW: &str = "main";
/// DSH 服务默认地址
const DEFAULT_DSH_URL: &str = "http://127.0.0.1:3080";

/// 读取 DSH 服务地址：支持环境变量 DSH_URL 覆盖（便于不同端口/测试），非法值回退默认。
fn dsh_url() -> Url {
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

/// 显示并聚焦主窗口（若已最小化则还原）
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 发送系统通知（供前端调用或托盘菜单触发）
#[tauri::command]
fn notify(app: tauri::AppHandle, title: String, body: String) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

/// 返回主窗口所在显示器的工作区（x, y, width, height，物理像素，排除任务栏）。
/// 壳页伪最大化用它把窗口贴齐工作区：最大化时不覆盖任务栏。
#[tauri::command]
fn work_area(window: tauri::WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    let monitor = window.current_monitor().ok()??;
    let wa = monitor.work_area();
    Some((
        wa.position.x,
        wa.position.y,
        wa.size.width as i32,
        wa.size.height as i32,
    ))
}


/// DSH 外观主题持久化文件路径：$DSH_HOME/settings.yaml（无 DSH_HOME 用 USERPROFILE）。
fn theme_settings_path() -> std::path::PathBuf {
    let home = std::env::var("DSH_HOME").unwrap_or_else(|_| {
        std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into())
    });
    std::path::Path::new(&home).join("settings.yaml")
}

/// 读取 DSH 外观主题偏好（~/.dsh/settings.yaml 的 ui-theme.preference：
/// light | dark | system；由 DSH GUI 外观设置持久化）。
/// 壳页用它同步自定义标题栏配色（system 时前端跟随系统 prefers-color-scheme）。
#[tauri::command]
fn theme_preference() -> String {
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
fn watch_settings_theme(app: &tauri::AppHandle) {
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
        let _ = app.emit("theme-file-changed", ());
    }) else {
        return;
    };
    let Ok(_) = watcher.watch(&dir, RecursiveMode::NonRecursive) else {
        return;
    };
    // 事件回调运行在 notify 独立线程，app 克隆可跨线程使用
    let _ = WATCHER.set(watcher);
}

/// 构建系统托盘：左键单击显示窗口，菜单含「显示主窗口」「开机自启」「测试通知」「退出」
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
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

    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| {
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
            "show" => show_main_window(app),
            "autostart" => {
                let mgr = app.autolaunch();
                let target = !mgr.is_enabled().unwrap_or(false);
                let result = if target {
                    mgr.enable()
                } else {
                    mgr.disable()
                };
                match result {
                    Ok(()) => {
                        let _ = autostart_handle.set_checked(target);
                    }
                    Err(e) => eprintln!("切换开机自启失败: {e}"),
                }
            }
            "notify" => {
                let _ = notify(
                    app.clone(),
                    "DSH Desktop".to_string(),
                    "通知通道工作正常 ✓".to_string(),
                );
            }
            "quit" => app.exit(0),
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

/// 注册全局快捷键：Ctrl+Shift+D 唤起主窗口
fn setup_global_shortcut(app: &tauri::AppHandle) {
    let shortcut = Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::KeyD,
    );
    if let Err(e) = app.global_shortcut().on_shortcut(shortcut, |app, _sc, event| {
        if event.state() == ShortcutState::Pressed {
            show_main_window(app);
        }
    }) {
        eprintln!("注册全局快捷键回调失败: {e}");
    }
    if let Err(e) = app.global_shortcut().register(shortcut) {
        // 快捷键可能被其他应用占用，此时仅告警，不影响壳层运行
        eprintln!("注册全局快捷键 Ctrl+Shift+D 失败: {e}");
    }
}

/// 应用窗口背景效果：Windows 11 用 Mica（毛玻璃，跟随系统主题），Windows 10 回退 Acrylic
#[cfg(target_os = "windows")]
fn apply_window_effect(window: &tauri::WebviewWindow) {
    use window_vibrancy::{apply_acrylic, apply_mica};
    // dark=None：跟随系统主题（避免 GUI 浅色时窗口强制暗色）
    if apply_mica(window, None).is_err() {
        let _ = apply_acrylic(window, Some((18, 22, 34, 210)));
    }
}

/// 关闭 Windows 11 无边框窗口的默认圆角 + 隐藏 DWM 边框线。
/// 无边框窗口默认带 8px 圆角（最大化时四角露出背景），且可调整大小（WS_THICKFRAME）
/// 的窗口会被 DWM 绘制 1px 激活边框线（蓝色）和阴影带——这就是用户看到的
/// "四方小缝隙"。这里通过 DWM 设置：DWMWCP_DONOTROUND（直角）+ DWMWA_BORDER_COLOR
/// 透明（隐藏边框线，resize 热区保留）。
///
/// 注意：不直接链接 dwmapi.lib（windows-gnu 工具链缺该导入库），
/// 改用运行时 LoadLibrary + GetProcAddress 动态解析（系统必然自带 dwmapi.dll）。
#[cfg(target_os = "windows")]
fn disable_window_rounding(window: &tauri::WebviewWindow) {
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_DONOTROUND: u32 = 1;
    const DWMWA_BORDER_COLOR: u32 = 34;
    type DwmSetWindowAttributeFn = unsafe extern "system" fn(isize, u32, *const u32, u32) -> i32;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(name: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> isize;
    }

    let Ok(hwnd) = window.hwnd() else { return };
    unsafe {
        let name: Vec<u16> = "dwmapi.dll".encode_utf16().collect();
        let module = LoadLibraryW(name.as_ptr());
        if module == 0 {
            return;
        }
        let proc = GetProcAddress(module, b"DwmSetWindowAttribute\0".as_ptr() as *const u8);
        if proc == 0 {
            return;
        }
        let func: DwmSetWindowAttributeFn = std::mem::transmute(proc);
        let pref = DWMWCP_DONOTROUND;
        func(hwnd.0 as isize, DWMWA_WINDOW_CORNER_PREFERENCE, &pref, 4);
        // 边框颜色设为完全透明（ARGB 0x00000000），隐藏激活边框线
        let transparent = 0x0000_0000u32;
        func(hwnd.0 as isize, DWMWA_BORDER_COLOR, &transparent, 4);
    }
}

/// 查询窗口是否可见（Windows）
#[cfg(target_os = "windows")]
unsafe fn is_window_visible(hwnd: isize) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn IsWindowVisible(hwnd: isize) -> i32;
    }
    IsWindowVisible(hwnd) != 0
}

/// 移动窗口（直接 Win32 SetWindowPos，绕过 tauri set_position 的异步时序问题）
#[cfg(target_os = "windows")]
unsafe fn set_window_pos(hwnd: isize, x: i32, y: i32) {
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOZORDER: u32 = 0x0004;
    #[link(name = "user32")]
    extern "system" {
        fn SetWindowPos(
            hwnd: isize,
            insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
    }
    SetWindowPos(hwnd, 0, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
}

/// 主显示器工作区（x, y, width, height，物理像素，排除任务栏）。
/// 用 Win32 SPI_GETWORKAREA 直接获取：与系统桌面（含 RDP 会话）一致。
#[cfg(target_os = "windows")]
unsafe fn work_area_ffi() -> (i32, i32, i32, i32) {
    const SPI_GETWORKAREA: u32 = 0x0030;
    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(
            ui_action: u32,
            ui_param: u32,
            pv_param: *mut std::ffi::c_void,
            f_win_ini: u32,
        ) -> i32;
    }
    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        &mut rect as *mut RECT as *mut std::ffi::c_void,
        0,
    );
    (
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    )
}

/// 移除无边框窗口的系统边框样式：WS_CAPTION（DWM 激活边框线）和
/// WS_SIZEBOX/WS_THICKFRAME（DWM 8px 隐藏 resize 边框，会把内容挤到中间，
/// 四周露出深色边框带——用户看到的"四方小缝隙"）。
/// 窗口改为纯 WS_POPUP：内容铺满整个窗口；边缘调整大小由壳页自绘热区 +
/// JS 拖拽（setPosition/setSize）实现。
/// 注意：tao 会在窗口状态变化时重置样式，因此有常驻线程每 500ms 重新应用。
#[cfg(target_os = "windows")]
fn make_window_popup_style(hwnd: isize) {
    const GWL_STYLE: i32 = -16;
    const WS_POPUP: isize = 0x8000_0000;
    const WS_CAPTION: isize = 0x00C0_0000;
    const WS_SIZEBOX: isize = 0x0004_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    #[link(name = "user32")]
    extern "system" {
        fn GetWindowLongPtrW(hwnd: isize, nIndex: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
        fn SetWindowPos(
            hwnd: isize,
            insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
    }
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let new_style = (style & !(WS_CAPTION | WS_SIZEBOX)) | WS_POPUP;
        SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);
        // SWP_FRAMECHANGED 强制 DWM 按新样式重绘边框/非客户区
        SetWindowPos(hwnd, 0, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED);
    }
}

/// 样式守卫：子类化窗口过程，拦截 WM_STYLECHANGED。
/// tao 每次用 SetWindowLongW 重置窗口样式（恢复 WS_CAPTION/WS_SIZEBOX）时，
/// Windows 都会发送 WM_STYLECHANGED——我们在处理该消息时**立即**把样式改回
/// WS_POPUP（无边框线）。完全事件驱动：零延迟、零空闲开销（对比轮询线程）。
#[cfg(target_os = "windows")]
mod style_guard {
    use std::sync::atomic::{AtomicIsize, Ordering};

    const GWLP_WNDPROC: i32 = -4;
    const GWL_STYLE: i32 = -16;
    const WM_STYLECHANGED: u32 = 0x007D;
    const WS_POPUP: isize = 0x8000_0000;
    const WS_CAPTION: isize = 0x00C0_0000;
    const WS_SIZEBOX: isize = 0x0004_0000;

    static ORIGINAL_PROC: AtomicIsize = AtomicIsize::new(0);

    #[link(name = "user32")]
    extern "system" {
        fn GetWindowLongPtrW(hwnd: isize, nIndex: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
        fn CallWindowProcW(
            prev: isize,
            hwnd: isize,
            msg: u32,
            wparam: isize,
            lparam: isize,
        ) -> isize;
    }

    fn wanted_style(style: isize) -> isize {
        (style & !(WS_CAPTION | WS_SIZEBOX)) | WS_POPUP
    }

    unsafe extern "system" fn guard_proc(
        hwnd: isize,
        msg: u32,
        wparam: isize,
        lparam: isize,
    ) -> isize {
        if msg == WM_STYLECHANGED {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let wanted = wanted_style(style);
            if style != wanted {
                // tao 刚把样式重置回装饰样式，立即改回无边框样式。
                // 这次 SetWindowLongPtrW 会再次触发 WM_STYLECHANGED，
                // 但那时样式已符合 wanted，不再修改，递归收敛。
                SetWindowLongPtrW(hwnd, GWL_STYLE, wanted);
            }
        }
        let prev = ORIGINAL_PROC.load(Ordering::SeqCst);
        CallWindowProcW(prev, hwnd, msg, wparam, lparam)
    }

    /// 安装样式守卫（子类化窗口过程）。安装后窗口样式始终保持 WS_POPUP。
    pub fn install(hwnd: isize) {
        unsafe {
            let prev = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            if prev == 0 {
                return;
            }
            ORIGINAL_PROC.store(prev, Ordering::SeqCst);
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, guard_proc as *const () as isize);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            notify,
            work_area,
            theme_preference
        ])
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // 单实例：托盘常驻应用，重复启动时聚焦已有窗口而不是开第二个进程
        // （否则新旧实例会争夺全局快捷键 Ctrl+Shift+D，注册失败并弹出错误）
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            // 壳页入口（自定义标题栏 + iframe 嵌入 DSH GUI），服务地址经 ?dsh= 传入
            let url = dsh_url();
            let encoded: String = url
                .as_str()
                .chars()
                .map(|c| match c {
                    ':' | '/' | '?' | '&' | '=' | '%' => format!("%{:02X}", c as u32),
                    _ => c.to_string(),
                })
                .collect();
            let initial_url = WebviewUrl::App(format!("index.html?dsh={encoded}").into());

            // 初始尺寸自适应：大桌面保持 1280x840（"原来的大小"），
            // 小桌面（如手机 RDP 远程，工作区可能只有 640x480）让窗口贴近屏幕，
            // 避免窗口比桌面还大、显示缩得很小。
            let (_wa_x, _wa_y, wa_w, wa_h) = unsafe { work_area_ffi() };
            let init_w = 1280u32.min(wa_w.max(0) as u32);
            let init_h = 840u32.min(wa_h.max(0) as u32);
            let min_w = 860u32.min(wa_w.max(0) as u32);
            let min_h = 620u32.min(wa_h.max(0) as u32);

            let builder = tauri::WebviewWindowBuilder::new(app, MAIN_WINDOW, initial_url)
                .title("DSH Desktop")
                .min_inner_size(min_w as f64, min_h as f64)
                // 原生窗口质感：无系统边框，由本地壳页提供自定义标题栏
                .decorations(false)
                // 关闭 DWM 阴影：tao 的无边框阴影机制会保留 8px 阴影边
                // （客户区偏移，内容四周露出深色带 = "四方小缝隙"）。
                // 同时配合样式改造（WS_POPUP 无边框线）+ 壳页自绘 resize。
                .shadow(false)
                // 透明背景：配合 Mica/Acrylic 毛玻璃效果
                .transparent(true)
                // 关键配置：让 WebView2 把拖拽事件放行给页面。
                // DSH GUI 自带 document 级 drop 监听（intakeImages），
                // 若开启 Tauri 默认的窗口级拖拽拦截，页面收不到文件。
                .drag_and_drop(false)
                // 禁用系统最大化：无边框窗口的系统最大化会被 DWM 扩展到 -8px 偏移
                // （标题栏顶部被裁）。最大化改由壳页伪最大化实现（手动贴齐工作区）。
                .maximizable(false)
                // 窗口创建即隐藏：壳页 iframe 加载完成后由壳页调用 show()，
                // 避免 GUI（浅色主题）加载完成前露出暗色背景（启动闪色），
                // 同时避免任务栏图标"出现-消失-再出现"的闪烁。
                // 用显式 hide() 兜底：实测 visible(false) 在透明窗口上可能不生效。
                .visible(false);
            // 仅调试构建开启 CDP 端口（自动化验证用；发布版不带）
            #[cfg(debug_assertions)]
            let builder = builder.additional_browser_args("--remote-debugging-port=9226");
            let window = builder.build()?;

            // 显式设置初始尺寸（不依赖 builder 的 inner_size 传递，
            // 实测某些组合下 tao 会以默认尺寸创建窗口）。
            let _ = window.set_size(tauri::PhysicalSize::new(init_w, init_h));
            // 窗口显示后居中：隐藏窗口上 set_position 无效，壳页 show() 时机不定，
            // 用固定延迟尝试（窗口通常 1.5s 内显示；5s 兜底场景由第二次尝试覆盖）。
            {
                let win = window.clone();
                std::thread::spawn(move || {
                    for delay in [2500u64, 6500] {
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                        let Ok(hwnd) = win.hwnd() else { continue };
                        if !unsafe { is_window_visible(hwnd.0 as isize) } {
                            continue;
                        }
                        if let Ok(Some(monitor)) = win.current_monitor() {
                            let wa = monitor.work_area();
                            if let Ok(size) = win.outer_size() {
                                let x = wa.position.x
                                    + (wa.size.width as i32 - size.width as i32) / 2;
                                let y = wa.position.y
                                    + (wa.size.height as i32 - size.height as i32) / 2;
                                unsafe { set_window_pos(hwnd.0 as isize, x.max(0), y.max(0)) };
                            }
                        }
                        break;
                    }
                });
            }

            // 窗口初始隐藏：壳页 iframe 加载完成后由壳页调用 show()，
            // 避免 GUI（浅色主题）加载完成前露出暗色背景（启动闪色）。
            // 用显式 hide() 兜底：实测 visible(false) 在透明窗口上可能不生效。
            let _ = window.hide();

            #[cfg(target_os = "windows")]
            apply_window_effect(&window);
            // 关闭四角圆角（最大化时避免露出桌面背景的小缝隙）
            disable_window_rounding(&window);
            // 去除 WS_CAPTION 边框线（最大化后四边可见的细边框）
            if let Ok(hwnd) = window.hwnd() {
                let hwnd = hwnd.0 as isize;
                make_window_popup_style(hwnd);
                // tao 会在窗口状态变化时把样式重置回含 WS_CAPTION 的装饰样式。
                // 用样式守卫（子类化 + WM_STYLECHANGED）事件驱动地立即修正，
                // 替代常驻轮询线程。
                style_guard::install(hwnd);
            }

            setup_tray(app.handle())?;
            setup_global_shortcut(app.handle());
            // 监听 DSH 主题持久化文件变化，壳页标题栏即时跟随（取代固定轮询）
            watch_settings_theme(app.handle());
            Ok(())
        })
        // 关闭窗口时最小化到托盘，应用常驻后台
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("运行 DSH Desktop 失败");
}
