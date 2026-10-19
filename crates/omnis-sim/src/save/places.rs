//! The places a save depends on, checked against the loaded maps when the packs changed (M8.5
//! 7h4). A broken place loads and warns (owner, 2026-10-19): the party is moved to the nearest
//! cell it can stand on, a pile to the nearest that holds one, a lost object's contents laid on
//! the ground, so no item is lost; what held nothing (an open door no longer a door, a cleared
//! encounter the map lost, automap tiles outside the map) is forgotten.

use crate::objects::{ObjectState, state_of};
use crate::world::{Mode, World};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{Facing, ItemId, Level, MapId};
use omnis_data::placement::{Pile, covers_centre, in_state};
use omnis_data::{Data, MapData};
use serde::{Deserialize, Serialize};

/// A place a save held that the loaded maps no longer allow, and what the load made of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceWarning {
    /// The map, by its string id.
    pub map: String,
    /// The level of the cell.
    #[serde(default, skip_serializing_if = "Level::is_ground")]
    pub level: Level,
    /// Its column.
    pub x: u16,
    /// Its row.
    pub y: u16,
    /// What was done.
    pub fix: PlaceFix,
}

/// What a load did with a place the loaded maps no longer allow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceFix {
    /// The party could not stand there; it stands here now, on the same map.
    PartyMoved {
        /// The level.
        #[serde(default, skip_serializing_if = "Level::is_ground")]
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
    },
    /// An open door on this edge is no longer a door; it is forgotten.
    DoorForgotten {
        /// The edge.
        facing: Facing,
    },
    /// A pile lay where no pile can; it lies here now, items whole.
    PileMoved {
        /// Column.
        x: u16,
        /// Row.
        y: u16,
    },
    /// The map no longer places this object; its state is forgotten and what it held, if it
    /// held anything changed, laid on the ground at `laid`.
    ObjectForgotten {
        /// The placement's id.
        id: String,
        /// Where its contents were laid, when it held any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        laid: Option<(Level, u16, u16)>,
    },
}

impl fmt::Display for PlaceWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = Cell(self.level, self.x, self.y);
        match &self.fix {
            PlaceFix::PartyMoved { level, x, y } => write!(
                f,
                "the party could not stand at {} {at}; moved to {}",
                self.map,
                Cell(*level, *x, *y)
            ),
            PlaceFix::DoorForgotten { facing } => write!(
                f,
                "the open door {facing:?} of {} {at} is no longer a door",
                self.map
            ),
            PlaceFix::PileMoved { x, y } => write!(
                f,
                "a pile at {} {at} has no ground; moved to {}",
                self.map,
                Cell(self.level, *x, *y)
            ),
            PlaceFix::ObjectForgotten { id, laid: None } => {
                write!(
                    f,
                    "{} no longer places '{id}'; its state is forgotten",
                    self.map
                )
            }
            PlaceFix::ObjectForgotten {
                id,
                laid: Some((level, x, y)),
            } => write!(
                f,
                "{} no longer places '{id}'; what it held lies at {}",
                self.map,
                Cell(*level, *x, *y)
            ),
        }
    }
}

/// A cell as the warnings write it: `(x, y)`, with its level when not the ground.
struct Cell(Level, u16, u16);

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Cell(level, x, y) = *self;
        if level.is_ground() {
            write!(f, "({x}, {y})")
        } else {
            write!(f, "({x}, {y}) on level {}", level.0)
        }
    }
}

/// What the places check did: its warnings, the automap tiles dropped outside a map, and
/// whether it changed the world at all (a dropped cleared encounter warns of nothing), which
/// the load must fold so its journal still rebuilds it.
#[derive(Debug, Default)]
pub(crate) struct Checked {
    pub(crate) places: Vec<PlaceWarning>,
    pub(crate) trimmed: usize,
    pub(crate) changed: bool,
}

/// Check and fix every place the world holds against the loaded maps. Deterministic: maps in
/// number order, cells in key order, the nearest cell searched ring by ring in row order.
pub(crate) fn fix(world: &mut World, data: &Data) -> Checked {
    let before = (world.position, world.maps.clone(), world.automap.clone());
    let mut out = Checked::default();
    party(world, data, &mut out.places);
    let maps: Vec<MapId> = world.maps.keys().copied().collect();
    for id in maps {
        let Some(map) = data.maps.get(&id) else {
            continue;
        };
        let name = map_name(data, id);
        doors(world, id, map, &name, &mut out.places);
        piles(world, id, map, &name, &mut out.places);
        objects(world, data, id, &name, &mut out.places);
        if let Some(state) = world.maps.get_mut(&id) {
            state
                .cleared
                .retain(|e| map.encounters.iter().any(|x| &x.id == e));
        }
    }
    out.trimmed = trim(world, data);
    out.changed = before != (world.position, world.maps.clone(), world.automap.clone());
    out
}

