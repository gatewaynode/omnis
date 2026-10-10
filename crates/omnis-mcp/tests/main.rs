//! The integration tests of the MCP bridge, one binary: macOS holds every newly linked executable
//! for a fixed wait on its first launch, so each test binary costs that wait once per build.

mod api_doc;
mod bridge;
mod common;
mod replies;
mod schema_proof;
mod vocabulary;
