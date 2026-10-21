//! Levels in one map (M8.5 step 1b): the map's own `layout` and `heights` are level 0, and
//! `MapDef.levels` adds levels above (positive) and below (negative), each a whole grid of the
//! map's size with a `base` height added to its tiles. A tile written `_` has no ground on that
//! level: air above the ground, rock below; the party cannot stand there, and it hides nothing.
//!
//! Levels join through terrains with a `join`: a stair up or down (stepped onto), a ladder
//! (climbed), a hole (dropped through). Each lands on the same column and row of the next level
//! that way, and the loader checks that there is ground to land on.

use crate::error::DataError;
use crate::limits::{MAX_HEIGHT, MAX_LEVELS, MAX_RAMP, string_fits};
use crate::map::{Cell, Join, MapDef};
use omnis_core::{Edges, Facing, Level};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The tile glyph for no ground on a level.
pub const NO_GROUND: char = '_';

/// A level beside the map's own, as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelDef {
    /// Its number: above the ground is positive, below negative; never 0 (the map's own).
    pub level: i8,
    /// Its name's text key, for the interface (`pack:text:map.tower.roof`).
    pub name: String,
    /// The height added to every tile, in eighths of a cell.
    #[serde(default)]
    pub base: i16,
    /// The grid, written as the map's `layout` is, at the map's size.
    pub layout: Vec<String>,
    /// Tile heights above `base`, as the map's `heights`; empty is all 0.
    #[serde(default)]
    pub heights: Vec<String>,
}

/// One level after its grid is parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelData {
    /// Its number.
    pub level: Level,
    /// Its name's text key; empty for the map's own level, whose name is the map's.
    pub name: String,
    /// Row-major cells, `width × height`; `None` where there is no ground.
    pub cells: Vec<Option<Cell>>,
    /// The way each stair on the level rises, by column and row (M8.5 step 1c).
    pub rises: BTreeMap<(u16, u16), Facing>,
}

/// The grid of one level, borrowed from the file.
struct Grid<'a> {
    layout: &'a [String],
    heights: &'a [String],
    base: i16,
    /// Put before each message for a level other than the map's own: "level 2: ".
    label: String,
}

/// Every level of `def`, lowest first, the map's own among them. Every problem is reported;
/// the levels are returned only when there were none.
pub fn parse(def: &MapDef, file: &Path, errors: &mut Vec<DataError>) -> Option<Vec<LevelData>> {
    let before = errors.len();
    check_numbers(def, file, errors);
    let mut grids = vec![(
        Level::GROUND,
        String::new(),
        Grid {
            layout: &def.layout,
            heights: &def.heights,
            base: 0,
            label: String::new(),
        },
    )];
    for l in &def.levels {
        grids.push((
            Level(l.level),
            l.name.clone(),
            Grid {
                layout: &l.layout,
                heights: &l.heights,
                base: l.base,
                label: format!("level {}: ", l.level),
            },
        ));
    }
    grids.sort_by_key(|(level, _, _)| *level);
    let mut levels = Vec::with_capacity(grids.len());
    for (level, name, grid) in &grids {
        if let Some(cells) = cells(def, grid, file, errors) {
            levels.push(LevelData {
                level: *level,
                name: name.clone(),
                cells,
                rises: BTreeMap::new(),
            });
        }
    }
    if errors.len() > before {
        return None;
    }
    check_joins(def, &levels, file, errors);
    if errors.len() == before {
        crate::stairs::check(def, &mut levels, file, errors);
    }
    (errors.len() == before).then_some(levels)
}