fn map_name(data: &Data, id: MapId) -> String {
    data.registry.maps.name(id).unwrap_or_default().into()
}

/// Whether the party may stand on a cell: ground, terrain it may enter, no object across its
/// centre.
fn standable(
    world: &World,
    data: &Data,
    id: MapId,
    map: &MapData,
    (level, x, y): (Level, u16, u16),
) -> bool {
    let Some(cell) = map.cell(level, x, y) else {
        return false;
    };
    if !map.terrain(cell).passable {
        return false;
    }
    !map.objects_at(level, x, y).any(|(_, p)| {
        data.objects.get(&p.kind).is_some_and(|kind| {
            let removed = crate::objects::removed(world, id, p);
            let (look, moves, _) = in_state(kind, state_of(world, id, p));
            !removed && moves && covers_centre(&p.footprint(look), x, y)
        })
    })
}

/// The nearest cell of `level` around `(x, y)` that `ok` accepts, ring by ring, each ring in row
/// order; `(x, y)` itself is not tried.
fn nearest(
    map: &MapData,
    level: Level,
    (x, y): (u16, u16),
    mut ok: impl FnMut((Level, u16, u16)) -> bool,
) -> Option<(u16, u16)> {
    let (w, h) = (i32::from(map.def.width), i32::from(map.def.height));
    let (cx, cy) = (i32::from(x), i32::from(y));
    for r in 1..=w.max(h) {
        for ty in (cy - r)..=(cy + r) {
            for tx in (cx - r)..=(cx + r) {
                if (tx - cx).abs() != r && (ty - cy).abs() != r {
                    continue;
                }
                let (Ok(ux), Ok(uy)) = (u16::try_from(tx), u16::try_from(ty)) else {
                    continue;
                };
                if map.inside(ux, uy) && ok((level, ux, uy)) {
                    return Some((ux, uy));
                }
            }
        }
    }
    None
}

/// The party where it cannot stand moves to the nearest cell it can on its map and level, or
/// to the map's start. In a service it stays: the service is its tile's, and a load refuses a
/// service the tile no longer has.
fn party(world: &mut World, data: &Data, out: &mut Vec<PlaceWarning>) {
    let p = world.position;
    if matches!(world.mode, Mode::Town(_)) {
        return;
    }
    let Some(map) = data.maps.get(&p.map) else {
        return;
    };
    if standable(world, data, p.map, map, (p.level, p.x, p.y)) {
        return;
    }
    let level = if map.level(p.level).is_some() {
        p.level
    } else {
        Level::GROUND
    };
    let to = nearest(map, level, (p.x, p.y), |c| {
        standable(world, data, p.map, map, c)
    });
    let (level, x, y) = match to {
        Some((x, y)) => (level, x, y),
        None => (Level::GROUND, map.def.start.0, map.def.start.1),
    };
    out.push(PlaceWarning {
        map: map_name(data, p.map),
        level: p.level,
        x: p.x,
        y: p.y,
        fix: PlaceFix::PartyMoved { level, x, y },
    });
    world.position.level = level;
    world.position.x = x;
    world.position.y = y;
}

fn doors(world: &mut World, id: MapId, map: &MapData, name: &str, out: &mut Vec<PlaceWarning>) {
    let Some(state) = world.maps.get_mut(&id) else {
        return;
    };
    let is_door = |level, x, y, facing| map.cell(level, x, y).is_some_and(|c| c.doors.has(facing));
    let mut gone = Vec::new();
    state.open_doors.retain(|&(x, y, facing)| {
        let keep = is_door(Level::GROUND, x, y, facing);
        if !keep {
            gone.push((Level::GROUND, x, y, facing));
        }
        keep
    });
    state.level_doors.retain(|&(level, x, y, facing)| {
        let keep = is_door(level, x, y, facing);
        if !keep {
            gone.push((level, x, y, facing));
        }
        keep
    });
    gone.sort_unstable();
    out.extend(gone.into_iter().map(|(level, x, y, facing)| PlaceWarning {
        map: name.into(),
        level,
        x,
        y,
        fix: PlaceFix::DoorForgotten { facing },
    }));
}

