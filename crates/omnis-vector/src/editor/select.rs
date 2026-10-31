//! The lasso and the selection (the owner's drawing-GUI column, step 4, 2026-10-31): a freehand
//! loop dragged over the plan selects the cells inside it (`inside`: the loop's own cells and
//! every cell whose centre the even-odd rule puts inside the polygon of the loop's cell centres);
//! the selection stands, outlined on the plan (`outline`), until a lasso click clears it, the
//! map or the level changes, or Move carries it: everything on its cells (the terrain, the
//! heights, the sides, the placements, the start) to the cells the drag's offset names, the
//! source left bare and the destination replaced (`moving`), one undo step
//! (`Document::batch`). Owner's choices: a freehand loop; everything moves; the destination is
//! replaced, a tile-keyed kind already there refusing. Bevy-free.

use super::grab::Held;
use super::run::{self, Paint, Side};
use super::{Editor, PlaceKey};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::MapDef;
use omnis_sim::omnis_data::edit::{Brush, EdgeKind, Grid, Place};
use std::collections::BTreeSet;

/// The cells selected, on one level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The level the cells are on.
    pub level: Level,
    /// The cells.
    pub cells: BTreeSet<(u16, u16)>,
}

/// The cells a loop of cells encloses: the loop's own cells (joined cell to cell, the last back
/// to the first) and every cell of its bounding box whose centre lies inside the polygon through
/// the loop's cell centres (the even-odd rule). One cell, or a loop that is a line, selects its
/// cells alone.
#[must_use]
pub fn inside(loop_cells: &[(u16, u16)]) -> BTreeSet<(u16, u16)> {
    let mut cells: BTreeSet<(u16, u16)> = BTreeSet::new();
    let Some(first) = loop_cells.first() else {
        return cells;
    };
    let mut closed: Vec<(u16, u16)> = loop_cells.to_vec();
    closed.push(*first);
    for pair in closed.windows(2) {
        cells.extend(run::cells(pair[0], pair[1]));
    }
    let (Some(x0), Some(x1), Some(y0), Some(y1)) = (
        loop_cells.iter().map(|c| c.0).min(),
        loop_cells.iter().map(|c| c.0).max(),
        loop_cells.iter().map(|c| c.1).min(),
        loop_cells.iter().map(|c| c.1).max(),
    ) else {
        return cells;
    };
    let polygon: Vec<(f64, f64)> = loop_cells
        .iter()
        .map(|&(x, y)| (f64::from(x), f64::from(y)))
        .collect();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !cells.contains(&(x, y)) && encloses(&polygon, (f64::from(x), f64::from(y))) {
                cells.insert((x, y));
            }
        }
    }
    cells
}

/// Whether `point` lies inside `polygon` by the even-odd rule: a ray cast to the right crosses
/// an odd number of edges.
fn encloses(polygon: &[(f64, f64)], (px, py): (f64, f64)) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut odd = false;
    for i in 0..n {
        let (x1, y1) = polygon[i];
        let (x2, y2) = polygon[(i + 1) % n];
        if (y1 > py) != (y2 > py) {
            let at = x1 + (py - y1) * (x2 - x1) / (y2 - y1);
            if px < at {
                odd = !odd;
            }
        }
    }
    odd
}

/// The sides of `cells` that face a cell not among them: the selection's outline, as the plan
/// draws it. A side is keyed as the north or west side of a cell (`run::Side`).
#[must_use]
pub fn outline(cells: &BTreeSet<(u16, u16)>) -> Vec<Side> {
    let mut out = Vec::new();
    for &(x, y) in cells {
        let has = |dx: i32, dy: i32| {
            let (nx, ny) = (i32::from(x) + dx, i32::from(y) + dy);
            u16::try_from(nx)
                .ok()
                .zip(u16::try_from(ny).ok())
                .is_some_and(|n| cells.contains(&n))
        };
        if !has(0, -1) {
            out.push((true, x, y));
        }
        if !has(0, 1) {
            out.push((true, x, y + 1));
        }
        if !has(-1, 0) {
            out.push((false, x, y));
        }
        if !has(1, 0) {
            out.push((false, x + 1, y));
        }
    }
    out
}

/// A cell and the cell it moves to.
type Shift = ((u16, u16), (u16, u16));

/// `cells` moved by `d`; none when a cell would leave the map of `size`.
fn shifted(cells: &BTreeSet<(u16, u16)>, d: (i32, i32), size: (u16, u16)) -> Option<Vec<Shift>> {
    cells
        .iter()
        .map(|&(x, y)| {
            let nx = u16::try_from(i32::from(x) + d.0).ok()?;
            let ny = u16::try_from(i32::from(y) + d.1).ok()?;
            (nx < size.0 && ny < size.1).then_some(((x, y), (nx, ny)))
        })
        .collect()
}

