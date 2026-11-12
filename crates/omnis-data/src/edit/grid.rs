//! One level's grid as the editor holds it: a terrain index or no ground per tile, a kind per
//! edge, a height per tile, and the text rows the file writes. Reading a layout then writing
//! it gives the file's rows back byte for byte: the grammar (`map.rs` module docs) has one
//! spelling for every grid, corners always `+`, so the editor never reformats a map it did not
//! change. Heights are written one way (`heights`), which every fixture already uses.

use crate::edit::kinds::EditError;
use crate::levels::NO_GROUND;
use crate::limits::MAX_HEIGHT;
use omnis_core::Facing;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What an edge between two tiles is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Nothing: `' '`.
    #[default]
    Open,
    /// A wall: `-` or `|`.
    Wall,
    /// A door: `=` or `:`.
    Door,
    /// A ramp: `/`.
    Ramp,
}

impl EdgeKind {
    fn from_char(c: char, horizontal: bool) -> Option<EdgeKind> {
        match (c, horizontal) {
            (' ', _) => Some(EdgeKind::Open),
            ('-', true) | ('|', false) => Some(EdgeKind::Wall),
            ('=', true) | (':', false) => Some(EdgeKind::Door),
            ('/', _) => Some(EdgeKind::Ramp),
            _ => None,
        }
    }

    const fn to_char(self, horizontal: bool) -> char {
        match (self, horizontal) {
            (EdgeKind::Open, _) => ' ',
            (EdgeKind::Wall, true) => '-',
            (EdgeKind::Wall, false) => '|',
            (EdgeKind::Door, true) => '=',
            (EdgeKind::Door, false) => ':',
            (EdgeKind::Ramp, _) => '/',
        }
    }
}

/// One level, `width × height` tiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    width: u16,
    height: u16,
    /// Row-major; the terrain's index in the map's list, or `None` for no ground (`_`).
    tiles: Vec<Option<u8>>,
    /// `(height + 1) × width`: the edge north of tile `(x, y)` at `y * width + x`; row
    /// `height` is the south boundary.
    h_edges: Vec<EdgeKind>,
    /// `height × (width + 1)`: the edge west of tile `(x, y)` at `y * (width + 1) + x`; column
    /// `width` is the east boundary.
    v_edges: Vec<EdgeKind>,
    /// Row-major, as the file writes them: above the level's `base`.
    heights: Vec<i16>,
}

impl Grid {
    /// A grid of `fill` everywhere, walls round the boundary, open inside, flat.
    #[must_use]
    pub fn blank(width: u16, height: u16, fill: Option<u8>) -> Grid {
        let (w, h) = (usize::from(width), usize::from(height));
        let mut grid = Grid {
            width,
            height,
            tiles: vec![fill; w * h],
            h_edges: vec![EdgeKind::Open; (h + 1) * w],
            v_edges: vec![EdgeKind::Open; h * (w + 1)],
            heights: vec![0; w * h],
        };
        grid.wall_boundary();
        grid
    }

    /// The grid a file's `layout` and `heights` rows describe, `glyphs` being the map's terrain
    /// glyphs to their indices. The rows must already be the loader's (`levels::parse` accepted
    /// them); what the loader never reads, the edges beside no-ground tiles, is read here and
    /// must be an edge character too.
    ///
    /// # Errors
    /// A row of the wrong length, a glyph not in `glyphs`, a character that is not an edge, or
    /// a height that is not a number: the message names the place.
    pub fn from_text(
        width: u16,
        height: u16,
        layout: &[String],
        heights: &[String],
        glyphs: &BTreeMap<char, u8>,
    ) -> Result<Grid, String> {
        let (w, h) = (usize::from(width), usize::from(height));
        if layout.len() != 2 * h + 1 {
            return Err(format!(
                "layout has {} rows, not {}",
                layout.len(),
                2 * h + 1
            ));
        }
        let rows: Vec<Vec<char>> = layout.iter().map(|l| l.chars().collect()).collect();
        if let Some((r, row)) = rows.iter().enumerate().find(|(_, r)| r.len() != 2 * w + 1) {
            return Err(format!(
                "layout row {r} has {} characters, not {}",
                row.len(),
                2 * w + 1
            ));
        }
        let mut grid = Grid::blank(width, height, None);
        for y in 0..h {
            for x in 0..w {
                let glyph = rows[2 * y + 1][2 * x + 1];
                grid.tiles[y * w + x] =
                    if glyph == NO_GROUND {
                        None
                    } else {
                        Some(*glyphs.get(&glyph).ok_or_else(|| {
                            format!("tile ({x}, {y}) has unknown glyph '{glyph}'")
                        })?)
                    };
            }
        }
        for y in 0..=h {
            for x in 0..w {
                let c = rows[2 * y][2 * x + 1];
                grid.h_edges[y * w + x] = EdgeKind::from_char(c, true).ok_or_else(|| {
                    format!("edge row {y} column {x} has unknown character '{c}'")
                })?;
            }
        }
        for y in 0..h {
            for x in 0..=w {
                let c = rows[2 * y + 1][2 * x];
                grid.v_edges[y * (w + 1) + x] = EdgeKind::from_char(c, false).ok_or_else(|| {
                    format!("edge column {x} row {y} has unknown character '{c}'")
                })?;
            }
        }
        if !heights.is_empty() {
            if heights.len() != h {
                return Err(format!("heights has {} rows, not {h}", heights.len()));
            }
            for (y, row) in heights.iter().enumerate() {
                let words: Vec<&str> = row.split_whitespace().collect();
                if words.len() != w {
                    return Err(format!(
                        "heights row {y} has {} numbers, not {w}",
                        words.len()
                    ));
                }
                for (x, word) in words.iter().enumerate() {
                    grid.heights[y * w + x] = word
                        .parse()
                        .map_err(|_| format!("tile ({x}, {y}) height '{word}' is not a number"))?;
                }
            }
        }
        Ok(grid)
    }

