#!/usr/bin/env node
/**
 * 冒烟 4/5：窗口居中。
 * 验证窗口最终位置位于工作区中央（事件驱动居中，容差 60px）。
 * 等待位置稳定（两次采样一致）后再判定，避免启动中间态误报。
 */
import { evalShell, wait } from "./lib.mjs";

async function sample() {
  return JSON.parse(await evalShell(`(async()=>{
    const win = window.__TAURI__.window.getCurrentWindow();
    const pos = await win.outerPosition();
    const size = await win.outerSize();
    const wa = await window.__TAURI__.core.invoke("work_area");
    return JSON.stringify({ pos: { x: pos.x, y: pos.y }, size: { w: size.width, h: size.height }, wa });
  })()`));
}

// 等待窗口可见且位置稳定（最多 20s）
let stable = null;
for (let i = 0; i < 40; i++) {
  const a = await sample();
  await wait(500);
  const b = await sample();
  if (a.pos.x === b.pos.x && a.pos.y === b.pos.y && b.size.w > 500) {
    stable = b;
    break;
  }
}
if (!stable) {
  console.error("FAIL: 窗口位置未稳定（可能未显示）");
  await finish(1);
}

const expectX = stable.wa[0] + Math.round((stable.wa[2] - stable.size.w) / 2);
const expectY = stable.wa[1] + Math.round((stable.wa[3] - stable.size.h) / 2);
const dx = Math.abs(stable.pos.x - expectX);
const dy = Math.abs(stable.pos.y - expectY);
console.log(`pos=(${stable.pos.x},${stable.pos.y}) expect=(${expectX},${expectY}) dx=${dx} dy=${dy}`);
const ok = dx <= 60 && dy <= 60;
console.log(ok ? "PASS" : "FAIL: 窗口未居中");
process.exit(ok ? 0 : 1);

