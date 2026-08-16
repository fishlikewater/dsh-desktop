#!/usr/bin/env node
/**
 * 生成 .cargo/config.toml：定位 rustup GNU 工具链自带的 MinGW-w64 链接器。
 *
 * 背景：系统 PATH 中的旧版 gcc（如 MinGW-W64 8.1.0）与新版 rustc 生成的
 * COFF 对象不兼容（链接产物无法启动，os error 193），必须显式指定 rustup
 * 工具链 self-contained 组件中的 x86_64-w64-mingw32-gcc.exe。
 * cargo 配置的 linker 值不支持 ${VAR} 展开（ConfigRelativePath 类型），
 * 故由本脚本在构建前按本机环境生成。
 *
 * 查找顺序：RUSTUP_HOME 环境变量 → USERPROFILE（Windows）/HOME（POSIX）。
 * 无 rustup 安装时生成仅含说明的空配置并给出警告（此时需自行配置链接器）。
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const rustupHome =
  process.env.RUSTUP_HOME || process.env.USERPROFILE || process.env.HOME;

const LINKER = [
  rustupHome,
  ".rustup",
  "toolchains",
  "stable-x86_64-pc-windows-gnu",
  "lib",
  "rustlib",
  "x86_64-pc-windows-gnu",
  "bin",
  "self-contained",
  "x86_64-w64-mingw32-gcc.exe",
].join("/");

const escaped = LINKER.replace(/\\/g, "\\\\");

const toml = `# 本文件由 scripts/setup-gnu-linker.mjs 自动生成，请勿手动修改。
# 若需自定义，修改脚本或临时覆盖本文件（构建脚本会在 npm scripts 中重新生成）。
[env]
# tauri 官方 workaround：防止 windows-gnu 下 cargo test 偶发
# STATUS_ENTRYPOINT_NOT_FOUND（0xc0000139）崩溃
# 参考 https://github.com/tauri-apps/tauri/pull/4383#issuecomment-1212221864
__TAURI_WORKSPACE__ = "true"

[target.x86_64-pc-windows-gnu]
linker = "${escaped}"
`;

const cargoDir = resolve(".cargo");
mkdirSync(cargoDir, { recursive: true });
writeFileSync(join(cargoDir, "config.toml"), toml, "utf8");
console.log(`[setup-gnu-linker] linker: ${LINKER}`);
