//! The pointer states and the palette (the owner's drawing-GUI column, 2026-10-25, after M5.5's
//! brushes): what the pointer does is a `Tool` (select, brush, a run by corners, a freehand run,
//! the eraser), what it paints is an `Entry` of the palette (the open map's terrains in their
//! order, then wall, door, ramp and the height, then a stamp per placement kind, step 3). A click
//! paints the entry at a target: a cell, or one of its edges when the pointer is within a quarter
//! of the cell's side of that border (`plan::View::target`), the nearest side for an edge entry;
//! a stamp places a placement on the cell (`super::stamp`). The eraser needs no entry: it opens
//! an edge, removes a placement or clears the ground under the pointer. Bevy-free.

use super::forms::Placing;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::MapDef;
use omnis_sim::omnis_data::edit::{Brush, EdgeKind};

/// The heights the brush offers, in eighths of a cell: eight cells down to eight up (the
/// file's own limit lies far wider; a picked height beyond this is held to it).
pub const EIGHTHS: core::ops::RangeInclusive<i16> = -64..=64;

/// What the pointer does on the plan: a state of the tool grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// A click puts the cursor on the cell and paints nothing.
    Select,
    /// A click paints the palette's entry at the target; a drag is one stroke.
    Brush,
    /// A drag carries the selection when the press is inside it, else the first placement
    /// under the press, to the cell let go on, or pans the plan when the press took none
    /// (`super::grab`, `super::select`).
    Move,
    /// A drag draws a loop; the cells inside it are selected (`super::select`, step 4). A click
    /// clears the selection.
    Lasso,
    /// A click fills the region of one ground under it with the entry, bounded by every closed
    /// side (`super::fill`).
    Fill,
    /// A run drawn corner by corner, taken by a click on its last corner (`super::run`): of
    /// edges with an edge entry, of cells with a terrain or the height (step 4).
    Corners,
    /// A run drawn freehand, taken when the button is let go.
    Freehand,
    /// A click opens the edge, removes the first placement or clears the ground under it.
    Eraser,
}

impl Tool {
    /// Every tool, in the grid's order.
    pub const ALL: [Tool; 8] = [
        Tool::Select,
        Tool::Brush,
        Tool::Move,
        Tool::Lasso,
        Tool::Fill,
        Tool::Corners,
        Tool::Freehand,
        Tool::Eraser,
    ];

    /// Its name, under the grid.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Brush => "Brush",
            Tool::Move => "Move",
            Tool::Lasso => "Lasso",
            Tool::Fill => "Fill",
            Tool::Corners => "Run by corners",
            Tool::Freehand => "Freehand run",
            Tool::Eraser => "Eraser",
        }
    }

    /// Its icon in the set.
    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Tool::Select => "mouse-pointer-2",
            Tool::Brush => "brush",
            Tool::Move => "move",
            Tool::Lasso => "lasso-select",
            Tool::Fill => "paint-bucket",
            Tool::Corners => "pen-tool",
            Tool::Freehand => "pencil",
            Tool::Eraser => "eraser",
        }
    }

    /// Its key, the shortcut for its icon.
    #[must_use]
    pub const fn key(self) -> char {
        match self {
            Tool::Select => 'V',
            Tool::Brush => 'B',
            Tool::Move => 'M',
            Tool::Lasso => 'L',
            Tool::Fill => 'F',
            Tool::Corners => 'P',
            Tool::Freehand => 'N',
            Tool::Eraser => 'X',
        }
    }

    /// Whether this is a run tool, which aims at grid corners and paints through `take_run`.
    #[must_use]
    pub const fn is_run(self) -> bool {
        matches!(self, Tool::Corners | Tool::Freehand)
    }
}

/// Where a click lands: a cell, and the edge of it when the pointer is near that border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// The cell.
    pub cell: (u16, u16),
    /// The border the pointer is near, if any.
    pub edge: Option<Facing>,
}

/// An entry of the palette: what the brush paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// A terrain, by its index in the map's list.
    Terrain(u8),
    /// An edge of this kind, on the target's side or the side the cursor faces; a run takes it.
    Edge(EdgeKind),
    /// The height the details hold.
    Height,
    /// A placement of this kind, placed on the cell with the detail the palette holds (the
    /// owner's column, step 3, 2026-10-25; `super::stamp`).
    Stamp(Placing),
}

/// The palette of `def`: its terrains, then wall, door and ramp, the height, and a stamp a
/// placement kind. (An open edge and no ground are the eraser's.)
#[must_use]
pub fn palette(def: &MapDef) -> Vec<Entry> {
    let mut out: Vec<Entry> = (0..def.terrains.len())
        .filter_map(|i| u8::try_from(i).ok())
        .map(Entry::Terrain)
        .collect();
    out.extend([
        Entry::Edge(EdgeKind::Wall),
        Entry::Edge(EdgeKind::Door),
        Entry::Edge(EdgeKind::Ramp),
        Entry::Height,
    ]);
    out.extend(Placing::ALL.iter().map(|kind| Entry::Stamp(*kind)));
    out
}