/// The four sides of a cell, each with the cell and facing `Brush::Edge` names it by (the north
/// and west sides are the cell's own; the south and east are the next cells' north and west,
/// so a side is keyed once however it is reached).
fn sides_of((x, y): (u16, u16)) -> [(Side, (u16, u16), Facing); 4] {
    [
        ((true, x, y), (x, y), Facing::North),
        ((false, x, y), (x, y), Facing::West),
        ((true, x, y + 1), (x, y + 1), Facing::North),
        ((false, x + 1, y), (x + 1, y), Facing::West),
    ]
}

/// Whether a side keyed by `(horizontal, x, y)` lies on the map's boundary (walls for good).
fn on_boundary(size: (u16, u16), (horizontal, x, y): Side) -> bool {
    if horizontal {
        y == 0 || y >= size.1
    } else {
        x == 0 || x >= size.0
    }
}

/// The paints and places that move `sel` by `d` on `grid` and `def`: the source cells first
/// (no ground, height 0, every side open), then the destination (each cell's terrain with its
/// stair's rise, its height, its sides), then every placement on the moved cells put on its new
/// cell (the start too, on the ground level). A side on the map's boundary is left out, being
/// walls for good.
///
/// # Errors
/// A cell would leave the map, or a portal, a site or a pile already stands on a destination tile
/// the move does not vacate.
pub fn moving(
    grid: &Grid,
    def: &MapDef,
    sel: &Selection,
    d: (i32, i32),
) -> Result<(Vec<Paint>, Vec<Place>), String> {
    let size = (grid.width(), grid.height());
    let pairs = shifted(&sel.cells, d, size).ok_or("the selection would leave the map")?;
    let level = sel.level;
    let destinations: BTreeSet<(u16, u16)> = pairs.iter().map(|p| p.1).collect();
    // A tile-keyed kind on a destination tile the move does not vacate refuses the move.
    let there = |l: Level, x: u16, y: u16| {
        l == level && destinations.contains(&(x, y)) && !sel.cells.contains(&(x, y))
    };
    let clash = def
        .portals
        .iter()
        .find(|p| there(p.level, p.x, p.y))
        .map(|p| ("a portal", p.x, p.y))
        .or_else(|| {
            def.sites
                .iter()
                .find(|s| there(s.level, s.x, s.y))
                .map(|s| ("a site", s.x, s.y))
        })
        .or_else(|| {
            def.ground
                .iter()
                .find(|g| there(g.level, g.x, g.y))
                .map(|g| ("a ground pile", g.x, g.y))
        });
    if let Some((what, x, y)) = clash {
        return Err(format!("{what} is already at ({x}, {y})"));
    }
    let mut paints: Vec<Paint> = Vec::new();
    let mut done: BTreeSet<Side> = BTreeSet::new();
    // The source, bare.
    for &(from, _) in &pairs {
        paints.push((from, Brush::Clear));
        paints.push((from, Brush::Height { eighths: 0 }));
        for (side, cell, facing) in sides_of(from) {
            if !on_boundary(size, side) && done.insert(side) {
                paints.push((
                    cell,
                    Brush::Edge {
                        side: facing,
                        kind: EdgeKind::Open,
                    },
                ));
            }
        }
    }
    // The destination, as the source was.
    let mut done: BTreeSet<Side> = BTreeSet::new();
    for &(from, to) in &pairs {
        let tile = grid.tile(from.0, from.1).flatten();
        let brush = match tile.and_then(|i| def.terrains.get(usize::from(i))) {
            Some(terrain) => {
                let rise = def
                    .stairs
                    .iter()
                    .find(|s| Level(s.level) == level && (s.x, s.y) == from)
                    .map(|s| s.rise);
                Brush::Tile {
                    glyph: terrain.glyph,
                    rise,
                }
            }
            None => Brush::Clear,
        };
        paints.push((to, brush));
        paints.push((
            to,
            Brush::Height {
                eighths: grid.height_at(from.0, from.1).unwrap_or(0),
            },
        ));
        for (side, cell, facing) in sides_of(from) {
            let kind = grid.edge(cell.0, cell.1, facing).unwrap_or_default();
            let moved_side = (
                side.0,
                u16::try_from(i32::from(side.1) + d.0).unwrap_or(u16::MAX),
                u16::try_from(i32::from(side.2) + d.1).unwrap_or(u16::MAX),
            );
            if on_boundary(size, moved_side) || !done.insert(moved_side) {
                continue;
            }
            paints.push((
                (moved_side.1, moved_side.2),
                Brush::Edge { side: facing, kind },
            ));
        }
    }
    // The placements, by their keys: an id-keyed kind is put again under its id, a tile-keyed
    // kind removed and put.
    let moved_to = |x: u16, y: u16| pairs.iter().find(|p| p.0 == (x, y)).map(|p| p.1);
    let on = |l: Level, x: u16, y: u16| l == level && sel.cells.contains(&(x, y));
    let mut places: Vec<Place> = Vec::new();
    if level.is_ground()
        && sel.cells.contains(&(def.start.0, def.start.1))
        && let Some((x, y)) = moved_to(def.start.0, def.start.1)
    {
        places.push(Place::Start {
            x,
            y,
            facing: def.start.2,
        });
    }
    for p in def.portals.iter().filter(|p| on(p.level, p.x, p.y)) {
        if let Some((x, y)) = moved_to(p.x, p.y) {
            places.push(Place::Remove(PlaceKey::Portal {
                level,
                x: p.x,
                y: p.y,
            }));
            let mut p = p.clone();
            (p.x, p.y) = (x, y);
            places.push(Place::Portal(p));
        }
    }
    for s in def.sites.iter().filter(|s| on(s.level, s.x, s.y)) {
        if let Some((x, y)) = moved_to(s.x, s.y) {
            places.push(Place::Remove(PlaceKey::Site {
                level,
                x: s.x,
                y: s.y,
            }));
            let mut s = s.clone();
            (s.x, s.y) = (x, y);
            places.push(Place::Site(s));
        }
    }
    for g in def.ground.iter().filter(|g| on(g.level, g.x, g.y)) {
        if let Some((x, y)) = moved_to(g.x, g.y) {
            places.push(Place::Remove(PlaceKey::Ground {
                level,
                x: g.x,
                y: g.y,
            }));
            let mut g = g.clone();
            (g.x, g.y) = (x, y);
            places.push(Place::Ground(g));
        }
    }
    for e in def.encounters.iter().filter(|e| on(e.level, e.x, e.y)) {
        if let Some((x, y)) = moved_to(e.x, e.y) {
            let mut e = e.clone();
            (e.x, e.y) = (x, y);
            places.push(Place::Encounter(e));
        }
    }
    for o in def.objects.iter().filter(|o| on(o.level, o.x, o.y)) {
        if let Some((x, y)) = moved_to(o.x, o.y) {
            let mut o = o.clone();
            (o.x, o.y) = (x, y);
            places.push(Place::Object(o));
        }
    }
    for l in def.lore.iter().filter(|l| on(l.level, l.x, l.y)) {
        if let Some((x, y)) = moved_to(l.x, l.y) {
            let mut l = l.clone();
            (l.x, l.y) = (x, y);
            places.push(Place::Lore(l));
        }
    }
    Ok((paints, places))
}