/// Items laid on a cell: onto the pile the world holds there, or the map's, or a new one.
fn lay(
    world: &mut World,
    id: MapId,
    map: &MapData,
    at: (Level, u16, u16),
    items: &[(ItemId, u16)],
) {
    let Some(state) = world.maps.get_mut(&id) else {
        return;
    };
    let pile = state.ground.entry(at).or_insert_with(|| {
        map.ground.get(&at).cloned().unwrap_or(Pile {
            pos: (0, 0),
            items: Vec::new(),
        })
    });
    for &(item, n) in items {
        match pile.items.iter_mut().find(|(i, _)| *i == item) {
            Some((_, have)) => *have = have.saturating_add(n),
            None => pile.items.push((item, n)),
        }
    }
}

/// Where a pile may lie on `level` near `(x, y)`: the cell itself if it has ground, else the
/// nearest that does; on a level the map lost, its ground level.
fn ground_near(map: &MapData, level: Level, (x, y): (u16, u16)) -> Option<(Level, u16, u16)> {
    let level = if map.level(level).is_some() {
        level
    } else {
        Level::GROUND
    };
    if map.cell(level, x, y).is_some() {
        return Some((level, x, y));
    }
    let has_ground = |(l, x, y)| map.cell(l, x, y).is_some();
    nearest(map, level, (x, y), has_ground).map(|(x, y)| (level, x, y))
}

fn piles(world: &mut World, id: MapId, map: &MapData, name: &str, out: &mut Vec<PlaceWarning>) {
    let Some(state) = world.maps.get_mut(&id) else {
        return;
    };
    let lost: Vec<((Level, u16, u16), Pile)> = state
        .ground
        .iter()
        .filter(|((l, x, y), _)| map.cell(*l, *x, *y).is_none())
        .map(|(k, p)| (*k, p.clone()))
        .collect();
    for ((level, x, y), pile) in lost {
        if let Some(state) = world.maps.get_mut(&id) {
            state.ground.remove(&(level, x, y));
        }
        if pile.items.is_empty() {
            continue;
        }
        let Some(to) = ground_near(map, level, (x, y)) else {
            continue;
        };
        lay(world, id, map, to, &pile.items);
        out.push(PlaceWarning {
            map: name.into(),
            level,
            x,
            y,
            fix: PlaceFix::PileMoved { x: to.1, y: to.2 },
        });
    }
}

/// An object state whose placement the map lost is forgotten; contents it held, changed from
/// the map's, are laid where the party stands when it is on this map, else at the map's start.
fn objects(world: &mut World, data: &Data, id: MapId, name: &str, out: &mut Vec<PlaceWarning>) {
    let Some(map) = data.maps.get(&id) else {
        return;
    };
    let Some(state) = world.maps.get_mut(&id) else {
        return;
    };
    let lost: BTreeMap<String, ObjectState> = state
        .objects
        .iter()
        .filter(|(key, _)| !map.objects.iter().any(|p| &p.id == *key))
        .map(|(k, s)| (k.clone(), s.clone()))
        .collect();
    state.objects.retain(|key, _| !lost.contains_key(key));
    let p = world.position;
    let near = if p.map == id {
        (p.level, p.x, p.y)
    } else {
        (Level::GROUND, map.def.start.0, map.def.start.1)
    };
    for (key, object) in lost {
        let items = object.contents.unwrap_or_default();
        let laid = if items.is_empty() {
            None
        } else {
            ground_near(map, near.0, (near.1, near.2))
        };
        if let Some(at) = laid {
            lay(world, id, map, at, &items);
        }
        out.push(PlaceWarning {
            map: name.into(),
            level: near.0,
            x: near.1,
            y: near.2,
            fix: PlaceFix::ObjectForgotten { id: key, laid },
        });
    }
}

/// Automap tiles outside their map, or on a level it lost, are dropped: what the party saw,
/// not what is (presentation never blocks a load, owner 2026-10-18).
fn trim(world: &mut World, data: &Data) -> usize {
    let mut trimmed = 0;
    for (id, tiles) in &mut world.automap.maps {
        let Some(map) = data.maps.get(id) else {
            continue;
        };
        let before = tiles.len();
        tiles.retain(|&(x, y), _| map.inside(x, y));
        trimmed += before - tiles.len();
    }
    for ((id, level), tiles) in &mut world.automap.levels {
        let Some(map) = data.maps.get(id) else {
            continue;
        };
        let before = tiles.len();
        if map.level(*level).is_some() {
            tiles.retain(|&(x, y), _| map.inside(x, y));
        } else {
            tiles.clear();
        }
        trimmed += before - tiles.len();
    }
    world.automap.levels.retain(|_, tiles| !tiles.is_empty());
    trimmed
}
