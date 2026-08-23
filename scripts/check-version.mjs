#!/usr/bin/env node
/**
 * 版本一致性检查：package.json / tauri.conf.json / Cargo.toml / CHANGELOG.md
 * 最新版本必须一致。版本漂移会在发版前（npm run check）暴露，而不是发版后。
 */
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const changelog = readFileSync("CHANGELOG.md", "utf8");

const pkgVersion = pkg.version;
const confVersion = conf.version;
const cargoVersion = (cargo.match(/^version\s*=\s*"([^"]+)"/m) ?? [])[1];
// CHANGELOG 最新已发布版本（首个 ## [x.y.z] 标题，排除 Unreleased）
const changelogVersion = (changelog.match(/^## \[(\d+\.\d+\.\d+)\]/m) ?? [])[1];

const entries = [
  ["package.json", pkgVersion],
  ["tauri.conf.json", confVersion],
  ["Cargo.toml", cargoVersion],
  ["CHANGELOG.md", changelogVersion],
];
const missing = entries.filter(([, v]) => !v);
if (missing.length > 0) {
  console.error(`FAIL: 无法读取版本 -> ${missing.map(([f]) => f).join(", ")}`);
  process.exit(1);
}
const unique = new Set(entries.map(([, v]) => v));
if (unique.size !== 1) {
  console.error("FAIL: 版本不一致：");
  for (const [file, v] of entries) console.error(`  ${file}: ${v}`);
  console.error("发版时需同步递增四处版本（见 docs/release-process.md）");
  process.exit(1);
}
console.log(`OK: 四处版本一致（${confVersion}）`);
