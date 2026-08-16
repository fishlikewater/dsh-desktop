//! 日志模块：tauri-plugin-log 文件轮转输出 + 调试构建保留 stdout。
//!
//! - 文件：`%APPDATA%\com.dsh.desktop\logs\dsh-desktop.log`（1MB 大小轮转，保留全部旧文件）
//! - debug 构建追加 stdout 目标（开发时终端可见，release 不输出）
//! - 级别：Info（审计日志使用 info!，调试细节用 debug!）

use log::LevelFilter;
use tauri_plugin_log::{Builder, RotationStrategy, Target, TargetKind};

/// 构建日志插件（注册到 tauri Builder 的 plugin 链）。
pub fn init() -> Builder {
    let mut targets = vec![Target::new(TargetKind::LogDir {
        file_name: Some("dsh-desktop".into()),
    })];
    #[cfg(debug_assertions)]
    targets.push(Target::new(TargetKind::Stdout));

    Builder::new()
        .level(LevelFilter::Info)
        .targets(targets)
        .max_file_size(1_000_000)
        .rotation_strategy(RotationStrategy::KeepAll)
}
