//! A Model Context Protocol server over stdio, so any Claude session can read and change Quarterdeck.
//! Started as `<app binary> mcp`. It works on the same database and config folder as the app.

pub mod command_line;
mod ctx;
mod protocol;
mod tool;
mod tools;

pub use ctx::Ctx;
pub use tool::Tool;

use serde_json::Value;

use crate::error::{AppError, AppResult};

/// Every Quarterdeck tool agents get, through the MCP server.
pub fn registry() -> Vec<Tool> {
    tools::all()
}

/// What peers get: every tool, plus the ones only the user's own app may use.
pub fn peer_registry() -> Vec<Tool> {
    tools::all().into_iter().chain(tools::peer_only()).collect()
}

/// Runs one tool by name.
pub async fn call(ctx: &Ctx, tools: &[Tool], name: &str, args: Value) -> AppResult<Value> {
    let tool = tools
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| AppError::BadInput(format!("no tool called {name}")))?;
    (tool.call)(ctx.clone(), args).await
}

use std::io::Write as _;
use std::process::ExitCode;

/// Serves MCP requests from stdin until it closes.
pub fn serve() -> ExitCode {
    tauri::async_runtime::block_on(serve_stdio())
}

async fn serve_stdio() -> ExitCode {
    let Ok(ctx) = Ctx::open().await else {
        return ExitCode::FAILURE;
    };
    let registry = registry();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    // Requests come one at a time and each is answered before the next is read.
    let mut line = String::new();
    loop {
        line.clear();
        match stdin.read_line(&mut line) {
            Ok(0) => return ExitCode::SUCCESS,
            Ok(_) => {}
            Err(_) => return ExitCode::FAILURE,
        }
        if let Some(reply) = protocol::handle(&ctx, &registry, line.trim_end()).await
            && (writeln!(stdout, "{reply}").is_err() || stdout.flush().is_err())
        {
            return ExitCode::FAILURE;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::tool::Tool;
    use super::*;

    async fn session(name: &str) -> (Ctx, Vec<Tool>) {
        let dir =
            std::env::temp_dir().join(format!("quarterdeck-mcp-{name}-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let ctx = Ctx::at(&dir.join("data"), dir.join("config"))
            .await
            .unwrap();
        (ctx, tools::all())
    }

    async fn send(ctx: &Ctx, registry: &[Tool], method: &str, params: &Value) -> Value {
        let line =
            json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string();
        serde_json::from_str(&protocol::handle(ctx, registry, &line).await.unwrap()).unwrap()
    }

    async fn call(ctx: &Ctx, registry: &[Tool], name: &str, args: &Value) -> String {
        let reply = send(
            ctx,
            registry,
            "tools/call",
            &json!({ "name": name, "arguments": args }),
        )
        .await;
        assert_eq!(reply["result"]["isError"], Value::Null, "{reply}");
        reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn a_session_can_list_tools_and_round_trip_an_item() {
        let (ctx, registry) = session("roundtrip").await;
        let init = send(
            &ctx,
            &registry,
            "initialize",
            &json!({ "protocolVersion": "2025-06-18" }),
        )
        .await;
        assert_eq!(init["result"]["serverInfo"]["name"], "quarterdeck");
        let listed = send(&ctx, &registry, "tools/list", &json!({})).await;
        let all = listed["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = all.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(
            names.contains(&"create_item")
                && names.contains(&"add_todo")
                && names.contains(&"pause_all")
        );
        assert!(all.iter().all(|t| t["inputSchema"]["type"] == "object"));
        let id = call(
            &ctx,
            &registry,
            "create_item",
            &json!({ "title": "Fix login", "area": "work", "key": "JD-1" }),
        )
        .await;
        assert_eq!(id, "\"JD-1\"");
        call(
            &ctx,
            &registry,
            "update_item",
            &json!({ "id": "JD-1", "stage": "in_review", "status": "review" }),
        )
        .await;
        let item = call(&ctx, &registry, "get_item", &json!({ "id": "JD-1" })).await;
        assert!(item.contains("in_review") && item.contains("\"kind\": \"stage\""));
    }

    #[tokio::test]
    async fn bad_input_is_a_readable_tool_error() {
        let (ctx, registry) = session("bad-input").await;
        let args = json!({ "title": "x", "area": "life", "due": "friday" });
        let reply = send(
            &ctx,
            &registry,
            "tools/call",
            &json!({ "name": "add_todo", "arguments": args }),
        )
        .await;
        assert_eq!(reply["result"]["isError"], true);
        assert!(
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("YYYY-MM-DD")
        );
    }
}
