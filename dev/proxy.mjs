// 本地开发代理 —— 与线上 Cloudflare Pages Function 行为一致。
//
// 用法：
//   RANDOM_ORG_API_KEY=你的key node dev/proxy.mjs
// 然后另开终端：trunk serve（Trunk 会把 /api/* 转发到本服务，见 Trunk.toml）。
//
// 健康检查：curl http://127.0.0.1:8787/healthz
// 上游请求带 15s 超时；失败会回 502 错误 JSON，前端因此能看到明确报错，
// 而不是无限停留在「生成中…」。
import http from "node:http";

const API_KEY = process.env.RANDOM_ORG_API_KEY;
const UPSTREAM = "https://api.random.org/json-rpc/4/invoke";
const UPSTREAM_TIMEOUT_MS = 15_000;

const server = http.createServer(async (req, res) => {
  if (req.method === "GET" && req.url === "/healthz") {
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ ok: true, apiKey: API_KEY ? "set" : "missing" }));
    return;
  }
  if (req.method !== "POST") {
    res.writeHead(405, { "content-type": "application/json" });
    res.end(JSON.stringify({ error: "method not allowed" }));
    return;
  }

  let raw = "";
  for await (const chunk of req) raw += chunk;

  let body;
  try {
    body = JSON.parse(raw);
  } catch {
    res.writeHead(400, { "content-type": "application/json" });
    res.end(JSON.stringify({ error: "invalid json" }));
    return;
  }
  if (
    !body ||
    typeof body !== "object" ||
    typeof body.params !== "object" ||
    body.params === null
  ) {
    res.writeHead(400, { "content-type": "application/json" });
    res.end(JSON.stringify({ error: "missing params" }));
    return;
  }
  if (!API_KEY) {
    res.writeHead(500, { "content-type": "application/json" });
    res.end(JSON.stringify({ error: "server not configured" }));
    return;
  }

  body.params.apiKey = API_KEY;

  try {
    const upstream = await fetch(UPSTREAM, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
    });

    const text = await upstream.text();
    res.writeHead(upstream.status, { "content-type": "application/json" });
    res.end(text);
  } catch (err) {
    console.error("[proxy] upstream error:", err?.message ?? err);
    res.writeHead(502, { "content-type": "application/json" });
    res.end(JSON.stringify({ error: "upstream error: " + (err?.message ?? String(err)) }));
  }
});

server.listen(8787, () => {
  console.log("dev proxy listening on http://127.0.0.1:8787");
  console.log("apiKey:", API_KEY ? "set" : "MISSING (RANDOM_ORG_API_KEY env)");
});
