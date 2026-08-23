#!/usr/bin/env node
/**
 * 冒烟 8/8：多会话窗口（Task 14）——场景列在最后（见 run-all 顺序说明）。
 * Windows WebView2 创建第二个 WebView 后主窗口 CDP evaluate 会失效
 * （多 WebView 调试 session 平台限制，局限见 docs/testing.md），
 * 因此本场景断言尽量走「Rust 命令 + CDP 列表计数」：
 * 1) open 前：list_session_windows 为空（基线）
 * 2) open_session_window → label（主窗口 evaluate 此时仍可用——前置
 *    场景已证明，且尚无第二个 WebView）
 * 3) CDP 列表 page target >= 2（纯 HTTP 计数，不依赖 evaluate）——
 *    第二 WebView 真实存在的核心证据
 * 4) 尽力清理：主窗口 evaluate 若仍存活则 close_session_window + 断言
 *    清单移除；挂起则 WARN（平台限制已知，不阻塞结论）
 */
import { evalShell, wait, finish } from "./lib.mjs";

let ok = true;
function check(label, cond, detail) {
  console.log(`${cond ? "PASS" : "FAIL"}: ${label}${detail ? " " + detail : ""}`);
  ok = ok && cond;
}

const invokeMain = (cmd, args) =>
  evalShell(`window.__TAURI__.core.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args ?? {})})`, 8000);

// 1) 基线：无会话窗口
const before = await invokeMain("list_session_windows");
check("基线无会话窗口", Array.isArray(before) && before.length === 0, JSON.stringify(before));

// 2) 打开会话窗口（主窗口 evaluate 在单 WebView 阶段可靠）
const label = await invokeMain("open_session_window", { url: null });
check("open_session_window 返回 label", typeof label === "string" && /^session-\d+$/.test(label), `label=${label}`);

// 3) CDP 列表计数（HTTP 层面证明第二 WebView 存在）
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
check("CDP 出现第二个壳页 target（第二 WebView 存在）", targetCount >= 2, `count=${targetCount}`);

// 4) 尽力清理（主窗口 CDP 可能已失效——平台限制，WARN 不 FAIL）
try {
  const sessions = await invokeMain("list_session_windows");
  check("list_session_windows 含新会话", Array.isArray(sessions) && sessions.includes(label), JSON.stringify(sessions ?? "ERR"));
  const r = await invokeMain("close_session_window", { label });
  await wait(1200);
  const after = await invokeMain("list_session_windows");
  check("close 后清单移除", Array.isArray(after) && !after.includes(label), JSON.stringify(after ?? "ERR"));
  check("close 返回值", r === null || r === undefined, String(r));
} catch (e) {
  console.log(`WARN: 主窗口 evaluate 失效（WebView2 多 WebView 平台限制），清理跳过: ${String(e).slice(0, 100)}`);
  // 结论仍成立：open 返回 label + CDP 计数证明第二窗口；清理交给
  // pwsh Stop-Process（CI 冒烟结束杀进程）
}

console.log(ok ? "PASS: 多会话窗口（命令 + CDP 计数）" : "FAIL");
await finish(ok ? 0 : 1);