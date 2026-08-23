#!/usr/bin/env node
/**
 * 极简 DSH mock 服务：让冒烟测试脱离真实 DSH 服务运行（CI 与本机 mock 模式共用）。
 *
 * 用途：
 * - `GET /`           返回极简 HTML（作为 iframe 加载目标，供窗口居中/隔离/窗口控制场景）
 * - `GET /health`     返回 200（可选：配合状态切换模拟服务在线/离线）
 * - 其余路径          404（壳页只关心可达性 + iframe 加载，无需真实 DSH 路由）
 *
 * 用法:
 *   node scripts/smoke/mock-server.mjs [port]      # 默认 3099（与 3080 区分，避免误连真实服务）
 *   环境变量 MOCK_HTML_FILE 可指定自定义 HTML（默认内置极简页）
 *
 * 注意：
 * - 端口默认 3099：run-all 的 DSH_URL 需与之一致（DSH_URL=http://127.0.0.1:3099）
 * - 该服务只做"可达 + 能加载页面"，不模拟任何 DSH API 语义
 */
import { createServer } from "node:http";
import { readFileSync, existsSync } from "node:fs";

const PORT = Number(process.argv[2] ?? process.env.MOCK_PORT ?? 3099);
const HOST = "127.0.0.1";

const DEFAULT_HTML = `<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <title>DSH Mock</title>
  </head>
  <body style="font-family:system-ui;padding:24px;background:#fff;color:#1f1f1f">
    <h1>DSH Mock Server</h1>
    <p>冒烟测试用 mock 页面（非真实 DSH GUI）</p>
  </body>
</html>`;

function loadHtml() {
  const custom = process.env.MOCK_HTML_FILE;
  if (custom && existsSync(custom)) return readFileSync(custom, "utf8");
  return DEFAULT_HTML;
}

const server = createServer((req, res) => {
  const url = new URL(req.url ?? "/", `http://${HOST}:${PORT}`);
  if (url.pathname === "/") {
    res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    res.end(loadHtml());
    return;
  }
  if (url.pathname === "/health") {
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ ok: true }));
    return;
  }
  res.writeHead(404, { "content-type": "text/plain" });
  res.end("not found");
});

server.listen(PORT, HOST, () => {
  console.log(`[mock-server] listening on http://${HOST}:${PORT}`);
});

// 优雅退出（run-all 用 child 进程拉起时 SIGTERM 结束）
process.on("SIGTERM", () => server.close(() => process.exit(0)));
process.on("SIGINT", () => server.close(() => process.exit(0)));