    /// The file's `layout` rows, `glyphs` being the map's terrain glyphs in index order.
    #[must_use]
    pub fn layout(&self, glyphs: &[char]) -> Vec<String> {
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        let mut rows = Vec::with_capacity(2 * h + 1);
        for y in 0..=h {
            let mut row = String::with_capacity(2 * w + 1);
            row.push('+');
            for x in 0..w {
                row.push(self.h_edges[y * w + x].to_char(true));
                row.push('+');
            }
            rows.push(row);
            if y == h {
                break;
            }
            let mut row = String::with_capacity(2 * w + 1);
            for x in 0..=w {
                row.push(self.v_edges[y * (w + 1) + x].to_char(false));
                if x < w {
                    row.push(match self.tiles[y * w + x] {
                        None => NO_GROUND,
                        Some(t) => glyphs.get(usize::from(t)).copied().unwrap_or('?'),
                    });
                }
            }
            rows.push(row);
        }
        rows
    }

    /// The file's `heights` rows: none when the level is flat; otherwise one row per tile row,
    /// the numbers right-aligned to the widest on the level and one space apart, with no
    /// leading spaces.
    #[must_use]
    pub fn heights(&self) -> Vec<String> {
        if self.heights.iter().all(|&h| h == 0) {
            return Vec::new();
        }
        let w = usize::from(self.width);
        let widest = self
            .heights
            .iter()
            .map(|h| h.to_string().len())
            .max()
            .unwrap_or(1);
        self.heights
            .chunks(w.max(1))
            .map(|row| {
                row.iter()
                    .map(|h| format!("{h:>widest$}"))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim_start()
                    .to_owned()
            })
            .collect()
    }

