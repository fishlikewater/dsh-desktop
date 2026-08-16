#!/usr/bin/env node
/**
 * 冒烟 1/5：主题即时同步。
 * 修改 ~/.dsh/settings.yaml 的 ui-theme.preference（light↔dark），
 * 验证标题栏在 2s 内跟随（事件驱动，实测 ~100ms；旧轮询为 0~3s）。
 *
 * 用法: node scripts/smoke/theme-sync.mjs [dark|light|system]
 * 注意: 会临时修改 settings.yaml（保留其余内容），结束后恢复为 light 基线。
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { evalShell, finish } from "./lib.mjs";

const SETTINGS = `${process.env.USERPROFILE}/.dsh/settings.yaml`;

// 备份原文件（首次运行时）
const backup = SETTINGS + ".bak-smoke";
if (!existsSync(backup)) writeFileSync(backup, readFileSync(SETTINGS, "utf8"));

/** 设置偏好（保留文件其余内容，替换/追加 ui-theme 段） */
function setPreference(mode) {
  let text = readFileSync(SETTINGS, "utf8");
  const lines = text.split("\n");
  const idx = lines.findIndex((l) => l.trim().startsWith("ui-theme:"));
  if (idx >= 0) {
    let end = idx + 1;
    while (end < lines.length && /^\s/.test(lines[end])) end++;
    lines.splice(idx, end - idx, `ui-theme:\n  preference: ${mode}`);
    text = lines.join("\n");
  } else {
    text = text.trimEnd() + `\nui-theme:\n  preference: ${mode}\n`;
  }
  writeFileSync(SETTINGS, text, "utf8");
}

async function snapshot() {
  return JSON.parse(await evalShell(`JSON.stringify({
    cls: document.body.className,
    bg: getComputedStyle(document.getElementById("titlebar")).backgroundColor,
    titleFg: getComputedStyle(document.getElementById("app-title")).color
  })`));
}

const want = process.argv[2] ?? "dark";

setPreference("light");
await new Promise((r) => setTimeout(r, 800));
const baseline = await snapshot();
if (!baseline.cls.includes("theme-light")) {
  console.error(`FAIL: 基线应为 light，实际 ${baseline.cls}`);
  await finish(1);
}
console.log("baseline(light):", JSON.stringify(baseline));

const t0 = Date.now();
setPreference(want);
let got = null;
for (let i = 0; i < 40; i++) {
  await new Promise((r) => setTimeout(r, 100));
  const s = await snapshot();
  if (s.cls.includes(want === "dark" ? "theme-dark" : "theme-light")) { got = s; break; }
}
const elapsed = Date.now() - t0;
if (!got) {
  console.error(`FAIL: 标题栏 ${elapsed}ms 内未切换到 ${want}`);
  await finish(1);
}
console.log(`OK: 标题栏 ${elapsed}ms 内切换为 ${want} ->`, JSON.stringify(got));
if (elapsed > 2000) {
  console.error("FAIL: 超过 2s（疑似仍走轮询路径）");
  await finish(1);
}

// 恢复 light 基线
setPreference("light");
let restored = false;
for (let i = 0; i < 40; i++) {
  await new Promise((r) => setTimeout(r, 100));
  const s = await snapshot();
  if (s.cls.includes("theme-light") && !s.cls.includes("theme-dark")) {
    restored = true;
    break;
  }
}
if (!restored) {
  console.error("FAIL: 恢复 light 基线超时");
  await finish(1);
}
console.log("OK: 已恢复 light 基线");
await finish(0);

