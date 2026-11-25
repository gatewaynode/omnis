//! Repair (PRD §8.3; `tasks/plans/repair.md`, step 10 of combat in feet): a thing that has
//! taken damage or broken is made whole, by the mending spell (`casting`) or by a member with
//! the tool of its material, for time, a Dexterity check and, when it is broken, supplies by
//! its size. The owner's choices (2026-11-25): a failed check loses the supplies with the time;
//! the tool and the supplies may be in any member's pack or the party's stores; a thing only
//! damaged needs no supplies; a doorway that fell in is cleared first, then rebuilt as any
//! broken door is. A smith does the same for coin (`service`, step 11): any harmed thing on the
//! maps of its region, named by a [`Job`], with no tool, check or supplies.

use crate::apply::advance;
use crate::breaking;
use crate::combat::cast::Target;
use crate::combat::{Roller, state};
use crate::command::Rejection;
use crate::event::{ActorRef, CheckKind, Event, ItemPlace};
use crate::items::take_from;
use crate::objects;
use crate::objects::removed_in;
use crate::targets::map::{self, Spot};
use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, Facing, ItemId, Level, MapId};
use omnis_data::placement::ResolvedPlacement;
use omnis_data::{Ability, Craft, Data, Material, ObjectKind, Size, Targeting};
use omnis_rules::RollMode;
use serde::{Deserialize, Serialize};

/// The difficulty of the repairer's check when the rules do not say.
const REPAIR_DC: i64 = 10;

/// What is to be repaired.
enum What<'d> {
    /// An object, by its placement and kind.
    Object {
        placement: &'d ResolvedPlacement,
        kind: &'d ObjectKind,
    },
    /// A door, by its edge.
    Door { edge: (u16, u16, Facing) },
}

/// A thing that can be repaired, wherever it stands: it can be damaged, and it is damaged or
/// broken.
pub(crate) struct Worn<'d> {
    what: What<'d>,
    /// The map it is on.
    map: MapId,
    /// Its level there.
    level: Level,
    /// Its size; a thing with none written counts as Medium.
    pub size: Size,
    /// What it is made of, when that is written.
    pub material: Option<Material>,
    /// Broken, not only damaged: a repair by tools takes supplies.
    pub broken: bool,
}

/// A thing the party can repair itself: one it knows of on its own level.
pub(crate) struct Harmed<'d> {
    /// The thing.
    pub thing: Worn<'d>,
    /// Where it stands to the party.
    pub spot: Spot,
}

/// A thing a smith is asked to repair: it may stand on another map or level than the party.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Job {
    /// An object, by its map's string id and its placement's id there.
    Object {
        /// The map's string id.
        map: String,
        /// The placement's id.
        object: String,
    },
    /// A door, by its map's string id, its level and an edge of a tile.
    Door {
        /// The map's string id.
        map: String,
        /// The level; unwritten when it is the ground.
        #[serde(default, skip_serializing_if = "Level::is_ground")]
        level: Level,
        /// A tile the door is an edge of.
        x: u16,
        /// A tile the door is an edge of.
        y: u16,
        /// The edge of that tile the door is on.
        facing: Facing,
    },
}

impl Job {
    /// The string id of the map the thing is on.
    #[must_use]
    pub fn map(&self) -> &str {
        match self {
            Job::Object { map, .. } | Job::Door { map, .. } => map,
        }
    }
}

/// What a repair by tools would take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairQuote {
    /// Party-clock minutes, pass or fail.
    pub minutes: u32,
    /// Supplies used up, pass or fail: none for a thing only damaged.
    pub supplies: u16,
    /// What the thing is made of: the tool and the supplies are this material's.
    pub material: Material,
}

/// Supplies a broken thing of a size takes.
pub(crate) const fn supplies_for(size: Size) -> u16 {
    match size {
        Size::Tiny => 0,
        Size::Small => 1,
        Size::Medium => 2,
        Size::Large => 4,
        Size::Huge => 8,
        Size::Gargantuan => 16,
    }
}

/// Minutes a repair of a thing of a size takes.
const fn minutes_for(size: Size) -> u32 {
    match size {
        Size::Tiny => 10,
        Size::Small => 30,
        Size::Medium => 60,
        Size::Large => 120,
        Size::Huge => 240,
        Size::Gargantuan => 480,
    }
}

