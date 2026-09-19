//! The bridge: MCP methods in, protocol ops out. Dual era (ARCHITECTURE.md §9.2; MCP
//! 2026-07-28 "Versioning and Compatibility"): a request carrying the modern per-request
//! `_meta` is served statelessly under that revision; an `initialize` request selects the
//! legacy handshake for the life of the process. Both eras share `tools/list` and `tools/call`.

use crate::backend::Backend;
use crate::rpc::{self, Request};
use crate::{base64, tools};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Legacy revisions accepted by `initialize`, newest first; the first is offered when the
/// client asks for one we do not know.
pub const LEGACY_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
/// Modern revisions accepted per request.
pub const MODERN_VERSIONS: [&str; 1] = ["2026-07-28"];
/// The `_meta` key carrying a modern request's protocol version.
pub const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
/// The `_meta` key carrying a modern request's client capabilities.
pub const META_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
/// The `_meta` key a modern result carries the server identity under.
pub const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";
/// `UnsupportedProtocolVersionError`.
pub const UNSUPPORTED_VERSION: i64 = -32022;

const INSTRUCTIONS: &str = "Omnis is a turn-based first-person grid crawler. sim_command steps the party; map_text and viewport_get show where it is; screenshot returns the canvas when the game window is running.";

/// An error line for a given request id.
type ErrorLine = Box<dyn Fn(&Value) -> String>;

/// Which handshake a request used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Era {
    /// Nothing seen yet.
    Unknown,
    /// `initialize`, or a request without modern `_meta` after it.
    Legacy,
    /// Per-request `_meta` with a protocol version.
    Modern,
}

/// The bridge state.
pub struct Bridge {
    backend: Backend,
    /// The era of the last request, for the log.
    pub era: Era,
    /// In game mode, the directory the game's relative paths are resolved against, so a
    /// screenshot's file can be read back as image content; `None` when headless.
    root: Option<PathBuf>,
}

fn server_info() -> Value {
    json!({"name": "omnis-mcp", "version": env!("CARGO_PKG_VERSION")})
}

fn capabilities() -> Value {
    json!({"tools": {"listChanged": false}})
}

impl Bridge {
    /// A bridge over a backend; `root` is where the game runs, for reading files it writes.
    #[must_use]
    pub fn new(backend: Backend, root: PathBuf) -> Bridge {
        let root = matches!(backend, Backend::Game(_)).then_some(root);
        Bridge {
            backend,
            era: Era::Unknown,
            root,
        }
    }