impl Editor {
    /// The lasso's press on `cell`: a loop begins there.
    pub(super) fn lasso_start(&mut self, cell: (u16, u16)) {
        self.lasso = Some(vec![cell]);
        self.point(cell);
        self.revision = self.revision.wrapping_add(1);
    }

    /// The lasso's button held over `cell`: the loop goes on through it.
    pub(super) fn lasso_extend(&mut self, cell: (u16, u16)) {
        if let Some(lasso) = &mut self.lasso
            && lasso.last() != Some(&cell)
        {
            lasso.push(cell);
            self.point(cell);
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// The lasso's button let go: the loop's cells are selected; a loop of one cell (a click)
    /// clears the selection instead.
    pub(super) fn lasso_end(&mut self) {
        let Some(lasso) = self.lasso.take() else {
            return;
        };
        self.revision = self.revision.wrapping_add(1);
        if lasso.len() <= 1 {
            self.selection = None;
            self.message = Some("selection cleared".to_owned());
            return;
        }
        let cells = inside(&lasso);
        let n = cells.len();
        self.message = Some(format!(
            "{n} {} selected",
            if n == 1 { "cell" } else { "cells" }
        ));
        self.selection = Some(Selection {
            level: self.level,
            cells,
        });
    }

    /// The selection on the shown level, if one stands.
    #[must_use]
    pub fn selected(&self) -> Option<&Selection> {
        self.selection.as_ref().filter(|s| s.level == self.level)
    }

    /// The selection's outline for the plan: displaced by the drag's offset while Move carries
    /// it, so the outline shows where the cells would land.
    #[must_use]
    pub fn selection_outline(&self) -> Option<Vec<Side>> {
        let sel = self.selected()?;
        let mut sides = outline(&sel.cells);
        if let Some(grab) = &self.grab
            && grab.held == Some(Held::Selection)
            && let Some(from) = grab.from
        {
            let d = (
                i32::from(self.cursor.0) - i32::from(from.0),
                i32::from(self.cursor.1) - i32::from(from.1),
            );
            sides = sides
                .into_iter()
                .filter_map(|(h, x, y)| {
                    Some((
                        h,
                        u16::try_from(i32::from(x) + d.0).ok()?,
                        u16::try_from(i32::from(y) + d.1).ok()?,
                    ))
                })
                .collect();
        }
        Some(sides)
    }

    /// Move the selection by `d`, one undo step (`moving`, `Document::batch`); the selection
    /// goes with it and the message says how many cells landed where, with the loader's first
    /// word. A refusal is said and changes nothing. Whether the map changed.
    pub fn move_selection(&mut self, data: &omnis_sim::omnis_data::Data, d: (i32, i32)) -> bool {
        let (Some(sel), Some(def), Some(grid)) =
            (self.selected().cloned(), self.def(), self.grid())
        else {
            return false;
        };
        let (paints, places) = match moving(grid, &def, &sel, d) {
            Ok(moved) => moved,
            Err(why) => {
                self.message = Some(why);
                return false;
            }
        };
        let level = self.level;
        let result = self
            .session
            .document_mut()
            .and_then(|doc| doc.batch(level, &paints, places));
        if !self.took(data, result) {
            return false;
        }
        let cells: BTreeSet<(u16, u16)> = sel
            .cells
            .iter()
            .filter_map(|&(x, y)| {
                Some((
                    u16::try_from(i32::from(x) + d.0).ok()?,
                    u16::try_from(i32::from(y) + d.1).ok()?,
                ))
            })
            .collect();
        let n = cells.len();
        let (x0, y0) = cells.iter().next().copied().unwrap_or((0, 0));
        let (x0, y0) = cells
            .iter()
            .fold((x0, y0), |(ax, ay), &(x, y)| (ax.min(x), ay.min(y)));
        self.selection = Some(Selection { level, cells });
        self.report(format!(
            "{n} {} moved to ({x0}, {y0})",
            if n == 1 { "cell" } else { "cells" }
        ));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(cells: &[(u16, u16)]) -> BTreeSet<(u16, u16)> {
        cells.iter().copied().collect()
    }

    #[test]
    fn a_loop_selects_what_it_encloses_and_a_line_or_a_cell_selects_itself() {
        // A square loop round (2, 2): the ring and the middle.
        let ring = [
            (1, 1),
            (2, 1),
            (3, 1),
            (3, 2),
            (3, 3),
            (2, 3),
            (1, 3),
            (1, 2),
        ];
        let got = inside(&ring);
        assert_eq!(got.len(), 9, "{got:?}");
        assert!(got.contains(&(2, 2)));
        // Corners only: the sides are joined cell to cell, and the inside filled.
        let corners = [(0, 0), (4, 0), (4, 4), (0, 4)];
        let got = inside(&corners);
        assert_eq!(got.len(), 25, "{got:?}");
        // A concave L leaves the notch out.
        let l_shape = [(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)];
        let got = inside(&l_shape);
        assert!(got.contains(&(1, 3)) && got.contains(&(3, 1)), "{got:?}");
        assert!(!got.contains(&(3, 3)) && !got.contains(&(4, 4)), "{got:?}");
        // A line selects its cells, one cell itself.
        assert_eq!(
            inside(&[(0, 0), (3, 0)]),
            set(&[(0, 0), (1, 0), (2, 0), (3, 0)])
        );
        assert_eq!(inside(&[(5, 5)]), set(&[(5, 5)]));
        assert!(inside(&[]).is_empty());
    }

    #[test]
    fn the_outline_is_the_sides_facing_out() {
        let two = set(&[(1, 1), (2, 1)]);
        let sides = outline(&two);
        assert_eq!(sides.len(), 6, "{sides:?}");
        assert!(
            sides.contains(&(true, 1, 1)) && sides.contains(&(true, 2, 1)),
            "north"
        );
        assert!(
            sides.contains(&(true, 1, 2)) && sides.contains(&(true, 2, 2)),
            "south"
        );
        assert!(sides.contains(&(false, 1, 1)), "west");
        assert!(sides.contains(&(false, 3, 1)), "east");
        assert!(
            !sides.contains(&(false, 2, 1)),
            "the side between them is inside"
        );
    }
}
