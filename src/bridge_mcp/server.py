"""JSON-RPC over stdio 协议处理，按 UTF-8 字节读写。"""

import json
import sys

from .tools import HANDLERS, INSTRUCTIONS, tool_list

SERVER_INFO = {"name": "bridge", "version": "1.0.0"}


def handle(req):
    method, params = req.get("method"), req.get("params") or {}
    if method == "initialize":
        return {"protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {}}, "serverInfo": SERVER_INFO, "instructions": INSTRUCTIONS}
    if method == "ping":
        return {}
    if method == "tools/list":
        return {"tools": tool_list()}
    if method == "tools/call":
        fn = HANDLERS.get(params.get("name"))
        if not fn:
            return {"content": [{"type": "text", "text": f"未知工具：{params.get('name')}"}], "isError": True}
        try:
            return {"content": [{"type": "text", "text": fn(params.get("arguments") or {})}]}
        except Exception as e:
            return {"content": [{"type": "text", "text": f"出错：{e}"}], "isError": True}
    raise LookupError(method)


def send(obj):
    sys.stdout.buffer.write(json.dumps(obj, ensure_ascii=False).encode("utf-8") + b"\n")
    sys.stdout.buffer.flush()


def serve():
    for line in sys.stdin.buffer:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line.decode("utf-8"))
        except ValueError:
            send({"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}})
            continue
        if "id" not in req:  # 通知，无需回复
            continue
        try:
            send({"jsonrpc": "2.0", "id": req["id"], "result": handle(req)})
        except LookupError:
            send({"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32601, "message": f"Method not found: {req.get('method')}"}})
        except Exception as e:
            send({"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32603, "message": str(e)}})
