//! The editor's document model (M5.2; `tasks/plans/m5-editor-v1.md`): a map under edit as grids
//! of tiles and edges, one per level, with undo and redo; a session over a pack root that opens,
//! checks, saves and reseals maps, reads and writes every other file as text checked by the
//! loader, and keeps the author's notes. Bevy-free and integers only, like the rest of the
//! crate; the loader and the writer are the same code path (ARCHITECTURE §8.2).

pub mod answer;
pub mod document;
pub mod files;
pub mod grid;
pub mod kinds;
pub mod notes_store;
pub mod session;

pub use answer::{Answer, Request, answer};
pub use document::{Document, UNDO_CAP};
pub use files::check_text;
pub use grid::{EdgeKind, Grid};
pub use kinds::{Brush, Edit, EditError, EditorStatus, MapEntry, MapFields, NewMap, Place, Target};
pub use notes_store::Notes;
pub use session::Session;
