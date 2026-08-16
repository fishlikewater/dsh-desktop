#!/usr/bin/env node
/**
 * 壳页静态检查（frontend-dist/index.html）：
 * 1. 提取内联 <script> 做 Node 语法检查（node --check）
 * 2. JS 中 getElementById/querySelector("#...") 引用的 id 必须存在于 HTML
 */
import { readFileSync, writeFileSync, unlinkSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HTML_PATH = "frontend-dist/index.html";
const html = readFileSync(HTML_PATH, "utf8");

// 1. JS 语法检查
const m = html.match(/<script>([\s\S]*?)<\/script>/);
if (!m) {
  console.error(`FAIL: ${HTML_PATH} 未找到内联 <script>`);
  process.exit(1);
}
const js = m[1];
const tmp = join(tmpdir(), `dsh-check-${process.pid}.js`);
writeFileSync(tmp, js, "utf8");
try {
  execFileSync(process.execPath, ["--check", tmp], { stdio: "inherit" });
  console.log("OK: 内联 script 语法通过");
} finally {
  try { unlinkSync(tmp); } catch {}
}

// 2. id 一致性
const idsInHtml = new Set(
  [...html.matchAll(/id="([^"]+)"/g)].map((x) => x[1]),
);
const jsRefs = new Set(
  [
    ...[...js.matchAll(/getElementById\("([^"]+)"\)/g)],
    ...[...js.matchAll(/querySelector\("#([^"]+)"\)/g)],
  ].map((x) => x[1]),
);
const missing = [...jsRefs].filter((id) => !idsInHtml.has(id));
if (missing.length > 0) {
  console.error(`FAIL: JS 引用了不存在的 id -> ${missing.join(", ")}`);
  process.exit(1);
}
console.log(`OK: ${jsRefs.size} 个 JS 引用的 id 均存在于 HTML`);
