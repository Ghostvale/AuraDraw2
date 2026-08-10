// Cloudflare Pages Function —— 同源端点 /api/random
//
// 前端把不带 apiKey 的 JSON-RPC 请求 POST 到这里，本函数注入
// 环境变量 RANDOM_ORG_API_KEY 后转发给 RANDOM.ORG Signed API，
// 响应（含 signature）原样透传。
//
// 部署：本文件位于仓库 functions/api/random.js，
// Cloudflare Pages 构建输出 dist/，环境变量在平台侧配置。

export async function onRequestPost(context) {
  const { request, env } = context;

  let body;
  try {
    body = await request.json();
  } catch {
    return json({ error: "invalid json" }, 400);
  }
  if (
    !body ||
    typeof body !== "object" ||
    typeof body.params !== "object" ||
    body.params === null
  ) {
    return json({ error: "missing params" }, 400);
  }
  if (!env.RANDOM_ORG_API_KEY) {
    return json({ error: "server not configured" }, 500);
  }

  body.params.apiKey = env.RANDOM_ORG_API_KEY;

  const upstream = await fetch("https://api.random.org/json-rpc/4/invoke", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });

  const text = await upstream.text();
  return new Response(text, {
    status: upstream.status,
    headers: {
      "content-type": "application/json",
      "cache-control": "no-store",
    },
  });
}

function json(obj, status) {
  return new Response(JSON.stringify(obj), {
    status,
    headers: { "content-type": "application/json" },
  });
}
