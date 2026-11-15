//! Growing and cutting a map on any of its four sides (owner, 2026-11-14;
//! `tasks/plans/editor-resize.md`). Everything on the map moves together: every level's cells,
//! heights and edges, and each thing placed by cell. A side that grows keeps its old outer wall
//! as an inside wall; what a cut leaves outside is dropped.

use super::document::State;
use super::kinds::EditError;
use crate::limits::MAX_MAP_SIDE;
use crate::map::MapDef;
use std::sync::Arc;

/// Cells added on each side; a negative number removes them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sides {
    /// Rows added above row 0.
    pub north: i16,
    /// Columns added past the last.
    pub east: i16,
    /// Rows added past the last.
    pub south: i16,
    /// Columns added before column 0.
    pub west: i16,
}

impl Sides {
    /// The sides that take a map of `(width, height)` to `(to_width, to_height)` with its
    /// top-left kept: what `Edit::Resize` asks for.
    #[must_use]
    pub fn to_size((width, height): (u16, u16), (to_width, to_height): (u16, u16)) -> Sides {
        let delta =
            |from: u16, to: u16| i16::try_from(i32::from(to) - i32::from(from)).unwrap_or(i16::MAX);
        Sides {
            east: delta(width, to_width),
            south: delta(height, to_height),
            ..Sides::default()
        }
    }

    /// The size a map of `(width, height)` comes to.
    ///
    /// # Errors
    /// `OutOfRange` when a side would be under 1 or over `MAX_MAP_SIDE`.
    pub fn size(self, (width, height): (u16, u16)) -> Result<(u16, u16), EditError> {
        let side = |old: u16, a: i16, b: i16| {
            let new = i32::from(old) + i32::from(a) + i32::from(b);
            u16::try_from(new)
                .ok()
                .filter(|n| (1..=MAX_MAP_SIDE).contains(n))
                .ok_or(EditError::OutOfRange)
        };
        Ok((
            side(width, self.west, self.east)?,
            side(height, self.north, self.south)?,
        ))
    }

    /// Where the cell `(x, y)` of the old map lies in the new one of `size`, when it is kept.
    #[must_use]
    pub fn moved(self, (x, y): (u16, u16), size: (u16, u16)) -> Option<(u16, u16)> {
        let x = u16::try_from(i32::from(x) + i32::from(self.west)).ok()?;
        let y = u16::try_from(i32::from(y) + i32::from(self.north)).ok()?;
        (x < size.0 && y < size.1).then_some((x, y))
    }
}

/// A coordinate moved `by` and held inside a side of `side` cells.
fn clamp(at: u16, by: i16, side: u16) -> u16 {
    let moved = (i32::from(at) + i32::from(by)).clamp(0, i32::from(side) - 1);
    u16::try_from(moved).unwrap_or(0)
}

/// The terrain new ground cells get: the first that joins no level (a stair, a ladder or a
/// hole would need more than a cell), none when the map has no such terrain.
fn fill(def: &MapDef) -> Option<u8> {
    let plain = def.terrains.iter().position(|t| t.join.is_none())?;
    u8::try_from(plain).ok()
}

/// Keep what stays on the map, each at its new cell.
fn shift<T>(
    things: &mut Vec<T>,
    at: impl Fn(&mut T) -> (&mut u16, &mut u16),
    to: impl Fn((u16, u16)) -> Option<(u16, u16)>,
) {
    things.retain_mut(|thing| {
        let (x, y) = at(thing);
        match to((*x, *y)) {
            Some((nx, ny)) => {
                (*x, *y) = (nx, ny);
                true
            }
            None => false,
        }
    });
}

/// Grow or cut the map by `sides`.
pub(super) fn extend(state: &mut State, sides: Sides) -> Result<(), EditError> {
    let old = (state.def.width, state.def.height);
    let size = sides.size(old)?;
    let ground = fill(&state.def);
    for (level, grid) in &mut state.grids {
        let fill = if level.is_ground() { ground } else { None };
        *grid = Arc::new(grid.extended((sides.west, sides.north), size, fill));
    }
    state.shift.0 += i32::from(sides.west);
    state.shift.1 += i32::from(sides.north);
    let def = &mut state.def;
    (def.width, def.height) = size;
    let to = |cell| sides.moved(cell, size);
    // The start goes with its cell; one left outside stands on the nearest cell inside.
    def.start.0 = clamp(def.start.0, sides.west, size.0);
    def.start.1 = clamp(def.start.1, sides.north, size.1);
    shift(&mut def.stairs, |s| (&mut s.x, &mut s.y), to);
    shift(&mut def.portals, |p| (&mut p.x, &mut p.y), to);
    shift(&mut def.encounters, |e| (&mut e.x, &mut e.y), to);
    shift(&mut def.sites, |s| (&mut s.x, &mut s.y), to);
    shift(&mut def.objects, |o| (&mut o.x, &mut o.y), to);
    shift(&mut def.ground, |g| (&mut g.x, &mut g.y), to);
    shift(&mut def.lore, |l| (&mut l.x, &mut l.y), to);
    Ok(())
}

