#!/usr/bin/env node
/**
 * 冒烟 3/5：隔离边界。
 * 验证安全模型：壳页 __TAURI__ 可用；iframe（DSH GUI 远程页面）中
 * __TAURI__ 对象存在（WebView2 注入机制）但其 IPC 调用全部被拒
 * （capabilities 按 URL 粒度匹配，iframe URL 非 local）。
 */
import { findTarget, evalIn, finish } from "./lib.mjs";

const shell = await findTarget("page", "index.html");
const frame = await findTarget("iframe", null, 10000);

const shellTauri = await evalIn(shell, "typeof window.__TAURI__");
console.log("SHELL __TAURI__:", shellTauri);

if (!frame) {
  console.error("FAIL: iframe target 未找到（DSH GUI 未加载？）");
  await finish(1);
}
const frameTauri = await evalIn(frame, "typeof window.__TAURI__");
console.log("IFRAME __TAURI__ 对象:", frameTauri);

const frameIpc = JSON.parse(await evalIn(frame, `(async()=>{
  const out = {};
  try {
    const win = window.__TAURI__.window.getCurrentWindow();
    await win.outerSize();
    out.outerSize = 'ALLOWED';
  } catch(e) { out.outerSize = 'DENIED'; }
  try {
    await window.__TAURI__.core.invoke("theme_preference");
    out.invoke = 'ALLOWED';
  } catch(e) { out.invoke = 'DENIED'; }
  return JSON.stringify(out);
})()`));
console.log("IFRAME IPC:", JSON.stringify(frameIpc));

const ok = shellTauri === "object" && frameTauri === "object"
  && frameIpc.outerSize === "DENIED" && frameIpc.invoke === "DENIED";
console.log(ok ? "PASS: 壳页可用、iframe 对象存在但 IPC 全部被拒（能力层隔离）" : "FAIL");
process.exit(ok ? 0 : 1);

