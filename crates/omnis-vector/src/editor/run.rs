//! The boundary tools (the owner, 2026-10-24; built 2026-10-25): wall runs drawn corner to
//! corner (`Tool::Corners`: a click a corner, the run pending until taken by a click on its
//! last corner, Enter or the Take button) or freehand (`Tool::Freehand`: a drag, taken when
//! the button is let go), snapped to the grid's corners and rasterised onto cell sides,
//! previewed in the caution colour, one undo step (`Document::paint_many`). The plan's
//! pointer for every tool lives here too (`Editor::pointer`), moved from the shell so the
//! pointer's rules are the model's. Bevy-free.

use super::Editor;
use super::brush::Tool;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::{Brush, EdgeKind};
use std::collections::BTreeSet;

/// A corner of the grid: `0..=width` across, `0..=height` down.
pub type Corner = (u16, u16);

/// A side of the grid by its key: horizontal (a cell's north side) or vertical (a west side),
/// with the column and row of the cell whose north or west side it is; the map's south and
/// east boundaries are row `height` and column `width`. The shape of `raster::EdgeKey`, in
/// the map's cells.
pub type Side = (bool, u16, u16);

/// A paint of the run: the cell and the brush on it, as `Document::paint_many` takes them.
pub type Paint = ((u16, u16), Brush);

/// The run being drawn: its corners in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Run {
    /// The corners clicked or dragged through, first to last.
    pub corners: Vec<Corner>,
}

/// What the plan previews in the caution colour: the run's sides, with the stretch to the
/// corner under the pointer, and that corner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Preview {
    /// The sides the run would wall.
    pub sides: Vec<Side>,
    /// The corner under the pointer.
    pub corner: Option<Corner>,
}

/// The pointer over the plan, as the shell reads it each frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    /// The pane pixel under the pointer; none off the plan or over a control.
    pub pixel: Option<(f32, f32)>,
    /// The left button.
    pub left: Press,
    /// The right button went down this frame.
    pub right_down: bool,
}

/// A button's state this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// Not pressed.
    Up,
    /// Went down this frame.
    Down,
    /// Held since an earlier frame.
    Held,
}

/// The sides along the grid from corner `a` to corner `b`: a straight run when the corners
/// share a row or column, else the staircase that hugs the line between them (each step
/// along the axis that leaves the walk nearer the line; the horizontal step at a tie).
/// `|dx| + |dy|` sides, none for `a == b`.
#[must_use]
pub fn sides(a: Corner, b: Corner) -> Vec<Side> {
    let (ax, ay) = (i64::from(a.0), i64::from(a.1));
    let (bx, by) = (i64::from(b.0), i64::from(b.1));
    let (dx, dy) = (bx - ax, by - ay);
    let (sx, sy) = (dx.signum(), dy.signum());
    let (mut x, mut y) = (ax, ay);
    let mut out = Vec::with_capacity(usize::try_from(dx.abs() + dy.abs()).unwrap_or(0));
    while (x, y) != (bx, by) {
        // How far off the line the walk would be after each step: the cross product with the
        // line's direction (its distance scaled by the line's length).
        let (ix, iy) = (x - ax, y - ay);
        let after_x = ((ix + sx) * dy - iy * dx).abs();
        let after_y = (ix * dy - (iy + sy) * dx).abs();
        let horizontal = x != bx && (y == by || after_x <= after_y);
        let key = if horizontal {
            (true, x.min(x + sx), y)
        } else {
            (false, x, y.min(y + sy))
        };
        if let (Ok(col), Ok(row)) = (u16::try_from(key.1), u16::try_from(key.2)) {
            out.push((key.0, col, row));
        }
        if horizontal {
            x += sx;
        } else {
            y += sy;
        }
    }
    out
}

/// The paints that wall `sides` on a map of `size`: each side once, as the wall of the cell
/// whose north or west side it is; the map's boundary is left out, being walls already. The
/// paints, and how many sides lay on the boundary.
#[must_use]
pub fn paints(size: (u16, u16), sides: &[Side]) -> (Vec<Paint>, usize) {
    let (w, h) = size;
    let unique: BTreeSet<Side> = sides.iter().copied().collect();
    let mut out = Vec::with_capacity(unique.len());
    let mut boundary = 0;
    for (horizontal, x, y) in unique {
        let on_boundary = if horizontal {
            y == 0 || y >= h
        } else {
            x == 0 || x >= w
        };
        if on_boundary || x >= w || y >= h {
            boundary += 1;
            continue;
        }
        let side = if horizontal {
            Facing::North
        } else {
            Facing::West
        };
        out.push((
            (x, y),
            Brush::Edge {
                side,
                kind: EdgeKind::Wall,
            },
        ));
    }
    (out, boundary)
}

