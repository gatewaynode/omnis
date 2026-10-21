//! `Command::Act` (M8.5 step 2): which object the party reaches, which of its kind's
//! interactions the command names, and why one is refused before anything happens. What a
//! named interaction does is `outcome.rs`.

use super::{Choice, Refusal, found_in, outcome, removed_in, saved, state_of};
use crate::combat::Roller;
use crate::combat::state::can_fight;
use crate::command::Rejection;
use crate::event::Event;
use crate::world::World;
use alloc::collections::BTreeSet;
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
    let mut roller = Roller::take_stream(world, "object");
    admit(world, data, placement, index, interaction, &mut roller)?;
    outcome::resolve(world, data, placement, index, interaction, roller, events)
}

/// Whether an offered interaction may be tried now, before any die: not a `once` already spent,
/// someone able to act, and every requirement that needs no die holding. `roller` is a copy of
/// the `object` stream, since a pack's rule slot may roll. The command and the view both ask
/// here, so they cannot disagree.
pub(crate) fn admit(
    world: &World,
    data: &Data,
    placement: &ResolvedPlacement,
    index: u8,
    interaction: &Interaction,
    roller: &mut Roller,
) -> Result<(), Rejection> {
    let refused = |reason| Rejection::VerbRefused { reason };
    let spent =
        saved(world, world.position.map, &placement.id).is_some_and(|s| s.used.contains(&index));
    if interaction.once && spent {
        return Err(refused(Refusal::Spent));
    }
    if !world.party.members.iter().any(|m| can_fight(m, data)) {
        return Err(refused(Refusal::NoneCanAct));
    }
    for requirement in &interaction.requires {
        outcome::gate(world, data, requirement, roller)?;
    }
    Ok(())
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

/// What Use could do to objects in reach: every interaction offered now and not already spent,
/// in the order the objects are reached and the kind lists them.
pub(crate) fn choices(world: &World, data: &Data) -> Vec<Choice> {
    let map = world.position.map;
    let mut choices = Vec::new();
    for placement in reachable(world, data) {
        let Some(kind) = data.objects.get(&placement.kind) else {
            continue;
        };
        let used = saved(world, map, &placement.id).map(|s| &s.used);
        let mut verbs = Vec::new();
        for (index, interaction) in offered(kind, state_of(world, map, placement)) {
            let verb = interaction.verb;
            let earlier = verbs.iter().filter(|v| **v == verb).count();
            let option = u8::try_from(earlier).unwrap_or(u8::MAX);
            verbs.push(verb);
            if interaction.once && used.is_some_and(|u| u.contains(&index)) {
                continue;
            }
            choices.push(Choice::Object {
                object: placement.id.clone(),
                verb,
                option,
            });
        }
    }
    choices
}

/// The placement `id` if the party can reach it.
pub(crate) fn in_reach<'d>(
    world: &World,
    data: &'d Data,
    id: &str,
) -> Option<&'d ResolvedPlacement> {
    reachable(world, data).find(|p| p.id == id)
}

/// The placements the party can reach: on its level, their footprints in the party's cell or in
/// the one it faces across an open edge, not hidden and not taken away. One spanning both cells
/// is reached once.
pub(crate) fn reachable<'w, 'd>(
    world: &'w World,
    data: &'d Data,
) -> impl Iterator<Item = &'d ResolvedPlacement> + use<'w, 'd> {
    let pos = world.position;
    let map = data.maps.get(&pos.map);
    let state = world.maps.get(&pos.map);
    let cells = map.map(|m| reach(world, m, pos)).unwrap_or_default();
    let mut seen = BTreeSet::new();
    cells
        .into_iter()
        .flat_map(move |(x, y)| {
            map.into_iter()
                .flat_map(move |m| m.objects_at(pos.level, x, y))
        })
        .map(|(_, p)| p)
        .filter(move |p| !removed_in(state, p) && found_in(&world.known, pos.map, p))
        .filter(move |p| seen.insert(p.id.as_str()))
}

/// The cells the party reaches into: its own, and the one it faces when no wall, closed door
/// or cliff stands on the edge between.
pub(crate) fn reach(world: &World, map: &MapData, pos: Position) -> Vec<(u16, u16)> {
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
