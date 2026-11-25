//! JSON-RPC 2.0 handling for the few MCP methods Quarterdeck needs.

use serde::Deserialize;
use serde_json::{Value, json};

use super::ctx::Ctx;
use super::tool::Tool;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_PROTOCOL: &str = "2025-06-18";
const INSTRUCTIONS: &str = "Quarterdeck is the user's calm work-and-life dashboard. Use these tools to \
read what is going on (start with get_overview) and to keep it current: create and update work \
items, raise asks when a decision needs the user, keep every step of the work on its item (even \
the ones only the user can do), add to-dos only for their personal errands, post \
short updates about what you did, and manage recurring schedules. Every item in flight is \
someone's turn: when you take one on, claim_item; when you stop, hand it on with pass_turn. Keep \
state labels under six words. Prefer updating an existing item over creating a duplicate.";

#[derive(Debug, Deserialize)]
struct Request {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

/// Handles one line of input. Notifications get no reply.
pub async fn handle(ctx: &Ctx, tools: &[Tool], line: &str) -> Option<String> {
    let request: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => return Some(error(&Value::Null, -32700, &e.to_string())),
    };
    let id = request.id.clone()?;
    let reply = match request.method.as_str() {
        "initialize" => ok(&id, &initialize(&request.params)),
        "ping" => ok(&id, &json!({})),
        "tools/list" => ok(&id, &list(tools)),
        "tools/call" => ok(&id, &call(ctx, tools, &request.params).await),
        other => error(&id, -32601, &format!("unknown method {other}")),
    };
    Some(reply)
}

fn initialize(params: &Value) -> Value {
    let version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "quarterdeck", "version": SERVER_VERSION },
        "instructions": INSTRUCTIONS,
    })
}

fn list(tools: &[Tool]) -> Value {
    let entries: Vec<Value> = tools
        .iter()
        .map(|t| json!({ "name": t.name, "description": t.description, "inputSchema": t.schema }))
        .collect();
    json!({ "tools": entries })
}

/// Tool failures are results the agent can read and react to, not protocol errors.
async fn call(ctx: &Ctx, tools: &[Tool], params: &Value) -> Value {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let outcome = match tools.iter().find(|t| t.name == name) {
        Some(t) => (t.call)(ctx.clone(), args).await.map_err(|e| e.to_string()),
        None => Err(format!("no tool called {name}")),
    };
    match outcome {
        Ok(value) => json!({ "content": [{ "type": "text", "text": pretty(&value) }] }),
        Err(message) => {
            json!({ "content": [{ "type": "text", "text": message }], "isError": true })
        }
    }
}

fn pretty(value: &Value) -> String {
    if value.is_null() {
        return "done".to_owned();
    }
    serde_json::to_string_pretty(value).unwrap_or_default()
}

fn ok(id: &Value, result: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error(id: &Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}
