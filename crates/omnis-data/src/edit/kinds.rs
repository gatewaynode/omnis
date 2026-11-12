//! What the editor's requests and refusals are made of: the wire types a client or an op sends
//! (`Brush`, `Place`, `Edit`, `NewMap`), what the session reports (`EditorStatus`, `MapEntry`),
//! and why an edit was refused (`EditError`).

use crate::error::DataError;
use std::fmt;

/// Why the editor refused an edit. Validation of the whole map is `Invalid`, with the loader's
/// messages; the rest are the edit's own rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// No map is open.
    NoDocument,
    /// The tile is not on the map.
    Outside {
        /// Column.
        x: u16,
        /// Row.
        y: u16,
    },
    /// The map has no such level.
    NoLevel(i8),
    /// Level 0 is the map's own and cannot be added or removed.
    LevelZero,
    /// The level is already there.
    LevelExists(i8),
    /// No terrain has this glyph.
    UnknownGlyph(char),
    /// A terrain already has this glyph.
    GlyphTaken(char),
    /// The glyph is an edge, portal or no-ground character.
    Reserved(char),
    /// Tiles still use the terrain.
    TerrainInUse(char),
    /// A boundary edge is always a wall.
    BoundaryEdge,
    /// A number is beyond its limit.
    OutOfRange,
    /// A stair up or down needs the way it rises.
    NeedsRise,
    /// The tile holds no stair.
    NotAStair,
    /// Nothing matches the target.
    NoSuchTarget(String),
    /// A collection is at its limit; which.
    TooMany(&'static str),
    /// An id is not of the required shape.
    BadId(String),
    /// A file or entry already exists.
    Exists(String),
    /// The path is not under a directory the loader reads.
    UnknownDirectory(String),
    /// Nothing to undo or redo.
    NothingToUndo,
    /// The map does not pass the loader's checks: nothing was written.
    Invalid(Vec<DataError>),
    /// A file could not be read or written.
    Io(DataError),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::NoDocument => write!(f, "no map is open"),
            EditError::Outside { x, y } => write!(f, "({x}, {y}) is outside the map"),
            EditError::NoLevel(level) => write!(f, "the map has no level {level}"),
            EditError::LevelZero => write!(f, "level 0 is the map's own layout"),
            EditError::LevelExists(level) => write!(f, "the map already has level {level}"),
            EditError::UnknownGlyph(c) => write!(f, "no terrain has glyph '{c}'"),
            EditError::GlyphTaken(c) => write!(f, "a terrain already has glyph '{c}'"),
            EditError::Reserved(c) => write!(f, "glyph '{c}' is reserved for edges and no ground"),
            EditError::TerrainInUse(c) => write!(f, "tiles still use terrain '{c}'"),
            EditError::BoundaryEdge => write!(f, "a boundary edge is always a wall"),
            EditError::OutOfRange => write!(f, "the number is beyond its limit"),
            EditError::NeedsRise => write!(f, "a stair needs the way it rises"),
            EditError::NotAStair => write!(f, "the tile holds no stair"),
            EditError::NoSuchTarget(what) => write!(f, "nothing matches {what}"),
            EditError::TooMany(what) => write!(f, "too many {what}"),
            EditError::BadId(id) => write!(f, "id '{id}' is not of the required shape"),
            EditError::Exists(what) => write!(f, "{what} already exists"),
            EditError::UnknownDirectory(path) => {
                write!(f, "'{path}' is not under a directory the loader reads")
            }
            EditError::NothingToUndo => write!(f, "nothing to undo"),
            EditError::Invalid(errors) => {
                write!(f, "{} error(s): nothing was written", errors.len())?;
                for e in errors {
                    write!(f, "\n{e}")?;
                }
                Ok(())
            }
            EditError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<DataError> for EditError {
    fn from(e: DataError) -> Self {
        EditError::Io(e)
    }
}

// ---- Requests ----

use crate::edit::grid::EdgeKind;
use crate::encounter::{FixedEncounter, RandomEncounters};
use crate::lore::CellLore;
use crate::map::{MapKind, Portal, Terrain, WallSurfaces};
use crate::placement::{GroundDef, Placement};
use crate::rest_event::{RestEventDef, RestKind};
use crate::service::Site;
use omnis_core::{Facing, Level};
use serde::{Deserialize, Serialize};

/// What a click on a tile or an edge paints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Brush {
    /// A terrain, by its glyph; a stair up or down also needs the way it rises.
    Tile {
        /// The terrain's glyph.
        glyph: char,
        /// The way a stair climbs; required when the terrain is a stair up or down.
        #[serde(default)]
        rise: Option<Facing>,
    },
    /// An edge of the tile.
    Edge {
        /// Which edge.
        side: Facing,
        /// What it becomes.
        kind: EdgeKind,
    },
    /// The tile's height above the level's base, in eighths of a cell.
    Height {
        /// The height written.
        eighths: i16,
    },
    /// No ground.
    Clear,
}

/// What names one placement to remove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// The portal on a tile.
    Portal {
        /// Level.
        #[serde(default)]
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
    },
    /// A fixed encounter, by id.
    Encounter {
        /// Its id.
        id: String,
    },
    /// The service on a tile.
    Site {
        /// Level.
        #[serde(default)]
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
    },
    /// A rest event, by terrain name and rest kind.
    RestEvent {
        /// The terrain's name.
        terrain: String,
        /// Which rest.
        rest: RestKind,
    },
    /// An object placement, by id.
    Object {
        /// Its id.
        id: String,
    },
    /// The pile on a tile.
    Ground {
        /// Level.
        #[serde(default)]
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
    },
    /// Lore on a cell, by id.
    Lore {
        /// Its id.
        id: String,
    },
}

