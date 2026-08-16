//! DSH 服务自管理：发现系统 dsh CLI 并拉起服务（方案 B：不捆绑 sidecar）。
//!
//! 说明：`dsh` 是 npm 全局 shim（dsh.cmd / dsh.ps1，无 dsh.exe），
//! Rust 的 `Command::new` 不解析 PATH 中的 .cmd/.ps1，因此：
//! - .cmd → `cmd /c "<path>" --profile web`
//! - .ps1 → `powershell -NoProfile -File "<path>" --profile web`
//! - .exe → 直接执行

use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::config::dsh_url;

/// 子进程句柄（防回收）；拉起的服务进程由用户/系统独立管理生命周期，
/// 壳层只负责"拉起"（服务关闭后用户可再次点击拉起）。
static SERVICE_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

fn service_child() -> &'static Mutex<Option<Child>> {
    SERVICE_CHILD.get_or_init(|| Mutex::new(None))
}

/// 在 PATH 中查找 dsh CLI（优先 .cmd > .exe > .ps1；DSH_CLI 环境变量可显式指定）。
/// 纯函数（path 可注入），便于单测。
fn find_dsh_in(paths: &[PathBuf], candidates: &[&str]) -> Option<PathBuf> {
    for dir in paths {
        for name in candidates {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// 查找 dsh CLI：DSH_CLI 环境变量 > PATH 搜索。
pub fn find_dsh() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("DSH_CLI") {
        let p = PathBuf::from(v);
        if p.is_file() {
            return Some(p);
        }
        log::warn!(target: "service", "DSH_CLI 指定的路径不存在: {}", p.display());
    }
    let paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    find_dsh_in(&paths, &["dsh.cmd", "dsh.exe", "dsh.ps1", "dsh"])
}

/// 3080 服务探活（HTTP GET，2s 超时）。
fn service_reachable() -> bool {
    let url = dsh_url();
    let Ok(addr) = url.socket_addrs(|| None).map(|v| v[0]) else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(1500))
        .is_ok()
}

/// 启动 DSH 服务（幂等）：服务已可达 → Ok(0)；否则拉起 dsh，返回 pid。
/// 找不到 dsh CLI 时返回可读错误（前端提示安装）。
#[tauri::command]
pub fn service_start() -> Result<u32, String> {
    start_service()
}

/// 启动 DSH 服务（托盘与 command 共用核心逻辑）。
pub fn start_service() -> Result<u32, String> {
    if service_reachable() {
        return Ok(0); // 已在运行
    }
    let Some(dsh) = find_dsh() else {
        let msg = "未找到 dsh 命令：请安装 DSH 后重试（或设置 DSH_CLI 环境变量指向 dsh 可执行文件）";
        log::warn!(target: "service", "{msg}");
        return Err(msg.into());
    };
    spawn_dsh(&dsh)
}

/// 停止**本应用拉起的** DSH 服务进程树（taskkill /T /F）。
/// 不触碰用户自启的服务：仅当 SERVICE_CHILD 记录且进程存活时执行；
/// 否则返回提示（不误杀）。
#[tauri::command]
pub fn service_stop() -> Result<String, String> {
    stop_service()
}

/// 停止本应用拉起的服务（托盘与 command 共用核心逻辑）。
pub fn stop_service() -> Result<String, String> {
    let mut guard = service_child().lock().unwrap();
    let Some(child) = guard.as_mut() else {
        return Err("未找到由本应用拉起的 DSH 服务（可能由其他方式启动，未做处理）".into());
    };
    // try_wait 检查进程是否还活着（已退出则清理句柄）
    match child.try_wait() {
        Ok(Some(_status)) => {
            *guard = None;
            return Err("本应用拉起的 DSH 服务已退出".into());
        }
        Ok(None) => {}
        Err(e) => log::warn!(target: "service", "检查服务进程状态失败: {e}"),
    }
    let pid = child.id();
    // taskkill /T：连同进程树（dsh.cmd → node 子进程）一起结束
    let kill = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(0x0000_0008)
        .output();
    match kill {
        Ok(out) if out.status.success() => {
            *guard = None;
            log::info!(target: "service", "已停止本应用拉起的 DSH 服务（pid={pid}）");
            Ok(format!("已停止 DSH 服务（pid={pid}）"))
        }
        Ok(out) => Err(format!(
            "停止服务失败（pid={pid}）: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("停止服务失败（pid={pid}）: {e}")),
    }
}

/// 按扩展名选择启动方式拉起 dsh --profile web。
/// 注意：不用 CREATE_NO_WINDOW(0x08000000)——rustc 1.95 windows-gnu 下该标志
/// 会使链接出的 test 二进制加载失败（0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND，
/// cargo test 必崩）；用 DETACHED_PROCESS(0x8) 同样达到"不弹控制台窗口"的效果。
fn spawn_dsh(dsh: &Path) -> Result<u32, String> {
    let ext = dsh.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let child = match ext.as_str() {
        "cmd" => std::process::Command::new("cmd")
            .args(["/c", dsh.to_str().unwrap_or_default(), "--profile", "web"])
            .creation_flags(0x0000_0008) // DETACHED_PROCESS：不弹黑窗
            .spawn(),
        "ps1" => std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                dsh.to_str().unwrap_or_default(),
                "--profile",
                "web",
            ])
            .creation_flags(0x0000_0008)
            .spawn(),
        _ => std::process::Command::new(dsh)
            .arg("--profile")
            .arg("web")
            .creation_flags(0x0000_0008)
            .spawn(),
    };
    let child = child.map_err(|e| format!("拉起 dsh 失败: {e}"))?;
    let pid = child.id();
    *service_child().lock().unwrap() = Some(child);
    log::info!(target: "service", "已拉起 dsh（{}，pid={pid}），等待服务就绪", dsh.display());
    Ok(pid)
}

#[cfg(test)]
mod tests {
    use super::find_dsh_in;

    #[test]
    fn finds_cmd_before_exe() {
        let dir = std::env::temp_dir().join("dsh-find-test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("dsh.cmd"), "x").unwrap();
        let found = find_dsh_in(std::slice::from_ref(&dir), &["dsh.cmd", "dsh.exe", "dsh.ps1"]);
        assert_eq!(found, Some(dir.join("dsh.cmd")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn falls_back_to_exe() {
        let dir = std::env::temp_dir().join("dsh-find-test2");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("dsh.exe"), "x").unwrap();
        let found = find_dsh_in(std::slice::from_ref(&dir), &["dsh.cmd", "dsh.exe", "dsh.ps1"]);
        assert_eq!(found, Some(dir.join("dsh.exe")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn returns_none_when_absent() {
        let dir = std::env::temp_dir().join("dsh-find-test3");
        let found = find_dsh_in(&[dir], &["dsh.cmd", "dsh.exe", "dsh.ps1"]);
        assert!(found.is_none());
    }
}