/// The object `placement` of `map` as something to repair, or why it is none: one that cannot
/// be damaged, or one that is whole.
fn object_thing<'d>(
    world: &World,
    data: &'d Data,
    map: MapId,
    placement: &'d ResolvedPlacement,
) -> Result<Worn<'d>, Rejection> {
    let none = || Rejection::NoSuchObject {
        object: placement.id.clone(),
    };
    let kind = data.objects.get(&placement.kind).ok_or_else(none)?;
    breaking::make(kind).ok_or(Rejection::CannotBeDamaged)?;
    let saved = objects::saved(world, map, &placement.id);
    let (damage, broken) = saved.map_or((0, false), |s| (s.damage, s.broken));
    if damage == 0 && !broken {
        return Err(Rejection::NothingToRepair);
    }
    Ok(Worn {
        what: What::Object { placement, kind },
        map,
        level: placement.level,
        size: kind.attributes.size.unwrap_or(Size::Medium),
        material: kind.attributes.material,
        broken,
    })
}

/// The door on an edge of `map` (the caller has found that one is there) as something to
/// repair, or why it is none: a map whose doors cannot be damaged, a doorway still buried, or
/// a door that is whole.
fn door_thing<'d>(
    world: &World,
    data: &'d Data,
    map: MapId,
    level: Level,
    edge: (u16, u16, Facing),
) -> Result<Worn<'d>, Rejection> {
    let (x, y, facing) = edge;
    let make = data.maps.get(&map).map(|m| &m.def.door_make);
    let make = make
        .filter(|m| m.numbers().is_some())
        .ok_or(Rejection::CannotBeDamaged)?;
    let state = world.maps.get(&map);
    let wear = state
        .map(|s| s.door_wear(level, x, y, facing))
        .unwrap_or_default();
    if wear.collapsed {
        return Err(Rejection::Buried);
    }
    if wear.damage == 0 && !wear.broken {
        return Err(Rejection::NothingToRepair);
    }
    Ok(Worn {
        what: What::Door { edge },
        map,
        level,
        size: make.size,
        material: Some(make.material),
        broken: wear.broken,
    })
}

/// The thing `target` names as something the party can repair, or why it is none: no such
/// thing where the party knows of it, one that cannot be damaged, a doorway still buried, or
/// one that is whole.
pub(crate) fn harmed<'d>(
    world: &World,
    data: &'d Data,
    target: &Target,
) -> Result<Harmed<'d>, Rejection> {
    let pos = world.position;
    match target {
        Target::Object { object } => {
            let none = || Rejection::NoSuchObject {
                object: object.clone(),
            };
            let (placement, spot) = map::object(world, data, object).ok_or_else(none)?;
            let thing = object_thing(world, data, pos.map, placement)?;
            Ok(Harmed { thing, spot })
        }
        Target::Door { x, y, facing } => {
            let spot = map::door(world, data, *x, *y, *facing).ok_or(Rejection::NoDoor)?;
            let thing = door_thing(world, data, pos.map, pos.level, (*x, *y, *facing))?;
            Ok(Harmed { thing, spot })
        }
        _ => Err(Rejection::WrongTarget),
    }
}

/// The maps a smith on `town` works over: those of the region that holds it, or only its own
/// when no region does, by their string ids.
fn region_maps(data: &Data, town: MapId) -> Vec<(&str, MapId)> {
    let region = data.regions.values().find(|r| r.maps.contains(&town));
    let maps = region.map_or_else(|| alloc::vec![town], |r| r.maps.clone());
    let named = maps
        .into_iter()
        .filter_map(|m| Some((data.registry.maps.name(m)?, m)));
    let mut named: Vec<_> = named.collect();
    named.sort_unstable();
    named
}

/// The thing a smith on `town` is asked to repair, or why it is none: `NotOffered` for a map
/// that is not of the smith's region, then as the party's own repair has it, but for sight:
/// the smith knows its region.
pub(crate) fn job_thing<'d>(
    world: &World,
    data: &'d Data,
    town: MapId,
    job: &Job,
) -> Result<Worn<'d>, Rejection> {
    let maps = region_maps(data, town);
    let found = maps.iter().find(|(name, _)| *name == job.map());
    let map = found.ok_or(Rejection::NotOffered)?.1;
    let def = data.maps.get(&map).ok_or(Rejection::NotOffered)?;
    match job {
        Job::Object { object, .. } => {
            let state = world.maps.get(&map);
            let placed = def.objects.iter().find(|p| p.id == *object);
            let placement = placed.filter(|p| !removed_in(state, p)).ok_or_else(|| {
                Rejection::NoSuchObject {
                    object: object.clone(),
                }
            })?;
            object_thing(world, data, map, placement)
        }
        Job::Door {
            level,
            x,
            y,
            facing,
            ..
        } => {
            let cell = def.cell(*level, *x, *y);
            if !cell.is_some_and(|c| c.doors.has(*facing)) {
                return Err(Rejection::NoDoor);
            }
            door_thing(world, data, map, *level, (*x, *y, *facing))
        }
    }
}

