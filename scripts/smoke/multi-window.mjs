#!/usr/bin/env node
/**
 * 冒烟 8/8：多会话窗口（Task 14）。
 * 1) 主窗口 invoke open_session_window → 返回 label（session-N）
 * 2) invoke list_session_windows → 包含新 label（Rust 侧窗口清单，托盘同源）
 * 3) CDP target 列表出现第二个壳页（列表层面验证，不注入第二个窗口——
 *    隐藏窗口的 CDP evaluate 不可靠，逐窗口断言改由 Rust 命令覆盖）
 * 4) 清理：close_session_window(label) 关闭会话窗口，恢复单窗口状态
 *    （保证后续场景 findTarget 顺序稳定）
 */
import { evalShell, wait, finish } from "./lib.mjs";

let ok = true;
function check(label, cond, detail) {
  console.log(`${cond ? "PASS" : "FAIL"}: ${label}${detail ? " " + detail : ""}`);
  ok = ok && cond;
}

const invokeInMain = (cmd, args) =>
  evalShell(`window.__TAURI__.core.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args ?? {})})`);

// 1) 经主窗口打开会话窗口
const label = await invokeInMain("open_session_window", { url: null });
check("open_session_window 返回 label", typeof label === "string" && /^session-\d+$/.test(label), `label=${label}`);

// 2) Rust 窗口清单包含新窗口（不依赖第二窗口 CDP 注入）
await wait(1500); // 等窗口初始化（隐藏窗口不响应 CDP evaluate，但注册即时）
const sessions = await invokeInMain("list_session_windows");
check("list_session_windows 包含新会话", Array.isArray(sessions) && sessions.includes(label), JSON.stringify(sessions));

// 3) CDP target 层面出现第二个壳页（列表可见即可）
let targetCount = 1;
for (let i = 0; i < 20; i++) {
  try {
    const ac = new AbortController();
    const to = setTimeout(() => ac.abort(), 3000);
    const list = await (await fetch("http://127.0.0.1:9226/json", { headers: { connection: "close" }, signal: ac.signal })).json();
    clearTimeout(to);
    targetCount = list.filter((t) => t.type === "page" && t.url.includes("index.html")).length;
    if (targetCount >= 2) break;
  } catch (e) { /* CDP 瞬时不可达：重试 */ }
  await wait(500);
}
check("CDP 出现第二个壳页 target", targetCount >= 2, `count=${targetCount}`);

// 4) 清理：关闭会话窗口（恢复单窗口，后续场景 target 顺序稳定）
try {
  const r = await invokeInMain("close_session_window", { label });
  check("close_session_window 清理成功", r === null || r === undefined, String(r));
  await wait(1500);
} catch (e) {
  console.log(`WARN: 会话窗口关闭失败: ${String(e).slice(0, 120)}`);
}
const sessionsAfter = await invokeInMain("list_session_windows");
check("会话窗口已从清单移除", Array.isArray(sessionsAfter) && !sessionsAfter.includes(label), JSON.stringify(sessionsAfter));

console.log(ok ? "PASS: 多会话窗口（创建 + 清单 + 清理）" : "FAIL");
await finish(ok ? 0 : 1);