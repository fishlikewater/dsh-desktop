#!/usr/bin/env node
/**
 * 冒烟 8/8：多会话窗口（Task 14）。
 * 1) 主窗口 invoke open_session_window → 返回 label（session-N）
 * 2) CDP target 列表出现第二个 page；分别 evalIn `getCurrentWindow().label`
 *    区分两窗口：一个 "main"、一个 "session-*"
 * 3) 会话窗口独立上下文可用（独立检测循环/URL 由壳页通用逻辑天然保证）
 */
import { evalShell, evalIn, wait, finish } from "./lib.mjs";

let ok = true;
function check(label, cond, detail) {
  console.log(`${cond ? "PASS" : "FAIL"}: ${label}${detail ? " " + detail : ""}`);
  ok = ok && cond;
}

// 1) 经主窗口打开会话窗口
const label = await evalShell(`(async()=>{
  const invoke = window.__TAURI__.core.invoke;
  return await invoke("open_session_window", { url: null });
})()`);
check("open_session_window 返回 label", typeof label === "string" && /^session-\d+$/.test(label), `label=${label}`);

// 2) 等待 CDP 出现第二个 page target（index.html 壳页）
let targets = [];
for (let i = 0; i < 20; i++) {
  try {
    const ac = new AbortController();
    const to = setTimeout(() => ac.abort(), 3000);
    const list = await (await fetch("http://127.0.0.1:9226/json", { headers: { connection: "close" }, signal: ac.signal })).json();
    clearTimeout(to);
    targets = list.filter((t) => t.type === "page" && t.url.includes("index.html"));
    if (targets.length >= 2) break;
  } catch (e) {
    /* CDP 瞬时不可达：重试 */
  }
  await wait(500);
}
check("CDP 出现两个壳页 target", targets.length >= 2, `count=${targets.length}`);

// 3) 逐 target evalIn label，收集窗口 label 集合
const labels = [];
for (const t of targets.slice(0, 4)) {
  try {
    const l = await evalIn(t, `window.__TAURI__.window.getCurrentWindow().label`);
    labels.push(l);
  } catch (e) {
    labels.push("ERR:" + String(e).slice(0, 40));
  }
}
check("两窗口 label 可区分", labels.includes("main"), JSON.stringify(labels));
const sessionLabels = labels.filter((l) => /^session-\d+$/.test(l));
check("会话窗口 label=session-N", sessionLabels.length >= 1 && labels.includes(label), JSON.stringify(labels));

// 4) 会话窗口与主窗口独立：会话窗口内可独立求值（不含主窗口专属引用）
if (sessionLabels.length >= 1) {
  const idx = labels.indexOf(label);
  const t = targets[idx >= 0 ? idx : 1];
  const v = await evalIn(t, `(async()=>{
    const url = document.getElementById("dsh-frame") ? "shell-ok" : "shell-missing";
    return url;
  })()`);
  check("会话窗口壳页独立可用", v === "shell-ok", `v=${v}`);
}

// 5) 清理：关闭会话窗口（触发 CloseRequested → 销毁），恢复单窗口状态，
//    保证后续场景 findTarget 顺序稳定
if (sessionLabels.length >= 1) {
  const idx = labels.indexOf(label);
  const t = targets[idx >= 0 ? idx : 1];
  try {
    await evalIn(t, `window.__TAURI__.window.getCurrentWindow().close()`, 8000);
    await wait(1000); // 等窗口销毁
    console.log("会话窗口已关闭（场景清理）");
  } catch (e) {
    console.log(`WARN: 会话窗口关闭失败（不影响主断言结果）: ${String(e).slice(0, 80)}`);
  }
}

console.log(ok ? "PASS: 多会话窗口（独立 target + label 区分）" : "FAIL");
await finish(ok ? 0 : 1);