    /// Tiles across.
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// Tiles down.
    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }

    /// Whether `(x, y)` is a tile of the grid.
    #[must_use]
    pub const fn inside(&self, x: u16, y: u16) -> bool {
        x < self.width && y < self.height
    }

    fn at(&self, x: u16, y: u16) -> Result<usize, EditError> {
        if self.inside(x, y) {
            Ok(usize::from(y) * usize::from(self.width) + usize::from(x))
        } else {
            Err(EditError::Outside { x, y })
        }
    }

    /// The tile at `(x, y)`: `Some(None)` is no ground; `None` is outside the grid.
    #[must_use]
    pub fn tile(&self, x: u16, y: u16) -> Option<Option<u8>> {
        self.at(x, y).ok().map(|i| self.tiles[i])
    }

    /// Set the tile at `(x, y)` to a terrain index, or to no ground.
    ///
    /// # Errors
    /// `Outside`.
    pub fn set_tile(&mut self, x: u16, y: u16, tile: Option<u8>) -> Result<(), EditError> {
        let i = self.at(x, y)?;
        self.tiles[i] = tile;
        Ok(())
    }

    /// Where the edge on `side` of tile `(x, y)` lives: `(horizontal, index)`.
    fn edge_at(&self, x: u16, y: u16, side: Facing) -> Result<(bool, usize), EditError> {
        self.at(x, y)?;
        let (w, x, y) = (usize::from(self.width), usize::from(x), usize::from(y));
        Ok(match side {
            Facing::North => (true, y * w + x),
            Facing::South => (true, (y + 1) * w + x),
            Facing::West => (false, y * (w + 1) + x),
            Facing::East => (false, y * (w + 1) + x + 1),
        })
    }

    /// Whether the edge on `side` of `(x, y)` is on the map's boundary.
    #[must_use]
    pub const fn on_boundary(&self, x: u16, y: u16, side: Facing) -> bool {
        match side {
            Facing::North => y == 0,
            Facing::South => y + 1 == self.height,
            Facing::West => x == 0,
            Facing::East => x + 1 == self.width,
        }
    }

    /// The edge on `side` of tile `(x, y)`; `None` outside the grid.
    #[must_use]
    pub fn edge(&self, x: u16, y: u16, side: Facing) -> Option<EdgeKind> {
        let (horizontal, i) = self.edge_at(x, y, side).ok()?;
        Some(if horizontal {
            self.h_edges[i]
        } else {
            self.v_edges[i]
        })
    }

    /// Set the edge on `side` of tile `(x, y)`. A boundary edge is always a wall.
    ///
    /// # Errors
    /// `Outside`; `BoundaryEdge` for anything but a wall on the boundary.
    pub fn set_edge(
        &mut self,
        x: u16,
        y: u16,
        side: Facing,
        kind: EdgeKind,
    ) -> Result<(), EditError> {
        let (horizontal, i) = self.edge_at(x, y, side)?;
        if self.on_boundary(x, y, side) && kind != EdgeKind::Wall {
            return Err(EditError::BoundaryEdge);
        }
        if horizontal {
            self.h_edges[i] = kind;
        } else {
            self.v_edges[i] = kind;
        }
        Ok(())
    }

    /// The height written for tile `(x, y)`, above the level's base; `None` outside.
    #[must_use]
    pub fn height_at(&self, x: u16, y: u16) -> Option<i16> {
        self.at(x, y).ok().map(|i| self.heights[i])
    }

    /// Set the height written for tile `(x, y)`.
    ///
    /// # Errors
    /// `Outside`; `OutOfRange` beyond `±MAX_HEIGHT`.
    pub fn set_height(&mut self, x: u16, y: u16, height: i16) -> Result<(), EditError> {
        let i = self.at(x, y)?;
        if !(-MAX_HEIGHT..=MAX_HEIGHT).contains(&height) {
            return Err(EditError::OutOfRange);
        }
        self.heights[i] = height;
        Ok(())
    }

    /// Resize, keeping the top-left; new tiles are `fill`, flat and open; the boundary is
    /// walled again.
    pub fn resize(&mut self, width: u16, height: u16, fill: Option<u8>) {
        let mut next = Grid::blank(width, height, fill);
        let (w, h) = (usize::from(width), usize::from(height));
        for y in 0..h.min(usize::from(self.height)) {
            for x in 0..w.min(usize::from(self.width)) {
                let (ow, i) = (usize::from(self.width), y * w + x);
                next.tiles[i] = self.tiles[y * ow + x];
                next.heights[i] = self.heights[y * ow + x];
                next.h_edges[y * w + x] = self.h_edges[y * ow + x];
                next.v_edges[y * (w + 1) + x] = self.v_edges[y * (ow + 1) + x];
            }
        }
        next.wall_boundary();
        *self = next;
    }

    /// Whether any tile is terrain `index`.
    #[must_use]
    pub fn uses(&self, index: u8) -> bool {
        self.tiles.contains(&Some(index))
    }

    /// After terrain `removed` left the map's list, every index above it moves down one.
    pub fn reindex_above(&mut self, removed: u8) {
        for tile in self.tiles.iter_mut().flatten() {
            if *tile > removed {
                *tile -= 1;
            }
        }
    }

    /// How many bytes the grid holds, for the snapshot budget.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.tiles.len() * size_of::<Option<u8>>()
            + (self.h_edges.len() + self.v_edges.len()) * size_of::<EdgeKind>()
            + self.heights.len() * size_of::<i16>()
    }

    fn wall_boundary(&mut self) {
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        for x in 0..w {
            self.h_edges[x] = EdgeKind::Wall;
            self.h_edges[h * w + x] = EdgeKind::Wall;
        }
        for y in 0..h {
            self.v_edges[y * (w + 1)] = EdgeKind::Wall;
            self.v_edges[y * (w + 1) + w] = EdgeKind::Wall;
        }
    }
}