/// Every harmed thing a smith on `town` could be asked about, a buried doorway included: by
/// map id, a map's objects by their ids and then its doors by level and edge. Read from what
/// the world has saved, so a thing that never changed costs nothing to pass over.
#[must_use]
pub fn jobs(world: &World, data: &Data, town: MapId) -> Vec<Job> {
    let mut jobs = Vec::new();
    for (name, map) in region_maps(data, town) {
        let Some(state) = world.maps.get(&map) else {
            continue;
        };
        let harmed = state.objects.iter();
        for (object, _) in harmed.filter(|(_, s)| (s.damage > 0 || s.broken) && !s.removed) {
            jobs.push(Job::Object {
                map: name.into(),
                object: object.clone(),
            });
        }
        for (level, x, y, facing) in state.worn_doors.keys().copied() {
            jobs.push(Job::Door {
                map: name.into(),
                level,
                x,
                y,
                facing,
            });
        }
    }
    jobs
}

/// What a listed job is, for a view to say: the string id of an object's kind (a door has
/// none), and whether the thing is broken and not only damaged. A buried doorway is broken.
#[must_use]
pub fn described(world: &World, data: &Data, job: &Job) -> (Option<String>, bool) {
    let map = data.registry.maps.get(job.map());
    let state = map.and_then(|m| world.maps.get(&m));
    match job {
        Job::Object { object, .. } => {
            let def = map.and_then(|m| data.maps.get(&m));
            let placed = def.and_then(|d| d.objects.iter().find(|p| p.id == *object));
            let kind = placed.and_then(|p| data.registry.objects.name(p.kind));
            let broken = state.and_then(|s| s.objects.get(object));
            (kind.map(String::from), broken.is_some_and(|s| s.broken))
        }
        Job::Door {
            level,
            x,
            y,
            facing,
            ..
        } => {
            let wear = state.map(|s| s.door_wear(*level, *x, *y, *facing));
            (None, wear.is_some_and(|w| w.broken))
        }
    }
}

/// The thing is whole again, by the member `by` or by a smith (`None`): an object leaves its
/// kind's broken state for the one it was placed in (what it spilled stays where it fell), and
/// a door is hung shut.
pub(crate) fn restore(
    world: &mut World,
    data: &Data,
    thing: &Worn,
    by: Option<CharacterId>,
    mended: bool,
    events: &mut Vec<Event>,
) {
    let map = thing.map;
    let name = crate::names::id_of(&data.registry.maps, map);
    match thing.what {
        What::Object { placement, kind } => {
            let durability = kind.attributes.durability.as_ref();
            let wreck = durability.and_then(|d| d.broken_state.as_deref());
            objects::edit(world, map, &placement.id, |s| {
                s.damage = 0;
                s.broken = false;
                if wreck.is_some() && s.state.as_deref() == wreck {
                    s.state = None;
                }
            });
            events.push(Event::ObjectRepaired {
                map: name,
                object: placement.id.clone(),
                by,
                mended,
            });
        }
        What::Door { edge } => {
            let (x, y, facing) = edge;
            let level = thing.level;
            let state = world.map_state(map);
            state.wear_door(level, edge, |w| *w = crate::world::DoorWear::default());
            if state.door_open(level, x, y, facing) {
                state.toggle_door(level, x, y, facing);
            }
            events.push(Event::DoorRepaired {
                map: name,
                level,
                x,
                y,
                facing,
                by,
                mended,
            });
        }
    }
}

/// What a repair of `target` by tools would take, whoever does it and whatever the party
/// carries.
///
/// # Errors
/// Why the thing is none to repair (`harmed`), or `WrongTarget` for a thing with no material
/// written: no craft is its own.
pub fn quote(world: &World, data: &Data, target: &Target) -> Result<RepairQuote, Rejection> {
    let harmed = harmed(world, data, target)?;
    cost(&harmed.thing)
}

fn cost(thing: &Worn) -> Result<RepairQuote, Rejection> {
    let material = thing.material.ok_or(Rejection::WrongTarget)?;
    let supplies = if thing.broken {
        supplies_for(thing.size)
    } else {
        0
    };
    Ok(RepairQuote {
        minutes: minutes_for(thing.size),
        supplies,
        material,
    })
}

/// Whether an item is the craft `wanted` picks, for `material`.
fn serves(data: &Data, item: ItemId, material: Material, tool: bool) -> bool {
    let craft = data.items.get(&item).and_then(|d| d.craft.as_ref());
    craft.is_some_and(|c| matches!(c, Craft::Tool(_)) == tool && c.materials().contains(&material))
}