/// Level numbers: never 0, never twice, at most [`MAX_LEVELS`] with the map's own, names that
/// fit.
fn check_numbers(def: &MapDef, file: &Path, errors: &mut Vec<DataError>) {
    if def.levels.len() + 1 > MAX_LEVELS {
        errors.push(DataError::new(
            file,
            format!("a map may have at most {MAX_LEVELS} levels, its own among them"),
        ));
    }
    let mut seen = BTreeSet::new();
    for l in &def.levels {
        if l.level == 0 {
            errors.push(DataError::new(
                file,
                "level 0 is the map's own layout; another level needs another number",
            ));
        } else if !seen.insert(l.level) {
            errors.push(DataError::new(
                file,
                format!("level {} is written twice", l.level),
            ));
        }
        if !string_fits(&l.name) {
            errors.push(DataError::new(
                file,
                format!("level {} has a name over the size limit", l.level),
            ));
        }
    }
}

/// The level after `level` going up (`up`) or down, if the map has one.
#[must_use]
pub fn beyond(levels: &[LevelData], level: Level, up: bool) -> Option<&LevelData> {
    if up {
        levels.iter().find(|l| l.level > level)
    } else {
        levels.iter().rev().find(|l| l.level < level)
    }
}

/// Every stair, ladder and hole lands on ground the party can enter on the next level its way.
fn check_joins(def: &MapDef, levels: &[LevelData], file: &Path, errors: &mut Vec<DataError>) {
    let width = usize::from(def.width);
    let lands = |level: &LevelData, up: bool, i: usize| -> Result<(), &'static str> {
        let Some(next) = beyond(levels, level.level, up) else {
            return Err(if up {
                "has no level above"
            } else {
                "has no level below"
            });
        };
        match next.cells.get(i).copied().flatten() {
            None => Err("lands where there is no ground"),
            Some(c) if !def.terrains[usize::from(c.terrain)].passable => {
                Err("lands on a tile the party cannot enter")
            }
            Some(_) => Ok(()),
        }
    };
    for level in levels {
        for (i, cell) in level.cells.iter().enumerate() {
            let Some(cell) = cell else { continue };
            let Some(join) = def.terrains[usize::from(cell.terrain)].join else {
                continue;
            };
            let (x, y) = (i % width, i / width);
            let (what, result) = match join {
                Join::Up => ("stair up", lands(level, true, i)),
                Join::Down => ("stair down", lands(level, false, i)),
                Join::Hole => ("hole", lands(level, false, i)),
                Join::Ladder => (
                    "ladder",
                    lands(level, true, i)
                        .or_else(|_| lands(level, false, i))
                        .map_err(|_| "leads to no ground above or below"),
                ),
            };
            if let Err(why) = result {
                let label = if level.level.is_ground() {
                    String::new()
                } else {
                    format!("level {}: ", level.level)
                };
                errors.push(DataError::new(
                    file,
                    format!("{label}the {what} at ({x}, {y}) {why}"),
                ));
            }
        }
    }
}

