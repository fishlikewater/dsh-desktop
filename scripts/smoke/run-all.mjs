#!/usr/bin/env node
/**
 * 端到端冒烟编排：前置检查 + 顺序执行全部场景。
 *
 * 前置条件：
 * 1. debug 构建存在（CDP 端口 9226）
 * 2. 应用已启动（脚本不会自动拉起应用；CI 中由工作流负责启动）
 *
 * 模式：
 * - 默认（真实模式）：DSH 服务在线（默认 http://127.0.0.1:3080，可用 DSH_URL 覆盖）
 * - --mock：使用自带 mock-server（scripts/smoke/mock-server.mjs），脱离真实 DSH 服务；
 *   自动拉起 mock、自动准备临时 DSH_HOME 基线（theme-sync 需要 settings.yaml）。
 *   DSH_URL 默认为 http://127.0.0.1:3099（与真实 3080 区分）。
 *
 * 用法:
 *   node scripts/smoke/run-all.mjs              # 真实模式
 *   node scripts/smoke/run-all.mjs --mock       # mock 模式（无需 DSH）
 */
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");

const MOCK = process.argv.includes("--mock");
const MOCK_PORT = Number(process.env.MOCK_PORT ?? 3099);

// exe 路径按平台解析（不再硬编码 .exe）
const exeName = process.platform === "win32" ? "dsh-desktop.exe" : "dsh-desktop";
const exe = join(root, "src-tauri", "target", "debug", exeName);

const DSH_URL = MOCK
  ? process.env.DSH_URL ?? `http://127.0.0.1:${MOCK_PORT}`
  : process.env.DSH_URL ?? "http://127.0.0.1:3080";

// 场景顺序说明：window-ctrl 会最小化窗口（前端无 unminimize 权限，最小化后
// 尺寸无法经 setSize 恢复），因此必须放在最后，避免污染后续场景。
// snap 与 status-dot 在 window-ctrl 之前（只改几何/类名，且各自校验后自洽）。
const scenarios = [
  ["window-center.mjs", "窗口居中"],
  ["theme-sync.mjs", "主题即时同步"],
  ["isolation.mjs", "隔离边界"],
  ["status-dot.mjs", "状态圆点"],
  ["address-switch.mjs", "地址切换"],
  ["multi-window.mjs", "多会话窗口"],
  ["snap.mjs", "贴靠状态机"],
  ["window-ctrl.mjs", "窗口控制权限"],
];

// mock 模式：DSH_HOME 准备工作区 + settings.yaml 基线（Rust 主题 watcher 监听该文件）。
// 若外部已注入 DSH_HOME（CI 启动应用时也用它，必须一致），直接复用；否则自建临时目录。
let mockHome = null;
let mockServer = null;

function setupMockEnv() {
  if (process.env.DSH_HOME && existsSync(process.env.DSH_HOME)) {
    mockHome = process.env.DSH_HOME;
  } else {
    mockHome = process.env.DSH_HOME ?? join(tmpdir(), `dsh-smoke-${process.pid}`);
    mkdirSync(mockHome, { recursive: true });
    process.env.DSH_HOME = mockHome;
  }
  const settings = join(mockHome, "settings.yaml");
  if (!existsSync(settings)) {
    writeFileSync(settings, "ui-theme:\n  preference: light\n", "utf8");
  }
  console.log(`[run-all] mock DSH_HOME=${mockHome}（settings.yaml 基线就绪）`);
}

function startMockServer() {
  mockServer = spawn(
    process.execPath,
    [join(here, "mock-server.mjs"), String(MOCK_PORT)],
    { stdio: "inherit" },
  );
  return new Promise((resolve) => {
    // 等待端口就绪（简单轮询）
    const deadline = Date.now() + 5000;
    const poll = async () => {
      try {
        const res = await fetch(`http://127.0.0.1:${MOCK_PORT}/health`, {
          signal: AbortSignal.timeout(1000),
        });
        if (res.ok) return resolve();
      } catch {}
      if (Date.now() > deadline) return resolve();
      setTimeout(poll, 150);
    };
    poll();
  });
}

async function stopMockServer() {
  if (mockServer && mockServer.exitCode === null) {
    mockServer.kill("SIGTERM");
    await new Promise((r) => mockServer.once("exit", () => r()));
  }
}

// 前置检查
const problems = [];
if (!existsSync(exe)) {
  problems.push(`debug 构建不存在: ${exe}（先运行 npm run dev 或 cargo build）`);
}

const run = async () => {
  if (MOCK) {
    setupMockEnv();
    await startMockServer();
    problems.push(...[]); // mock 起完后再验证可达性
  }

  try {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), 3000);
    const res = await fetch(DSH_URL, { signal: ctrl.signal });
    clearTimeout(timer);
    if (!res.ok) problems.push(`DSH 服务响应异常: ${DSH_URL} -> HTTP ${res.status}`);
  } catch (e) {
    if (MOCK) {
      problems.push(`mock 服务不可达: ${DSH_URL}（${e.cause?.code ?? e.message}）`);
    } else {
      problems.push(
        `DSH 服务不可达: ${DSH_URL}（${e.cause?.code ?? e.message}；请先启动 dsh --profile web，或使用 --mock 模式）`,
      );
    }
  }
  try {
    await fetch("http://127.0.0.1:9226/json", { signal: AbortSignal.timeout(2000) });
  } catch {
    // 应用启动可能与 run-all 并行（CI 后台拉起）：轮询等待 CDP 就绪（最长 60s）
    let ready = false;
    const deadline = Date.now() + 60000;
    while (Date.now() < deadline) {
      try {
        await fetch("http://127.0.0.1:9226/json", { signal: AbortSignal.timeout(2000) });
        ready = true;
        break;
      } catch {}
      await new Promise((r) => setTimeout(r, 1000));
    }
    if (!ready) problems.push("CDP 端口 9226 不可达（应用未启动或非 debug 构建）");
  }
  if (problems.length > 0) {
    console.error("前置检查失败：");
    for (const p of problems) console.error(`  - ${p}`);
    console.error("请确认前置条件后重试（见 docs/testing.md）");
    await stopMockServer();
    process.exit(1);
  }

  // 顺序执行
  let failed = 0;
  for (const [file, name] of scenarios) {
    process.stdout.write(`\n===== ${name} (${file}) =====\n`);
    try {
      execFileSync(process.execPath, [join(here, file)], {
        stdio: "inherit",
        cwd: root,
        env: process.env,
      });
      console.log(`===== ${name}: PASS =====`);
    } catch (e) {
      failed += 1;
      console.error(`===== ${name}: FAIL（exit ${e.status ?? "?"}）=====`);
    }
  }
  await stopMockServer();

  if (failed > 0) {
    console.error(`\n${failed}/${scenarios.length} 个场景失败`);
    process.exit(1);
  }
  console.log(`\n全部 ${scenarios.length} 个冒烟场景通过`);
};

run();