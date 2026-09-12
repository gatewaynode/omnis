//! Omnis data: schema structs for every content type, the pack manifest and loader, the ID
//! registry, validation of packs as untrusted input, schema migrations, and RON read and write.
//!
//! This is the only simulation crate permitted to read or write files (ARCHITECTURE.md §3).
//! Everything else receives loaded data. Integers only; ordered collections only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
