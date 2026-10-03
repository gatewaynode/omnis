//! Omnis Vector: the 3D presentation experiment (`alt-PRD.md`, `alt-ARCHITECTURE.md`).
//!
//! Free movement in a vector-line 3D view over the simulation's grid. Every cell the pose
//! enters becomes a `Step` the simulation processes as in the 2D game, so the grid's rules
//! (time, the automap, portals, encounters) are the simulation's, never this crate's.
//!
//! Bevy-free core: `geom`, `grid`, `pose`, `collide`, `bind`, `geometry`, `minimap`, `party`,
//! `raster`, `rolllog`, `cinema`, `trial`, `combat_menu`.
//! The Bevy shell lives in `shell` and holds only wiring.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bind;
pub mod cinema;
pub mod collide;
pub mod combat_menu;
pub mod geom;
pub mod geometry;
pub mod grid;
pub mod minimap;
pub mod party;
pub mod pose;
pub mod raster;
pub mod rolllog;
pub mod shell;
pub mod trial;
