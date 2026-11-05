//! Serves this machine's Quarterdeck to peers: `GET /v1/health` and `POST /v1/tools/<name>`, which runs
//! a Quarterdeck tool exactly as the MCP server would. This machine only, or also the local network
//! (`[sync] lan`); never anywhere else. Every request needs the token.

use std::io::Cursor;
use std::net::SocketAddr;

use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, Server};

use super::addresses;
use super::reach::{Listen, allowed};
use crate::error::{AppError, AppResult};
use crate::mcp::{self, Ctx, Tool};

/// Tools are async; this server answers on plain threads, so each call waits for its result.
fn block_on<T>(fut: impl Future<Output = T>) -> T {
    tauri::async_runtime::block_on(fut)
}

type Reply = Response<Cursor<Vec<u8>>>;

fn bind(listen: Listen) -> AppResult<Server> {
    Server::http((listen.host(), listen.port))
        .map_err(|e| AppError::Desktop(format!("sync server: {e}")))
}

/// Starts the server on its own thread for as long as the app runs.
pub fn spawn(listen: Listen) -> AppResult<()> {
    let server = bind(listen)?;
    std::thread::spawn(move || {
        if let Ok(ctx) = block_on(Ctx::open()) {
            serve(&server, &ctx);
        }
    });
    Ok(())
}

/// Serves in the foreground until the process ends, for `<binary> serve` on a headless machine.
pub fn run(listen: Listen) -> AppResult<()> {
    let server = bind(listen)?;
    serve(&server, &block_on(Ctx::open())?);
    Ok(())
}

fn serve(server: &Server, ctx: &Ctx) {
    let tools = mcp::peer_registry();
    for mut request in server.incoming_requests() {
        // Read fresh each time, so a new token (`new_sync_token`) works without a restart.
        let token = ctx.settings().map(|s| s.sync.token).unwrap_or_default();
        let reply = respond(ctx, &tools, &token, &mut request);
        // A peer that hung up doesn't need its answer.
        drop(request.respond(reply));
    }
}

fn json_reply(status: u16, body: &Value) -> Reply {
    let header = Header::from_bytes("Content-Type", "application/json").ok();
    let reply = Response::from_data(body.to_string().into_bytes()).with_status_code(status);
    match header {
        Some(h) => reply.with_header(h),
        None => reply,
    }
}

/// Compares in time independent of where the strings differ.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0_u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

fn authorised(request: &Request, token: &str) -> bool {
    let expected = format!("Bearer {token}");
    !token.is_empty()
        && request
            .headers()
            .iter()
            .any(|h| h.field.equiv("Authorization") && same(h.value.as_str(), &expected))
}

fn respond(ctx: &Ctx, tools: &[Tool], token: &str, request: &mut Request) -> Reply {
    if !allowed(request.remote_addr()) {
        return json_reply(403, &json!({ "error": "only this network may connect" }));
    }
    if !authorised(request, token) {
        return json_reply(401, &json!({ "error": "wrong or missing token" }));
    }
    let path = request.url().to_owned();
    match (request.method(), path.as_str()) {
        (Method::Get, "/v1/health") => health(ctx),
        (Method::Post, p) if p.starts_with("/v1/tools/") => {
            let name = p.trim_start_matches("/v1/tools/").to_owned();
            let seen = request.remote_addr().copied();
            run_tool(ctx, tools, (&name, seen), request)
        }
        _ => json_reply(404, &json!({ "error": "unknown path" })),
    }
}

fn health(ctx: &Ctx) -> Reply {
    match ctx.settings() {
        Ok(s) => json_reply(200, &json!({ "ok": { "node": s.node } })),
        Err(e) => json_reply(500, &json!({ "error": e.to_string() })),
    }
}

/// A `hello` is told where the call came from, whatever the caller said.
fn with_seen(name: &str, seen: Option<SocketAddr>, mut args: Value) -> Value {
    if name != "hello" {
        return args;
    }
    let port = args.get("port").and_then(Value::as_u64);
    let url = seen
        .zip(port.and_then(|p| u16::try_from(p).ok()))
        .map(|(addr, port)| addresses::url_at(addr, port));
    if let Some(fields) = args.as_object_mut() {
        fields.insert("seen".to_owned(), json!(url));
    }
    args
}

fn run_tool(
    ctx: &Ctx,
    tools: &[Tool],
    call: (&str, Option<SocketAddr>),
    request: &mut Request,
) -> Reply {
    let (name, seen) = call;
    let mut body = String::new();
    let parsed = request
        .as_reader()
        .read_to_string(&mut body)
        .map_err(AppError::from)
        .and_then(|_| {
            Ok(if body.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(&body)?
            })
        })
        .map(|args| with_seen(name, seen, args))
        .and_then(|args| block_on(mcp::call(ctx, tools, name, args)));
    match parsed {
        Ok(value) => json_reply(200, &json!({ "ok": value })),
        Err(e) => json_reply(400, &json!({ "error": e.to_string() })),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::config::sync::Peer;
    use crate::peers::client;

    fn peer_on(port: u16, token: &str) -> Peer {
        Peer {
            name: "other".to_owned(),
            url: format!("http://127.0.0.1:{port}"),
            token: token.to_owned(),
        }
    }

    #[test]
    fn peers_round_trip_tools_and_need_the_token() {
        let dir = std::env::temp_dir().join(format!("quarterdeck-peer-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let ctx = block_on(Ctx::at(&dir.join("data"), dir.join("config"))).unwrap();
        crate::config::edit::set_sync_token(&ctx.config, "secret").unwrap();
        let server = Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        std::thread::spawn(move || serve(&server, &ctx));
        let good = peer_on(port, "secret");
        client::call(
            &good,
            "add_todo",
            &json!({ "title": "Make dinner", "area": "life" }),
        )
        .unwrap();
        let todos = client::call(&good, "list_todos", &json!({})).unwrap();
        assert_eq!(todos[0]["title"], "Make dinner");
        assert!(client::health(&peer_on(port, "wrong")).is_err());
        assert!(client::call(&good, "no_such_tool", &json!({})).is_err());
        let hello = json!({ "from": "work-mac", "port": 4791, "seen": "http://forged:1",
                            "urls": ["http://Work-Mac.local:4791"] });
        client::call(&good, "hello", &hello).unwrap();
        let db = block_on(crate::store::Db::open(&dir.join("data"))).unwrap();
        let learned = block_on(addresses::learned(&db, "work-mac")).unwrap();
        assert_eq!(
            learned,
            ["http://127.0.0.1:4791", "http://Work-Mac.local:4791"]
        );
    }
}
