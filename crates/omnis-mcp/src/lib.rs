//! `omnis-mcp`: the MCP bridge (ARCHITECTURE.md §9.2). JSON-RPC 2.0 over stdio, one message
//! per line, translating MCP tool calls one to one into protocol ops against either the
//! running game (over the dev socket) or an in-process headless game. Own JSON-RPC and own
//! schema builder: no MCP SDK, no `schemars` (self-supporting rule).
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod backend;
pub mod base64;
pub mod bridge;
pub mod rpc;
pub mod schema;
pub mod tools;

pub use backend::Backend;
pub use bridge::Bridge;
