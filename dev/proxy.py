#!/usr/bin/env python3
"""本地开发代理（python3 版，与 dev/proxy.mjs 行为一致）。

用法：
    RANDOM_ORG_API_KEY=你的key python3 dev/proxy.py
然后另开终端：trunk serve（Trunk 会把 /api/* 转发到本服务，见 Trunk.toml）。

健康检查：curl http://127.0.0.1:8787/healthz
上游请求带 15s 超时；失败会回 502 错误 JSON，前端因此能看到明确报错，
而不是无限停留在「生成中…」。
"""
import http.server
import json
import os
import sys
import urllib.error
import urllib.request

API_KEY = os.environ.get("RANDOM_ORG_API_KEY")
UPSTREAM = "https://api.random.org/json-rpc/4/invoke"
UPSTREAM_TIMEOUT = 15  # 秒


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/healthz":
            return self._json(
                200, {"ok": True, "apiKey": "set" if API_KEY else "missing"}
            )
        return self._json(405, {"error": "method not allowed"})

    def do_POST(self):
        try:
            length = int(self.headers.get("content-length", 0))
            raw = self.rfile.read(length) if length else b""
            payload = json.loads(raw or b"{}")
        except Exception:
            return self._json(400, {"error": "invalid json"})

        params = payload.get("params")
        if not isinstance(params, dict):
            return self._json(400, {"error": "missing params"})
        if not API_KEY:
            return self._json(500, {"error": "server not configured"})

        params["apiKey"] = API_KEY
        request = urllib.request.Request(
            UPSTREAM,
            data=json.dumps(payload).encode("utf-8"),
            headers={"content-type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=UPSTREAM_TIMEOUT) as resp:
                body = resp.read()
                self.send_response(resp.status)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
        except urllib.error.HTTPError as err:
            body = err.read()
            self.send_response(err.code)
            self.send_header("content-type", "application/json")
            self.end_headers()
            self.wfile.write(body)
        except Exception as err:  # noqa: BLE001 - 任何上游错误都回 502
            print(f"[proxy] upstream error: {err}", file=sys.stderr)
            self._json(502, {"error": f"upstream error: {err}"})

    def _json(self, code, obj):
        body = json.dumps(obj).encode("utf-8")
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):  # 静音访问日志
        pass


if __name__ == "__main__":
    print("dev proxy (python) listening on http://127.0.0.1:8787")
    print("apiKey:", "set" if API_KEY else "MISSING (RANDOM_ORG_API_KEY env)")
    http.server.HTTPServer(("127.0.0.1", 8787), Handler).serve_forever()