/// The sides of a run's corners, pair by pair.
fn run_sides(corners: &[Corner]) -> impl Iterator<Item = Side> + '_ {
    corners.windows(2).flat_map(|pair| sides(pair[0], pair[1]))
}

impl Editor {
    /// The pointer's frame over the plan. With a brush: the aim under the pointer is the
    /// hover, a right click picks what is there, a left click paints a new stroke, a held
    /// button paints on when the aim moved, a button up ends the stroke. With a run tool the
    /// aim is the grid corner nearest the pointer: by corners, a click starts the run or adds
    /// the corner, and a click on the run's last corner takes it; freehand, a press starts
    /// the run, the held button adds each corner passed and letting go takes it; a right click
    /// picks the nearest side as with an edge brush. Whether a button went down on the plan
    /// (the shell takes the keyboard from a field then).
    pub fn pointer(&mut self, data: &Data, p: Pointer) -> bool {
        if !self.tool.is_run() {
            if self.corner.is_some() {
                self.corner = None;
                self.revision = self.revision.wrapping_add(1);
            }
            let target = p.pixel.and_then(|(px, py)| self.aim(px, py));
            if self.hover != target {
                self.hover = target;
            }
            let mut down = false;
            if let Some(target) = target {
                down = p.right_down || p.left == Press::Down;
                if p.right_down {
                    self.pick(target);
                } else if p.left == Press::Down {
                    self.paint(data, target, false);
                } else if p.left == Press::Held && self.stroke.is_some_and(|last| last != target) {
                    self.paint(data, target, true);
                }
            }
            if p.left == Press::Up {
                self.end_stroke();
            }
            return down;
        }
        if self.hover.is_some() {
            self.hover = None;
        }
        let size = self.size();
        let corner = p.pixel.and_then(|(px, py)| self.view.corner(size, px, py));
        if self.corner != corner {
            self.corner = corner;
            self.revision = self.revision.wrapping_add(1);
        }
        let Some(corner) = corner else {
            // A freehand drag let go off the plan is taken as drawn.
            if p.left == Press::Up && self.tool == Tool::Freehand && self.run.is_some() {
                self.take_run(data);
            }
            return false;
        };
        if p.right_down {
            if let Some(target) = p
                .pixel
                .and_then(|(px, py)| self.view.edge_target(size, px, py))
            {
                self.pick(target);
            }
            return true;
        }
        match (self.tool, p.left) {
            (Tool::Corners, Press::Down) => {
                match &mut self.run {
                    Some(run) if run.corners.last() == Some(&corner) => self.take_run(data),
                    Some(run) => run.corners.push(corner),
                    None => {
                        self.run = Some(Run {
                            corners: vec![corner],
                        })
                    }
                }
                self.revision = self.revision.wrapping_add(1);
                true
            }
            (Tool::Freehand, Press::Down) => {
                self.run = Some(Run {
                    corners: vec![corner],
                });
                self.revision = self.revision.wrapping_add(1);
                true
            }
            (Tool::Freehand, Press::Held) => {
                if let Some(run) = &mut self.run
                    && run.corners.last() != Some(&corner)
                {
                    run.corners.push(corner);
                    self.revision = self.revision.wrapping_add(1);
                }
                false
            }
            (Tool::Freehand, Press::Up) => {
                if self.run.is_some() {
                    self.take_run(data);
                }
                false
            }
            _ => false,
        }
    }

    /// Take the pending run: its sides become walls on the shown level, one undo step, each
    /// side once and the map's boundary left as it is. Says how many were walled.
    pub fn take_run(&mut self, data: &Data) {
        let Some(run) = self.run.take() else {
            return;
        };
        let all: Vec<Side> = run_sides(&run.corners).collect();
        let (walls, boundary) = paints(self.size(), &all);
        let n = walls.len();
        let level = self.level;
        let result = self
            .session
            .document_mut()
            .and_then(|d| d.paint_many(level, &walls));
        match result {
            Ok(()) => {
                let walled = match n {
                    0 => "nothing to wall".to_owned(),
                    1 => "1 side walled".to_owned(),
                    n => format!("{n} sides walled"),
                };
                self.message = Some(if boundary == 0 {
                    walled
                } else {
                    format!("{walled}, {boundary} on the boundary stay walls")
                });
                if n > 0 {
                    self.changed(data);
                }
            }
            Err(e) => self.message = Some(e.to_string()),
        }
        self.revision = self.revision.wrapping_add(1);
    }

