//! Grid geometry: absolute facings, relative moves, turns, positions, and wall edges.
//!
//! Maps are row-major with `y` growing south, so north is `-y`.

use crate::MapId;
use core::fmt;
use serde::{Deserialize, Serialize};

/// A compass direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Facing {
    /// Toward `-y`.
    North,
    /// Toward `+x`.
    East,
    /// Toward `+y`.
    South,
    /// Toward `-x`.
    West,
}

impl Facing {
    /// The four facings clockwise from north.
    pub const ALL: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

    /// The tile offset one step this way.
    #[must_use]
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Facing::North => (0, -1),
            Facing::East => (1, 0),
            Facing::South => (0, 1),
            Facing::West => (-1, 0),
        }
    }

    /// The facing after a turn.
    #[must_use]
    pub const fn rotated(self, rotation: Rotation) -> Facing {
        let quarter_turns = match rotation {
            Rotation::Right => 1,
            Rotation::Around => 2,
            Rotation::Left => 3,
        };
        Facing::ALL[(self as usize + quarter_turns) % 4]
    }

    /// The facing after a right turn.
    #[must_use]
    pub const fn right(self) -> Facing {
        self.rotated(Rotation::Right)
    }

    /// The facing after a left turn.
    #[must_use]
    pub const fn left(self) -> Facing {
        self.rotated(Rotation::Left)
    }

    /// The facing after turning around.
    #[must_use]
    pub const fn opposite(self) -> Facing {
        self.rotated(Rotation::Around)
    }

    /// The absolute facing of a relative move while facing this way.
    #[must_use]
    pub const fn toward(self, direction: Direction) -> Facing {
        match direction {
            Direction::Forward => self,
            Direction::Back => self.opposite(),
            Direction::Left => self.left(),
            Direction::Right => self.right(),
        }
    }
}

impl fmt::Display for Facing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Facing::North => "north",
            Facing::East => "east",
            Facing::South => "south",
            Facing::West => "west",
        })
    }
}

/// A turn in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rotation {
    /// A quarter turn counter-clockwise.
    Left,
    /// A quarter turn clockwise.
    Right,
    /// A half turn.
    Around,
}

/// A move relative to the current facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Direction {
    /// The tile ahead.
    Forward,
    /// The tile behind, without turning.
    Back,
    /// Sidestep to the left.
    Left,
    /// Sidestep to the right.
    Right,
}

/// Where the party stands and which way it faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Position {
    /// The map.
    pub map: MapId,
    /// Column, growing east.
    pub x: u16,
    /// Row, growing south.
    pub y: u16,
    /// The direction the party looks.
    pub facing: Facing,
}

impl Position {
    /// The coordinates one tile toward `facing`, or `None` when that leaves the coordinate
    /// space. Map bounds are the simulation's to check.
    #[must_use]
    pub fn neighbour(&self, facing: Facing) -> Option<(u16, u16)> {
        let (dx, dy) = facing.delta();
        let x = i32::from(self.x) + dx;
        let y = i32::from(self.y) + dy;
        Some((u16::try_from(x).ok()?, u16::try_from(y).ok()?))
    }

    /// The same tile facing another way.
    #[must_use]
    pub const fn turned(self, rotation: Rotation) -> Position {
        Position {
            facing: self.facing.rotated(rotation),
            ..self
        }
    }

    /// The same facing on another tile.
    #[must_use]
    pub const fn at(self, x: u16, y: u16) -> Position {
        Position { x, y, ..self }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {}) facing {}",
            self.map, self.x, self.y, self.facing
        )
    }
}

/// A set of tile edges, one bit per facing. A wall mask is the edges that block movement.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Edges(pub u8);

impl Edges {
    /// No edges.
    pub const NONE: Edges = Edges(0);
    /// All four edges.
    pub const ALL: Edges = Edges(0b1111);

    const fn bit(facing: Facing) -> u8 {
        1 << (facing as u8)
    }

    /// Whether the edge toward `facing` is in the set.
    #[must_use]
    pub const fn has(self, facing: Facing) -> bool {
        self.0 & Self::bit(facing) != 0
    }

    /// This set plus the edge toward `facing`.
    #[must_use]
    pub const fn with(self, facing: Facing) -> Edges {
        Edges(self.0 | Self::bit(facing))
    }

    /// This set minus the edge toward `facing`.
    #[must_use]
    pub const fn without(self, facing: Facing) -> Edges {
        Edges(self.0 & !Self::bit(facing))
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Edges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (facing, letter) in Facing::ALL.iter().zip(['N', 'E', 'S', 'W']) {
            write!(f, "{}", if self.has(*facing) { letter } else { '-' })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn turns_compose() {
        for facing in Facing::ALL {
            assert_eq!(facing.left().right(), facing);
            assert_eq!(facing.right().right(), facing.opposite());
            assert_eq!(facing.opposite().opposite(), facing);
            assert_eq!(facing.left().left().left().left(), facing);
        }
        assert_eq!(Facing::North.right(), Facing::East);
        assert_eq!(Facing::North.left(), Facing::West);
        assert_eq!(Facing::West.toward(Direction::Left), Facing::South);
        assert_eq!(Facing::West.toward(Direction::Back), Facing::East);
    }

    #[test]
    fn neighbour_refuses_negative_coordinates() {
        let origin = Position {
            map: MapId(0),
            x: 0,
            y: 0,
            facing: Facing::North,
        };
        assert_eq!(origin.neighbour(Facing::North), None);
        assert_eq!(origin.neighbour(Facing::West), None);
        assert_eq!(origin.neighbour(Facing::South), Some((0, 1)));
        assert_eq!(origin.neighbour(Facing::East), Some((1, 0)));
        let far = origin.at(u16::MAX, u16::MAX);
        assert_eq!(far.neighbour(Facing::East), None);
        assert_eq!(far.neighbour(Facing::North), Some((u16::MAX, u16::MAX - 1)));
    }

    #[test]
    fn edges_are_a_bit_set() {
        let e = Edges::NONE.with(Facing::North).with(Facing::West);
        assert!(e.has(Facing::North) && e.has(Facing::West));
        assert!(!e.has(Facing::East) && !e.has(Facing::South));
        assert_eq!(e.without(Facing::North).without(Facing::West), Edges::NONE);
        assert_eq!(Edges::ALL.0, 15);
        assert_eq!(format!("{e}"), "N--W");
        assert_eq!(format!("{}", Edges::ALL), "NESW");
    }
}
