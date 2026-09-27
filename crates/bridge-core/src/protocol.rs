use crate::Bridge;
use serde_json::{json, Value};

pub const INSTRUCTIONS: &str = include_str!("../resources/instructions.txt");
const TOOL_LIST: &str = include_str!("../resources/tools_list.json");

pub fn response(bridge: &Bridge, request: &Value) -> Option<Value> {
    let id = request.get("id")?;
    let method = request["method"].as_str().unwrap_or("None");
    let params = &request["params"];
    let result = match method {
        "initialize" => json!({
            "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
            "serverInfo": {"name": "bridge", "version": env!("CARGO_PKG_VERSION")},
            "capabilities": {"tools": {}}, "instructions": INSTRUCTIONS.replace("\r\n", "\n"),
        }),
        "ping" => json!({}),
        "tools/list" => serde_json::from_str(TOOL_LIST).expect("内置工具清单必须是合法 JSON"),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("None");
            match bridge.tool(name, &params["arguments"]) {
                Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
                Err(error) => {
                    let text = error.to_string();
                    let text = if text.starts_with("未知工具：") {
                        text
                    } else {
                        format!("出错：{text}")
                    };
                    json!({"content": [{"type": "text", "text": text}], "isError": true})
                }
            }
        }
        _ => {
            return Some(json!({"jsonrpc": "2.0", "id": id,
            "error": {"code": -32601, "message": format!("Method not found: {method}")}}))
        }
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

pub fn parse_error() -> Value {
    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}})
}
