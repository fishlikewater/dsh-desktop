#!/usr/bin/env node
/**
 * CI 发布前置：把 tauri.conf.json 中 updater endpoints 的占位 OWNER/REPO
 * 替换为真实 GitHub 仓库（GITHUB_REPOSITORY env，形如 owner/repo）。
 * 仅 CI 构建前调用；本地构建不设该 env、无需替换。幂等：重复运行不产生变化。
 */
import { readFileSync, writeFileSync } from "node:fs";

const CONFIG_PATH = "src-tauri/tauri.conf.json";
const PLACEHOLDER = "OWNER/REPO";
const REPO_PATTERN = /^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/;

const repo = process.env.GITHUB_REPOSITORY?.trim();
if (!repo) {
  console.error("FAIL: 未设置 GITHUB_REPOSITORY（形如 owner/repo）。本地构建无需替换。");
  process.exit(1);
}
if (!REPO_PATTERN.test(repo)) {
  console.error(`FAIL: GITHUB_REPOSITORY 格式非法：${repo}`);
  process.exit(1);
}

let conf;
try {
  conf = JSON.parse(readFileSync(CONFIG_PATH, "utf8"));
} catch (err) {
  console.error(`FAIL: ${CONFIG_PATH} 不是合法 JSON：${err.message}`);
  process.exit(1);
}

const endpoints = conf.plugins?.updater?.endpoints;
if (!Array.isArray(endpoints)) {
  console.error(`FAIL: ${CONFIG_PATH} 缺少 plugins.updater.endpoints`);
  process.exit(1);
}

let replaced = 0;
for (let i = 0; i < endpoints.length; i++) {
  const before = endpoints[i];
  const after = typeof before === "string" ? before.replaceAll(PLACEHOLDER, repo) : before;
  if (after !== before) {
    endpoints[i] = after;
    replaced++;
  }
}

writeFileSync(CONFIG_PATH, JSON.stringify(conf, null, 2) + "\n", "utf8");
console.log(`OK: 已替换 ${replaced} 处 endpoint 为 ${repo}${replaced === 0 ? "（幂等，无变化）" : ""}`);
