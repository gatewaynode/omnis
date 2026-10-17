//! Beings in view (M8.5 step 2c): a fixed encounter that stands (`FixedEncounter.stand`) is
//! seen as one figure, its front stack's monster, while its group stands: not once cleared, and
//! not while the party is meeting or fighting it. Stepping into its cell meets it, as before;
//! `Approach` meets it from the next cell, with the retreat where the party stands, and Use
//! lists it among the things in reach.

use crate::command::Rejection;
use crate::encounter::{self, EncounterSource};
use crate::event::Event;
use crate::names::id_of;
use crate::objects::reach;
use crate::world::{Mode, World};
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Facing, Level, MapId};
use omnis_data::encounter::Stand;
use omnis_data::{Data, Look, MapData, ResolvedEncounter};
use serde::{Deserialize, Serialize};

/// A standing group as a client draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeingView {
    /// The map's id for it, which `Approach` names.
    pub being: String,
    /// Its front stack's monster: the one drawn.
    pub monster: String,
    /// How many stand there, every stack together.
    pub count: u16,
    /// Its cell.
    pub x: u16,
    /// Its cell.
    pub y: u16,
    /// Offset from the cell's centre, eighths east and south.
    pub pos: (i8, i8),
    /// The way it faces.
    pub facing: Facing,
    /// Its figure.
    pub look: Look,
}

/// The groups of `map` that stand now, with their indices: not cleared, and not the one the
/// party is meeting or fighting.
fn standing<'d>(
    world: &World,
    data: &'d Data,
    map: MapId,
) -> impl Iterator<Item = (u16, &'d ResolvedEncounter, &'d Stand)> + use<'d> {
    let cleared: BTreeSet<u16> = world
        .maps
        .get(&map)
        .map(|s| s.cleared.clone())
        .unwrap_or_default();
    let met = match &world.mode {
        Mode::Encounter(state) => Some(state.source),
        Mode::Combat(state) => Some(state.encounter.source),
        _ => None,
    }
    .filter(|_| world.position.map == map)
    .and_then(|s| match s {
        EncounterSource::Fixed(i) => Some(i),
        _ => None,
    });
    data.maps
        .get(&map)
        .into_iter()
        .flat_map(|m| m.encounters.iter().enumerate())
        .filter_map(|(i, e)| Some((u16::try_from(i).ok()?, e, e.stand.as_ref()?)))
        .filter(move |(i, _, _)| !cleared.contains(i) && met != Some(*i))
}

fn view(data: &Data, e: &ResolvedEncounter, stand: &Stand) -> Option<BeingView> {
    let front = e.front()?;
    let look = data.monsters.get(&front)?.figure();
    Some(BeingView {
        being: stand.id.clone(),
        monster: id_of(&data.registry.monsters, front),
        count: e.stacks.iter().map(|s| u16::from(s.1)).sum(),
        x: e.x,
        y: e.y,
        pos: stand.pos,
        facing: stand.facing,
        look,
    })
}

/// The groups standing on the cells in sight, `tiles` nearest first, on the party's level: one
/// whose figure reaches into a seen cell is seen.
pub(crate) fn in_sight(world: &World, data: &Data, tiles: &[(u16, u16)]) -> Vec<BeingView> {
    let pos = world.position;
    let mut found: Vec<(usize, BeingView)> = standing(world, data, pos.map)
        .filter(|(_, e, _)| e.level == pos.level)
        .filter_map(|(_, e, stand)| {
            let being = view(data, e, stand)?;
            let footprint = e.footprint(&being.look)?;
            let nearest = footprint
                .cells()
                .filter_map(|(x, y)| Some((u16::try_from(x).ok()?, u16::try_from(y).ok()?)))
                .filter_map(|cell| tiles.iter().position(|t| *t == cell))
                .min()?;
            Some((nearest, being))
        })
        .collect();
    found.sort_by_key(|(nearest, _)| *nearest);
    found.into_iter().map(|(_, being)| being).collect()
}

/// Every group standing on `level` of `map`, wherever the party is: for a client that draws
/// the whole level. In file order.
#[must_use]
pub fn on_level(world: &World, data: &Data, map: MapId, level: Level) -> Vec<BeingView> {
    standing(world, data, map)
        .filter(|(_, e, _)| e.level == level)
        .filter_map(|(_, e, stand)| view(data, e, stand))
        .collect()
}

/// The ids of the groups standing in reach: in the party's cell or the one it faces across an
/// open edge, on its level, in file order.
pub(crate) fn in_reach(world: &World, data: &Data) -> Vec<String> {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return Vec::new();
    };
    let cells = reach(world, map, pos);
    standing(world, data, pos.map)
        .filter(|(_, e, _)| e.level == pos.level && cells.contains(&(e.x, e.y)))
        .map(|(_, _, stand)| stand.id.clone())
        .collect()
}

/// Meet the group `being` in reach: as stepping into its cell does, but the party stays where
/// it is, which is where Run puts it back, facing away.
pub(crate) fn approach(
    world: &mut World,
    data: &Data,
    being: &str,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let refused = || Rejection::NoSuchBeing {
        being: String::from(being),
    };
    if !in_reach(world, data).iter().any(|b| b == being) {
        return Err(refused());
    }
    let pos = world.position;
    let index = data
        .maps
        .get(&pos.map)
        .and_then(|m| index_of(m, being))
        .ok_or_else(refused)?;
    let retreat = omnis_core::Position {
        facing: pos.facing.opposite(),
        ..pos
    };
    encounter::meet(world, data, index, retreat, events).map_err(Rejection::Rule)
}

fn index_of(map: &MapData, being: &str) -> Option<u16> {
    let i = map
        .encounters
        .iter()
        .position(|e| e.stand.as_ref().is_some_and(|s| s.id == being))?;
    u16::try_from(i).ok()
}
