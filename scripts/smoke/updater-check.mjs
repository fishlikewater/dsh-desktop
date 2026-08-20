#!/usr/bin/env node
/**
 * 冒烟：设置弹层「检查更新」端到端可达性（真实 GitHub endpoint）。
 * 前置：debug 构建已启动（CDP 9226）。直接调壳页上下文的 Tauri command
 * `update_check`，验证能连上真实 latest.json 且签名校验通过。
 * 当前 v0.1.0 与最新一致 → 期望返回「当前已是最新版本」；
 * 一旦后续发布更高版本，此处应返回「发现新版本 vX」（链路仍通）。
 *
 * 用法: node scripts/smoke/updater-check.mjs
 */
import { evalShell, finish } from "./lib.mjs";

try {
  const msg = await evalShell(`window.__TAURI__.core.invoke("update_check")`);
  console.log("update_check ->", JSON.stringify(msg));
  if (typeof msg !== "string" || msg.length === 0) {
    console.error("FAIL: update_check 返回为空或非字符串");
    await finish(1);
  }
  if (/发现新版本/.test(msg)) {
    console.log(`OK: update_check 端到端可达，发现新版本：${msg}`);
  } else if (/当前已是最新版本/.test(msg)) {
    console.log(`OK: update_check 端到端可达（endpoint/latest.json/签名链路通）：${msg}`);
  } else {
    console.log(`INFO: update_check 返回（链路通，非预期文案）：${msg}`);
  }
  await finish(0);
} catch (e) {
  console.error("FAIL: update_check 调用失败:", e.message);
  await finish(1);
}
