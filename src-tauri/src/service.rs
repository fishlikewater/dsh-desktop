//! DSH 服务自管理：发现系统 dsh CLI 并拉起服务（方案 B：不捆绑 sidecar）。
//!
//! 说明：`dsh` 是 npm 全局 shim（dsh.cmd / dsh.ps1，无 dsh.exe），
//! Rust 的 `Command::new` 不解析 PATH 中的 .cmd/.ps1，因此：
//! - .cmd → `cmd /c "<path>" --profile web`
//! - .ps1 → `powershell -NoProfile -File "<path>" --profile web`
//! - .exe → 直接执行
//!
//! 平台说明：
//! - Windows：PATH 探测（DSH_CLI 环境变量优先）+ 拉起 + taskkill /T 停止；
//! - macOS：GUI 应用 PATH 是精简的（Homebrew / nvm / npm 全局目录不在其中，
//!   且 dsh 的 shebang `env node` 依赖 PATH 找到 node），因此探测覆盖常见
//!   目录 + nvm 版本目录 + 登录 shell（zsh/bash `command -v dsh`）兜底，
//!   拉起时给子进程合并一个富 PATH；停止用进程组 SIGTERM（spawn 时
//!   process_group(0)），无 libc 依赖（近似 Windows 的 taskkill /T）；
//! - 其他 Unix：stub（返回可读错误）。
//!
//! 测试说明：0xc0000139 链接 bug（Task 2 已修：windows-gnu 测试 exe 缺 Common
//! Controls v6 manifest）修复后，服务逻辑的纯函数（探测决策、命令构造）可在
//! 全平台跑单测；真实 env/fs 交互仍只在本机冒烟（macOS e2e 假 dsh 用例）。

#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::config::dsh_url;

/// DETACHED_PROCESS(0x8)：子进程不继承调用方控制台（不弹黑窗），
/// 效果与 CREATE_NO_WINDOW 相近。settings.rs 打开日志目录共用。
#[cfg(target_os = "windows")]
pub(crate) const DETACHED_PROCESS: u32 = 0x0000_0008;

/// 本应用拉起的服务进程 pid（不持有 Child 句柄：
/// windows-gnu 下 `static` 含 `std::process::Child` 类型会使 test 二进制
/// 加载失败 0xc0000139；且关闭句柄不影响子进程存活，停止时用 taskkill/信号）。
#[cfg(any(target_os = "windows", target_os = "macos"))]
static SERVICE_PID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn service_pid() -> &'static Mutex<Option<u32>> {
    SERVICE_PID.get_or_init(|| Mutex::new(None))
}

/// 在 PATH 中查找 dsh CLI（优先 .cmd > .exe > .ps1；DSH_CLI 环境变量可显式指定）。
/// 纯函数（path 可注入），便于手工/集成验证。
#[cfg(target_os = "windows")]
fn find_dsh_in(paths: &[PathBuf], candidates: &[&str]) -> Option<PathBuf> {
    find_dsh_in_with(paths, candidates, |p| p.is_file())
}

/// 带"存在性判断"的查找内核：生产代码传 p.is_file()；验证可注入模拟 exists。
/// 平台无关纯函数（无 fs/env 依赖），windows 与 macOS 生产共用，测试全平台可跑。
#[cfg(any(target_os = "windows", target_os = "macos", test))]
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

/// 解析 DSH_CLI 环境变量覆盖（可注入 env 值与 exists 判断的纯函数）：
/// 有值且指向存在的文件 → Some(path)；否则（未设置/路径不存在）→ None。
/// 平台无关，供 windows 与 macOS 探测共用，测试全平台可跑。
#[cfg(any(target_os = "windows", target_os = "macos", test))]
fn resolve_dsh_cli_with<F: Fn(&Path) -> bool>(
    env_value: Option<&std::ffi::OsStr>,
    exists: F,
) -> Option<PathBuf> {
    let v = env_value?;
    let p = PathBuf::from(v);
    exists(&p).then_some(p)
}

/// 查找 dsh CLI：DSH_CLI 环境变量 > PATH 搜索。
#[cfg(target_os = "windows")]
pub fn find_dsh() -> Option<PathBuf> {
    let env_override = std::env::var_os("DSH_CLI");
    if let Some(p) = resolve_dsh_cli_with(env_override.as_deref(), |p| p.is_file()) {
        return Some(p);
    }
    if env_override.is_some() {
        log::warn!(target: "service", "DSH_CLI 指定的路径不存在，回退 PATH 搜索");
    }
    let paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    find_dsh_in(&paths, &["dsh.cmd", "dsh.exe", "dsh.ps1", "dsh"])
}