/// What a cut would destroy (owner, 2026-11-14: "a warning ... when shrinking a map destroys
/// things"): everything but empty cells.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Losses {
    /// Cells that hold something: ground on any level, a height, or a wall, door or ramp that
    /// is not the map's own outer wall.
    pub cells: u32,
    /// Each placed thing left outside, in words: "portal town-gate at (11, 2)".
    pub things: Vec<String>,
    /// Where the start is put, when its cell is cut.
    pub start: Option<(u16, u16)>,
    /// Notes written on cells that go.
    pub notes: u32,
}

impl Losses {
    /// Whether the cut destroys nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells == 0 && self.things.is_empty() && self.start.is_none() && self.notes == 0
    }

    /// The losses, a line each: the cells counted, then each thing, the start, the notes.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let count = |n: u32, one: &str, many: &str| match n {
            0 => None,
            1 => Some(format!("1 {one}")),
            n => Some(format!("{n} {many}")),
        };
        let mut lines: Vec<String> = count(self.cells, "drawn cell", "drawn cells")
            .into_iter()
            .collect();
        lines.extend(self.things.iter().cloned());
        if let Some((x, y)) = self.start {
            lines.push(format!("the start's cell (the start moves to ({x}, {y}))"));
        }
        lines.extend(count(self.notes, "note on a cell", "notes on cells"));
        lines
    }

    /// The losses in a few words, for the step's name: "4 cells, 3 things lost".
    #[must_use]
    pub fn summary(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.cells > 0 {
            parts.push(format!("{} cells", self.cells));
        }
        let things = self.things.len() + usize::from(self.start.is_some());
        if things > 0 {
            parts.push(format!("{things} things"));
        }
        if self.notes > 0 {
            parts.push(format!("{} notes", self.notes));
        }
        (!parts.is_empty()).then(|| format!("{} lost", parts.join(", ")))
    }
}

/// Whether cutting a cell of a grid loses anything: its ground, its height, or an edge. An
/// edge shared with a cell that stays becomes the map's outer wall, so only a door or a ramp
/// on it is lost; the grid's own outer wall is no loss.
fn drawn(
    grid: &super::grid::Grid,
    (x, y): (u16, u16),
    (width, height): (u16, u16),
    gone: &dyn Fn(u16, u16) -> bool,
) -> bool {
    use super::grid::EdgeKind;
    use omnis_core::Facing;
    let across = |side: Facing| {
        let (dx, dy) = side.delta();
        let nx = u16::try_from(i32::from(x) + dx)
            .ok()
            .filter(|n| *n < width)?;
        let ny = u16::try_from(i32::from(y) + dy)
            .ok()
            .filter(|n| *n < height)?;
        Some((nx, ny))
    };
    let edged = Facing::ALL.iter().any(|&side| {
        let stays = across(side).is_none_or(|(nx, ny)| !gone(nx, ny));
        match grid.edge(x, y, side) {
            Some(EdgeKind::Open) | None => false,
            Some(EdgeKind::Wall) => !stays,
            Some(_) => true,
        }
    });
    grid.tile(x, y).flatten().is_some() || grid.height_at(x, y).is_some_and(|h| h != 0) || edged
}

/// What growing or cutting by `sides` would destroy; the notes are the session's to add.
pub(super) fn losses(state: &State, sides: Sides) -> Result<Losses, EditError> {
    let def = &state.def;
    let old = (def.width, def.height);
    let size = sides.size(old)?;
    let gone = |x: u16, y: u16| sides.moved((x, y), size).is_none();
    let mut lost = Losses::default();
    for y in 0..old.1 {
        for x in 0..old.0 {
            let any = || {
                let mut grids = state.grids.values();
                grids.any(|grid| drawn(grid, (x, y), old, &gone))
            };
            if gone(x, y) && any() {
                lost.cells += 1;
            }
        }
    }
    let mut thing = |what: String, x: u16, y: u16| {
        if gone(x, y) {
            lost.things.push(format!("{what} at ({x}, {y})"));
        }
    };
    for s in &def.stairs {
        thing("stair".to_owned(), s.x, s.y);
    }
    for p in &def.portals {
        thing(format!("portal {}", p.id), p.x, p.y);
    }
    for e in &def.encounters {
        thing(format!("encounter {}", e.id), e.x, e.y);
    }
    for s in &def.sites {
        thing(format!("site {}", s.service), s.x, s.y);
    }
    for o in &def.objects {
        thing(format!("object {}", o.id), o.x, o.y);
    }
    for g in &def.ground {
        thing("pile".to_owned(), g.x, g.y);
    }
    for l in &def.lore {
        thing(format!("lore {}", l.id), l.x, l.y);
    }
    if gone(def.start.0, def.start.1) {
        lost.start = Some((
            clamp(def.start.0, sides.west, size.0),
            clamp(def.start.1, sides.north, size.1),
        ));
    }
    Ok(lost)
}
