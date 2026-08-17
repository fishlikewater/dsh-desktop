// 生成自动更新清单 latest.json（签名版）。
// CI（release.yml Windows job）在 tauri build 之后运行：读取 tauri CLI 生成的
// .exe.sig 签名文件，按 tauri-plugin-updater 的清单格式组装 latest.json。
// 背景：tauri-action 的 updater JSON 上传路径要求 .sig 已是 Release 资产，
// 而其上传环节不包含 .sig（实测），故在 workflow 侧自行完成签名上传与清单发布。
// 用法（release.yml）：node scripts/build-latest-json.mjs
// 依赖 env：GITHUB_REPOSITORY（owner/repo）、GITHUB_REF_NAME（tag，如 v0.1.0）
import fs from "node:fs";
import path from "node:path";

const repo = process.env.GITHUB_REPOSITORY ?? "";
if (!/^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/.test(repo)) {
  console.error("GITHUB_REPOSITORY 无效（需 owner/repo 形式）: " + JSON.stringify(repo));
  process.exit(1);
}
const tag = process.env.GITHUB_REF_NAME ?? "";
if (!tag) {
  console.error("GITHUB_REF_NAME 未设置");
  process.exit(1);
}

const root = process.cwd();
const conf = JSON.parse(
  fs.readFileSync(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"),
);
const version = conf.version;
if (!version) {
  console.error("tauri.conf.json 缺少 version");
  process.exit(1);
}

const nsisDir = path.join(root, "src-tauri", "target", "release", "bundle", "nsis");
const sigFiles = fs.existsSync(nsisDir)
  ? fs.readdirSync(nsisDir).filter((f) => f.endsWith(".exe.sig"))
  : [];
if (sigFiles.length === 0) {
  console.error("未找到 .exe.sig（检查 TAURI_SIGNING_PRIVATE_KEY secret 是否配置正确）");
  process.exit(1);
}
const sigFile = path.join(nsisDir, sigFiles[0]);
const signature = fs.readFileSync(sigFile, "utf8").trim();
if (!signature) {
  console.error("签名文件内容为空: " + sigFile);
  process.exit(1);
}
// 对应 exe 文件名（GitHub 资产名会把空格规范化为点，如 DSH Desktop → DSH.Desktop）
const exeName = sigFiles[0].replace(/\.exe\.sig$/, ".exe").replace(/ /g, ".");
const url =
  "https://github.com/" + repo + "/releases/download/" + tag + "/" + exeName;

const latest = {
  version,
  notes: "",
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": { signature, url },
  },
};
const outPath = path.join(root, "latest.json");
fs.writeFileSync(outPath, JSON.stringify(latest, null, 2) + "\n", "utf8");
console.log("latest.json 已生成: " + outPath);
console.log("  version=" + version + " url=" + url);
