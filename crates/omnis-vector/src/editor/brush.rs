//! Brushes (M5.5, `tasks/plans/m5-editor-v1.md` §Client side "Input"): what a click paints.
//! The palette is the open map's terrains in their order, the four kinds of edge, a height
//! and no ground; the tools column marks the current one and the first nine answer the digit
//! keys. What is painted is a target: a cell, or one of its edges when the pointer is within a
//! quarter of the cell's side of that border (`plan::View::target`). Bevy-free.

use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::MapDef;
use omnis_sim::omnis_data::edit::{Brush, EdgeKind};

/// The heights the brush offers, in eighths of a cell: eight cells down to eight up (the
/// file's own limit lies far wider; a picked height beyond this is held to it).
pub const EIGHTHS: core::ops::RangeInclusive<i16> = -64..=64;

/// A brush of the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// A terrain, by its index in the map's list.
    Terrain(u8),
    /// An edge of this kind, on the target's side or the side the cursor faces.
    Edge(EdgeKind),
    /// The height the tools column holds.
    Height,
    /// No ground.
    Clear,
}

/// Where a click lands: a cell, and the edge of it when the pointer is near that border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// The cell.
    pub cell: (u16, u16),
    /// The border the pointer is near, if any.
    pub edge: Option<Facing>,
}

/// The palette of `def`: its terrains, then wall, door, ramp and open edge, the height and no
/// ground.
#[must_use]
pub fn palette(def: &MapDef) -> Vec<Tool> {
    let mut out: Vec<Tool> = (0..def.terrains.len())
        .filter_map(|i| u8::try_from(i).ok())
        .map(Tool::Terrain)
        .collect();
    out.extend([
        Tool::Edge(EdgeKind::Wall),
        Tool::Edge(EdgeKind::Door),
        Tool::Edge(EdgeKind::Ramp),
        Tool::Edge(EdgeKind::Open),
        Tool::Height,
        Tool::Clear,
    ]);
    out
}

impl Tool {
    /// The first brush of `def`: its first terrain, else no ground.
    #[must_use]
    pub fn first(def: &MapDef) -> Tool {
        if def.terrains.is_empty() {
            Tool::Clear
        } else {
            Tool::Terrain(0)
        }
    }

    /// Its name in the tools column.
    #[must_use]
    pub fn label(self, def: &MapDef) -> String {
        match self {
            Tool::Terrain(i) => def
                .terrains
                .get(usize::from(i))
                .map_or_else(|| "?".to_owned(), |t| format!("{} {}", t.glyph, t.name)),
            Tool::Edge(EdgeKind::Wall) => "Wall".to_owned(),
            Tool::Edge(EdgeKind::Door) => "Door".to_owned(),
            Tool::Edge(EdgeKind::Ramp) => "Ramp".to_owned(),
            Tool::Edge(EdgeKind::Open) => "Open edge".to_owned(),
            Tool::Height => "Height".to_owned(),
            Tool::Clear => "No ground".to_owned(),
        }
    }

    /// The brush this tool paints at `target`: a terrain on the cell (a stair rising the way
    /// the cursor `facing`s), an edge on the target's side or, from the keys, the side faced,
    /// the height in `eighths`, or no ground. None for a terrain the map no longer has.
    #[must_use]
    pub fn brush(
        self,
        def: &MapDef,
        target: Target,
        facing: Facing,
        eighths: i16,
    ) -> Option<Brush> {
        Some(match self {
            Tool::Terrain(i) => Brush::Tile {
                glyph: def.terrains.get(usize::from(i))?.glyph,
                rise: Some(facing),
            },
            Tool::Edge(kind) => Brush::Edge {
                side: target.edge.unwrap_or(facing),
                kind,
            },
            Tool::Height => Brush::Height { eighths },
            Tool::Clear => Brush::Clear,
        })
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
    fn the_palette_is_the_terrains_then_the_edges_the_height_and_no_ground() {
        let def = town();
        let palette = palette(&def);
        let n = def.terrains.len();
        assert_eq!(palette.len(), n + 6);
        assert_eq!(palette[0], Tool::Terrain(0));
        assert_eq!(palette[n], Tool::Edge(EdgeKind::Wall));
        assert_eq!(palette[n + 4], Tool::Height);
        assert_eq!(palette[n + 5], Tool::Clear);
        assert_eq!(Tool::first(&def), Tool::Terrain(0));
        let first = &def.terrains[0];
        assert_eq!(
            Tool::Terrain(0).label(&def),
            format!("{} {}", first.glyph, first.name)
        );
        assert_eq!(Tool::Terrain(200).label(&def), "?");
        let mut bare = def;
        bare.terrains.clear();
        assert_eq!(Tool::first(&bare), Tool::Clear);
    }

    #[test]
    fn a_brush_takes_the_target_s_edge_or_the_facing_and_a_terrain_rises_the_way_faced() {
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
            Tool::Edge(EdgeKind::Door).brush(&def, cell, Facing::South, 0),
            Some(Brush::Edge {
                side: Facing::South,
                kind: EdgeKind::Door
            })
        );
        assert_eq!(
            Tool::Edge(EdgeKind::Door).brush(&def, east, Facing::South, 0),
            Some(Brush::Edge {
                side: Facing::East,
                kind: EdgeKind::Door
            })
        );
        assert_eq!(
            Tool::Terrain(0).brush(&def, east, Facing::North, 0),
            Some(Brush::Tile {
                glyph: def.terrains[0].glyph,
                rise: Some(Facing::North)
            })
        );
        assert_eq!(Tool::Terrain(200).brush(&def, cell, Facing::North, 0), None);
        assert_eq!(
            Tool::Height.brush(&def, cell, Facing::North, -5),
            Some(Brush::Height { eighths: -5 })
        );
        assert_eq!(
            Tool::Clear.brush(&def, cell, Facing::North, 0),
            Some(Brush::Clear)
        );
        assert_eq!(held(1000), 64);
        assert_eq!(held(-1000), -64);
        assert_eq!(held(3), 3);
    }
}
