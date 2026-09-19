//! The party: up to six members in marching order, the shared purse and larder, and the
//! commands that build and reorder it (PRD §7.1, D10).

use crate::command::Rejection;
use crate::event::Event;
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::{CharacterId, ItemId, Pcg32, StreamName};
use omnis_data::Data;
use omnis_rules::{ActiveEffect, Character, Draft, create};
use serde::{Deserialize, Serialize};

/// Slots when the rules give no `party_slots` value.
pub const DEFAULT_SLOTS: usize = 6;
/// Front-row size when the rules give no `front_row` value.
pub const DEFAULT_FRONT_ROW: usize = 3;

/// The party. Member order is marching order: the first `front_row` members can melee and be
/// meleed, the rest are the back row.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Party {
    /// Members in marching order.
    pub members: Vec<Character>,
    /// Gold pieces.
    pub gold: u32,
    /// Deprecated and never written: spell components are items in `inventory` (the gem item
    /// first). Kept so saves and the protocol keep their shape.
    pub gems: u32,
    /// Food units.
    pub food: u32,
    /// Shared items as `(item, count)`.
    pub inventory: Vec<(ItemId, u16)>,
    /// The next character id to hand out.
    pub next_character: u32,
    /// Party-wide spell effects in force (light); only live ones are kept.
    #[serde(default)]
    pub effects: Vec<ActiveEffect>,
}

/// A change to the party.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyCommand {
    /// Create a character from a draft and add it to the last free slot.
    Create(Draft),
    /// Set the marching order: a permutation of the current member indices.
    Reorder {
        /// New order, old indices.
        order: Vec<u8>,
    },
}

/// How many members the rules allow.
#[must_use]
pub fn slots(data: &Data) -> usize {
    data.rules
        .value("party_slots")
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(DEFAULT_SLOTS)
}

/// How many members stand in the front row.
#[must_use]
pub fn front_row(data: &Data) -> usize {
    data.rules
        .value("front_row")
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(DEFAULT_FRONT_ROW)
}

/// Apply a party command. Validation completes before anything changes, so a rejection
/// leaves the world as it was.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    command: &PartyCommand,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    match command {
        PartyCommand::Create(draft) => create_member(world, data, draft)?,
        PartyCommand::Reorder { order } => reorder(world, order)?,
    }
    events.push(Event::PartyChanged);
    Ok(())
}

fn create_member(world: &mut World, data: &Data, draft: &Draft) -> Result<(), Rejection> {
    if world.party.members.len() >= slots(data) {
        return Err(Rejection::PartyFull);
    }
    let id = CharacterId(world.party.next_character);
    let elapsed = world.party_clock().elapsed;
    // The stream is read without being created, so a refused draft leaves `rngs` untouched.
    let stream = StreamName::new("party");
    let mut rng = world
        .rngs
        .get(&stream)
        .copied()
        .unwrap_or_else(|| Pcg32::for_stream(world.seed, &stream));
    let character = create(draft, data, id, elapsed, &mut rng).map_err(Rejection::Character)?;
    world.rngs.insert(stream, rng);
    let gold = data
        .backgrounds
        .get(&character.background)
        .map_or(0, |b| u32::from(b.gold));
    let food = data
        .rules
        .value("starting_food")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    let party = &mut world.party;
    party.gold = party.gold.saturating_add(gold);
    party.food = party.food.saturating_add(food);
    party.next_character = party.next_character.saturating_add(1);
    party.members.push(character);
    Ok(())
}

fn reorder(world: &mut World, order: &[u8]) -> Result<(), Rejection> {
    let len = world.party.members.len();
    let mut seen = alloc::vec![false; len];
    if order.len() != len {
        return Err(Rejection::BadOrder);
    }
    for &index in order {
        match seen.get_mut(usize::from(index)) {
            Some(slot) if !*slot => *slot = true,
            _ => return Err(Rejection::BadOrder),
        }
    }
    let old = core::mem::take(&mut world.party.members);
    let mut old: Vec<Option<Character>> = old.into_iter().map(Some).collect();
    world.party.members = order
        .iter()
        .filter_map(|&i| old[usize::from(i)].take())
        .collect();
    Ok(())
}
