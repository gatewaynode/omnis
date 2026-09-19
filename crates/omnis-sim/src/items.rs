//! Item stock and the item commands (PRD §8.1, §8.2): counted lists of items on a member or
//! in the party's stores, the spell components a cast consumes from the stores, and what a
//! member does with a carried item while exploring: wear it, take it off, hand it over, stow
//! it, take it back, use it. A fight uses an item through `CombatCommand::Use`, which shares
//! the validation and the resolution here. Every command is validated in full before anything
//! changes, so a rejection leaves the world as it was.

use crate::apply::advance;
use crate::combat::{Roller, state};
use crate::command::Rejection;
use crate::event::{Event, ItemPlace};
use crate::party::{self, Party};
use crate::sense;
use crate::world::World;
use alloc::vec;
use alloc::vec::Vec;
use omnis_core::{Dice, ItemId};
use omnis_data::{Data, EquipSlot, SenseSource, UseEffect};
use omnis_rules::{Character, EquipRefusal, RuleError};
use serde::{Deserialize, Serialize};

/// Minutes donning or doffing armor costs when the rules do not say.
const DEFAULT_DON_MINUTES: u32 = 5;
/// Minutes using an item costs when the rules do not say.
const DEFAULT_USE_MINUTES: u32 = 1;

/// A change to who carries, wears, or uses what. Every `item` is a row of the list it names
/// (a member's kit, or the stores for `Take`) as it stands when the command is applied, the
/// way `spell` is a row of a caster's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ItemCommand {
    /// Wear or wield a carried item; what the slot held stays carried. Armor takes
    /// `don_armor_minutes`.
    Equip {
        /// The member's slot.
        member: u8,
        /// The row of the member's kit.
        item: u8,
    },
    /// Empty a slot; the item stays carried. Armor takes `don_armor_minutes` to doff too.
    Unequip {
        /// The member's slot.
        member: u8,
        /// Which slot.
        slot: EquipSlot,
    },
    /// Hand items from one member's kit to another's.
    Give {
        /// The giver's slot.
        from: u8,
        /// The receiver's slot.
        to: u8,
        /// The row of the giver's kit.
        item: u8,
        /// How many; at least one.
        count: u16,
    },
    /// Put items from a member's kit into the party's stores.
    Stow {
        /// The member's slot.
        member: u8,
        /// The row of the member's kit.
        item: u8,
        /// How many; at least one.
        count: u16,
    },
    /// Take items from the stores into a member's kit.
    Take {
        /// The member's slot.
        member: u8,
        /// The row of the stores.
        item: u8,
        /// How many; at least one.
        count: u16,
    },
    /// Use a carried item, for `use_item_minutes`: a potion heals `target`, or the user when
    /// none is named. A consumable loses one count.
    Use {
        /// The member's slot.
        member: u8,
        /// The row of the member's kit.
        item: u8,
        /// Whom a potion goes to; the user when `None`.
        target: Option<u8>,
    },
}

/// What a use does, from the item's `use_effect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UseKind {
    /// Heal the target with these dice.
    Heal(Dice),
    /// Look from afar.
    Sense(SenseSource),
}

/// A use that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsePlan {
    /// The user's slot.
    pub own: usize,
    /// The item.
    pub item: ItemId,
    /// The member a heal goes to.
    pub target: usize,
    /// What it does.
    pub kind: UseKind,
    /// Whether one count is spent.
    pub consumable: bool,
}

/// One of the two counted lists an item can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// A member's kit, by slot.
    Kit(usize),
    /// The party's stores.
    Stores,
}

