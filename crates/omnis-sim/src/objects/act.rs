//! `Command::Act` (M8.5 step 2): which object the party reaches, which of its kind's
//! interactions the command names, and why one is refused before anything happens. What a
//! named interaction does is `outcome.rs`.

use super::{Refusal, found_in, outcome, removed_in, saved, state_of};
use crate::combat::state::can_fight;
use crate::command::Rejection;
use crate::event::Event;
use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::Position;
use omnis_data::placement::ResolvedPlacement;
use omnis_data::{Data, Interaction, MapData, ObjectKind, Verb};

/// Do the `option`th interaction of `verb` that the object `object` offers now.
pub(crate) fn act(
    world: &mut World,
    data: &Data,
    object: &str,
    verb: Verb,
    option: u8,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let placement = in_reach(world, data, object).ok_or_else(|| Rejection::NoSuchObject {
        object: String::from(object),
    })?;
    let kind = data
        .objects
        .get(&placement.kind)
        .ok_or_else(|| Rejection::NoSuchObject {
            object: String::from(object),
        })?;
    let refused = |reason| Rejection::VerbRefused { reason };
    let state = String::from(state_of(world, world.position.map, placement));
    let (index, interaction) = offered(kind, &state)
        .filter(|(_, it)| it.verb == verb)
        .nth(usize::from(option))
        .ok_or(refused(Refusal::NotOffered))?;
    let spent =
        saved(world, world.position.map, &placement.id).is_some_and(|s| s.used.contains(&index));
    if interaction.once && spent {
        return Err(refused(Refusal::Spent));
    }
    if !world.party.members.iter().any(|m| can_fight(m, data)) {
        return Err(refused(Refusal::NoneCanAct));
    }
    outcome::resolve(world, data, placement, index, interaction, events)
}

/// The interactions a kind offers in `state`, with their indices in the kind.
pub(crate) fn offered<'k>(
    kind: &'k ObjectKind,
    state: &'k str,
) -> impl Iterator<Item = (u8, &'k Interaction)> + use<'k> {
    kind.interactions
        .iter()
        .enumerate()
        .filter(move |(_, it)| it.when_state.as_deref().is_none_or(|s| s == state))
        .filter_map(|(i, it)| u8::try_from(i).ok().map(|i| (i, it)))
}

/// The placement `id` if the party can reach it: on its level, its footprint in the party's
/// cell or in the one it faces across an open edge, not hidden and not taken away.
pub(crate) fn in_reach<'d>(
    world: &World,
    data: &'d Data,
    id: &str,
) -> Option<&'d ResolvedPlacement> {
    let pos = world.position;
    let map = data.maps.get(&pos.map)?;
    let state = world.maps.get(&pos.map);
    reach(world, map, pos)
        .into_iter()
        .flat_map(|(x, y)| map.objects_at(pos.level, x, y))
        .map(|(_, p)| p)
        .find(|p| p.id == id && !removed_in(state, p) && found_in(state, p))
}

/// The cells the party reaches into: its own, and the one it faces when no wall, closed door
/// or cliff stands on the edge between.
fn reach(world: &World, map: &MapData, pos: Position) -> Vec<(u16, u16)> {
    let mut cells = Vec::from([(pos.x, pos.y)]);
    let Some(cell) = map.cell(pos.level, pos.x, pos.y) else {
        return cells;
    };
    let facing = pos.facing;
    let shut = cell.doors.has(facing)
        && !world
            .maps
            .get(&pos.map)
            .is_some_and(|s| s.door_open(pos.level, pos.x, pos.y, facing));
    if cell.walls.has(facing) || shut || cell.cliffs.has(facing) {
        return cells;
    }
    if let Some((x, y)) = pos.neighbour(facing).filter(|&(x, y)| map.inside(x, y)) {
        cells.push((x, y));
    }
    cells
}
