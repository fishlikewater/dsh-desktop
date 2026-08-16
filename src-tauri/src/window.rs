//! 窗口模块：主窗口创建、无边框样式、伪最大化几何、居中与背景效果。

use tauri::{Manager, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::config::{INITIAL_WINDOW_SIZE, MAIN_WINDOW, MIN_WINDOW_SIZE};

/// 显示并聚焦主窗口（若已最小化则还原）
pub fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 返回主窗口所在显示器的工作区（x, y, width, height，物理像素，排除任务栏）。
/// 壳页伪最大化用它把窗口贴齐工作区：最大化时不覆盖任务栏。
#[tauri::command]
pub fn work_area(window: tauri::WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    let monitor = window.current_monitor().ok()??;
    let wa = monitor.work_area();
    Some((
        wa.position.x,
        wa.position.y,
        wa.size.width as i32,
        wa.size.height as i32,
    ))
}

/// 创建主窗口：无边框、透明、Mica 毛玻璃、样式守卫、居中与初始隐藏。
/// 返回窗口句柄；调用方负责后续初始化（托盘、快捷键、主题监听）。
pub fn create_main_window(
    app: &tauri::AppHandle,
    initial_url: WebviewUrl,
) -> tauri::Result<WebviewWindow> {
    // 初始尺寸自适应：大桌面保持 1280x840（"原来的大小"），
    // 小桌面（如手机 RDP 远程，工作区可能只有 640x480）让窗口贴近屏幕，
    // 避免窗口比桌面还大、显示缩得很小。
    let (_wa_x, _wa_y, wa_w, wa_h) = unsafe { work_area_ffi() };
    let init_w = INITIAL_WINDOW_SIZE.0.min(wa_w.max(0) as u32);
    let init_h = INITIAL_WINDOW_SIZE.1.min(wa_h.max(0) as u32);
    let min_w = MIN_WINDOW_SIZE.0.min(wa_w.max(0) as u32);
    let min_h = MIN_WINDOW_SIZE.1.min(wa_h.max(0) as u32);

    let builder = WebviewWindowBuilder::new(app, MAIN_WINDOW, initial_url)
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
    let _ = window.set_size(PhysicalSize::new(init_w, init_h));
    log::info!(target: "window", "主窗口创建完成 {init_w}x{init_h}（工作区 {wa_w}x{wa_h}）");
    // 窗口居中由壳页事件驱动：iframe 加载完成 → 壳页 show() 回调后立即居中
    // （见 frontend-dist/index.html showWindow），不再使用固定延迟睡眠线程。

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

    Ok(window)
}

/// 应用窗口背景效果：Windows 11 用 Mica（毛玻璃，跟随系统主题），Windows 10 回退 Acrylic
#[cfg(target_os = "windows")]
fn apply_window_effect(window: &WebviewWindow) {
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
fn disable_window_rounding(window: &WebviewWindow) {
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
        let proc = GetProcAddress(module, c"DwmSetWindowAttribute".as_ptr().cast::<u8>());
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

/// 主显示器工作区（x, y, width, height，物理像素，排除任务栏）。
/// 用 Win32 SPI_GETWORKAREA 直接获取：与系统桌面（含 RDP 会话）一致。
#[cfg(target_os = "windows")]
unsafe fn work_area_ffi() -> (i32, i32, i32, i32) {
    const SPI_GETWORKAREA: u32 = 0x0030;
    #[repr(C)]
    struct Rect {
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
    let mut rect = Rect { left: 0, top: 0, right: 0, bottom: 0 };
    SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        &mut rect as *mut Rect as *mut std::ffi::c_void,
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