/// Apply an item command while exploring.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    command: ItemCommand,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    match command {
        ItemCommand::Equip { member, item } => equip(world, data, member, item, events),
        ItemCommand::Unequip { member, slot } => unequip(world, data, member, slot, events),
        ItemCommand::Give {
            from,
            to,
            item,
            count,
        } => {
            if from == to {
                return Err(Rejection::SameMember);
            }
            let (from, to) = (member_index(world, from)?, member_index(world, to)?);
            transfer(world, Place::Kit(from), Place::Kit(to), item, count, events)
        }
        ItemCommand::Stow {
            member,
            item,
            count,
        } => {
            let own = member_index(world, member)?;
            transfer(world, Place::Kit(own), Place::Stores, item, count, events)
        }
        ItemCommand::Take {
            member,
            item,
            count,
        } => {
            let own = member_index(world, member)?;
            transfer(world, Place::Stores, Place::Kit(own), item, count, events)
        }
        ItemCommand::Use {
            member,
            item,
            target,
        } => {
            let own = actor(world, data, member)?;
            let plan = validate_use(world, data, own, item, target, false)?;
            let (stream, minutes) = match &plan.kind {
                UseKind::Heal(_) => (
                    "items",
                    minutes(data, "use_item_minutes", DEFAULT_USE_MINUTES),
                ),
                UseKind::Sense(source) => ("sense", source.minutes),
            };
            let mut roller = Roller::take_stream(world, stream);
            use_item(world, data, &plan, &mut roller, events).map_err(Rejection::Rule)?;
            roller.store(world);
            advance(world, minutes, events);
            Ok(())
        }
    }
}

/// A rules value in minutes, or the default.
fn minutes(data: &Data, key: &str, default: u32) -> u32 {
    data.rules
        .value(key)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(default)
}

/// A member by slot.
fn member_index(world: &World, index: u8) -> Result<usize, Rejection> {
    let own = usize::from(index);
    if own < world.party.members.len() {
        Ok(own)
    } else {
        Err(Rejection::NoSuchMember { index })
    }
}

/// A member by slot who can act: alive and up.
fn actor(world: &World, data: &Data, index: u8) -> Result<usize, Rejection> {
    let own = member_index(world, index)?;
    let member = &world.party.members[own];
    if state::is_dead(member, data) {
        return Err(Rejection::MemberDead { index });
    }
    if member.is_down() {
        return Err(Rejection::MemberDown { index });
    }
    Ok(own)
}

/// The item at a row of a list.
fn row(list: &[(ItemId, u16)], item: u8) -> Option<ItemId> {
    list.get(usize::from(item)).map(|(id, _)| *id)
}

/// The refusal of the equipment rules as a rejection.
fn refusal(refusal: EquipRefusal) -> Rejection {
    match refusal {
        EquipRefusal::NotCarried => Rejection::NotCarried,
        EquipRefusal::NotEquippable => Rejection::NotEquippable,
        EquipRefusal::HandsFull => Rejection::HandsFull,
    }
}

/// Armor on or off takes its minutes; the rest is free.
fn don(world: &mut World, data: &Data, slot: EquipSlot, events: &mut Vec<Event>) {
    if slot == EquipSlot::Body {
        advance(
            world,
            minutes(data, "don_armor_minutes", DEFAULT_DON_MINUTES),
            events,
        );
    }
}

fn equip(
    world: &mut World,
    data: &Data,
    member: u8,
    item: u8,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let own = actor(world, data, member)?;
    let m = &mut world.party.members[own];
    let id = row(&m.equipment, item).ok_or(Rejection::UnknownItem { item })?;
    let (slot, displaced) =
        omnis_rules::equip(data, &m.equipment, &mut m.equipped, id).map_err(refusal)?;
    if let Some(old) = displaced {
        events.push(Event::Unequipped {
            member: m.id,
            slot,
            item: old,
        });
    }
    events.push(Event::Equipped {
        member: m.id,
        slot,
        item: id,
    });
    don(world, data, slot, events);
    Ok(())
}

fn unequip(
    world: &mut World,
    data: &Data,
    member: u8,
    slot: EquipSlot,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let own = actor(world, data, member)?;
    let m = &mut world.party.members[own];
    let item = omnis_rules::unequip(&mut m.equipped, slot).ok_or(Rejection::SlotEmpty { slot })?;
    events.push(Event::Unequipped {
        member: m.id,
        slot,
        item,
    });
    don(world, data, slot, events);
    Ok(())
}

