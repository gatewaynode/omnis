//! `data/tiles/*.ron`: a tileset is the whole art contract for the first-person viewport
//! (ARCHITECTURE.md §8.3). It declares a detail depth and a lateral width, and for each named
//! surface the sprite slot at every `(depth, offset)` the renderer may ask for.

use crate::error::DataError;
use crate::limits::{
    IMAGE_EXTENSIONS, MAX_COLLECTION, MAX_DETAIL_DEPTH, check_asset_path, string_fits,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Which part of a tile a surface paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SlotKind {
    /// The ground of a tile.
    Floor,
    /// The ceiling of a tile; outdoor tilesets have none and draw sky instead.
    Ceiling,
    /// A wall on the far edge of a tile, facing the party.
    WallFront,
    /// A wall on the left edge of a tile, seen at an angle.
    WallLeft,
    /// A wall on the right edge of a tile, seen at an angle.
    WallRight,
    /// A door on the far edge of a tile.
    Door,
    /// An object standing on a tile.
    Object,
    /// A monster standing on a tile.
    Monster,
}

/// One sprite at one viewport position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// Distance from the party, `0` being the party's own tile.
    pub depth: u8,
    /// Lateral offset, negative to the left.
    pub offset: i8,
    /// Pack-relative image path.
    pub path: String,
    /// Where the sprite's top-left corner sits in the viewport, in pixels.
    #[serde(default)]
    pub x: i16,
    /// Where the sprite's top-left corner sits in the viewport, in pixels.
    #[serde(default)]
    pub y: i16,
}

/// A named surface: a kind plus its slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Surface {
    /// What the surface paints.
    pub kind: SlotKind,
    /// The slots, any order in the file; looked up by `(depth, offset)`.
    pub slots: Vec<Slot>,
}

/// The tileset file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tileset {
    /// Schema version of this file.
    pub schema: u32,
    /// `pack:tileset:name`.
    pub id: String,
    /// How many rows are drawn with sprites; deeper tiles are the horizon band.
    pub detail_depth: u8,
    /// Lateral half-width covered by slots; offsets beyond it are clamped by the renderer.
    pub width: u8,
    /// The viewport canvas the slots are laid out on, in pixels.
    pub viewport: (u16, u16),
    /// Surfaces by name, referenced from map terrains.
    pub surfaces: BTreeMap<String, Surface>,
}

impl Tileset {
    /// The slot path for a surface at a viewport position, if declared.
    #[must_use]
    pub fn slot(&self, surface: &str, depth: u8, offset: i8) -> Option<&str> {
        let surface = self.surfaces.get(surface)?;
        surface
            .slots
            .iter()
            .find(|s| s.depth == depth && s.offset == offset)
            .map(|s| s.path.as_str())
    }

    /// Structural checks that need no other file. Every problem is reported.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.detail_depth == 0 || self.detail_depth > MAX_DETAIL_DEPTH {
            errors.push(DataError::new(
                file,
                format!(
                    "detail_depth {} must be 1..={MAX_DETAIL_DEPTH}",
                    self.detail_depth
                ),
            ));
        }
        if self.viewport.0 == 0 || self.viewport.1 == 0 {
            errors.push(DataError::new(
                file,
                "viewport must have a non-zero width and height",
            ));
        }
        if self.width > MAX_DETAIL_DEPTH {
            errors.push(DataError::new(
                file,
                format!("width {} must be at most {MAX_DETAIL_DEPTH}", self.width),
            ));
        }
        if self.surfaces.len() > MAX_COLLECTION {
            errors.push(DataError::new(file, "too many surfaces"));
        }
        for (name, surface) in &self.surfaces {
            if name.is_empty() || !string_fits(name) {
                errors.push(DataError::new(file, "surface name is empty or too long"));
            }
            if surface.slots.len() > MAX_COLLECTION {
                errors.push(DataError::new(
                    file,
                    format!("surface '{name}' has too many slots"),
                ));
                continue;
            }
            let mut seen = BTreeMap::new();
            for slot in &surface.slots {
                if slot.depth >= self.detail_depth {
                    errors.push(DataError::new(
                        file,
                        format!(
                            "surface '{name}' slot depth {} is beyond detail_depth",
                            slot.depth
                        ),
                    ));
                }
                if slot.offset.unsigned_abs() > self.width {
                    errors.push(DataError::new(
                        file,
                        format!(
                            "surface '{name}' slot offset {} is beyond width",
                            slot.offset
                        ),
                    ));
                }
                if let Err(fault) = check_asset_path(&slot.path, &IMAGE_EXTENSIONS) {
                    errors.push(DataError::new(
                        file,
                        format!(
                            "surface '{name}' slot path '{}': {}",
                            slot.path,
                            fault.reason()
                        ),
                    ));
                }
                if seen.insert((slot.depth, slot.offset), ()).is_some() {
                    errors.push(DataError::new(
                        file,
                        format!(
                            "surface '{name}' declares ({}, {}) twice",
                            slot.depth, slot.offset
                        ),
                    ));
                }
            }
        }
    }
}
