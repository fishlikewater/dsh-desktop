// CDP 冒烟共享工具：target 发现、连接、求值。
// 前置：debug 构建（additional_browser_args 开启 9226 CDP 端口）且应用已启动。
//
// 注意：使用 `ws` 包而非 Node 全局 WebSocket——Node 24 在 Windows 上对全局
// WebSocket 的关闭存在 libuv 断言崩溃（Assertion failed: UV_HANDLE_CLOSING）。
import WebSocket from "ws";

export const CDP = "http://127.0.0.1:9226/json";

export function wait(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

/** 等待 CDP 可用并返回匹配 target（type + url 关键字） */
export async function findTarget(type, urlKeyword, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      // Connection: close —— 禁用 undici keep-alive，避免退出时残留连接句柄
      const list = await (await fetch(CDP, { headers: { connection: "close" } })).json();
      const t = list.find(
        (x) => x.type === type && (urlKeyword ? x.url.includes(urlKeyword) : true),
      );
      if (t) return t;
    } catch {}
    await wait(500);
  }
  throw new Error(`CDP target 未找到（type=${type}, url~=${urlKeyword}）——确认应用已启动且为 debug 构建`);
}

/** 在 target 上求值（awaitPromise 支持 async 表达式）。
 * 两阶段均带超时：打开的 WebSocket 或永不响应的页面不应挂死冒烟流程。 */
export async function evalIn(target, expression, timeoutMs = 15000) {
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((res, rej) => {
    const t = setTimeout(() => { ws.terminate(); rej(new Error(`evalIn 打开超时（${timeoutMs}ms）`)); }, timeoutMs);
    ws.on("open", () => { clearTimeout(t); res(); });
    ws.on("error", (e) => { clearTimeout(t); rej(e); });
  });
  const result = await new Promise((res, rej) => {
    const t = setTimeout(() => { ws.terminate(); rej(new Error(`evalIn 响应超时（${timeoutMs}ms，target ${target.url}）`)); }, timeoutMs);
    ws.on("message", (data) => {
      const msg = JSON.parse(data.toString());
      if (msg.id === 1) { clearTimeout(t); res(msg); }
    });
    ws.send(JSON.stringify({
      id: 1,
      method: "Runtime.evaluate",
      params: { expression, awaitPromise: true, returnByValue: true },
    }));
  });
  ws.terminate();
  if (result.result?.exceptionDetails) {
    throw new Error(`求值异常: ${JSON.stringify(result.result.exceptionDetails).slice(0, 300)}`);
  }
  return result.result?.result?.value;
}

/** 应用内联脚本执行（壳页上下文）。
 * 多窗口场景下 CDP target 列表顺序不可靠（会话窗口与主窗口 URL 相同，
 * 且隐藏会话窗口不响应 evaluate）——遍历所有 page target，逐个探测
 * `getCurrentWindow().label`，确定是主窗口（main）才执行表达式。 */
export async function evalShell(expression) {
  const list = await (await fetch(CDP, { headers: { connection: "close" } })).json();
  const pages = list.filter((t) => t.type === "page" && t.url.includes("index.html"));
  let lastErr = null;
  for (const t of pages) {
    try {
      const label = await evalIn(t, `window.__TAURI__.window.getCurrentWindow().label`, 3000);
      if (label === "main") {
        return await evalIn(t, expression);
      }
      // 会话窗口：跳过（其几何/状态与主窗口不同，且不响应时超时兜底）
    } catch (e) {
      lastErr = e;
    }
  }
  if (lastErr) throw lastErr;
  throw new Error("主窗口 target 未找到（CDP 列表无 index.html page）");
}

/**
 * 进程退出：优先自然退出（事件循环无残留句柄时 Node 以 exitCode 干净退出，
 * 规避 Node 24 Windows 下 process.exit 与活动句柄的 libuv 断言崩溃）；
 * 1.5s 兜底强制退出，防止残留句柄导致挂起。
 */
export function finish(code) {
  process.exitCode = code;
  setTimeout(() => process.exit(code), 1500).unref();
}
