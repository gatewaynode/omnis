//! Which way each stair climbs (M8.5 step 1c): a map lists its stairs with the way each rises,
//! and every stair up or down must be listed. A stair up is taken by stepping onto it from its
//! foot, moving the way it rises; a stair down from its head, moving against it. Here the list
//! is checked against the levels and each level's `rises` filled in.

use crate::error::DataError;
use crate::levels::{LevelData, beyond};
use crate::map::{Join, MapDef};
use omnis_core::{Facing, Level};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One stair's direction, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StairDef {
    /// The level the stair is on; 0, the map's own, when left out.
    #[serde(default)]
    pub level: i8,
    /// Its column.
    pub x: u16,
    /// Its row.
    pub y: u16,
    /// The way the flight climbs: its foot is on the other side, its head on this one.
    pub rise: Facing,
}

/// "level 2: " for a level other than the map's own, else nothing.
fn label(level: Level) -> String {
    if level.is_ground() {
        String::new()
    } else {
        format!("level {level}: ")
    }
}

/// Check `def.stairs` against `levels` and fill in each level's `rises`. Every problem is
/// reported.
pub fn check(def: &MapDef, levels: &mut [LevelData], file: &Path, errors: &mut Vec<DataError>) {
    let mut err = |message: String| errors.push(DataError::new(file, message));
    let width = usize::from(def.width);
    let join_at = |levels: &[LevelData], level: Level, x: u16, y: u16| {
        let l = levels.iter().find(|l| l.level == level)?;
        if x >= def.width || y >= def.height {
            return None;
        }
        let cell = l.cells[usize::from(y) * width + usize::from(x)]?;
        def.terrains[usize::from(cell.terrain)].join
    };
    let mut rises: BTreeMap<(Level, u16, u16), Facing> = BTreeMap::new();
    for s in &def.stairs {
        let level = Level(s.level);
        let at = format!("{}the stair entry at ({}, {})", label(level), s.x, s.y);
        if !levels.iter().any(|l| l.level == level) {
            err(format!("{at} is on a level the map does not have"));
        } else if s.x >= def.width || s.y >= def.height {
            err(format!("{at} is outside the map"));
        } else if !matches!(
            join_at(levels, level, s.x, s.y),
            Some(Join::Up | Join::Down)
        ) {
            err(format!("{at} is not on a stair"));
        } else if rises.insert((level, s.x, s.y), s.rise).is_some() {
            err(format!("{at} is written twice"));
        }
    }
    for level in levels.iter() {
        for (i, cell) in level.cells.iter().enumerate() {
            let Some(cell) = cell else { continue };
            let join = def.terrains[usize::from(cell.terrain)].join;
            if !matches!(join, Some(Join::Up | Join::Down)) {
                continue;
            }
            // A tile index fits the map's u16 sides.
            #[allow(clippy::cast_possible_truncation)]
            let (x, y) = ((i % width) as u16, (i / width) as u16);
            let what = if join == Some(Join::Up) {
                "stair up"
            } else {
                "stair down"
            };
            let at = format!("{}the {what} at ({x}, {y})", label(level.level));
            let Some(&rise) = rises.get(&(level.level, x, y)) else {
                err(format!(
                    "{at} has no entry in `stairs` saying which way it rises"
                ));
                continue;
            };
            // A stair up is stepped onto from its foot, a stair down from its head.
            let (from, side) = if join == Some(Join::Up) {
                (rise.opposite(), "foot")
            } else {
                (rise, "head")
            };
            if let Err(why) = approach(def, level, x, y, from) {
                err(format!("{at} rises {rise:?}, but its {side} {why}"));
            }
            // A pair is checked once, from its stair up.
            let other = beyond(levels, level.level, true).map(|l| l.level);
            if join == Some(Join::Up)
                && let Some(other) = other
                && join_at(levels, other, x, y) == Some(Join::Down)
                && let Some(&theirs) = rises.get(&(other, x, y))
                && theirs != rise
            {
                err(format!(
                    "{at} rises {rise:?}, but the stair it joins on level {other} rises {theirs:?}; \
                     they are one flight"
                ));
            }
        }
    }
    for level in levels.iter_mut() {
        level.rises = rises
            .iter()
            .filter(|((l, _, _), _)| *l == level.level)
            .map(|(&(_, x, y), &rise)| ((x, y), rise))
            .collect();
    }
}

/// Whether the tile beside `(x, y)` on its `from` side can step onto it: on the map, ground,
/// enterable, and no wall or door on the edge between.
fn approach(
    def: &MapDef,
    level: &LevelData,
    x: u16,
    y: u16,
    from: Facing,
) -> Result<(), &'static str> {
    let (dx, dy) = from.delta();
    let (nx, ny) = (i32::from(x) + dx, i32::from(y) + dy);
    let (Ok(nx), Ok(ny)) = (u16::try_from(nx), u16::try_from(ny)) else {
        return Err("is off the map");
    };
    if nx >= def.width || ny >= def.height {
        return Err("is off the map");
    }
    let width = usize::from(def.width);
    let here = level.cells[usize::from(y) * width + usize::from(x)].ok_or("is off the map")?;
    if here.walls.has(from) || here.doors.has(from) {
        return Err("is behind a wall or a door");
    }
    match level.cells[usize::from(ny) * width + usize::from(nx)] {
        None => Err("is no ground"),
        Some(c) if !def.terrains[usize::from(c.terrain)].passable => {
            Err("is a tile the party cannot enter")
        }
        Some(_) => Ok(()),
    }
}