    /// Handle one inbound line: the response line, or `None` for a notification.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let request = match rpc::parse(line) {
            Ok(request) => request,
            Err((id, code, message)) => return Some(rpc::error(&id, code, &message)),
        };
        let Some(id) = request.id.clone() else {
            self.notification(&request);
            return None;
        };
        let modern = match self.modern_version(&request) {
            Ok(version) => version,
            Err(line) => return Some(line(&id)),
        };
        let outcome = if modern.is_some() {
            self.era = Era::Modern;
            self.modern(&request)
        } else {
            self.legacy(&request)
        };
        Some(match outcome {
            Ok(mut result) => {
                if modern.is_some() {
                    result["resultType"] = json!("complete");
                    result["_meta"] = json!({META_SERVER_INFO: server_info()});
                }
                rpc::result(&id, result)
            }
            Err((code, message)) => rpc::error(&id, code, &message),
        })
    }

    /// The modern protocol version a request carries, `None` for a legacy request, or the
    /// error line to answer with.
    fn modern_version(&self, request: &Request) -> Result<Option<String>, ErrorLine> {
        let meta = &request.params["_meta"];
        let Some(version) = meta[META_VERSION].as_str() else {
            return Ok(None);
        };
        if !MODERN_VERSIONS.contains(&version) {
            let requested = version.to_owned();
            return Err(Box::new(move |id| {
                rpc::error_data(
                    id,
                    UNSUPPORTED_VERSION,
                    "Unsupported protocol version",
                    json!({"supported": MODERN_VERSIONS, "requested": requested}),
                )
            }));
        }
        if meta.get(META_CLIENT_CAPABILITIES).is_none() {
            return Err(Box::new(|id| {
                rpc::error(
                    id,
                    rpc::INVALID_PARAMS,
                    "_meta lacks io.modelcontextprotocol/clientCapabilities",
                )
            }));
        }
        Ok(Some(version.to_owned()))
    }

    fn notification(&mut self, request: &Request) {
        match request.method.as_str() {
            "notifications/initialized" => eprintln!("omnis-mcp: legacy client initialized"),
            other => eprintln!("omnis-mcp: notification {other} ignored"),
        }
    }

    fn modern(&mut self, request: &Request) -> Result<Value, (i64, String)> {
        match request.method.as_str() {
            "server/discover" => Ok(json!({
                "supportedVersions": MODERN_VERSIONS,
                "capabilities": capabilities(),
                "instructions": INSTRUCTIONS,
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(tools::list()),
            "tools/call" => self.tools_call(&request.params),
            other => Err((rpc::METHOD_NOT_FOUND, format!("unknown method '{other}'"))),
        }
    }

    fn legacy(&mut self, request: &Request) -> Result<Value, (i64, String)> {
        match request.method.as_str() {
            "initialize" => {
                self.era = Era::Legacy;
                let asked = request.params["protocolVersion"].as_str().unwrap_or("");
                let version = if LEGACY_VERSIONS.contains(&asked) {
                    asked
                } else {
                    LEGACY_VERSIONS[0]
                };
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": capabilities(),
                    "serverInfo": server_info(),
                    "instructions": INSTRUCTIONS,
                }))
            }
            "server/discover" => Err((
                rpc::INVALID_PARAMS,
                format!(
                    "server/discover needs _meta with {META_VERSION}; supported: {MODERN_VERSIONS:?}"
                ),
            )),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(tools::list()),
            "tools/call" => self.tools_call(&request.params),
            other => Err((rpc::METHOD_NOT_FOUND, format!("unknown method '{other}'"))),
        }
    }

    fn tools_call(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params["name"]
            .as_str()
            .ok_or((rpc::INVALID_PARAMS, "tools/call needs a name".to_owned()))?;
        let tool =
            tools::find(name).ok_or((rpc::INVALID_PARAMS, format!("Unknown tool: {name}")))?;
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
        if !arguments.is_object() {
            return Err((rpc::INVALID_PARAMS, "arguments must be an object".into()));
        }
        let request = if tool.takes_args {
            json!({"op": tool.op, "args": arguments})
        } else {
            json!({"op": tool.op})
        };
        Ok(match self.backend.call(&request) {
            Ok(value) => self.success(&tool, value),
            Err(error) => {
                let message = error["message"]
                    .as_str()
                    .map_or_else(|| error.to_string(), str::to_owned);
                let kind = error["kind"].as_str().unwrap_or("Failed");
                json!({
                    "content": [{"type": "text", "text": format!("{kind}: {message}")}],
                    "isError": true,
                    "structuredContent": error
                })
            }
        })
    }

    fn success(&self, tool: &tools::Tool, value: Value) -> Value {
        let value = compact_tiles(value);
        let mut content = Vec::new();
        match tool.op {
            "map.text" => content.push(json!({"type": "text", "text": value["text"].as_str().unwrap_or("")})),
            "screenshot" if self.root.is_some() => {
                let path = value["path"].as_str().unwrap_or("");
                let file = self.root.as_deref().map_or_else(|| PathBuf::from(path), |r| r.join(path));
                match std::fs::read(&file) {
                    Ok(bytes) => content.push(json!({"type": "image", "data": base64::encode(&bytes), "mimeType": "image/png"})),
                    Err(e) => content.push(json!({"type": "text", "text": format!("{}: {e}", file.display())})),
                }
                content.push(json!({"type": "text", "text": format!("saved to {path}")}));
            }
            _ => content.push(json!({"type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default()})),
        }
        json!({"content": content, "isError": false, "structuredContent": value})
    }
}

/// Replace every `Visible` event with its tile count and every `Sensed` event's tile list
/// with its count: a script of twenty steps on the meadow otherwise returns thousands of
/// tiles the model did not ask for. `viewport_get` and `automap_get` are the tools for tiles.
fn compact_tiles(mut value: Value) -> Value {
    if let Some(events) = value.get_mut("events").and_then(Value::as_array_mut) {
        for event in events {
            if let Some(tiles) = event
                .get_mut("Visible")
                .and_then(|v| v.get_mut("tiles"))
                .and_then(Value::as_array_mut)
            {
                let count = tiles.len();
                event["Visible"] = json!({"count": count});
            }
            if let Some(tiles) = event
                .get_mut("Sensed")
                .and_then(|v| v.get_mut("tiles"))
                .and_then(Value::as_array_mut)
            {
                let count = tiles.len();
                event["Sensed"]["tiles"] = json!({"count": count});
            }
        }
    }
    value
}
