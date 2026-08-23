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
    // clippy question_mark lint 推荐的惯用形式：ok()? 解 Result 后 ? 解 Option
    let monitor = window.current_monitor().ok()??;
    let wa = monitor.work_area();
    Some((
        wa.position.x,
        wa.position.y,
        wa.size.width as i32,
        wa.size.height as i32,
    ))
}

/// 把窗口几何夹取进目标工作区（纯函数，tuple 接口便于 JS 调用与单测）。
/// 语义委托 config::clamp_window_state（Task 5 同款不变量）：
///
/// - 尺寸：超出工作区 → 缩到工作区；
/// - 位置：左上角至少与工作区左/上沿对齐，且不越出右/下边界。
///
/// 显示器切换/DPI 变化后，还原路径用它保证窗口始终落在当前显示器工作区内。
pub fn clamp_to_work_area(
    rect: (i32, i32, u32, u32),
    wa: (i32, i32, i32, i32),
) -> (i32, i32, u32, u32) {
    let c = crate::config::clamp_window_state(
        crate::config::WindowState {
            x: rect.0,
            y: rect.1,
            w: rect.2,
            h: rect.3,
            maximized: false,
        },
        (wa.0, wa.1, wa.2.max(0) as u32, wa.3.max(0) as u32),
    );
    (c.x, c.y, c.w, c.h)
}

/// 前端还原路径调用：把目标几何夹取进当前显示器工作区。
/// 显示器切换后 savedRect 的旧坐标可能越界，切换/还原时先经此钳制。
#[tauri::command]
pub fn clamp_rect(
    window: tauri::WebviewWindow,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> (i32, i32, u32, u32) {
    match work_area(window) {
        Some(wa) => clamp_to_work_area((x, y, w, h), wa),
        None => (x, y, w, h), // 拿不到工作区（罕见）：原样返回，不越界处理兜底
    }
}

/// 计算初始/最小窗口尺寸（纯函数，无 FFI 依赖，便于单测）：
/// 初始尺寸保持 1280x840（"原来的大小"），小桌面（如手机 RDP 远程，工作区
/// 可能只有 640x480）时贴近屏幕，避免窗口比桌面还大、显示缩得很小；
/// 最小尺寸同样受工作区裁剪。
/// 返回 (init_w, init_h, min_w, min_h)。
fn initial_window_size(wa_w: u32, wa_h: u32) -> (u32, u32, u32, u32) {
    let init_w = INITIAL_WINDOW_SIZE.0.min(wa_w.max(1));
    let init_h = INITIAL_WINDOW_SIZE.1.min(wa_h.max(1));
    let min_w = MIN_WINDOW_SIZE.0.min(wa_w.max(1));
    let min_h = MIN_WINDOW_SIZE.1.min(wa_h.max(1));
    (init_w, init_h, min_w, min_h)
}

/// 创建主窗口：无边框、透明、Mica 毛玻璃、样式守卫、居中与初始隐藏。
/// 返回窗口句柄；调用方负责后续初始化（托盘、快捷键、主题监听）。
pub fn create_main_window(
    app: &tauri::AppHandle,
    initial_url: WebviewUrl,
) -> tauri::Result<WebviewWindow> {
    create_window(app, MAIN_WINDOW, "DSH Desktop", initial_url, true)
}

/// 创建会话窗口（Task 14：多会话独立小窗）。
/// - label：`session-N`（与主窗口区分，N 由调用方保证唯一）；
/// - 与主窗口同一套壳页（index.html 是通用壳页，URL 及检测循环独立）；
/// - 参与窗口状态记忆分域（记忆只对主窗口）。
pub fn create_session_window(
    app: &tauri::AppHandle,
    label: &str,
    title: &str,
    initial_url: WebviewUrl,
) -> tauri::Result<WebviewWindow> {
    create_window(app, label, title, initial_url, false)
}

/// 会话窗口序号（label 唯一性）
static SESSION_SEQ: std::sync::OnceLock<std::sync::atomic::AtomicU32> = std::sync::OnceLock::new();

fn session_seq() -> &'static std::sync::atomic::AtomicU32 {
    SESSION_SEQ.get_or_init(|| std::sync::atomic::AtomicU32::new(1))
}

