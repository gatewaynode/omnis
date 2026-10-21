//! The places a save depends on, checked against the loaded maps when the packs changed (M8.5
//! 7h4). A broken place loads and warns (owner, 2026-10-19). The party is moved to the nearest
//! cell it can stand on. What lies in the world is the world's (owner, 2026-10-19): a pile whose
//! ground went away is lost, and so is what a container the map no longer places held; only the
//! party's own inventory and stores are kept whatever the packs do (a bank, a house or a vehicle
//! may keep more, later). What held nothing (an open door no longer a door, a cleared encounter
//! the map lost, automap tiles outside the map) is forgotten.

use crate::objects::{ObjectState, state_of};
use crate::world::{Mode, World};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{Facing, ItemId, Level, MapId};
use omnis_data::placement::{covers_centre, in_state};
use omnis_data::{Data, MapData};
use serde::{Deserialize, Serialize};

/// A place a save held that the loaded maps no longer allow, and what the load made of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceWarning {
    /// The map, by its string id.
    pub map: String,
    /// What was done.
    pub fix: PlaceFix,
}

/// A cell of a map: its level, column and row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spot {
    /// The level; left out on the ground.
    #[serde(default, skip_serializing_if = "Level::is_ground")]
    pub level: Level,
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
}

impl fmt::Display for Spot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Spot { level, x, y } = *self;
        if level.is_ground() {
            write!(f, "({x}, {y})")
        } else {
            write!(f, "({x}, {y}) on level {}", level.0)
        }
    }
}

/// What a load did with a place the loaded maps no longer allow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceFix {
    /// The party could not stand at `from`; it stands at `to` now, on the same map.
    PartyMoved {
        /// Where it stood.
        from: Spot,
        /// Where it stands.
        to: Spot,
    },
    /// An open door on this edge is no longer a door; it is forgotten.
    DoorForgotten {
        /// The cell.
        at: Spot,
        /// The edge.
        facing: Facing,
    },
    /// A pile lay where the ground went away; it is lost.
    PileLost {
        /// Its cell.
        at: Spot,
        /// How many items it held.
        items: u32,
    },
    /// The map no longer places this object; its state is forgotten, and what it held, changed
    /// from what the map gave it, is lost.
    ObjectForgotten {
        /// The placement's id.
        id: String,
        /// How many items it held that are lost.
        items: u32,
    },
}

/// How many items a list holds.
fn count(items: &[(ItemId, u16)]) -> u32 {
    items.iter().map(|(_, n)| u32::from(*n)).sum()
}

fn items(n: u32) -> String {
    if n == 1 {
        "1 item".into()
    } else {
        alloc::format!("{n} items")
    }
}

impl fmt::Display for PlaceWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let map = &self.map;
        match &self.fix {
            PlaceFix::PartyMoved { from, to } => {
                write!(
                    f,
                    "the party could not stand at {map} {from}; moved to {to}"
                )
            }
            PlaceFix::DoorForgotten { at, facing } => {
                write!(
                    f,
                    "the open door {facing:?} of {map} {at} is no longer a door"
                )
            }
            PlaceFix::PileLost { at, items: n } => {
                write!(
                    f,
                    "the ground at {map} {at} is gone, and the {} lying there with it",
                    items(*n)
                )
            }
            PlaceFix::ObjectForgotten { id, items: 0 } => {
                write!(f, "{map} no longer places '{id}'; its state is forgotten")
            }
            PlaceFix::ObjectForgotten { id, items: n } => {
                write!(
                    f,
                    "{map} no longer places '{id}'; the {} left in it are lost",
                    items(*n)
                )
            }
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
        fix: PlaceFix::PartyMoved {
            from: Spot {
                level: p.level,
                x: p.x,
                y: p.y,
            },
            to: Spot { level, x, y },
        },
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
        fix: PlaceFix::DoorForgotten {
            at: Spot { level, x, y },
            facing,
        },
    }));
}

/// A pile on a cell the ground went from is lost with what it held.
fn piles(world: &mut World, id: MapId, map: &MapData, name: &str, out: &mut Vec<PlaceWarning>) {
    let Some(state) = world.maps.get_mut(&id) else {
        return;
    };
    let mut lost = Vec::new();
    state.ground.retain(|&(level, x, y), pile| {
        let keep = map.cell(level, x, y).is_some();
        if !keep && !pile.items.is_empty() {
            lost.push((Spot { level, x, y }, count(&pile.items)));
        }
        keep
    });
    out.extend(lost.into_iter().map(|(at, items)| PlaceWarning {
        map: name.into(),
        fix: PlaceFix::PileLost { at, items },
    }));
}

/// An object state whose placement the map lost is forgotten, with what it held.
fn objects(world: &mut World, data: &Data, id: MapId, name: &str, out: &mut Vec<PlaceWarning>) {
    let Some(map) = data.maps.get(&id) else {
        return;
    };
    let Some(state) = world.maps.get_mut(&id) else {
        return;
    };
    let mut lost: BTreeMap<String, ObjectState> = BTreeMap::new();
    state.objects.retain(|key, object| {
        let keep = map.objects.iter().any(|p| &p.id == key);
        if !keep {
            lost.insert(key.clone(), object.clone());
        }
        keep
    });
    out.extend(lost.into_iter().map(|(key, object)| PlaceWarning {
        map: name.into(),
        fix: PlaceFix::ObjectForgotten {
            id: key,
            items: count(object.contents.as_deref().unwrap_or_default()),
        },
    }));
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