/// 3080 服务探活（TCP connect，1.5s 超时）。
/// 地址解析失败（无 host/无可用地址）一律视为不可达，绝不 panic。
fn service_reachable() -> bool {
    let url = dsh_url();
    let Ok(addrs) = url.socket_addrs(|| None) else {
        return false;
    };
    let Some(addr) = addrs.first() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(addr, std::time::Duration::from_millis(1500)).is_ok()
}

/// 启动 DSH 服务（幂等）：服务已可达 → Ok(0)；否则拉起 dsh，返回 pid。
/// 找不到 dsh CLI 时返回可读错误（前端提示安装）。
#[tauri::command]
pub fn service_start() -> Result<u32, String> {
    start_service()
}

/// 启动 DSH 服务（托盘与 command 共用核心逻辑）。
#[cfg(target_os = "windows")]
pub fn start_service() -> Result<u32, String> {
    if service_reachable() {
        return Ok(0); // 已在运行
    }
    let Some(dsh) = find_dsh() else {
        let msg =
            "未找到 dsh 命令：请安装 DSH 后重试（或设置 DSH_CLI 环境变量指向 dsh 可执行文件）";
        log::warn!(target: "service", "{msg}");
        return Err(msg.into());
    };
    spawn_dsh(&dsh)
}

// ===== macOS 服务自管理 =====
// GUI 应用（Dock 启动）的 PATH 只有系统目录：Homebrew/nvm/npm 全局目录
// 均不在其中，且 dsh（node 脚本，shebang `env node`）运行还依赖能找到
// node。因此探测覆盖常见目录 + nvm 版本目录 + 登录 shell 兜底；
// 拉起时给子进程合并富 PATH。

/// macOS 常见安装目录（brew / npm 全局 / nvm / pnpm / volta / 用户目录）
#[cfg(target_os = "macos")]
fn macos_path_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = vec![
        "/opt/homebrew/bin".into(), // Apple Silicon Homebrew
        "/usr/local/bin".into(),    // Intel Homebrew / npm 默认全局
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for sub in [
            ".npm-global/bin",
            ".npm/bin",
            "Library/pnpm",
            ".volta/bin",
            ".local/bin",
            "bin",
        ] {
            dirs.push(home.join(sub));
        }
        // nvm：~/.nvm/versions/node/<version>/bin（dsh 与 node 同目录）
        if let Ok(rd) = std::fs::read_dir(home.join(".nvm").join("versions").join("node")) {
            for e in rd.flatten() {
                dirs.push(e.path().join("bin"));
            }
        }
    }
    dirs
}