/// 打开新会话窗口（托盘「新建会话」/前端调用）。
/// url 为空时用当前配置的 DSH 地址；返回新窗口 label。
#[tauri::command]
pub fn open_session_window(app: tauri::AppHandle, url: Option<String>) -> Result<String, String> {
    let target = url
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| crate::config::dsh_url().to_string());
    let label = format!(
        "session-{}",
        session_seq().fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let encoded = crate::config::encode_query_param(
        &tauri::Url::parse(&target).unwrap_or_else(|_| crate::config::dsh_url()),
    );
    let initial_url = WebviewUrl::App(format!("index.html?dsh={encoded}").into());
    let title = format!("DSH 会话 - {target}");
    create_session_window(&app, &label, &title, initial_url)
        .map(|_| label.clone())
        .map_err(|e| format!("创建会话窗口失败: {e}"))
}

/// 列出全部会话窗口 label（冒烟断言/托盘重建同源逻辑）。
#[tauri::command]
pub fn list_session_windows(app: tauri::AppHandle) -> Vec<String> {
    app.webview_windows()
        .keys()
        .filter(|l| l.as_str() != crate::config::MAIN_WINDOW)
        .cloned()
        .collect()
}

/// 关闭指定会话窗口（冒烟清理/托盘「关闭会话」）。
#[tauri::command]
pub fn close_session_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if label == crate::config::MAIN_WINDOW {
        return Err("不能关闭主窗口".into());
    }
    let Some(w) = app.get_webview_window(&label) else {
        return Err(format!("会话窗口不存在: {label}"));
    };
    crate::tray::on_session_window_closed(&app, &label);
    let _ = w.destroy();
    Ok(())
}

