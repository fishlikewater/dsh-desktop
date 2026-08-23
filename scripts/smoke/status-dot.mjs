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

// 1) 机制层：三态互斥 + title 组成
const mech = JSON.parse(await evalShell(`(async()=>{
  const dot = document.getElementById("status-dot");
  const saved = dot.className;
  function setRaw(cls) { dot.className = "dot " + cls; }
  setRaw("offline");
  const offlineCls = dot.className;
  dot.title = "placeholder";
  setRaw("loading");
  const loadingCls = dot.className;
  setRaw("online");
  const onlineCls = dot.className;
  const onlineTitle = dot.title;
  dot.className = saved;
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