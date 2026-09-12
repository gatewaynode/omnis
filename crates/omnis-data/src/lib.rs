//! Omnis data: schema structs for every content type, the pack manifest and loader, the ID
//! registry, validation of packs as untrusted input, schema migrations, and RON read and write.
//!
//! This is the only simulation crate permitted to read or write files (ARCHITECTURE.md §3).
//! Everything else receives loaded data. Integers only; ordered collections only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

pub mod error;
pub mod limits;
pub mod loader;
pub mod manifest;
pub mod map;
pub mod registry;
pub mod ron_io;
pub mod text;
pub mod tileset;

pub use error::{DataError, LoadReport};
pub use loader::{Data, MapData, PackFingerprint, ResolvedPortal, load_packs};
pub use manifest::{Attribution, PackManifest};
pub use map::{Cell, MapDef, MapKind, Portal, Terrain, WallSurfaces};
pub use registry::Registry;
pub use text::TextFile;
pub use tileset::{Slot, SlotKind, Surface, Tileset};

/// The schema version every data type is written at today. Older files are migrated on load.
pub const SCHEMA: u32 = 1;