/// 共享窗口工厂：主窗口/会话窗口均经此创建（样式、隐藏、CDP 等一致）。
/// `remember_state`：主窗口参与窗口状态记忆（load/save_window_state 按
/// label 分域，见 config.rs）；会话窗口不参与（打开即居中）。
fn create_window(
    app: &tauri::AppHandle,
    label: &str,
    title: &str,
    initial_url: WebviewUrl,
    _remember_state: bool,
) -> tauri::Result<WebviewWindow> {
    // 初始尺寸自适应：大桌面保持 1280x840，小桌面贴近屏幕
    let (wa_w, wa_h) = primary_work_area(app);
    let (init_w, init_h, min_w, min_h) = initial_window_size(wa_w, wa_h);

    let builder = WebviewWindowBuilder::new(app, label, initial_url)
        .title(title)
        .min_inner_size(min_w as f64, min_h as f64)
        // 窗口创建即隐藏：壳页 iframe 加载完成后由壳页调用 show()，
        // 避免 GUI（浅色主题）加载完成前露出暗色背景（启动闪色），
        // 同时避免任务栏图标"出现-消失-再出现"的闪烁。
        // 用显式 hide() 兜底：实测 visible(false) 在透明窗口上可能不生效。
        .visible(false);

    // ===== 平台差异化窗口样式 =====
    // - Windows：无边框 + Mica 毛玻璃（壳页自绘标题栏/伪最大化/自绘 resize）；
    // - macOS：原生装饰（红绿灯按钮 = 原生最小化/全屏/关闭）+ Overlay 标题栏，
    //   壳页标题栏改为 28px 与红绿灯垂直中心对齐（见 frontend-dist/index.html）。

    #[cfg(target_os = "windows")]
    let builder = builder
        // 原生窗口质感：无系统边框，由本地壳页提供自定义标题栏
        .decorations(false)
        // 关闭 DWM 阴影：tao 的无边框阴影机制会保留 8px 阴影边
        // （客户区偏移，内容四周露出深色带 = "四方小缝隙"）。
        // 同时配合样式改造（WS_POPUP 无边框线）+ 壳页自绘 resize。
        .shadow(false)
        // 透明背景：配合 Mica/Acrylic 毛玻璃效果
        .transparent(true)
        // 禁用系统最大化：无边框窗口的系统最大化会被 DWM 扩展到 -8px 偏移
        // （标题栏顶部被裁）。最大化改由壳页伪最大化实现（手动贴齐工作区）。
        .maximizable(false);

    // macOS：保留原生窗口装饰（红绿灯按钮），Overlay 让内容延伸到标题栏下、
    // 红绿灯悬浮于内容之上（VS Code 同款）；原生标题文字隐藏，由壳页自绘标题栏。
    // 不启用 transparent：透明窗口在 macOS 原生全屏下有渲染问题（黑边/不刷新）；
    // 不设 maximizable(false)（默认 true）→ 绿色按钮（全屏）保持可用，
    // 由此获得系统原生全屏空间 + 动画（此前无边框 + maximizable(false) 没有全屏入口）。
    #[cfg(target_os = "macos")]
    let builder = builder
        .decorations(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    // 关键配置：让 WebView 把拖拽事件放行给页面。
    // DSH GUI 自带 document 级 drop 监听（intakeImages），
    // 若开启 Tauri 默认的窗口级拖拽拦截，页面收不到文件。
    // macOS 不暴露 drag_and_drop（tauri 平台 API 差异），其默认行为
    // 即页面级拖放处理，无需设置。
    #[cfg(not(target_os = "macos"))]
    let builder = builder.drag_and_drop(false);
    // 仅调试构建开启 CDP 端口（自动化验证用；发布版不带）
    #[cfg(debug_assertions)]
    let builder = builder.additional_browser_args("--remote-debugging-port=9226");
    let window = builder.build()?;

    // 显式设置初始尺寸（不依赖 builder 的 inner_size 传递，
    // 实测某些组合下 tao 会以默认尺寸创建窗口）。
    let _ = window.set_size(PhysicalSize::new(init_w, init_h));
    log::info!(target: "window", "窗口创建完成 {label} {init_w}x{init_h}（工作区 {wa_w}x{wa_h}）");
    // 窗口居中由壳页事件驱动：iframe 加载完成 → 壳页 show() 回调后立即居中
    // （见 frontend-dist/index.html showWindow），不再使用固定延迟睡眠线程。

    // 显式 hide() 兜底：实测 visible(false) 在透明窗口上可能不生效
    // （隐藏动机见上方 builder.visible(false) 注释）
    let _ = window.hide();

    #[cfg(target_os = "windows")]
    {
        apply_window_effect(&window);
        // 关闭四角圆角（最大化时避免露出桌面背景的小缝隙）
        disable_window_rounding(&window);
        // 去除 WS_CAPTION 边框线（最大化后四边可见的细边框）
        if let Ok(hwnd) = window.hwnd() {
            let hwnd = hwnd.0 as isize;
            make_window_popup_style(hwnd);
            // tao 会在窗口状态变化时把样式重置回含 WS_CAPTION 的装饰样式。
            // 用样式守卫（子类化 + WM_STYLECHANGED）事件驱动地立即修正（无轮询线程）。
            style_guard::install(hwnd);
        }
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

/// 主显示器工作区宽高（物理像素，排除任务栏/菜单栏）。
/// Windows 用 SPI_GETWORKAREA（与系统桌面含 RDP 会话一致）；
/// 其他平台用 primary_monitor（获取失败时回退设计尺寸）。
#[cfg(target_os = "windows")]
fn primary_work_area(_app: &tauri::AppHandle) -> (u32, u32) {
    let (_x, _y, w, h) = unsafe { work_area_ffi() };
    (w.max(0) as u32, h.max(0) as u32)
}

#[cfg(not(target_os = "windows"))]
fn primary_work_area(app: &tauri::AppHandle) -> (u32, u32) {
    if let Some(monitor) = app.primary_monitor().ok().flatten() {
        let wa = monitor.work_area();
        (wa.size.width.max(1), wa.size.height.max(1))
    } else {
        INITIAL_WINDOW_SIZE
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
    let mut rect = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
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
/// 注意：tao 会在窗口状态变化时重置样式，由 WM_STYLECHANGED 样式守卫
/// 即时修正（见 style_guard 模块，事件驱动、无轮询线程）。
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
        SetWindowPos(
            hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
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

#[cfg(test)]
mod tests {
    use super::{clamp_to_work_area, initial_window_size, INITIAL_WINDOW_SIZE, MIN_WINDOW_SIZE};

    #[test]
    fn clamp_moves_oversized_rect_into_work_area() {
        // 显示器切换后 savedRect 仍在旧显示器坐标（如旧 1920 屏 → 新 1280 屏）
        let c = clamp_to_work_area((1400, 200, 1000, 800), (0, 0, 1280, 720));
        assert_eq!(c, (280, 0, 1000, 720), "右/下越界应拉回工作区内");
        let c = clamp_to_work_area((200, 900, 900, 600), (0, 0, 1280, 720));
        assert_eq!(c, (200, 120, 900, 600), "下越界应上移");
    }

    #[test]
    fn clamp_moves_negative_coordinates_into_work_area() {
        // 负坐标（如从第二显示器切回主显示器后 x 为负）
        let c = clamp_to_work_area((-400, -200, 800, 600), (0, 0, 1920, 1040));
        assert_eq!(c, (0, 0, 800, 600), "负坐标应归位到工作区原点");
    }

    #[test]
    fn clamp_keeps_rect_inside_work_area_unchanged() {
        let c = clamp_to_work_area((100, 50, 800, 600), (0, 0, 1920, 1040));
        assert_eq!(c, (100, 50, 800, 600), "工作区内原样保留");
    }

    #[test]
    fn clamp_handles_zero_work_area_fallback() {
        // 工作区异常（0x0）：至少 1x1 存在，且位置归零
        let c = clamp_to_work_area((100, 50, 800, 600), (0, 0, 0, 0));
        assert_eq!(c.2, 1);
        assert_eq!(c.3, 1);
        assert_eq!(c.0, 0);
        assert_eq!(c.1, 0);
    }

    #[test]
    fn large_desktop_keeps_design_size() {
        // 大桌面：初始 1280x840，最小 860x620 原样保留
        assert_eq!(initial_window_size(1920, 1080), (1280, 840, 860, 620));
    }

    #[test]
    fn tiny_desktop_clamps_to_work_area() {
        // 小桌面（如手机 RDP，工作区 640x480）：初始/最小都贴近工作区
        assert_eq!(initial_window_size(640, 480), (640, 480, 640, 480));
    }

    #[test]
    fn medium_desktop_clamps_min_only() {
        // 中等桌面（如 1024x768）：初始完整，最小被裁剪到工作区
        assert_eq!(initial_window_size(1024, 700), (1024, 700, 860, 620));
    }

    #[test]
    fn zero_work_area_falls_back_to_minimum() {
        // 工作区异常（0x0）：至少保持 1x1 的窗口存在
        assert_eq!(initial_window_size(0, 0), (1, 1, 1, 1));
    }

    #[test]
    fn design_size_constants_are_sane() {
        // 设计不变量：初始尺寸 ≥ 最小尺寸
        assert!(INITIAL_WINDOW_SIZE.0 >= MIN_WINDOW_SIZE.0);
        assert!(INITIAL_WINDOW_SIZE.1 >= MIN_WINDOW_SIZE.1);
    }
}
