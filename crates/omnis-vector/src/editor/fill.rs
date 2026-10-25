//! The fill tool (the owner's drawing-GUI column, step 2, 2026-10-25): a click paints the
//! palette's entry over the region of one ground the cell belongs to, the cells of the same
//! terrain (or of no ground) joined through open sides; a wall, a door or a ramp bounds it,
//! as the owner chose. One undo step (`Document::paint_many`). Bevy-free.

use super::Editor;
use super::brush::{Entry, Target};
use super::run::Paint;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::{EdgeKind, Grid};
use std::collections::{BTreeSet, VecDeque};

/// The cells of `grid` that share `start`'s ground and are joined to it through open sides:
/// the same terrain index, or no ground with no ground. `start` first; empty off the grid.
#[must_use]
pub fn region(grid: &Grid, start: (u16, u16)) -> Vec<(u16, u16)> {
    let Some(ground) = grid.tile(start.0, start.1) else {
        return Vec::new();
    };
    let mut taken = BTreeSet::from([start]);
    let mut out = vec![start];
    let mut queue = VecDeque::from([start]);
    while let Some((x, y)) = queue.pop_front() {
        for side in [Facing::North, Facing::East, Facing::South, Facing::West] {
            if grid.edge(x, y, side) != Some(EdgeKind::Open) {
                continue;
            }
            let (dx, dy) = side.delta();
            let next = (
                u16::try_from(i32::from(x) + dx).ok(),
                u16::try_from(i32::from(y) + dy).ok(),
            );
            let (Some(nx), Some(ny)) = next else {
                continue;
            };
            if grid.tile(nx, ny) == Some(ground) && taken.insert((nx, ny)) {
                out.push((nx, ny));
                queue.push_back((nx, ny));
            }
        }
    }
    out
}

impl Editor {
    /// Fill the region `cell` belongs to with the palette's entry, one step, and put the cursor
    /// on the cell: a terrain over every cell of the region (rising the way faced, as the brush
    /// paints), or the height held. A terrain the region already is fills nothing and says so.
    /// Whether the map changed.
    pub fn fill(&mut self, data: &Data, cell: (u16, u16)) -> bool {
        let Some(def) = self.def() else {
            self.message = Some("open a map first".to_owned());
            return false;
        };
        let Some(grid) = self.grid() else {
            return false;
        };
        let Some(ground) = grid.tile(cell.0, cell.1) else {
            return false;
        };
        let entry = self.entry;
        let cells = region(grid, cell);
        let what = match entry {
            Entry::Terrain(i) if Some(i) == ground => {
                self.message = Some(format!("already {}", entry.label(&def)));
                self.point(cell);
                return false;
            }
            Entry::Terrain(_) => format!("filled with {}", entry.label(&def)),
            Entry::Height => format!("set to height {}", self.eighths),
            Entry::Edge(_) | Entry::Stamp(_) => {
                self.message = Some("the fill takes a terrain or the height".to_owned());
                return false;
            }
        };
        let (facing, eighths) = (self.facing, self.eighths);
        let paints: Vec<Paint> = cells
            .iter()
            .filter_map(|&c| {
                let target = Target {
                    cell: c,
                    edge: None,
                };
                Some((c, entry.brush(&def, target, facing, eighths)?))
            })
            .collect();
        let n = paints.len();
        let level = self.level;
        let result = self
            .session
            .document_mut()
            .and_then(|d| d.paint_many(level, &paints));
        if !self.took(data, result) {
            return false;
        }
        self.cursor = cell;
        let cells = if n == 1 { "cell" } else { "cells" };
        self.message = Some(format!("{n} {cells} {what}"));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4 by 3 grid: the left two columns parted from the right two by a wall, a door and a
    /// ramp, row by row; (3, 0) another terrain; (0, 2) no ground.
    fn parted() -> Grid {
        let mut grid = Grid::blank(4, 3, Some(0));
        grid.set_tile(3, 0, Some(1)).unwrap();
        grid.set_tile(0, 2, None).unwrap();
        grid.set_edge(1, 0, Facing::East, EdgeKind::Wall).unwrap();
        grid.set_edge(1, 1, Facing::East, EdgeKind::Door).unwrap();
        grid.set_edge(1, 2, Facing::East, EdgeKind::Ramp).unwrap();
        grid
    }

    fn sorted(mut cells: Vec<(u16, u16)>) -> Vec<(u16, u16)> {
        cells.sort_unstable();
        cells
    }

    #[test]
    fn a_region_is_one_ground_joined_through_open_sides_and_any_closed_side_bounds_it() {
        let grid = parted();
        assert_eq!(
            sorted(region(&grid, (0, 0))),
            vec![(0, 0), (0, 1), (1, 0), (1, 1), (1, 2)],
            "the left, less the no-ground cell"
        );
        assert_eq!(region(&grid, (0, 0))[0], (0, 0), "the start first");
        assert_eq!(
            sorted(region(&grid, (2, 0))),
            vec![(2, 0), (2, 1), (2, 2), (3, 1), (3, 2)],
            "the right, less the other terrain"
        );
        assert_eq!(region(&grid, (3, 0)), vec![(3, 0)]);
        assert_eq!(
            region(&grid, (0, 2)),
            vec![(0, 2)],
            "no ground is a region of its own"
        );
        assert!(region(&grid, (9, 9)).is_empty());
        // Opening the door joins the two.
        let mut open = grid;
        open.set_edge(1, 1, Facing::East, EdgeKind::Open).unwrap();
        assert_eq!(region(&open, (0, 0)).len(), 10);
        assert_eq!(region(&open, (3, 2)).len(), 10);
    }
}
