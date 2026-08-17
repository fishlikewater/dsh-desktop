// 生成自动更新清单 latest.json（签名版，双平台）。
// 由聚合 job（publish-updater-json）在双平台构建完成后运行：
// 从 Release 资产下载各平台签名（.exe.sig / .app.tar.gz.sig），
// 按 tauri-plugin-updater 的清单格式组装并写 latest.json（workflow 随后上传）。
// 背景：签名产物由 tauri build 生成（bundle.createUpdaterArtifacts）并经构建
// job 上传为 Release 资产；本脚本只做读取与组装，不依赖任一 runner 的本地产物。
// 依赖 env：GITHUB_REPOSITORY（owner/repo）、GITHUB_REF_NAME（tag）、GITHUB_TOKEN。
import fs from "node:fs";
import path from "node:path";

const repo = process.env.GITHUB_REPOSITORY ?? "";
if (!/^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/.test(repo)) {
  console.error("GITHUB_REPOSITORY 无效（需 owner/repo 形式）: " + JSON.stringify(repo));
  process.exit(1);
}
const tag = process.env.GITHUB_REF_NAME ?? "";
const token = process.env.GITHUB_TOKEN ?? "";
if (!tag || !token) {
  console.error("GITHUB_REF_NAME 或 GITHUB_TOKEN 未设置");
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

const apiBase = "https://api.github.com/repos/" + repo;
const dlBase = "https://github.com/" + repo + "/releases/download/" + tag + "/";
const headers = {
  Authorization: "Bearer " + token,
  Accept: "application/vnd.github+json",
  "User-Agent": "dsh-desktop-release",
};

async function main() {
  // 注意：/releases/tags/{tag} 对草稿 Release 返回 404（GitHub 行为），
  // 改用列表端点过滤 tag_name（列表包含草稿）。
  const relResp = await fetch(apiBase + "/releases?per_page=100", { headers });
  if (!relResp.ok) {
    console.error("获取 Release 列表失败: " + relResp.status + " " + relResp.statusText);
    process.exit(1);
  }
  const releases = await relResp.json();
  const release = releases.find((r) => r.tag_name === tag);
  if (!release) {
    console.error("未找到 tag 对应的 Release: " + tag);
    process.exit(1);
  }
  const assets = release.assets ?? [];
  const findAsset = (suffix) => assets.find((a) => a.name.endsWith(suffix));

  async function sigContent(asset) {
    const r = await fetch(asset.url, {
      headers: { ...headers, Accept: "application/octet-stream" },
    });
    if (!r.ok) {
      console.error("下载签名失败: " + asset.name + " " + r.status);
      process.exit(1);
    }
    return (await r.text()).trim();
  }

  const platforms = {};
  const exeSig = findAsset(".exe.sig");
  if (exeSig) {
    const exeName = exeSig.name.slice(0, -4);
    platforms["windows-x86_64"] = {
      signature: await sigContent(exeSig),
      url: dlBase + exeName,
    };
  }
  const macSig = findAsset(".app.tar.gz.sig");
  if (macSig) {
    const appName = macSig.name.slice(0, -4);
    platforms["darwin-aarch64"] = {
      signature: await sigContent(macSig),
      url: dlBase + appName,
    };
  }
  if (Object.keys(platforms).length === 0) {
    console.error("Release 无任何 .sig 资产（检查 createUpdaterArtifacts 与签名密钥）");
    process.exit(1);
  }

  const latest = {
    version,
    notes: "",
    pub_date: new Date().toISOString(),
    platforms,
  };
  const outPath = path.join(root, "latest.json");
  fs.writeFileSync(outPath, JSON.stringify(latest, null, 2) + "\n", "utf8");
  console.log("latest.json 已生成: " + outPath);
  console.log("  platforms=" + Object.keys(platforms).join(", "));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