impl Entry {
    /// The first entry of `def`: its first terrain, else the wall.
    #[must_use]
    pub fn first(def: &MapDef) -> Entry {
        if def.terrains.is_empty() {
            Entry::Edge(EdgeKind::Wall)
        } else {
            Entry::Terrain(0)
        }
    }

    /// Its name.
    #[must_use]
    pub fn label(self, def: &MapDef) -> String {
        match self {
            Entry::Terrain(i) => def
                .terrains
                .get(usize::from(i))
                .map_or_else(|| "?".to_owned(), |t| format!("{} {}", t.glyph, t.name)),
            Entry::Edge(EdgeKind::Wall) => "Wall".to_owned(),
            Entry::Edge(EdgeKind::Door) => "Door".to_owned(),
            Entry::Edge(EdgeKind::Ramp) => "Ramp".to_owned(),
            Entry::Edge(EdgeKind::Open) => "Open edge".to_owned(),
            Entry::Height => "Height".to_owned(),
            Entry::Stamp(kind) => kind.label().to_owned(),
        }
    }

    /// What it does, for the details under the palette.
    #[must_use]
    pub fn describes(self, def: &MapDef) -> String {
        match self {
            Entry::Terrain(_) => self.label(def),
            Entry::Edge(EdgeKind::Wall) => "Wall: a solid side".to_owned(),
            Entry::Edge(EdgeKind::Door) => "Door: a door in the side".to_owned(),
            Entry::Edge(EdgeKind::Ramp) => "Ramp: a slope over the side".to_owned(),
            Entry::Edge(EdgeKind::Open) => "Open edge: nothing in the side".to_owned(),
            Entry::Height => "Height: the cell's floor, in eighths".to_owned(),
            Entry::Stamp(kind) => format!("{}, placed with a click", stamp_words(kind)),
        }
    }

    /// Its icon in the set; a terrain has a swatch of its colour instead.
    #[must_use]
    pub const fn icon(self) -> Option<&'static str> {
        match self {
            Entry::Terrain(_) => None,
            Entry::Edge(EdgeKind::Wall) => Some("brick-wall"),
            Entry::Edge(EdgeKind::Door) => Some("door-closed"),
            Entry::Edge(EdgeKind::Ramp) => Some("triangle-right"),
            Entry::Edge(EdgeKind::Open) => Some("square-dashed"),
            Entry::Height => Some("mountain"),
            Entry::Stamp(Placing::Portal) => Some("door-open"),
            Entry::Stamp(Placing::Encounter) => Some("swords"),
            Entry::Stamp(Placing::Activity) => Some(crate::icons::ACTIVITY),
            Entry::Stamp(Placing::Object) => Some("package"),
            Entry::Stamp(Placing::Ground) => Some("layers"),
            Entry::Stamp(Placing::Lore) => Some("scroll-text"),
        }
    }

    /// Whether it is an edge kind, which a run takes.
    #[must_use]
    pub const fn is_edge(self) -> bool {
        matches!(self, Entry::Edge(_))
    }

    /// Whether it is a terrain or the height, which the fill takes.
    #[must_use]
    pub const fn fills(self) -> bool {
        matches!(self, Entry::Terrain(_) | Entry::Height)
    }

    /// The placement kind it stamps, when it is a stamp.
    #[must_use]
    pub const fn stamp(self) -> Option<Placing> {
        match self {
            Entry::Stamp(kind) => Some(kind),
            Entry::Terrain(_) | Entry::Edge(_) | Entry::Height => None,
        }
    }

    /// The brush this entry paints at `target`: a terrain on the cell (a stair rising the way
    /// the cursor `facing`s), an edge on the target's side or, from the keys, the side faced, or
    /// the height in `eighths`. None for a terrain the map no longer has, and for a stamp, which
    /// places rather than paints (`super::stamp`).
    #[must_use]
    pub fn brush(
        self,
        def: &MapDef,
        target: Target,
        facing: Facing,
        eighths: i16,
    ) -> Option<Brush> {
        Some(match self {
            Entry::Terrain(i) => Brush::Tile {
                glyph: def.terrains.get(usize::from(i))?.glyph,
                rise: Some(facing),
            },
            Entry::Edge(kind) => Brush::Edge {
                side: target.edge.unwrap_or(facing),
                kind,
            },
            Entry::Height => Brush::Height { eighths },
            Entry::Stamp(_) => return None,
        })
    }
}

/// What a stamp places, for the details under the palette.
const fn stamp_words(kind: Placing) -> &'static str {
    match kind {
        Placing::Portal => "Portal: a way to another map",
        Placing::Encounter => "Encounter: creatures that stand on the cell",
        Placing::Activity => "Activity: a service's site",
        Placing::Object => "Object: a thing placed on the cell",
        Placing::Ground => "Ground pile: items on the floor",
        Placing::Lore => "Lore: a text a search turns up",
    }
}

