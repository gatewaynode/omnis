//! Containers (M8.5 step 2): an object whose kind has a capacity holds items, reached while it
//! is in a state marked `open` (or always, when its kind has no states). Its capacity counts
//! kinds of item, as a pile's stacks do. The save keeps the contents once they differ from the
//! placement's.

use super::{Refusal, act::in_reach, edit, saved, state_of};
use crate::command::Rejection;
use crate::event::{Event, ItemPlace};
use crate::ground::moved;
use crate::items::{actor, add_to, from_kit, from_list, out_of_kit, take_from};
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, ItemId};
use omnis_data::Data;
use omnis_data::placement::ResolvedPlacement;

/// What a container holds now: the saved contents, else the placement's.
#[must_use]
pub fn contents<'a>(world: &'a World, p: &'a ResolvedPlacement) -> &'a [(ItemId, u16)] {
    saved(world, world.position.map, &p.id)
        .and_then(|s| s.contents.as_deref())
        .unwrap_or(&p.contents)
}

/// Take `count` of an item out of an open container in reach into a member's kit.
pub(crate) fn take_out(
    world: &mut World,
    data: &Data,
    member: CharacterId,
    object: &str,
    item: &str,
    count: u16,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let (placement, _) = open(world, data, object)?;
    let own = actor(world, data, member)?;
    let mut held = Vec::from(contents(world, placement));
    let id = from_list(data, &held, item, count, |item| Rejection::NotInObject {
        item: String::from(item),
    })?;
    if take_from(&mut held, id, count).is_err() {
        return Err(Rejection::NotInObject { item: item.into() });
    }
    keep(world, placement, held);
    add_to(&mut world.party.members[own].equipment, id, count);
    let from = ItemPlace::Object(placement.id.clone());
    events.push(moved(data, id, count, from, ItemPlace::Member(member)));
    Ok(())
}

/// Put `count` of a kit's item into an open container in reach.
pub(crate) fn put_in(
    world: &mut World,
    data: &Data,
    member: CharacterId,
    object: &str,
    item: &str,
    count: u16,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let (placement, capacity) = open(world, data, object)?;
    let own = actor(world, data, member)?;
    let id = from_kit(world, data, own, item, count)?;
    let mut held = Vec::from(contents(world, placement));
    if held.len() >= capacity && !held.iter().any(|(i, _)| *i == id) {
        return Err(Rejection::VerbRefused {
            reason: Refusal::Full,
        });
    }
    out_of_kit(world, data, own, id, count, events);
    add_to(&mut held, id, count);
    keep(world, placement, held);
    let to = ItemPlace::Object(placement.id.clone());
    events.push(moved(data, id, count, ItemPlace::Member(member), to));
    Ok(())
}

/// The container `object` in reach, open, with its capacity in kinds of item.
fn open<'d>(
    world: &World,
    data: &'d Data,
    object: &str,
) -> Result<(&'d ResolvedPlacement, usize), Rejection> {
    if !matches!(world.mode, Mode::Explore) {
        return Err(Rejection::WrongMode);
    }
    let placement = in_reach(world, data, object).ok_or_else(|| Rejection::NoSuchObject {
        object: String::from(object),
    })?;
    let refused = |reason| Rejection::VerbRefused { reason };
    let kind = data
        .objects
        .get(&placement.kind)
        .filter(|k| k.attributes.capacity > 0)
        .ok_or(refused(Refusal::NotOffered))?;
    let state = state_of(world, world.position.map, placement);
    if !kind.states.is_empty() && !kind.states.get(state).is_some_and(|s| s.open) {
        return Err(refused(Refusal::Closed));
    }
    Ok((placement, usize::from(kind.attributes.capacity)))
}

/// Remember a container's contents, unless they are what it was placed with.
fn keep(world: &mut World, placement: &ResolvedPlacement, held: Vec<(ItemId, u16)>) {
    let changed = held != placement.contents;
    edit(world, world.position.map, &placement.id, |s| {
        s.contents = changed.then_some(held);
    });
}
