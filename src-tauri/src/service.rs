//! DSH 服务自管理：发现系统 dsh CLI 并拉起服务（方案 B：不捆绑 sidecar）。
//!
//! 说明：`dsh` 是 npm 全局 shim（dsh.cmd / dsh.ps1，无 dsh.exe），
//! Rust 的 `Command::new` 不解析 PATH 中的 .cmd/.ps1，因此：
//! - .cmd → `cmd /c "<path>" --profile web`
//! - .ps1 → `powershell -NoProfile -File "<path>" --profile web`
//! - .exe → 直接执行
//!
//! 单元测试位于 tests/service.rs（集成测试）：windows-gnu 下 lib 单测中
//! 调用 fs/env/PathBuf 等 API 或声明 `std::process::Child` 类型的 static
//! 会触发 rustc 链接 bug（0xc0000139），详见 docs/testing.md。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::config::dsh_url;

/// 本应用拉起的服务进程 pid（不持有 Child 句柄：
/// windows-gnu 下 `static` 含 `std::process::Child` 类型会使 test 二进制
/// 加载失败 0xc0000139；且关闭句柄不影响子进程存活，停止时用 taskkill pid）。
static SERVICE_PID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();

fn service_pid() -> &'static Mutex<Option<u32>> {
    SERVICE_PID.get_or_init(|| Mutex::new(None))
}

/// 在 PATH 中查找 dsh CLI（优先 .cmd > .exe > .ps1；DSH_CLI 环境变量可显式指定）。
/// 纯函数（path 可注入），便于集成测试。
fn find_dsh_in(paths: &[PathBuf], candidates: &[&str]) -> Option<PathBuf> {
    find_dsh_in_with(paths, candidates, |p| p.is_file())
}

/// 带"存在性判断"的查找内核：生产代码传 p.is_file()；测试注入模拟 exists。
/// 注意：本项目 service 逻辑无自动化单测——windows-gnu 下 lib 单测中调用
/// 此类 API 或集成测试引用 lib crate 都会使 test 二进制加载失败
/// （0xc0000139），T5.1 已用手工冒烟验证（假 dsh 拉起/停止）。详见 docs/testing.md。
#[doc(hidden)]
pub fn find_dsh_in_with<F: Fn(&Path) -> bool>(
    paths: &[PathBuf],
    candidates: &[&str],
    exists: F,
) -> Option<PathBuf> {
    for dir in paths {
        for name in candidates {
            let p = dir.join(name);
            if exists(&p) {
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

/// 3080 服务探活（TCP connect，1.5s 超时）。
fn service_reachable() -> bool {
    let url = dsh_url();
    let Ok(addr) = url.socket_addrs(|| None).map(|v| v[0]) else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(1500)).is_ok()
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
/// 不触碰用户自启的服务：仅当 SERVICE_PID 记录时执行；否则返回提示（不误杀）。
#[tauri::command]
pub fn service_stop() -> Result<String, String> {
    stop_service()
}

/// 停止本应用拉起的服务（托盘与 command 共用核心逻辑）。
pub fn stop_service() -> Result<String, String> {
    let mut guard = service_pid().lock().unwrap();
    let Some(pid) = guard.take() else {
        return Err("未找到由本应用拉起的 DSH 服务（可能由其他方式启动，未做处理）".into());
    };
    // taskkill /T：连同进程树（dsh.cmd → node 子进程）一起结束
    let kill = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(0x0000_0008)
        .output();
    match kill {
        Ok(out) if out.status.success() => {
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
/// 用 DETACHED_PROCESS(0x8) 不弹控制台窗口（CREATE_NO_WINDOW 语义相近）。
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
    // 句柄随 child drop 关闭，不影响子进程存活；记录 pid 供停止时 taskkill
    *service_pid().lock().unwrap() = Some(pid);
    log::info!(target: "service", "已拉起 dsh（{}，pid={pid}），等待服务就绪", dsh.display());
    Ok(pid)
}
