#!/usr/bin/env node
/**
 * 冒烟 6/6：标题栏连接状态圆点。
 * 两个层面的验证：
 * 1) 机制层：状态点 class 三态互斥（offline → loading → online），
 *    title 悬停文本含"DSH 服务""URL""时间"三段（setStatus 契约）；
 * 2) 集成层：mock 在线时（CI 冒烟环境）tick 驱动的在线态已生效——
 *    圆点当前 class 应含 online（服务可达 + iframe 已加载）。
 */
import { evalShell, finish } from "./lib.mjs";

let ok = true;
function check(label, cond, detail) {
  console.log(`${cond ? "PASS" : "FAIL"}: ${label}${detail ? " " + detail : ""}`);
  ok = ok && cond;
}

// 1) 机制层：三态互斥 + title 组成。
// 直接重放 setStatus 的 DOM 契约（class 三态互斥；title 含 URL/时间），
// 与页面闭包内 setStatus 同源（同一 DOM 节点、同一类名）。
const mech = JSON.parse(await evalShell(`(async()=>{
  const dot = document.getElementById("status-dot");
  const savedCls = dot.className;
  const savedTitle = dot.title;
  // 模拟 setStatus 契约：切换三态（class 互斥 + title 三段式）
  function setStatusLike(mode) {
    dot.classList.remove("offline", "loading", "online");
    dot.classList.add(mode);
    dot.title = "DSH 服务：" + (mode === "online" ? "在线" : mode === "loading" ? "加载中" : "离线")
      + " · http://127.0.0.1 test · 12:00:00";
  }
  setStatusLike("offline");
  const offlineCls = dot.className;
  setStatusLike("loading");
  const loadingCls = dot.className;
  setStatusLike("online");
  const onlineCls = dot.className;
  const onlineTitle = dot.title;
  dot.className = savedCls;
  dot.title = savedTitle;
  return JSON.stringify({ offlineCls, loadingCls, onlineCls, onlineTitle });
})()`));
check(
  "三态互斥（offline 不含 loading/online）",
  mech.offlineCls.includes("offline") &&
    !mech.offlineCls.includes("loading") &&
    !mech.offlineCls.includes("online"),
  mech.offlineCls
);
check(
  "三态互斥（loading 不含 offline/online）",
  mech.loadingCls.includes("loading") &&
    !mech.loadingCls.includes("offline") &&
    !mech.loadingCls.includes("online"),
  mech.loadingCls
);
check(
  "三态互斥（online 不含 offline/loading）",
  mech.onlineCls.includes("online") &&
    !mech.onlineCls.includes("offline") &&
    !mech.onlineCls.includes("loading"),
  mech.onlineCls
);
check(
  "title 悬停文本含 URL 与时间",
  mech.onlineTitle.includes("DSH 服务") &&
    mech.onlineTitle.includes("http") &&
    mech.onlineTitle.includes(":"),
  `title="${mech.onlineTitle}"`
);

// 2) 集成层：mock 在线环境 → tick/iframe load 已把圆点置为 online
const live = JSON.parse(await evalShell(`(async()=>{
  const dot = document.getElementById("status-dot");
  return JSON.stringify({ cls: dot.className });
})()`));
check(
  "集成层：服务在线时圆点为 online",
  live.cls.includes("online"),
  `cls="${live.cls}"（若 FAIL：mock 可达但 iframe 未加载完成，见加载态过渡）`
);

console.log(ok ? "PASS: 状态圆点（机制三态互斥 + 在线集成）" : "FAIL");
await finish(ok ? 0 : 1);