/// Parse one level's grid into cells, row-major, `None` where there is no ground. Every
/// problem is reported; cells are returned only when there were none.
fn cells(
    def: &MapDef,
    grid: &Grid<'_>,
    file: &Path,
    errors: &mut Vec<DataError>,
) -> Option<Vec<Option<Cell>>> {
    let before = errors.len();
    let label = &grid.label;
    let width = usize::from(def.width);
    let height = usize::from(def.height);
    let glyphs: BTreeMap<char, u8> = def
        .terrains
        .iter()
        .enumerate()
        .filter_map(|(i, t)| u8::try_from(i).ok().map(|i| (t.glyph, i)))
        .collect();
    if grid.layout.len() != 2 * height + 1 {
        errors.push(DataError::new(
            file,
            format!(
                "{label}layout has {} rows; a {}-tile-high map needs {}",
                grid.layout.len(),
                height,
                2 * height + 1
            ),
        ));
        return None;
    }
    let mut rows: Vec<Vec<char>> = Vec::with_capacity(grid.layout.len());
    for (r, line) in grid.layout.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() != 2 * width + 1 {
            errors.push(DataError::new(
                file,
                format!(
                    "{label}layout row {r} has {} characters; a {}-tile-wide map needs {}",
                    chars.len(),
                    width,
                    2 * width + 1
                ),
            ));
        }
        rows.push(chars);
    }
    if errors.len() > before {
        return None;
    }
    // A bad `heights` is reported and read as flat, so the tiles are still checked.
    let heights =
        heights(def, grid, file, errors).unwrap_or_else(|| vec![grid.base; width * height]);
    let mut cells = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let glyph = rows[2 * y + 1][2 * x + 1];
            if glyph == NO_GROUND {
                cells.push(None);
                continue;
            }
            let Some(&terrain) = glyphs.get(&glyph) else {
                errors.push(DataError::new(
                    file,
                    format!("{label}tile ({x}, {y}) has unknown glyph '{glyph}'"),
                ));
                continue;
            };
            let mut walls = Edges::NONE;
            let mut doors = Edges::NONE;
            let mut ramps = Edges::NONE;
            let edges = [
                (Facing::North, rows[2 * y][2 * x + 1], true),
                (Facing::South, rows[2 * y + 2][2 * x + 1], true),
                (Facing::West, rows[2 * y + 1][2 * x], false),
                (Facing::East, rows[2 * y + 1][2 * x + 2], false),
            ];
            for (facing, c, horizontal) in edges {
                let boundary = match facing {
                    Facing::North => y == 0,
                    Facing::South => y + 1 == height,
                    Facing::West => x == 0,
                    Facing::East => x + 1 == width,
                };
                match (c, horizontal) {
                    ('-', true) | ('|', false) => walls = walls.with(facing),
                    ('=', true) | (':', false) => doors = doors.with(facing),
                    ('/', _) => ramps = ramps.with(facing),
                    (' ', _) => {}
                    _ => errors.push(DataError::new(
                        file,
                        format!("{label}tile ({x}, {y}) {facing} edge has unknown character '{c}'"),
                    )),
                }
                if boundary && !walls.has(facing) {
                    errors.push(DataError::new(
                        file,
                        format!(
                            "{label}tile ({x}, {y}) {facing} edge is on the map boundary and must be a wall"
                        ),
                    ));
                }
            }
            cells.push(Some(Cell {
                terrain,
                walls,
                doors,
                height: heights[y * width + x],
                ramps,
                cliffs: Edges::NONE,
            }));
        }
    }
    if errors.len() > before {
        return None;
    }
    derive_cliffs(def, label, &mut cells, file, errors);
    (errors.len() == before).then_some(cells)
}

/// Parse a grid's `heights` above its `base`: none is all `base`; otherwise one row per tile
/// row, one integer per tile, the sum within `±MAX_HEIGHT`.
fn heights(
    def: &MapDef,
    grid: &Grid<'_>,
    file: &Path,
    errors: &mut Vec<DataError>,
) -> Option<Vec<i16>> {
    let label = &grid.label;
    let width = usize::from(def.width);
    let height = usize::from(def.height);
    let in_range = |h: i32| {
        i16::try_from(h)
            .ok()
            .filter(|h| (-MAX_HEIGHT..=MAX_HEIGHT).contains(h))
    };
    if grid.heights.is_empty() {
        return match in_range(i32::from(grid.base)) {
            Some(h) => Some(vec![h; width * height]),
            None => {
                errors.push(DataError::new(
                    file,
                    format!(
                        "{label}base {} must be within -{MAX_HEIGHT}..={MAX_HEIGHT}",
                        grid.base
                    ),
                ));
                None
            }
        };
    }
    if grid.heights.len() != height {
        errors.push(DataError::new(
            file,
            format!(
                "{label}heights has {} rows; a {height}-tile-high map needs {height}",
                grid.heights.len()
            ),
        ));
        return None;
    }
    let before = errors.len();
    let mut out = Vec::with_capacity(width * height);
    for (y, row) in grid.heights.iter().enumerate() {
        let words: Vec<&str> = row.split_whitespace().collect();
        if words.len() != width {
            errors.push(DataError::new(
                file,
                format!(
                    "{label}heights row {y} has {} numbers; a {width}-tile-wide map needs {width}",
                    words.len()
                ),
            ));
            continue;
        }
        for (x, word) in words.into_iter().enumerate() {
            match word
                .parse::<i16>()
                .ok()
                .and_then(|h| in_range(i32::from(h) + i32::from(grid.base)))
            {
                Some(h) => out.push(h),
                None => errors.push(DataError::new(
                    file,
                    format!(
                        "{label}tile ({x}, {y}) height '{word}' must be a whole number within \
                         -{MAX_HEIGHT}..={MAX_HEIGHT}"
                    ),
                )),
            }
        }
    }
    (errors.len() == before).then_some(out)
}