/// Where the party carries things, in the order they are taken from: each member's pack in
/// marching order, then the stores.
fn places(world: &World) -> Vec<(ItemPlace, &Vec<(ItemId, u16)>)> {
    let packs = world.party.members.iter();
    let mut places: Vec<_> = packs
        .map(|m| (ItemPlace::Member(m.id), &m.equipment))
        .collect();
    places.push((ItemPlace::Stores, &world.party.inventory));
    places
}

/// The supplies for `material` the party carries, by place and item, in the order they are
/// taken.
fn supplies(world: &World, data: &Data, material: Material) -> Vec<(ItemPlace, ItemId, u16)> {
    let mut found = Vec::new();
    for (place, list) in places(world) {
        for (item, count) in list {
            if *count > 0 && serves(data, *item, material, false) {
                found.push((place.clone(), *item, *count));
            }
        }
    }
    found
}

/// A repair by tools that passed every check.
pub(crate) struct Checked<'d> {
    own: usize,
    thing: Worn<'d>,
    quote: RepairQuote,
}

/// Every check of `Command::Repair` before anything is spent: a member who can act, a harmed
/// thing in touch, its material's tool somewhere in the party, and the supplies a broken thing
/// of its size takes.
pub(crate) fn validate<'d>(
    world: &World,
    data: &'d Data,
    member: CharacterId,
    target: &Target,
) -> Result<Checked<'d>, Rejection> {
    let own = usize::from(world.party.slot_of(member)?);
    let actor = &world.party.members[own];
    if state::is_dead(actor, data) {
        return Err(Rejection::MemberDead { member });
    }
    if actor.is_down() {
        return Err(Rejection::MemberDown { member });
    }
    let Harmed { thing, spot } = harmed(world, data, target)?;
    map::within(world, data, Targeting::TOUCH, spot)?;
    let quote = cost(&thing)?;
    let material = quote.material;
    let tool = places(world).iter().any(|(_, list)| {
        list.iter()
            .any(|(item, count)| *count > 0 && serves(data, *item, material, true))
    });
    if !tool {
        return Err(Rejection::NeedsTool { material });
    }
    let have = supplies(world, data, material)
        .iter()
        .fold(0u16, |sum, (_, _, n)| sum.saturating_add(*n));
    if have < quote.supplies {
        return Err(Rejection::NeedsSupplies {
            material,
            have,
            need: quote.supplies,
        });
    }
    Ok(Checked { own, thing, quote })
}

/// Use up `count` supplies for `material`, from the packs in marching order and then the stores.
fn spend(world: &mut World, data: &Data, material: Material, count: u16, events: &mut Vec<Event>) {
    let mut left = count;
    for (place, item, have) in supplies(world, data, material) {
        if left == 0 {
            break;
        }
        let taken = have.min(left);
        let party = &mut world.party;
        let list = match &place {
            ItemPlace::Member(id) => party
                .members
                .iter_mut()
                .find(|m| m.id == *id)
                .map(|m| &mut m.equipment),
            _ => Some(&mut party.inventory),
        };
        if list.is_some_and(|list| take_from(list, item, taken).is_ok()) {
            left -= taken;
            events.push(Event::ItemSpent {
                item: crate::names::id_of(&data.registry.items, item),
                count: taken,
                from: place,
            });
        }
    }
}

/// `Command::Repair`: the supplies are used, the member's Dexterity is tried against the
/// rules' `repair_dc`, the thing is whole when it passes, and the time goes by either way.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    member: CharacterId,
    target: &Target,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let Checked { own, thing, quote } = validate(world, data, member, target)?;
    let mut roller = Roller::take_stream(world, "repair");
    let actor = &world.party.members[own];
    let roll = omnis_rules::check(
        actor,
        data,
        None,
        Ability::Dexterity,
        RollMode::Normal,
        &mut roller.rng,
        &roller.stream,
    )
    .map_err(Rejection::Rule)?;
    roller.store(world);
    let dc = data.rules.value("repair_dc").unwrap_or(REPAIR_DC);
    let success = roll.total >= dc;
    spend(world, data, quote.material, quote.supplies, events);
    events.push(Event::Check {
        actor: ActorRef::Member(member),
        kind: CheckKind::Ability(Ability::Dexterity),
        roll: Some(roll),
        dc,
        success,
    });
    if success {
        restore(world, data, &thing, Some(member), false, events);
    }
    advance(world, data, quote.minutes, events);
    Ok(())
}