fn list(party: &Party, place: Place) -> &Vec<(ItemId, u16)> {
    match place {
        Place::Kit(own) => &party.members[own].equipment,
        Place::Stores => &party.inventory,
    }
}

fn list_mut(party: &mut Party, place: Place) -> &mut Vec<(ItemId, u16)> {
    match place {
        Place::Kit(own) => &mut party.members[own].equipment,
        Place::Stores => &mut party.inventory,
    }
}

fn place_of(party: &Party, place: Place) -> ItemPlace {
    match place {
        Place::Kit(own) => ItemPlace::Member(party.members[own].id),
        Place::Stores => ItemPlace::Stores,
    }
}

/// A slot that held an item the kit no longer carries empties, with the event.
fn drop_worn(member: &mut Character, item: ItemId, events: &mut Vec<Event>) {
    if count_of(&member.equipment, item) > 0 {
        return;
    }
    for slot in EquipSlot::ALL {
        if member.equipped.get(&slot) == Some(&item) {
            member.equipped.remove(&slot);
            events.push(Event::Unequipped {
                member: member.id,
                slot,
                item,
            });
        }
    }
}

/// Move `count` of the item at row `item` of `from` into `to`. A kit that loses the last of a
/// worn item takes it off first. No minutes pass.
fn transfer(
    world: &mut World,
    from: Place,
    to: Place,
    item: u8,
    count: u16,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    if count == 0 {
        return Err(Rejection::ZeroCount);
    }
    let source = list(&world.party, from);
    let id = row(source, item).ok_or(match from {
        Place::Kit(_) => Rejection::UnknownItem { item },
        Place::Stores => Rejection::NotInStores { item },
    })?;
    let have = count_of(source, id);
    if have < count {
        return Err(Rejection::NotEnough { item: id, have });
    }
    take_from(list_mut(&mut world.party, from), id, count)?;
    add_to(list_mut(&mut world.party, to), id, count);
    if let Place::Kit(own) = from {
        drop_worn(&mut world.party.members[own], id, events);
    }
    events.push(Event::ItemMoved {
        item: id,
        count,
        from: place_of(&world.party, from),
        to: place_of(&world.party, to),
    });
    Ok(())
}

/// The checks a use passes before any die: a carried item with a use, usable here (`fight`
/// says whether a fight is on; a sense item is not used from one), and for a heal a living
/// target. `own` is the user, already known to be able to act.
pub(crate) fn validate_use(
    world: &World,
    data: &Data,
    own: usize,
    item: u8,
    target: Option<u8>,
    fight: bool,
) -> Result<UsePlan, Rejection> {
    let id =
        row(&world.party.members[own].equipment, item).ok_or(Rejection::UnknownItem { item })?;
    let def = data.items.get(&id).ok_or(Rejection::UnknownItem { item })?;
    let kind = match &def.use_effect {
        Some(UseEffect::Heal { dice }) => UseKind::Heal(*dice),
        Some(UseEffect::Sense(_)) if fight => return Err(Rejection::NotUsableHere),
        Some(UseEffect::Sense(source)) => UseKind::Sense(source.clone()),
        None => return Err(Rejection::NotUsable),
    };
    let slot = match kind {
        UseKind::Heal(_) => {
            let index = target.unwrap_or(u8::try_from(own).unwrap_or(u8::MAX));
            let slot = member_index(world, index)?;
            if state::is_dead(&world.party.members[slot], data) {
                return Err(Rejection::TargetDead { index });
            }
            slot
        }
        UseKind::Sense(_) => own,
    };
    Ok(UsePlan {
        own,
        item: id,
        target: slot,
        kind,
        consumable: def.consumable,
    })
}