/// Mark each side a step may not cross for height, and refuse a ramp that joins more than
/// [`MAX_RAMP`]. A ramp is reported once, from its north or west tile. A side toward no
/// ground has no cliff: nothing stands there to climb to.
fn derive_cliffs(
    def: &MapDef,
    label: &str,
    cells: &mut [Option<Cell>],
    file: &Path,
    errors: &mut Vec<DataError>,
) {
    let width = usize::from(def.width);
    let height = usize::from(def.height);
    for y in 0..height {
        for x in 0..width {
            let Some(here) = cells[y * width + x] else {
                continue;
            };
            for facing in [Facing::North, Facing::South, Facing::West, Facing::East] {
                let beside = match facing {
                    Facing::North => y.checked_sub(1).map(|y| (x, y)),
                    Facing::South => (y + 1 < height).then_some((x, y + 1)),
                    Facing::West => x.checked_sub(1).map(|x| (x, y)),
                    Facing::East => (x + 1 < width).then_some((x + 1, y)),
                };
                let Some(there) = beside.and_then(|(bx, by)| cells[by * width + bx]) else {
                    continue;
                };
                let rise = (i32::from(there.height) - i32::from(here.height)).abs();
                if here.ramps.has(facing) {
                    if rise > MAX_RAMP && matches!(facing, Facing::North | Facing::West) {
                        errors.push(DataError::new(
                            file,
                            format!(
                                "{label}tile ({x}, {y}) {facing} ramp joins heights {} and {}, \
                                 more than {MAX_RAMP} apart",
                                here.height, there.height
                            ),
                        ));
                    }
                    continue;
                }
                let climb = def
                    .terrains
                    .get(usize::from(there.terrain))
                    .map_or(0, |t| i32::from(t.climb));
                if rise > climb
                    && let Some(cell) = cells[y * width + x].as_mut()
                {
                    cell.cliffs = cell.cliffs.with(facing);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{MapKind, Terrain, WallSurfaces};

    fn terrain(glyph: char, passable: bool, join: Option<Join>) -> Terrain {
        Terrain {
            glyph,
            name: format!("t{glyph}"),
            floor: "floor".into(),
            ceiling: None,
            block: None,
            passable,
            opaque: false,
            color: (128, 128, 128),
            visibility_depth: 4,
            step_minutes: 1,
            climb: 2,
            join,
        }
    }

    /// A one-row map as wide as `ground` is written (`|. .|` is two tiles): `.` floor, `#` rock,
    /// `u` stair up, `d` stair down, `l` ladder, `h` hole.
    fn map(ground: &str, levels: &[(i8, i16, &str)]) -> MapDef {
        let width = u16::try_from(ground.len() / 2).expect("a narrow map");
        let edge = format!("+{}", "-+".repeat(usize::from(width)));
        let row = |tiles: &str| vec![edge.clone(), tiles.to_owned(), edge.clone()];
        MapDef {
            schema: 1,
            id: "t:map:tower".into(),
            name: "t:text:tower".into(),
            kind: MapKind::Dungeon,
            tileset: "t:tileset:d".into(),
            width,
            height: 1,
            start: (0, 0, Facing::East),
            wall: WallSurfaces {
                front: "wall".into(),
                left: "wall.left".into(),
                right: "wall.right".into(),
            },
            door: "door".into(),
            door_open: None,
            terrains: vec![
                terrain('.', true, None),
                terrain('#', false, None),
                terrain('u', true, Some(Join::Up)),
                terrain('d', true, Some(Join::Down)),
                terrain('l', true, Some(Join::Ladder)),
                terrain('h', true, Some(Join::Hole)),
            ],
            layout: row(ground),
            heights: vec![],
            levels: levels
                .iter()
                .map(|&(level, base, tiles)| LevelDef {
                    level,
                    name: format!("L{level}"),
                    base,
                    layout: row(tiles),
                    heights: vec![],
                })
                .collect(),
            stairs: vec![],
            portals: vec![],
            encounters: vec![],
            random: None,
            sites: vec![],
            rest_events: vec![],
            objects: vec![],
            ground: vec![],
            lore: vec![],
        }
    }

    fn errors_of(def: &MapDef) -> Vec<String> {
        let mut errors = vec![];
        let levels = parse(def, Path::new("m"), &mut errors);
        assert_eq!(levels.is_none(), !errors.is_empty());
        errors.into_iter().map(|e| e.message).collect()
    }

    #[test]
    fn levels_sit_at_their_base_lowest_first_with_no_ground_where_written() {
        let def = map("|. .|", &[(1, 8, "|. _|"), (-1, -8, "|. .|")]);
        let mut errors = vec![];
        let levels = parse(&def, Path::new("m"), &mut errors).expect("valid");
        assert!(errors.is_empty(), "{errors:?}");
        let numbers: Vec<i8> = levels.iter().map(|l| l.level.0).collect();
        assert_eq!(numbers, [-1, 0, 1]);
        assert_eq!(levels[0].cells[0].map(|c| c.height), Some(-8));
        assert_eq!(levels[2].cells[0].map(|c| c.height), Some(8));
        assert_eq!(levels[2].cells[1], None, "no ground");
        assert_eq!(levels[2].name, "L1");
        assert_eq!(
            beyond(&levels, Level(-1), true).map(|l| l.level),
            Some(Level(0))
        );
        assert_eq!(beyond(&levels, Level(1), true).map(|l| l.level), None);
    }

    #[test]
    fn every_join_needs_ground_to_land_on() {
        assert_eq!(
            errors_of(&map("|u .|", &[])),
            ["the stair up at (0, 0) has no level above"]
        );
        assert!(
            errors_of(&map("|. .|", &[(1, 8, "|h .|")])).is_empty(),
            "a hole onto the floor below is fine"
        );
        assert_eq!(
            errors_of(&map("|# .|", &[(1, 8, "|h .|")])),
            ["level 1: the hole at (0, 0) lands on a tile the party cannot enter"]
        );
        assert_eq!(
            errors_of(&map("|u .|", &[(1, 8, "|_ .|")])),
            ["the stair up at (0, 0) lands where there is no ground"]
        );
        assert_eq!(
            errors_of(&map("|d .|", &[])),
            ["the stair down at (0, 0) has no level below"]
        );
        assert_eq!(
            errors_of(&map("|l .|", &[])),
            ["the ladder at (0, 0) leads to no ground above or below"]
        );
        assert!(
            errors_of(&map("|l .|", &[(-1, -8, "|l .|")])).is_empty(),
            "down is enough"
        );
    }

    #[test]
    fn level_numbers_sizes_and_bases_are_checked() {
        assert_eq!(
            errors_of(&map("|. .|", &[(0, 0, "|. .|")])),
            ["level 0 is the map's own layout; another level needs another number"]
        );
        assert_eq!(
            errors_of(&map("|. .|", &[(1, 8, "|. .|"), (1, 16, "|. .|")])),
            ["level 1 is written twice"]
        );
        assert_eq!(
            errors_of(&map("|. .|", &[(2, 8, "|. . .|")])),
            ["level 2: layout row 1 has 7 characters; a 2-tile-wide map needs 5"]
        );
        assert_eq!(
            errors_of(&map("|. .|", &[(2, 5000, "|. .|")])),
            ["level 2: base 5000 must be within -4096..=4096"]
        );
        let many: Vec<(i8, i16, &str)> = (1..=16).map(|l| (l, 0, "|. .|")).collect();
        assert_eq!(
            errors_of(&map("|. .|", &many)),
            ["a map may have at most 16 levels, its own among them"]
        );
    }

    /// `def` with these stairs: `(level, x, y, rise)`.
    fn stairs(mut def: MapDef, list: &[(i8, u16, u16, Facing)]) -> MapDef {
        def.stairs = list
            .iter()
            .map(|&(level, x, y, rise)| crate::stairs::StairDef { level, x, y, rise })
            .collect();
        def
    }

    #[test]
    fn a_stair_pair_rising_one_way_is_listed_on_its_levels() {
        let def = stairs(
            map("|. u .|", &[(1, 8, "|. d .|")]),
            &[(0, 1, 0, Facing::East), (1, 1, 0, Facing::East)],
        );
        let mut errors = vec![];
        let levels = parse(&def, Path::new("m"), &mut errors).expect("valid");
        let rise = |l: i8| {
            levels
                .iter()
                .find(|d| d.level == Level(l))
                .map(|d| d.rises.clone())
        };
        assert_eq!(rise(0), Some(BTreeMap::from([((1, 0), Facing::East)])));
        assert_eq!(rise(1), Some(BTreeMap::from([((1, 0), Facing::East)])));
    }

    #[test]
    fn every_stair_is_listed_once_and_every_entry_is_a_stair() {
        let pair = |list: &[(i8, u16, u16, Facing)]| {
            errors_of(&stairs(map("|. u .|", &[(1, 8, "|. d .|")]), list))
        };
        let (up, down) = ((0, 1, 0, Facing::East), (1, 1, 0, Facing::East));
        assert_eq!(
            pair(&[up]),
            [
                "level 1: the stair down at (1, 0) has no entry in `stairs` saying which way it rises"
            ]
        );
        assert_eq!(
            pair(&[up, down, (0, 0, 0, Facing::East)]),
            ["the stair entry at (0, 0) is not on a stair"]
        );
        assert_eq!(
            pair(&[up, down, up]),
            ["the stair entry at (1, 0) is written twice"]
        );
        assert_eq!(
            pair(&[up, down, (5, 1, 0, Facing::East)]),
            ["level 5: the stair entry at (1, 0) is on a level the map does not have"]
        );
        assert_eq!(
            pair(&[up, down, (0, 9, 0, Facing::East)]),
            ["the stair entry at (9, 0) is outside the map"]
        );
    }

    #[test]
    fn a_stair_up_is_reached_from_its_foot_and_a_stair_down_from_its_head() {
        let west = |ground: &str, upper: &str| {
            errors_of(&stairs(
                map(ground, &[(1, 8, upper)]),
                &[(0, 1, 0, Facing::West), (1, 1, 0, Facing::West)],
            ))
        };
        assert_eq!(
            west("|. u #|", "|. d .|"),
            ["the stair up at (1, 0) rises West, but its foot is a tile the party cannot enter"]
        );
        assert_eq!(
            west("|. u|.|", "|. d .|"),
            ["the stair up at (1, 0) rises West, but its foot is behind a wall or a door"]
        );
        assert_eq!(
            west("|. u .|", "|_ d .|"),
            ["level 1: the stair down at (1, 0) rises West, but its head is no ground"]
        );
        assert_eq!(
            errors_of(&stairs(
                map("|u . .|", &[(1, 8, "|d . .|")]),
                &[(0, 0, 0, Facing::East), (1, 0, 0, Facing::East)],
            )),
            ["the stair up at (0, 0) rises East, but its foot is off the map"]
        );
    }

    #[test]
    fn a_stair_pair_is_one_flight() {
        assert_eq!(
            errors_of(&stairs(
                map("|. u .|", &[(1, 8, "|. d .|")]),
                &[(0, 1, 0, Facing::East), (1, 1, 0, Facing::West)],
            )),
            [
                "the stair up at (1, 0) rises East, but the stair it joins on level 1 rises West; \
                 they are one flight"
            ]
        );
    }
}
