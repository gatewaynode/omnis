//! The words of an undo step (the editor's column, step 5, owner 2026-10-31): what a change did,
//! said once for the history list. Each change names itself from its own request, so the socket's
//! edits are named too; the editor replaces the words with its own richer ones when it has them
//! (`Document::name_step`). ASCII, clipped to `WORDS_MAX`.

use crate::edit::grid::EdgeKind;
use crate::edit::kinds::{Brush, Edit, Place, Target};
use crate::map::MapDef;
use omnis_core::Level;

/// The most bytes a step's words take; longer words end in "...".
pub const WORDS_MAX: usize = 72;

/// `words` clipped to `WORDS_MAX` bytes on a character boundary, "..." closing a clipped one.
#[must_use]
pub fn clip(words: &str) -> String {
    if words.len() <= WORDS_MAX {
        return words.to_owned();
    }
    let mut end = WORDS_MAX - 3;
    while !words.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &words[..end])
}

/// Where a cell is, with its level when it is not the ground.
fn at(level: Level, x: u16, y: u16) -> String {
    if level == Level::GROUND {
        format!("({x}, {y})")
    } else {
        format!("({x}, {y}) on level {}", level.0)
    }
}

/// A paint's words: the terrain by name (its glyph when the map has no such terrain), a side's
/// kind, a height, or a clearing.
#[must_use]
pub fn brush(def: &MapDef, level: Level, x: u16, y: u16, brush: &Brush) -> String {
    let where_ = at(level, x, y);
    clip(&match brush {
        Brush::Tile { glyph, rise } => {
            let terrain = def.terrains.iter().find(|t| t.glyph == *glyph);
            let name = terrain.map_or_else(|| glyph.to_string(), |t| t.name.clone());
            // The rise is a stair's (a joining terrain); a plain terrain's brush carries the
            // facing too, which says nothing about the tile.
            match rise {
                Some(facing) if terrain.is_some_and(|t| t.join.is_some()) => {
                    format!("painted {name} rising {facing} at {where_}")
                }
                _ => format!("painted {name} at {where_}"),
            }
        }
        Brush::Edge { side, kind } => {
            let what = match kind {
                EdgeKind::Open => "opened",
                EdgeKind::Wall => "wall on",
                EdgeKind::Door => "door on",
                EdgeKind::Ramp => "ramp on",
            };
            format!("{what} the {side} side of {where_}")
        }
        Brush::Height { eighths } => format!("height {eighths} at {where_}"),
        Brush::Clear => format!("cleared {where_}"),
    })
}

/// A stroke's words: how many cells it painted, then its first paint's words.
#[must_use]
pub fn stroke(cells: usize, first: &str) -> String {
    clip(&format!("a stroke of {cells} cells: {first}"))
}

/// A placement's words: what was put where, or what was removed.
#[must_use]
pub fn place(place: &Place) -> String {
    clip(&match place {
        Place::Start { x, y, facing } => format!("start set at ({x}, {y}) facing {facing}"),
        Place::Portal(p) => match &p.link {
            Some(link) => format!(
                "portal '{}' at {} linked to '{link}'",
                p.id,
                at(p.level, p.x, p.y)
            ),
            None => format!("portal '{}' at {}", p.id, at(p.level, p.x, p.y)),
        },
        Place::Encounter(e) => format!("encounter {} at {}", e.id, at(e.level, e.x, e.y)),
        Place::Random(Some(_)) => "random encounters set".to_owned(),
        Place::Random(None) => "random encounters cleared".to_owned(),
        Place::Site(s) => format!("site {} at {}", s.service, at(s.level, s.x, s.y)),
        Place::RestEvent(r) => format!("rest event on {} ({:?} rest)", r.terrain, r.rest),
        Place::Object(o) => format!("object {} at {}", o.id, at(o.level, o.x, o.y)),
        Place::Ground(g) => format!("pile at {}", at(g.level, g.x, g.y)),
        Place::Lore(l) => format!("lore {} at {}", l.id, at(l.level, l.x, l.y)),
        Place::Remove(target) => format!("removed {}", self::target(target)),
    })
}

/// A removal's target in words.
fn target(target: &Target) -> String {
    match target {
        Target::Portal { level, x, y } => format!("the portal at {}", at(*level, *x, *y)),
        Target::Encounter { id } => format!("encounter {id}"),
        Target::Site { level, x, y } => format!("the site at {}", at(*level, *x, *y)),
        Target::RestEvent { terrain, rest } => {
            format!("the rest event on {terrain} ({rest:?} rest)")
        }
        Target::Object { id } => format!("object {id}"),
        Target::Ground { level, x, y } => format!("the pile at {}", at(*level, *x, *y)),
        Target::Lore { id } => format!("lore {id}"),
    }
}

/// A map edit's words.
#[must_use]
pub fn edit(edit: &Edit) -> String {
    clip(&match edit {
        Edit::Resize { width, height } => format!("resized to {width} x {height}"),
        Edit::Map(_) => "map fields set".to_owned(),
        Edit::LevelAdd { level, .. } => format!("level {level} added"),
        Edit::LevelSet { level, .. } => format!("level {level} set"),
        Edit::LevelRemove { level } => format!("level {level} removed"),
        Edit::TerrainAdd(t) => format!("terrain {} ({}) added", t.name, t.glyph),
        Edit::TerrainSet { glyph, .. } => format!("terrain {glyph} set"),
        Edit::TerrainRemove { glyph } => format!("terrain {glyph} removed"),
        Edit::Rise { level, x, y, rise } => {
            format!("rise {rise} set at {}", at(*level, *x, *y))
        }
    })
}

/// Several changes in one step.
#[must_use]
pub fn many(cells: usize, placements: usize) -> String {
    let s = |n: usize| if n == 1 { "" } else { "s" };
    match (cells, placements) {
        (0, p) => format!("{p} placement{}", s(p)),
        (c, 0) => format!("{c} cell{} painted", s(c)),
        (c, p) => format!("{c} cell{} and {p} placement{}", s(c), s(p)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_ascii_and_clipped() {
        let long = "x".repeat(200);
        let clipped = clip(&long);
        assert_eq!(clipped.len(), WORDS_MAX);
        assert!(clipped.ends_with("..."));
        assert_eq!(clip("short"), "short");
        assert_eq!(many(0, 1), "1 placement");
        assert_eq!(many(5, 0), "5 cells painted");
        assert_eq!(many(6, 2), "6 cells and 2 placements");
        assert_eq!(
            stroke(3, "painted r at (1, 1)"),
            "a stroke of 3 cells: painted r at (1, 1)"
        );
        assert_eq!(
            place(&Place::Start {
                x: 1,
                y: 2,
                facing: omnis_core::Facing::South
            }),
            "start set at (1, 2) facing south"
        );
        assert_eq!(
            edit(&Edit::Resize {
                width: 20,
                height: 12
            }),
            "resized to 20 x 12"
        );
    }
}
