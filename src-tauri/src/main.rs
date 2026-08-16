// 禁止 Windows 弹出控制台窗口（debug/release 一律不弹）：
// 应用是托盘常驻 GUI 程序，eprintln 等输出不应打扰用户
#![windows_subsystem = "windows"]

fn main() {
    dsh_desktop_lib::run()
}
