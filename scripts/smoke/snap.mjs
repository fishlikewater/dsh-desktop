#!/usr/bin/env node
/**
 * 冒烟 5/5：Windows 11 贴靠（合成方向键事件）。
 * 验证壳页贴靠状态机：
 *   手动几何 A → Win+→ 贴靠右半屏 → Win+← 贴靠左半屏 → Win+← 还原回 A
 * （首次贴靠自动保存 A 为 savedRect；snapTo 语义：非贴靠态首存、重复同向还原）
 *
 * 注意：WebView2 中真实 Win 组合键可能被 OS 捕获（README 记录局限），
 * 本场景用合成 KeyboardEvent 验证壳页自身逻辑（系统放行时行为一致）。
 */
import { evalShell, finish } from "./lib.mjs";

// 合成 keydown（metaKey + 方向键）触发壳页贴靠监听
function sendKey(key) {
  return evalShell(`(function(){
    const e = new KeyboardEvent("keydown", {
      key: ${JSON.stringify(key)}, metaKey: true, bubbles: true, cancelable: true
    });
    document.dispatchEvent(e);
    return e.defaultPrevented ? "handled" : "ignored";
  })()`);
}

async function geom() {
  return JSON.parse(await evalShell(`(async()=>{
    const win = window.__TAURI__.window.getCurrentWindow();
    const pos = await win.outerPosition();
    const size = await win.outerSize();
    const wa = await window.__TAURI__.core.invoke("work_area");
    return JSON.stringify({ pos: { x: pos.x, y: pos.y }, size: { w: size.width, h: size.height }, wa });
  })()`));
}

// 手动设置几何（贴靠的 savedRect 种子）
async function setGeom(x, y, w, h) {
  await evalShell(`(async()=>{
    const win = window.__TAURI__.window.getCurrentWindow();
    await win.setPosition(new window.__TAURI__.window.PhysicalPosition(${x}, ${y}));
    await win.setSize(new window.__TAURI__.window.PhysicalSize(${w}, ${h}));
  })()`);
  await new Promise((r) => setTimeout(r, 400));
}

const wa0 = (await geom()).wa;

// 贴靠前的"手动几何 A"：工作区左上角 + 缩尺寸（避免与半屏/全屏混淆）
const A = {
  x: wa0[0] + 30,
  y: wa0[1] + 30,
  w: Math.max(900, Math.round(wa0[2] * 0.6)),
  h: Math.max(640, Math.round(wa0[3] * 0.7)),
};
// 确保 A 不超出工作区（最小尺寸约束内取合理值）
A.x = Math.min(A.x, wa0[0] + wa0[2] - A.w);
A.y = Math.min(A.y, wa0[1] + wa0[3] - A.h);
A.w = Math.min(A.w, wa0[2]);
A.h = Math.min(A.h, wa0[3]);

const expectRight = {
  x: wa0[0] + Math.round(wa0[2] / 2),
  y: wa0[1],
  w: wa0[2] - Math.round(wa0[2] / 2),
  h: wa0[3],
};
const expectLeft = { x: wa0[0], y: wa0[1], w: Math.round(wa0[2] / 2), h: wa0[3] };

let ok = true;
function check(label, cond, detail) {
  console.log(`${cond ? "PASS" : "FAIL"}: ${label}${detail ? " " + detail : ""}`);
  ok = ok && cond;
}
function near(g, expect) {
  return (
    Math.abs(g.pos.x - expect.x) <= 2 &&
    Math.abs(g.pos.y - expect.y) <= 2 &&
    Math.abs(g.size.w - expect.w) <= 2 &&
    Math.abs(g.size.h - expect.h) <= 2
  );
}

// 0) 播种：设置手动几何 A（首次贴靠将自动存为 savedRect）
await setGeom(A.x, A.y, A.w, A.h);

// 1) Win+→ → 右半屏贴靠
await sendKey("ArrowRight");
await new Promise((r) => setTimeout(r, 600));
let g = await geom();
check(
  "Win+→ 贴靠右半屏",
  near(g, expectRight),
  `pos=(${g.pos.x},${g.pos.y}) size=(${g.size.w},${g.size.h}) expect=(${expectRight.x},${expectRight.y},${expectRight.w}x${expectRight.h})`
);

// 2) Win+← → 左半屏贴靠（savedRect 不变，仍是 A）
await sendKey("ArrowLeft");
await new Promise((r) => setTimeout(r, 600));
g = await geom();
check(
  "Win+← 贴靠左半屏",
  near(g, expectLeft),
  `pos=(${g.pos.x},${g.pos.y}) size=(${g.size.w},${g.size.h}) expect=(${expectLeft.x},${expectLeft.y},${expectLeft.w}x${expectLeft.h})`
);

// 3) 再次 Win+← → 还原 savedRect（回到手动几何 A）
await sendKey("ArrowLeft");
await new Promise((r) => setTimeout(r, 600));
g = await geom();
check(
  "再次 Win+← 还原 savedRect",
  near(g, A),
  `pos=(${g.pos.x},${g.pos.y}) size=(${g.size.w},${g.size.h}) expect=(${A.x},${A.y},${A.w}x${A.h})`
);

console.log(ok ? "PASS: 贴靠状态机（贴靠×2 + 还原）" : "FAIL");
await finish(ok ? 0 : 1);