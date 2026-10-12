//! Omnis Vector: the 3D presentation experiment (`presentation-PRD.md`, `presentation-ARCHITECTURE.md`).
//!
//! Free movement in a vector-line 3D view over the simulation's grid. Every cell the pose
//! enters becomes a `Step` the simulation processes as in the 2D game, so the grid's rules
//! (time, the automap, portals, encounters) are the simulation's, never this crate's.
//!
//! Bevy-free core: `geom`, `grid`, `ground`, `look`, `pose`, `collide`, `bind`, `geometry`, `minimap`, `party`,
//! `raster`, `rolllog`, `cinema`, `trial`, `combat_menu`, `arena`, and `sim`, which keeps the
//! world private and reads it through the engine's views (ARCHITECTURE.md A17).
//! The Bevy shell lives in `shell` and holds only wiring.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod arena;
pub mod bind;
pub mod cinema;
pub mod collide;
pub mod combat_menu;
pub mod game_io;
pub mod geom;
pub mod geometry;
pub mod grid;
pub mod ground;
pub mod labels;
pub mod look;
pub mod minimap;
pub mod party;
pub mod pose;
pub mod raster;
pub mod rolllog;
pub mod scene;
pub mod screen;
pub mod settings;
pub mod shell;
pub mod sim;
pub mod style;
pub mod trial;