/// 登录 shell 探测 `command -v dsh`（覆盖用户自定义 PATH：asdf/mise 等）。
/// 带超时：shell 启动脚本（.zshrc 等）拖慢也不会卡死界面。
#[cfg(target_os = "macos")]
fn shell_probe(cmd: &mut std::process::Command, timeout: std::time::Duration) -> Option<String> {
    use std::io::Read;
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    let s = out.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// 查找 dsh CLI：DSH_CLI 环境变量 > PATH + 常见目录扫描 > 登录 shell 探测。
#[cfg(target_os = "macos")]
pub fn find_dsh_macos() -> Option<PathBuf> {
    let env_override = std::env::var_os("DSH_CLI");
    if let Some(p) = resolve_dsh_cli_with(env_override.as_deref(), |p| p.is_file()) {
        return Some(p);
    }
    if env_override.is_some() {
        log::warn!(target: "service", "DSH_CLI 指定的路径不存在，回退常规探测");
    }
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    dirs.extend(macos_path_dirs());
    // 目录+候选扫描与 windows 用同一内核（无平台依赖）
    if let Some(p) = find_dsh_in_with(&dirs, &["dsh"], |p| p.is_file()) {
        return Some(p);
    }
    // 兜底：登录 shell 已加载用户 PATH（zsh -i 会读 .zshrc）
    for shell in ["/bin/zsh", "/bin/bash"] {
        let mut cmd = std::process::Command::new(shell);
        cmd.args(["-lic", "command -v dsh"]);
        if let Some(out) = shell_probe(&mut cmd, std::time::Duration::from_millis(1500)) {
            let p = PathBuf::from(out);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// 启动 DSH 服务（macOS）：服务已可达 → Ok(0)；否则拉起 dsh，返回 pid。
#[cfg(target_os = "macos")]
pub fn start_service() -> Result<u32, String> {
    if service_reachable() {
        return Ok(0); // 已在运行
    }
    let Some(dsh) = find_dsh_macos() else {
        let msg =
            "未找到 dsh 命令：请安装 DSH 后重试（或设置 DSH_CLI 环境变量指向 dsh 可执行文件）";
        log::warn!(target: "service", "{msg}");
        return Err(msg.into());
    };
    spawn_dsh_macos(&dsh)
}

/// 拉起 dsh --profile web（macOS）：合并富 PATH 供 shebang `env node` 解析；
/// 新进程组（process_group(0)），停止时整组 SIGTERM（近似 Windows taskkill /T）。
#[cfg(target_os = "macos")]
fn spawn_dsh_macos(dsh: &Path) -> Result<u32, String> {
    use std::os::unix::process::CommandExt;

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(dir) = dsh.parent() {
        dirs.push(dir.to_path_buf()); // dsh 与 node 同目录（nvm/npm 全局）时优先
    }
    dirs.extend(macos_path_dirs());
    if let Some(p) = std::env::var_os("PATH") {
        let mut seen = std::collections::HashSet::new();
        for d in std::env::split_paths(&p) {
            if seen.insert(d.clone()) {
                dirs.push(d);
            }
        }
    }
    let path_env = dirs
        .iter()
        .map(|d| d.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":");

    let mut cmd = std::process::Command::new(dsh);
    cmd.arg("--profile")
        .arg("web")
        .env("PATH", path_env)
        .process_group(0); // 子进程自建进程组（pgid = pid），整组停止
    let child = cmd
        .spawn()
        .map_err(|e| format!("拉起 dsh 失败（请确认 dsh 与 node 可执行）: {e}"))?;
    let pid = child.id();
    *service_pid().lock().unwrap() = Some(pid);
    // 持有句柄：停止时可由 reaper 收割，避免僵尸残留
    *service_child().lock().unwrap() = Some(child);
    log::info!(target: "service", "已拉起 dsh（{}，pid={pid}），等待服务就绪", dsh.display());
    Ok(pid)
}

/// 停止**本应用拉起的** DSH 服务（macOS：进程组 SIGTERM + 僵尸收割）。
/// 不触碰用户自启的服务：仅当 SERVICE_PID 记录时执行；否则返回提示（不误杀）。
#[cfg(target_os = "macos")]
pub fn stop_service() -> Result<String, String> {
    let (pid, mut child) = {
        let pid_guard = service_pid().lock().unwrap();
        let mut ch_guard = service_child().lock().unwrap();
        (*pid_guard, ch_guard.take())
    };
    let Some(pid) = pid else {
        return Err("未找到由本应用拉起的 DSH 服务（可能由其他方式启动，未做处理）".into());
    };
    // 进程组 SIGTERM（dsh → node 子树一起收；/bin/kill 负数 pid 即进程组）
    let group_kill = std::process::Command::new("/bin/kill")
        .args(["-s", "TERM", "--", &format!("-{pid}")])
        .output();
    match group_kill {
        Ok(out) if out.status.success() => {
            *service_pid().lock().unwrap() = None;
            reap(child);
            log::info!(target: "service", "已停止本应用拉起的 DSH 服务（pid={pid}）");
            Ok(format!("已停止 DSH 服务（pid={pid}）"))
        }
        _ => {
            // 组杀失败：多为进程组已消失（进程已退出）
            let exited = child
                .as_mut()
                .map(|c| matches!(c.try_wait(), Ok(Some(_))))
                .unwrap_or(false);
            if exited {
                *service_pid().lock().unwrap() = None;
                log::info!(target: "service", "DSH 服务已不在运行（pid={pid}）");
                return Ok(format!("DSH 服务已不在运行（pid={pid}）"));
            }
            // 兜底：直接向 pid 发 SIGTERM（进程组可能被外部改变）
            match std::process::Command::new("/bin/kill")
                .args(["-s", "TERM", &pid.to_string()])
                .output()
            {
                Ok(out) if out.status.success() => {
                    *service_pid().lock().unwrap() = None;
                    reap(child);
                    log::info!(target: "service", "已停止本应用拉起的 DSH 服务（pid={pid}，直接信号）");
                    Ok(format!("已停止 DSH 服务（pid={pid}）"))
                }
                Ok(out) => Err(format!(
                    "停止服务失败（pid={pid}）: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )),
                Err(e) => Err(format!("停止服务失败（pid={pid}）: {e}")),
            }
        }
    }
}

/// macOS 持有子进程句柄用于停止时收割：Dropped Child 不等待退出，
/// 进程死亡后会留下僵尸（`kill -0` 仍报"存活"）。
/// 仅 macOS 编译（windows-gnu 下 static 含 Child 会触发测试二进制链接 bug）。
#[cfg(target_os = "macos")]
static SERVICE_CHILD: OnceLock<Mutex<Option<std::process::Child>>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn service_child() -> &'static Mutex<Option<std::process::Child>> {
    SERVICE_CHILD.get_or_init(|| Mutex::new(None))
}

/// 后台收割子进程：TERM 后轮询退出（最多 2s），未退出 SIGKILL 兜底；
/// wait()/try_wait() 收割后不残留僵尸。
#[cfg(target_os = "macos")]
fn reap(child: Option<std::process::Child>) {
    if let Some(mut c) = child {
        std::thread::spawn(move || {
            for _ in 0..40 {
                match c.try_wait() {
                    Ok(Some(_)) | Err(_) => return,
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                }
            }
            let _ = c.kill(); // TERM 无效 → SIGKILL 兜底
            let _ = c.wait();
        });
    }
}

/// 其他 Unix（如 Linux）：服务自管理未实现（stub，不扩功能）。
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn start_service() -> Result<u32, String> {
    Err("当前平台暂不支持从壳层拉起 DSH 服务（请手动启动 dsh）".into())
}

/// 停止**本应用拉起的** DSH 服务进程树（taskkill /T /F）。
/// 不触碰用户自启的服务：仅当 SERVICE_PID 记录时执行；否则返回提示（不误杀）。
#[tauri::command]
pub fn service_stop() -> Result<String, String> {
    stop_service()
}

/// 停止本应用拉起的服务（托盘与 command 共用核心逻辑）。
#[cfg(target_os = "windows")]
pub fn stop_service() -> Result<String, String> {
    // 先拷贝 pid 再执行：taskkill 失败时记录保留，允许重试
    let pid = {
        let guard = service_pid().lock().unwrap();
        let Some(pid) = *guard else {
            return Err("未找到由本应用拉起的 DSH 服务（可能由其他方式启动，未做处理）".into());
        };
        pid
    };
    // taskkill /T：连同进程树（dsh.cmd → node 子进程）一起结束
    let kill = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(DETACHED_PROCESS)
        .output();
    match kill {
        Ok(out) if out.status.success() => {
            *service_pid().lock().unwrap() = None;
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

/// 其他 Unix（如 Linux）：服务自管理未实现（stub，不扩功能）。
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn stop_service() -> Result<String, String> {
    Err("当前平台暂不支持从壳层停止 DSH 服务（请手动结束 dsh 进程）".into())
}

/// 按扩展名选择启动程序与参数（纯函数，平台无关）：
///
/// - `.cmd` → `cmd /c "<path>" --profile web`
/// - `.ps1` → `powershell -NoProfile -ExecutionPolicy Bypass -File "<path>" --profile web`
/// - 其他/无扩展名 → `<path> --profile web`
///
/// 返回 (程序, 参数列表)；扩展名比较大小写不敏感（`.CMD`/`.Ps1` 同分支）。
/// 生产仅 windows spawn_dsh 使用；测试全平台可跑（cfg(test) 下编译）。
#[cfg(any(target_os = "windows", test))]
fn dsh_command_parts(dsh: &Path) -> (String, Vec<String>) {
    let ext = dsh
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let path = dsh.to_string_lossy().into_owned();
    match ext.as_str() {
        "cmd" => (
            "cmd".into(),
            vec!["/c".into(), path, "--profile".into(), "web".into()],
        ),
        "ps1" => (
            "powershell".into(),
            vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                path,
                "--profile".into(),
                "web".into(),
            ],
        ),
        _ => (path, vec!["--profile".into(), "web".into()]),
    }
}

/// 按扩展名选择启动方式拉起 dsh --profile web。
/// 用 DETACHED_PROCESS(0x8) 不弹控制台窗口。
#[cfg(target_os = "windows")]
fn spawn_dsh(dsh: &Path) -> Result<u32, String> {
    let (program, args) = dsh_command_parts(dsh);
    let child = std::process::Command::new(program)
        .args(&args)
        .creation_flags(DETACHED_PROCESS)
        .spawn();
    let child = child.map_err(|e| format!("拉起 dsh 失败: {e}"))?;
    let pid = child.id();
    // 句柄随 child drop 关闭，不影响子进程存活；记录 pid 供停止时 taskkill
    *service_pid().lock().unwrap() = Some(pid);
    log::info!(target: "service", "已拉起 dsh（{}，pid={pid}），等待服务就绪", dsh.display());
    Ok(pid)
}

#[cfg(test)]
mod tests {
    // ===== 平台无关纯函数测试（全平台可跑，含 CI windows-gnu） =====
    use super::{dsh_command_parts, find_dsh_in_with, resolve_dsh_cli_with};
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// 候选顺序：目录顺序优先（外层），目录内按候选顺序（.cmd > .exe > .ps1）。
    #[test]
    fn find_dsh_prefers_cmd_over_exe_over_ps1() {
        let dirs = vec![p("dirA")];
        let candidates = ["dsh.cmd", "dsh.exe", "dsh.ps1", "dsh"];
        let hit = find_dsh_in_with(&dirs, &candidates, |path| {
            path.file_name().map(|n| n == "dsh.cmd").unwrap_or(false)
        });
        assert_eq!(hit, Some(p("dirA/dsh.cmd")));
    }

    /// 目录顺序：第一个目录命中即返回，不继续扫后续目录。
    #[test]
    fn find_dsh_stops_at_first_matching_dir() {
        let dirs = vec![p("dirA"), p("dirB")];
        let hit = find_dsh_in_with(&dirs, &["dsh.exe"], |path| {
            *path == p("dirA/dsh.exe")
        });
        assert_eq!(hit, Some(p("dirA/dsh.exe")));
    }

    /// 全部候选缺失 → None。
    #[test]
    fn find_dsh_returns_none_when_nothing_exists() {
        let dirs = vec![p("C:\\zero")];
        assert_eq!(find_dsh_in_with(&dirs, &["dsh"], |_| false), None);
    }

    /// 空目录列表 → None（不 panic）。
    #[test]
    fn find_dsh_handles_empty_dirs() {
        assert_eq!(find_dsh_in_with(&[], &["dsh"], |_| true), None);
    }

    /// 存在性判断的注入有效性：模拟 exists 只认目标文件，其余一律不存在。
    #[test]
    fn find_dsh_honors_injected_exists() {
        let dirs = vec![p("/tmp/x")];
        let existent = p("/tmp/x/dsh");
        let hit = find_dsh_in_with(&dirs, &["dsh"], |path| *path == existent);
        assert_eq!(hit, Some(existent));
        // 换一个候选：dsh 不存在 → None
        let dirs2 = vec![p("/tmp/y")];
        assert_eq!(find_dsh_in_with(&dirs2, &["dsh"], |_| false), None);
    }

    /// DSH_CLI 覆盖：值为存在文件 → 采用（优先于 PATH 扫描，由调用方保证顺序）。
    #[test]
    fn resolve_dsh_cli_uses_existing_override() {
        let hit = resolve_dsh_cli_with(Some(std::ffi::OsStr::new("D:\\dsh.exe")), |path| {
            *path == p("D:\\dsh.exe")
        });
        assert_eq!(hit, Some(p("D:\\dsh.exe")));
    }

    /// DSH_CLI 覆盖：值为不存在文件 → None（调用方回退 PATH 搜索）。
    #[test]
    fn resolve_dsh_cli_rejects_missing_path() {
        let hit = resolve_dsh_cli_with(Some(std::ffi::OsStr::new("D:\\nope.exe")), |_| false);
        assert_eq!(hit, None);
    }

    /// DSH_CLI 覆盖：未设置（None）→ None。
    #[test]
    fn resolve_dsh_cli_none_when_env_unset() {
        assert_eq!(resolve_dsh_cli_with(None, |_| true), None);
    }

    /// 命令构造 .cmd 分支。
    #[test]
    fn dsh_command_cmd_uses_cmd_wrapper() {
        let (prog, args) = dsh_command_parts(PathBuf::from("C:\\dsh\\dsh.cmd").as_path());
        assert_eq!(prog, "cmd");
        assert_eq!(
            args,
            vec!["/c", "C:\\dsh\\dsh.cmd", "--profile", "web"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    /// 命令构造 .ps1 分支（含参数顺序与 ExecutionPolicy Bypass）。
    #[test]
    fn dsh_command_ps1_uses_powershell_wrapper() {
        let (prog, args) = dsh_command_parts(PathBuf::from("C:\\dsh\\dsh.ps1").as_path());
        assert_eq!(prog, "powershell");
        assert_eq!(
            args,
            vec![
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                "C:\\dsh\\dsh.ps1",
                "--profile",
                "web",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
        );
    }

    /// 命令构造默认分支：无扩展名/exe 直接执行。
    #[test]
    fn dsh_command_default_direct_exec() {
        let (prog, args) = dsh_command_parts(PathBuf::from("C:\\dsh\\dsh.exe").as_path());
        assert_eq!(prog, "C:\\dsh\\dsh.exe");
        assert_eq!(args, vec!["--profile", "web"]);
    }

    /// 扩展名大小写不敏感：`.CMD` 走 cmd 分支、`.PS1` 走 powershell 分支。
    #[test]
    fn dsh_command_extension_case_insensitive() {
        let (prog, _) = dsh_command_parts(PathBuf::from("C:\\dsh\\dsh.CMD").as_path());
        assert_eq!(prog, "cmd");
        let (prog, _) = dsh_command_parts(PathBuf::from("C:\\dsh\\dsh.Ps1").as_path());
        assert_eq!(prog, "powershell");
    }

    #[cfg(target_os = "macos")]
    use super::find_dsh_macos;

    /// 环境相关冒烟（仅 macOS 本机）：装有 dsh 时探测结果必须存在；
    /// 未安装 dsh 时跳过（不失败）。CI 为 windows-gnu，不会执行。
    #[test]
    #[cfg(target_os = "macos")]
    fn macos_find_dsh_returns_existing_file_when_installed() {
        if let Some(p) = find_dsh_macos() {
            assert!(p.is_file(), "探测到的 dsh 不存在: {}", p.display());
        }
    }

    /// 端到端（仅 macOS 本机）：DSH_CLI 指向假 dsh 脚本 → 检查参数（--profile web）
    /// → stop_service 进程组终止（含脚本子进程）。DSH_URL 指向空闲端口，
    /// 不触碰真实 dsh/服务。CI 为 windows-gnu，不会执行此测试。
    #[test]
    #[cfg(target_os = "macos")]
    fn macos_start_stop_fake_dsh_end_to_end() {
        use std::os::unix::fs::PermissionsExt;

        let tag = std::process::id();
        let script = std::env::temp_dir().join(format!("dsh-fake-{tag}.sh"));
        let marker = std::env::temp_dir().join(format!("dsh-fake-args-{tag}.txt"));
        let _ = std::fs::remove_file(&marker);
        // 假 dsh：把收到的参数写入 marker，起一个子进程后驻留（验证整组终止）
        let body = format!(
            "#!/bin/bash\necho \"$@\" > \"{}\"\n(sleep 30) &\nwait\n",
            marker.display()
        );
        std::fs::write(&script, body).unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        std::env::set_var("DSH_CLI", &script);
        std::env::set_var("DSH_URL", "http://127.0.0.1:1"); // 空闲端口，避免交互真实服务

        let pid = super::start_service().expect("start_service 应成功拉起假 dsh");
        let mut args = String::new();
        for _ in 0..200 {
            if let Ok(s) = std::fs::read_to_string(&marker) {
                args = s;
                if !args.is_empty() {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(args.contains("--profile"), "dsh 命令缺 --profile: {args}");
        assert!(args.contains("web"), "dsh 命令缺 web profile: {args}");

        let msg = super::stop_service().expect("stop_service 应成功终止假 dsh");
        // 轮询等待进程退出（TERM → 退出 → 收割有短暂延迟），超时 3s 判定失败
        let mut dead = false;
        for _ in 0..60 {
            let alive = std::process::Command::new("/bin/kill")
                .args(["-0", &pid.to_string()])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !alive {
                dead = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // 先清理（脚本/标记/env），失败也不留残余、不污染其他测试
        std::env::remove_var("DSH_CLI");
        std::env::remove_var("DSH_URL");
        let _ = std::fs::remove_file(&script);
        let _ = std::fs::remove_file(&marker);
        assert!(dead, "假 dsh 进程仍存活（pid={pid}）: {msg}");
    }
}