/// The height held to the brush's range.
#[must_use]
pub fn held(eighths: i16) -> i16 {
    eighths.clamp(*EIGHTHS.start(), *EIGHTHS.end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_data::edit::Session;
    use std::path::PathBuf;

    /// The test pack's town, as its file holds it.
    fn town() -> MapDef {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/test");
        let mut session = Session::new(vec![root]).expect("the test pack");
        session.open("test:map:town").expect("the town");
        session.document().expect("open").def()
    }

    #[test]
    fn the_palette_is_the_terrains_then_wall_door_ramp_the_height_and_a_stamp_a_kind() {
        let def = town();
        let palette = palette(&def);
        let n = def.terrains.len();
        assert_eq!(palette.len(), n + 4 + Placing::ALL.len());
        assert_eq!(palette[0], Entry::Terrain(0));
        assert_eq!(palette[n], Entry::Edge(EdgeKind::Wall));
        assert_eq!(palette[n + 1], Entry::Edge(EdgeKind::Door));
        assert_eq!(palette[n + 2], Entry::Edge(EdgeKind::Ramp));
        assert_eq!(palette[n + 3], Entry::Height);
        for (i, kind) in Placing::ALL.iter().enumerate() {
            assert_eq!(palette[n + 4 + i], Entry::Stamp(*kind));
            assert_eq!(Entry::Stamp(*kind).stamp(), Some(*kind));
            assert!(!Entry::Stamp(*kind).fills() && !Entry::Stamp(*kind).is_edge());
        }
        assert!(Entry::Terrain(0).fills() && Entry::Height.fills());
        assert!(!Entry::Edge(EdgeKind::Door).fills());
        assert_eq!(Entry::Height.stamp(), None);
        assert_eq!(
            Entry::Stamp(Placing::Activity).describes(&def),
            "Activity: a service's site, placed with a click"
        );
        assert_eq!(Entry::first(&def), Entry::Terrain(0));
        let first = &def.terrains[0];
        assert_eq!(
            Entry::Terrain(0).label(&def),
            format!("{} {}", first.glyph, first.name)
        );
        assert_eq!(Entry::Terrain(200).label(&def), "?");
        for entry in &palette {
            assert!(entry.label(&def).is_ascii());
            assert!(entry.describes(&def).is_ascii());
            if let Some(icon) = entry.icon() {
                assert!(omnis_icon::get(icon).is_some(), "{icon}");
            }
        }
        assert!(Entry::Terrain(0).icon().is_none());
        assert!(Entry::Edge(EdgeKind::Wall).is_edge() && !Entry::Height.is_edge());
        let mut bare = def;
        bare.terrains.clear();
        assert_eq!(Entry::first(&bare), Entry::Edge(EdgeKind::Wall));
    }

    #[test]
    fn every_tool_has_a_name_an_icon_of_the_set_and_a_key() {
        assert_eq!(Tool::ALL.len(), 8);
        let mut keys = Vec::new();
        for tool in Tool::ALL {
            assert!(tool.label().is_ascii());
            assert!(omnis_icon::get(tool.icon()).is_some(), "{}", tool.icon());
            assert!(!keys.contains(&tool.key()), "{tool:?}");
            keys.push(tool.key());
        }
        assert!(Tool::Corners.is_run() && Tool::Freehand.is_run());
        assert!(!Tool::Brush.is_run() && !Tool::Eraser.is_run() && !Tool::Select.is_run());
        assert!(!Tool::Move.is_run() && !Tool::Fill.is_run() && !Tool::Lasso.is_run());
        assert_eq!(
            (Tool::Lasso.icon(), Tool::Lasso.key()),
            ("lasso-select", 'L')
        );
    }

    #[test]
    fn an_entry_s_brush_takes_the_target_s_edge_or_the_facing_and_a_terrain_rises_the_way_faced() {
        let def = town();
        let cell = Target {
            cell: (1, 1),
            edge: None,
        };
        let east = Target {
            cell: (1, 1),
            edge: Some(Facing::East),
        };
        assert_eq!(
            Entry::Edge(EdgeKind::Door).brush(&def, cell, Facing::South, 0),
            Some(Brush::Edge {
                side: Facing::South,
                kind: EdgeKind::Door
            })
        );
        assert_eq!(
            Entry::Edge(EdgeKind::Door).brush(&def, east, Facing::South, 0),
            Some(Brush::Edge {
                side: Facing::East,
                kind: EdgeKind::Door
            })
        );
        assert_eq!(
            Entry::Terrain(0).brush(&def, east, Facing::North, 0),
            Some(Brush::Tile {
                glyph: def.terrains[0].glyph,
                rise: Some(Facing::North)
            })
        );
        assert_eq!(
            Entry::Terrain(200).brush(&def, cell, Facing::North, 0),
            None
        );
        assert_eq!(
            Entry::Stamp(Placing::Lore).brush(&def, cell, Facing::North, 0),
            None,
            "a stamp places, it does not paint"
        );
        assert_eq!(
            Entry::Height.brush(&def, cell, Facing::North, -5),
            Some(Brush::Height { eighths: -5 })
        );
        assert_eq!(held(1000), 64);
        assert_eq!(held(-1000), -64);
        assert_eq!(held(3), 3);
    }
}