/// A placement put on the map (added, or replaced when its key is already there) or removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    /// Where a new party appears.
    Start {
        /// The level; the ground when left out (W1 step 3).
        #[serde(default)]
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
        /// Facing.
        facing: Facing,
    },
    /// A portal, linked or not; the key is its tile.
    Portal(Portal),
    /// A fixed encounter; the key is its id.
    Encounter(FixedEncounter),
    /// The random encounter table, or none.
    Random(Option<RandomEncounters>),
    /// A service on a tile; the key is its tile.
    Site(Site),
    /// A rest event; the key is its terrain and rest kind.
    RestEvent(RestEventDef),
    /// An object; the key is its id.
    Object(Placement),
    /// A pile on the ground; the key is its tile.
    Ground(GroundDef),
    /// Lore on a cell; the key is its id.
    Lore(CellLore),
    /// Take a placement away.
    Remove(Target),
}

impl Place {
    /// The key this placement is put or replaced by (`Document::place`); none for the start, the
    /// random table and a removal, which name no entry of a list.
    #[must_use]
    pub fn key(&self) -> Option<Target> {
        Some(match self {
            Place::Portal(p) => Target::Portal {
                level: p.level,
                x: p.x,
                y: p.y,
            },
            Place::Encounter(e) => Target::Encounter { id: e.id.clone() },
            Place::Site(s) => Target::Site {
                level: s.level,
                x: s.x,
                y: s.y,
            },
            Place::RestEvent(r) => Target::RestEvent {
                terrain: r.terrain.clone(),
                rest: r.rest,
            },
            Place::Object(o) => Target::Object { id: o.id.clone() },
            Place::Ground(g) => Target::Ground {
                level: g.level,
                x: g.x,
                y: g.y,
            },
            Place::Lore(l) => Target::Lore { id: l.id.clone() },
            Place::Start { .. } | Place::Random(_) | Place::Remove(_) => return None,
        })
    }
}

/// The map's own fields, each changed when given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapFields {
    /// Text key for the display name.
    #[serde(default)]
    pub name: Option<String>,
    /// The kind of place.
    #[serde(default)]
    pub kind: Option<MapKind>,
    /// Tileset id.
    #[serde(default)]
    pub tileset: Option<String>,
    /// Wall surfaces.
    #[serde(default)]
    pub wall: Option<WallSurfaces>,
    /// Closed door surface.
    #[serde(default)]
    pub door: Option<String>,
    /// Open door surface, or none; `Some(None)` clears it.
    #[serde(default)]
    pub door_open: Option<Option<String>>,
}

/// A change to the map's structure rather than to a tile or a placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edit {
    /// New tiles across and down; the top-left is kept, what falls outside is dropped.
    Resize {
        /// Tiles across.
        width: u16,
        /// Tiles down.
        height: u16,
    },
    /// The map's own fields.
    Map(MapFields),
    /// A level beside the map's own, all no ground.
    LevelAdd {
        /// Its number, never 0.
        level: i8,
        /// Its name's text key.
        name: String,
        /// The height added to every tile.
        #[serde(default)]
        base: i16,
    },
    /// A level's name or base.
    LevelSet {
        /// Which level.
        level: i8,
        /// Its new name key, when given.
        #[serde(default)]
        name: Option<String>,
        /// Its new base, when given.
        #[serde(default)]
        base: Option<i16>,
    },
    /// A level and everything placed on it.
    LevelRemove {
        /// Which level, never 0.
        level: i8,
    },
    /// A terrain the map did not have.
    TerrainAdd(Terrain),
    /// A terrain's fields, found by its glyph; a new glyph renames it.
    TerrainSet {
        /// The glyph it has now.
        glyph: char,
        /// What it becomes.
        terrain: Terrain,
    },
    /// A terrain no tile uses.
    TerrainRemove {
        /// Its glyph.
        glyph: char,
    },
    /// The way the stair on a tile rises.
    Rise {
        /// Level.
        #[serde(default)]
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
        /// The way it climbs.
        rise: Facing,
    },
}

/// What a new map starts from: its first terrain fills it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewMap {
    /// `pack:map:name`; the file is `data/maps/<name>.ron`.
    pub id: String,
    /// Text key for the display name.
    pub name: String,
    /// The kind of place.
    pub kind: MapKind,
    /// Tileset id.
    pub tileset: String,
    /// Tiles across.
    pub width: u16,
    /// Tiles down.
    pub height: u16,
    /// Wall surfaces.
    pub wall: WallSurfaces,
    /// Closed door surface.
    pub door: String,
    /// Open door surface, if any.
    #[serde(default)]
    pub door_open: Option<String>,
    /// The terrain every tile starts as.
    pub terrain: Terrain,
}

/// The editor's state, as a client reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorStatus {
    /// The pack root being edited.
    pub root: String,
    /// The open map's id, if one is open.
    pub map: Option<String>,
    /// Its file, root-relative.
    pub file: Option<String>,
    /// Tiles across.
    pub width: u16,
    /// Tiles down.
    pub height: u16,
    /// Its levels, lowest first, 0 among them.
    pub levels: Vec<i8>,
    /// Its terrains: glyph and name, in index order.
    pub terrains: Vec<(char, String)>,
    /// Whether the document differs from its file.
    pub dirty: bool,
    /// How many edits can be undone.
    pub undo: usize,
    /// How many can be redone.
    pub redo: usize,
    /// What the loader would refuse, in its words; empty when the map would load.
    pub errors: Vec<String>,
}

/// A map file of the pack, as listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapEntry {
    /// The id the file declares; empty when the file does not read.
    pub id: String,
    /// The file, root-relative.
    pub file: String,
    /// Why the file does not read, if it does not.
    pub error: Option<String>,
}
