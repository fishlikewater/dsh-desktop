#!/usr/bin/env node
/**
 * 端到端冒烟编排：前置检查 + 顺序执行全部场景。
 *
 * 前置条件：
 * 1. DSH 服务在线（默认 http://127.0.0.1:3080，可用 DSH_URL 环境变量覆盖）
 * 2. debug 构建存在（src-tauri/target/debug/dsh-desktop.exe，CDP 端口 9226）
 * 3. 应用已启动（脚本不会自动拉起应用；CI 中由工作流负责启动）
 *
 * 用法: node scripts/smoke/run-all.mjs
 */
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const exe = join(root, "src-tauri", "target", "debug", "dsh-desktop.exe");
const DSH_URL = process.env.DSH_URL ?? "http://127.0.0.1:3080";

// 场景顺序说明：window-ctrl 会最小化窗口（前端无 unminimize 权限，最小化后
// 尺寸无法经 setSize 恢复），因此必须放在最后，避免污染后续场景。
const scenarios = [
  ["window-center.mjs", "窗口居中"],
  ["theme-sync.mjs", "主题即时同步"],
  ["isolation.mjs", "隔离边界"],
  ["window-ctrl.mjs", "窗口控制权限"],
];

// 前置检查
const problems = [];
if (!existsSync(exe)) {
  problems.push(`debug 构建不存在: ${exe}（先运行 npm run dev 或 cargo build）`);
}
try {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), 3000);
  const res = await fetch(DSH_URL, { signal: ctrl.signal });
  clearTimeout(timer);
  if (!res.ok) problems.push(`DSH 服务响应异常: ${DSH_URL} -> HTTP ${res.status}`);
} catch (e) {
  problems.push(`DSH 服务不可达: ${DSH_URL}（${e.cause?.code ?? e.message}；请先启动 dsh --profile web）`);
}
try {
  await fetch("http://127.0.0.1:9226/json", { signal: AbortSignal.timeout(2000) });
} catch {
  problems.push("CDP 端口 9226 不可达（应用未启动或非 debug 构建）");
}
if (problems.length > 0) {
  console.error("前置检查失败：");
  for (const p of problems) console.error(`  - ${p}`);
  console.error("请确认前置条件后重试（见 docs/testing.md）");
  process.exit(1);
}

// 顺序执行
let failed = 0;
for (const [file, name] of scenarios) {
  process.stdout.write(`\n===== ${name} (${file}) =====\n`);
  try {
    execFileSync(process.execPath, [join(here, file)], { stdio: "inherit", cwd: root });
    console.log(`===== ${name}: PASS =====`);
  } catch (e) {
    failed += 1;
    console.error(`===== ${name}: FAIL（exit ${e.status ?? "?"}）=====`);
  }
}
if (failed > 0) {
  console.error(`\n${failed}/${scenarios.length} 个场景失败`);
  process.exit(1);
}
console.log(`\n全部 ${scenarios.length} 个冒烟场景通过`);
