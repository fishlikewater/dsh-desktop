#!/usr/bin/env node
/**
 * 冒烟 2/5：窗口控制权限。
 * 按壳页实际调用面验证 capabilities：伪最大化（setPosition+setSize 贴齐工作区）、
 * 最小化、show。iframe 内 IPC 拒绝见 isolation.mjs。
 */
import { evalShell, finish } from "./lib.mjs";

const out = JSON.parse(await evalShell(`(async()=>{
  const win = window.__TAURI__.window.getCurrentWindow();
  const out = {};
  // 伪最大化
  try {
    const wa = await window.__TAURI__.core.invoke("work_area");
    await win.setPosition(new window.__TAURI__.window.PhysicalPosition(wa[0], wa[1]));
    await win.setSize(new window.__TAURI__.window.PhysicalSize(wa[2], wa[3]));
    const s = await win.outerSize();
    out.maximize = { ok: s.width >= wa[2] - 30 && s.height >= wa[3] - 30, size: { w: s.width, h: s.height } };
  } catch(e) { out.maximize = 'ERR:' + e; }
  // 最小化
  try { await win.minimize(); out.minimize = true; } catch(e) { out.minimize = 'ERR:' + e; }
  // show（最小化后尺寸恢复由 Rust 托盘路径 unminimize 处理，不经前端权限）
  await new Promise(r => setTimeout(r, 800));
  try { await win.show(); out.show = true; } catch(e) { out.show = 'ERR:' + e; }
  return JSON.stringify(out);
})()`));

console.log("WINDOW-CTRL:", JSON.stringify(out));
const ok = out.maximize?.ok === true && out.minimize === true && out.show === true;
console.log(ok ? "PASS" : "FAIL");
process.exit(ok ? 0 : 1);