/// Resolve a validated use: the count, the event, then the healing on the target or the
/// look from afar.
pub(crate) fn use_item(
    world: &mut World,
    data: &Data,
    plan: &UsePlan,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let target = world.party.members[plan.target].id;
    let user = &mut world.party.members[plan.own];
    if plan.consumable {
        take_from(&mut user.equipment, plan.item, 1)
            .map_err(|_| RuleError::new("use", "the item left the kit mid-use"))?;
    }
    let heals = matches!(plan.kind, UseKind::Heal(_));
    events.push(Event::ItemUsed {
        member: user.id,
        item: plan.item,
        target: heals.then_some(target),
        consumed: plan.consumable,
    });
    match &plan.kind {
        UseKind::Heal(dice) => {
            let trace = dice
                .roll(&mut roller.rng, &roller.stream)
                .map_err(|e| RuleError::new("use", alloc::format!("{e}")))?;
            let amount = i64::from(trace.total);
            party::heal(world, data, plan.target, vec![trace], amount, events);
        }
        UseKind::Sense(source) => sense::resolve(world, data, plan.item, source, roller, events)?,
    }
    Ok(())
}

/// Add `count` of an item to a counted list.
pub(crate) fn add_to(list: &mut Vec<(ItemId, u16)>, item: ItemId, count: u16) {
    if count == 0 {
        return;
    }
    match list.iter_mut().find(|(i, _)| *i == item) {
        Some((_, have)) => *have = have.saturating_add(count),
        None => list.push((item, count)),
    }
}

/// How many of an item a counted list holds.
#[must_use]
pub fn count_of(list: &[(ItemId, u16)], item: ItemId) -> u16 {
    list.iter()
        .filter(|(i, _)| *i == item)
        .map(|(_, n)| *n)
        .fold(0u16, u16::saturating_add)
}

/// Remove `count` of an item from a counted list, dropping the entry at zero, or refuse
/// without touching it.
pub(crate) fn take_from(
    list: &mut Vec<(ItemId, u16)>,
    item: ItemId,
    count: u16,
) -> Result<(), Rejection> {
    let have = count_of(list, item);
    if have < count {
        return Err(Rejection::NotEnough { item, have });
    }
    let mut left = count;
    for (i, n) in list.iter_mut() {
        if *i == item && left > 0 {
            let taken = (*n).min(left);
            *n -= taken;
            left -= taken;
        }
    }
    list.retain(|(_, n)| *n > 0);
    Ok(())
}

/// Whether the party's stores hold every `(item, count)`.
#[must_use]
pub fn has_all(party: &Party, needs: &[(ItemId, u16)]) -> bool {
    needs
        .iter()
        .all(|(item, count)| count_of(&party.inventory, *item) >= *count)
}

/// Remove every `(item, count)` from the party's stores, or refuse without touching them.
pub fn consume(party: &mut Party, needs: &[(ItemId, u16)]) -> Result<(), Rejection> {
    if let Some((item, _)) = needs
        .iter()
        .find(|(item, count)| count_of(&party.inventory, *item) < *count)
    {
        return Err(Rejection::NotEnough {
            item: *item,
            have: count_of(&party.inventory, *item),
        });
    }
    for (item, count) in needs {
        take_from(&mut party.inventory, *item, *count)?;
    }
    Ok(())
}

/// The item a pack defines under `<pack>:item:<name>`: the first interned id whose name ends
/// in `:<name>`, so the simulation finds `gem` in any pack.
#[must_use]
pub fn item_id(data: &Data, name: &str) -> Option<ItemId> {
    let registry = &data.registry.items;
    (0..u32::try_from(registry.len()).unwrap_or(u32::MAX))
        .map(ItemId)
        .find(|id| {
            registry
                .name(*id)
                .is_some_and(|n| n.rsplit(':').next() == Some(name) && n.contains(':'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_equipment_refusal_has_its_rejection() {
        assert_eq!(refusal(EquipRefusal::NotCarried), Rejection::NotCarried);
        assert_eq!(
            refusal(EquipRefusal::NotEquippable),
            Rejection::NotEquippable
        );
        assert_eq!(refusal(EquipRefusal::HandsFull), Rejection::HandsFull);
    }
}