    /// Drop the pending run's last corner; the run with none left is dropped.
    pub fn drop_corner(&mut self) {
        if let Some(run) = &mut self.run {
            run.corners.pop();
            if run.corners.is_empty() {
                self.run = None;
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Drop the pending run, changing nothing.
    pub fn drop_run(&mut self) {
        if self.run.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// What the plan previews for a run tool: the pending run's sides, by corners with the
    /// stretch from its last corner to the corner under the pointer, and that corner. None
    /// with a brush, or with nothing to show.
    #[must_use]
    pub fn preview(&self) -> Option<Preview> {
        if !self.tool.is_run() {
            return None;
        }
        let mut sides: Vec<Side> = Vec::new();
        if let Some(run) = &self.run {
            sides.extend(run_sides(&run.corners));
            if self.tool == Tool::Corners
                && let (Some(last), Some(corner)) = (run.corners.last(), self.corner)
            {
                sides.extend(self::sides(*last, corner));
            }
        }
        if sides.is_empty() && self.corner.is_none() {
            return None;
        }
        Some(Preview {
            sides,
            corner: self.corner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The corners a side joins.
    fn ends((horizontal, x, y): Side) -> (Corner, Corner) {
        if horizontal {
            ((x, y), (x + 1, y))
        } else {
            ((x, y), (x, y + 1))
        }
    }

    /// Whether `sides` is one walk from `a` to `b`, each side joined to the next.
    fn walks(sides: &[Side], a: Corner, b: Corner) -> bool {
        let mut at = a;
        for side in sides {
            let (p, q) = ends(*side);
            at = if p == at {
                q
            } else if q == at {
                p
            } else {
                return false;
            };
        }
        at == b
    }

    #[test]
    fn a_run_along_a_grid_line_is_straight_and_a_slanted_one_hugs_its_line() {
        assert_eq!(
            sides((2, 1), (6, 1)),
            vec![(true, 2, 1), (true, 3, 1), (true, 4, 1), (true, 5, 1)]
        );
        assert_eq!(
            sides((6, 3), (6, 1)),
            vec![(false, 6, 2), (false, 6, 1)],
            "a run upward is the same sides, from the far end"
        );
        assert!(sides((4, 4), (4, 4)).is_empty());
        let slant = sides((0, 0), (6, 2));
        assert_eq!(slant.len(), 8, "|dx| + |dy| sides");
        assert!(walks(&slant, (0, 0), (6, 2)), "{slant:?}");
        // The staircase: three across, down, three across, down... never two downs together.
        let downs: Vec<usize> = slant
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.0)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(downs.len(), 2);
        assert!(downs[1] > downs[0] + 1, "{slant:?}");
        let back = sides((6, 2), (0, 0));
        assert_eq!(back.len(), 8);
        assert!(walks(&back, (6, 2), (0, 0)), "{back:?}");
        let steep = sides((1, 1), (2, 5));
        assert_eq!(steep.len(), 5);
        assert!(walks(&steep, (1, 1), (2, 5)), "{steep:?}");
    }

    #[test]
    fn the_paints_wall_each_side_once_and_leave_the_boundary_alone() {
        let size = (12, 5);
        let mut run = sides((2, 1), (6, 1));
        run.extend(sides((6, 1), (6, 3)));
        run.extend(sides((6, 3), (6, 1)));
        let (walls, boundary) = paints(size, &run);
        assert_eq!(boundary, 0);
        assert_eq!(walls.len(), 6, "the backtrack is the same two sides");
        let wall = |side| Brush::Edge {
            side,
            kind: EdgeKind::Wall,
        };
        assert!(walls.contains(&((2, 1), wall(Facing::North))));
        assert!(walls.contains(&((5, 1), wall(Facing::North))));
        assert!(walls.contains(&((6, 1), wall(Facing::West))));
        assert!(walls.contains(&((6, 2), wall(Facing::West))));
        // The north boundary, the east boundary and the south boundary are walls already.
        let mut edge = sides((0, 0), (12, 0));
        edge.extend(sides((12, 0), (12, 5)));
        edge.extend(sides((12, 5), (11, 5)));
        let (walls, boundary) = paints(size, &edge);
        assert!(walls.is_empty(), "{walls:?}");
        assert_eq!(boundary, 18);
        // The sides against the boundary from inside are inside.
        let (walls, boundary) = paints(size, &sides((11, 1), (11, 4)));
        assert_eq!(boundary, 0);
        assert_eq!(walls.len(), 3);
        assert_eq!(walls[0], ((11, 1), wall(Facing::West)));
    }
}
