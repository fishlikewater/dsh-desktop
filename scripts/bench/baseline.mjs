#!/usr/bin/env node
/**
 * 运行时基线测量：启动应用后采样内存/CPU，输出基线 JSON。
 *
 * 用法：
 *   node scripts/bench/baseline.mjs [exe路径] [采样秒数] [间隔ms]
 * 默认：release 构建、30s、1s 间隔。
 *
 * 输出：docs/perf.md 用的实测数据 + 控制台摘要。
 */
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const exe = resolve(process.argv[2] ?? "src-tauri/target/release/dsh-desktop.exe");
const totalSec = Number(process.argv[3] ?? 30);
const intervalMs = Number(process.argv[4] ?? 1000);

if (!existsSync(exe)) {
  console.error(`[baseline] 找不到 ${exe}（先 cargo build --release）`);
  process.exit(1);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
// 启动应用（后台）
import { spawn } from "node:child_process";
const app = spawn(exe, [], { stdio: "ignore", detached: false });

// 等启动完成（窗口 + 壳页加载）
await sleep(6000);

const samples = [];
const t0 = Date.now();
// 进程名：dsh-desktop
const getProc = () => {
  try {
    // Get-CimInstance 精确获取工作集与 CPU 时间
    const out = execFileSync(
      "powershell",
      [
        "-NoProfile",
        "-Command",
        "Get-CimInstance Win32_Process -Filter \"Name='dsh-desktop.exe'\" | Select-Object WorkingSetSize, KernelModeTime, UserModeTime, ProcessId | ConvertTo-Json -Compress",
      ],
      { encoding: "utf8", windowsHide: true }
    ).trim();
    if (!out) return null;
    const p = JSON.parse(out);
    const item = Array.isArray(p) ? p[0] : p;
    if (!item) return null;
    // KernelModeTime/UserModeTime 单位 100ns
    const cpu100ns = Number(item.KernelModeTime ?? 0) + Number(item.UserModeTime ?? 0);
    return { ws: Number(item.WorkingSetSize), cpu100ns, pid: item.ProcessId };
  } catch {
    return null;
  }
};

while (Date.now() - t0 < totalSec * 1000) {
  const p = getProc();
  if (p) samples.push({ t: Date.now() - t0, ws: p.ws, cpu100ns: p.cpu100ns });
  await sleep(intervalMs);
}

// 计算指标
const wss = samples.map((s) => s.ws);
const wsMb = (bytes) => (bytes / 1024 / 1024).toFixed(1);
const result = {
  exe,
  totalSec,
  samples: samples.length,
  mem: {
    minMb: wsMb(Math.min(...wss)),
    maxMb: wsMb(Math.max(...wss)),
    avgMb: wsMb(wss.reduce((a, b) => a + b, 0) / wss.length),
  },
  cpu: {},
};
// CPU 使用率：最后两个采样间的 CPU 时间差 / 墙钟差
if (samples.length >= 2) {
  const a = samples[samples.length - 2];
  const b = samples[samples.length - 1];
  const wallMs = b.t - a.t;
  const cpuMs = ((b.cpu100ns - a.cpu100ns) / 10000) * 1; // 100ns -> ms
  result.cpu.lastIntervalPct = ((cpuMs / wallMs) * 100).toFixed(2) + "%";
} else {
  result.cpu.lastIntervalPct = "n/a";
}

console.log(JSON.stringify(result, null, 2));
// 清理：退出应用
try {
  execFileSync("powershell", ["-NoProfile", "-Command", "Stop-Process -Name dsh-desktop -Force -ErrorAction SilentlyContinue"], {
    windowsHide: true,
  });
} catch {}
process.exit(0);
