//! Repair (PRD §8.3; `tasks/plans/repair.md`, step 10 of combat in feet): the mending cantrip
//! makes a small thing whole, and a member with the right tool repairs a thing of any size for
//! time, a Dexterity check and, when it is broken, supplies. The owner's choices, 2026-11-25: a
//! failed check loses the supplies too; tool and supplies may be anywhere in the party; a thing
//! only damaged needs no supplies; a fallen doorway is cleared, then rebuilt as a broken door.

use crate::breaking_doors::{FIGHTER, TORCH, attack, door};
use crate::objects::{objects, yard};
use crate::opening_strike::at;
use omnis_core::{Direction, Facing, ItemId, Level};
use omnis_data::{Data, Material};
use omnis_sim::event::ItemPlace;
use omnis_sim::query::EdgeView;
use omnis_sim::{
    BlockReason, Chosen, Command, Event, Rejection, Target, World, apply, objects as sim_objects,
    query, targets,
};

const WIZARD: usize = 2;
const BOLT: &str = "base:spell:fire_bolt";
const MENDING: &str = "base:spell:mending";
const TOOLS: &str = "base:item:carpenters_tools";
const TIMBER: &str = "base:item:timber";
const KIT: &str = "base:item:repair_kit";

/// The yard with doorways that fall in `percent` times in a hundred.
fn yard_data(percent: u8) -> Data {
    let mut data = objects();
    let yard = yard(&data);
    let map = data.maps.get_mut(&yard).unwrap();
    map.def.door_make.collapse_percent = percent;
    data
}

fn thing(id: &str) -> Target {
    Target::Object { object: id.into() }
}

fn item(data: &Data, id: &str) -> ItemId {
    data.registry.items.get(id).expect("a base item")
}

/// `count` of an item into the pack of the member in `slot`.
fn give(world: &mut World, data: &Data, slot: usize, id: &str, count: u16) {
    world.party.members[slot]
        .equipment
        .push((item(data, id), count));
}

/// How many of an item the party carries, packs and stores together.
fn carried(world: &World, data: &Data, id: &str) -> u16 {
    let item = item(data, id);
    let packs = world.party.members.iter().map(|m| &m.equipment);
    packs
        .chain([&world.party.inventory])
        .flatten()
        .filter(|(i, _)| *i == item)
        .map(|(_, n)| *n)
        .sum()
}

fn seeded(world: &World, seed: u64) -> World {
    let mut world = world.clone();
    world.seed = seed;
    world.rngs.clear();
    world
}

fn cast(
    world: &mut World,
    data: &Data,
    spell: &str,
    target: Target,
) -> Result<Vec<Event>, Rejection> {
    let command = Command::Cast {
        caster: world.party.members[WIZARD].id,
        spell: spell.into(),
        target,
    };
    apply(world, data, command)
}

/// The wizard knows mending, whatever a new wizard is given.
fn mender(world: &mut World, data: &Data) {
    let id = data.registry.spells.get(MENDING).expect("base's mending");
    let known = &mut world.party.members[WIZARD].known_spells;
    if !known.contains(&id) {
        known.push(id);
    }
}

fn mend(world: &mut World, data: &Data, target: Target) -> Result<Vec<Event>, Rejection> {
    mender(world, data);
    cast(world, data, MENDING, target)
}

fn repair(
    world: &mut World,
    data: &Data,
    slot: usize,
    target: Target,
) -> Result<Vec<Event>, Rejection> {
    let member = world.party.members[slot].id;
    apply(world, data, Command::Repair { member, target })
}

/// Whether the repair's check passed.
fn passed(events: &[Event]) -> bool {
    let mut checks = events.iter().filter_map(|e| match e {
        Event::Check { success, dc, .. } => {
            assert_eq!(*dc, 10, "the base difficulty");
            Some(*success)
        }
        _ => None,
    });
    let one = checks.next().expect("a check");
    assert!(checks.next().is_none(), "one check a repair");
    one
}

fn state(world: &World, data: &Data, id: &str) -> (u16, bool, Option<String>) {
    sim_objects::saved(world, yard(data), id)
        .map_or((0, false, None), |s| (s.damage, s.broken, s.state.clone()))
}

/// The fighter strikes the object until it is broken (`broken`) or only damaged.
fn hurt(world: &mut World, data: &Data, id: &str, broken: bool) {
    let command = Command::Attack {
        member: world.party.members[FIGHTER].id,
        target: thing(id),
    };
    for _ in 0..200 {
        let (damage, is_broken, _) = state(world, data, id);
        if is_broken || (!broken && damage > 0) {
            return;
        }
        apply(world, data, command.clone()).expect("a blow");
    }
    panic!("{id} took no harm");
}

fn wear(world: &World, data: &Data) -> (u16, bool, bool) {
    let (x, y, facing) = TORCH;
    let state = world.maps.get(&yard(data));
    let wear = state
        .map(|s| s.door_wear(Level::GROUND, x, y, facing))
        .unwrap_or_default();
    (wear.damage, wear.broken, wear.collapsed)
}

/// The fighter strikes the torch's door until it is broken (`broken`) or only worn.
fn hurt_door(world: &mut World, data: &Data, broken: bool) {
    let command = attack(world, FIGHTER, TORCH);
    for _ in 0..200 {
        let (damage, is_broken, _) = wear(world, data);
        if is_broken || (!broken && damage > 0) {
            return;
        }
        apply(world, data, command.clone()).expect("a blow");
    }
    panic!("the door took no harm");
}

/// A world like `world` on a seed where a repair of `target` by the fighter passes (`pass`) or
/// fails, and that repair's events.
fn outcome(world: &World, data: &Data, target: &Target, pass: bool) -> (World, Vec<Event>) {
    for seed in 0..200 {
        let mut world = seeded(world, seed);
        let events = repair(&mut world, data, FIGHTER, target.clone()).expect("a repair");
        if passed(&events) == pass {
            return (world, events);
        }
    }
    panic!("no seed for pass {pass}");
}

#[test]
fn mending_makes_a_small_thing_whole_and_refuses_what_is_bigger() {
    let data = yard_data(0);
    // South of the chest at (5, 3): Small and wood, with a broken state of its own.
    let mut world = at(&data, 5, 4, Facing::North);
    assert_eq!(
        mend(&mut world, &data, thing("chest_1")),
        Err(Rejection::NothingToRepair),
        "a whole thing"
    );
    hurt(&mut world, &data, "chest_1", true);
    let (_, broken, look) = state(&world, &data, "chest_1");
    assert!(broken && look.as_deref() == Some("smashed"));
    let before = world.party_clock().elapsed;
    let events = mend(&mut world, &data, thing("chest_1")).expect("a mend");
    let wizard = world.party.members[WIZARD].id;
    let mended = Event::ObjectRepaired {
        map: "objects:map:yard".into(),
        object: "chest_1".into(),
        by: Some(wizard),
        mended: true,
    };
    assert!(events.contains(&mended), "{events:?}");
    assert!(
        !events.iter().any(|e| matches!(e, Event::Check { .. })),
        "no roll"
    );
    assert_eq!(state(&world, &data, "chest_1"), (0, false, None));
    assert_eq!(world.party_clock().elapsed, before + 1, "the cast's minute");
    // It stands as it was placed, and offers what it offered.
    let placement = |data: &Data| {
        let map = data.maps.get(&yard(data)).unwrap();
        map.objects
            .iter()
            .find(|p| p.id == "chest_1")
            .unwrap()
            .clone()
    };
    let chest = placement(&data);
    assert_eq!(sim_objects::state_of(&world, yard(&data), &chest), "closed");
    assert!(!sim_objects::broken(&world, yard(&data), &chest));
    // Only damaged: whole again.
    hurt(&mut world, &data, "chest_1", false);
    assert!(state(&world, &data, "chest_1").0 > 0);
    mend(&mut world, &data, thing("chest_1")).expect("a mend");
    assert_eq!(state(&world, &data, "chest_1").0, 0);
    // The crate is Medium, and so is a door: too large for the cantrip.
    let mut world = at(&data, 2, 1, Facing::West);
    hurt(&mut world, &data, "crate_1", false);
    let was = world.clone();
    assert_eq!(
        mend(&mut world, &data, thing("crate_1")),
        Err(Rejection::TooLarge)
    );
    mender(&mut world.clone(), &data);
    assert_eq!(
        state(&world, &data, "crate_1"),
        state(&was, &data, "crate_1")
    );
    let mut world = at(&data, 6, 3, Facing::South);
    hurt_door(&mut world, &data, false);
    assert_eq!(
        mend(&mut world, &data, door(TORCH)),
        Err(Rejection::TooLarge)
    );
    // A bigger mend reaches them.
    let mut wide = data.clone();
    let id = wide.registry.spells.get(MENDING).unwrap();
    wide.spells.get_mut(&id).unwrap().effect = Some(omnis_data::SpellEffect::Utility(
        omnis_data::Utility::Mend {
            up_to: omnis_data::Size::Medium,
        },
    ));
    let events = mend(&mut world, &wide, door(TORCH)).expect("a mend");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::DoorRepaired { mended: true, .. })),
        "{events:?}"
    );
    assert_eq!(wear(&world, &wide), (0, false, false));
}

#[test]
fn mending_reaches_only_what_is_touched() {
    let data = yard_data(0);
    let mut world = at(&data, 5, 4, Facing::North);
    hurt(&mut world, &data, "chest_1", false);
    let far = omnis_sim::DevCommand::Teleport {
        map: "objects:map:yard".into(),
        level: Level::GROUND,
        x: 2,
        y: 4,
        facing: Facing::East,
    };
    apply(&mut world, &data, Command::Dev(far)).expect("a cell of the yard");
    assert_eq!(
        mend(&mut world, &data, thing("chest_1")),
        Err(Rejection::BeyondRange)
    );
    let fighter = world.party.members[FIGHTER].id;
    give(&mut world, &data, FIGHTER, TOOLS, 1);
    let command = Command::Repair {
        member: fighter,
        target: thing("chest_1"),
    };
    assert_eq!(
        apply(&mut world, &data, command),
        Err(Rejection::BeyondRange)
    );
}

#[test]
fn a_damaged_thing_is_repaired_with_the_tool_the_check_and_the_time_and_no_supplies() {
    let data = yard_data(0);
    let mut world = at(&data, 2, 1, Facing::West);
    hurt(&mut world, &data, "crate_1", false);
    let damage = state(&world, &data, "crate_1").0;
    let was = world.clone();
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("crate_1")),
        Err(Rejection::NeedsTool {
            material: Material::Wood
        })
    );
    assert_eq!(world, was, "a refusal changes nothing");
    // A mason's tools are not a carpenter's.
    give(&mut world, &data, FIGHTER, "base:item:masons_tools", 1);
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("crate_1")),
        Err(Rejection::NeedsTool {
            material: Material::Wood
        })
    );
    // The tool in another's pack serves.
    give(&mut world, &data, 4, TOOLS, 1);
    let before = world.party_clock().elapsed;
    let (fixed, events) = outcome(&world, &data, &thing("crate_1"), true);
    let fighter = world.party.members[FIGHTER].id;
    let repaired = Event::ObjectRepaired {
        map: "objects:map:yard".into(),
        object: "crate_1".into(),
        by: Some(fighter),
        mended: false,
    };
    assert!(events.contains(&repaired), "{events:?}");
    assert!(
        !events.iter().any(|e| matches!(e, Event::ItemSpent { .. })),
        "no supplies for damage alone"
    );
    assert_eq!(state(&fixed, &data, "crate_1"), (0, false, None));
    assert_eq!(fixed.party_clock().elapsed, before + 60, "Medium: an hour");
    assert_eq!(carried(&fixed, &data, TOOLS), 1, "the tool is kept");
    // A failed check: the hour passes and nothing is mended.
    let (failed, events) = outcome(&world, &data, &thing("crate_1"), false);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::ObjectRepaired { .. }))
    );
    assert_eq!(state(&failed, &data, "crate_1").0, damage);
    assert_eq!(failed.party_clock().elapsed, before + 60);
    // The chest is Small: half an hour.
    let mut world = at(&data, 5, 4, Facing::North);
    hurt(&mut world, &data, "chest_1", false);
    give(&mut world, &data, FIGHTER, TOOLS, 1);
    let before = world.party_clock().elapsed;
    let (fixed, _) = outcome(&world, &data, &thing("chest_1"), true);
    assert_eq!(fixed.party_clock().elapsed, before + 30);
}

#[test]
fn a_broken_thing_needs_supplies_by_its_size_and_a_failed_check_loses_them() {
    let data = yard_data(0);
    let mut world = at(&data, 2, 1, Facing::West);
    hurt(&mut world, &data, "crate_1", true);
    give(&mut world, &data, 3, TOOLS, 1);
    let short = |have| Rejection::NeedsSupplies {
        material: Material::Wood,
        have,
        need: 2,
    };
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("crate_1")),
        Err(short(0))
    );
    // Iron is no supply for wood.
    give(&mut world, &data, FIGHTER, "base:item:iron_stock", 5);
    give(&mut world, &data, 5, TIMBER, 1);
    let was = world.clone();
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("crate_1")),
        Err(short(1))
    );
    assert_eq!(world, was);
    // One piece in a pack and two in the stores: two are used, pass or fail.
    world.party.inventory.push((item(&data, TIMBER), 2));
    for pass in [true, false] {
        let (after, events) = outcome(&world, &data, &thing("crate_1"), pass);
        assert_eq!(carried(&after, &data, TIMBER), 1, "pass {pass}");
        assert_eq!(carried(&after, &data, TOOLS), 1);
        assert_eq!(carried(&after, &data, "base:item:iron_stock"), 5);
        let spent: u16 = events
            .iter()
            .filter_map(|e| match e {
                Event::ItemSpent { item, count, .. } if item == TIMBER => Some(*count),
                _ => None,
            })
            .sum();
        assert_eq!(spent, 2, "pass {pass}: {events:?}");
        assert_eq!(state(&after, &data, "crate_1").1, !pass);
    }
    // The members' packs are emptied before the stores, in marching order.
    let (after, events) = outcome(&world, &data, &thing("crate_1"), true);
    let from: Vec<&ItemPlace> = events
        .iter()
        .filter_map(|e| match e {
            Event::ItemSpent { from, .. } => Some(from),
            _ => None,
        })
        .collect();
    let fifth = ItemPlace::Member(world.party.members[5].id);
    assert_eq!(from, [&fifth, &ItemPlace::Stores]);
    assert_eq!(state(&after, &data, "crate_1"), (0, false, None));
    // A Small thing takes one.
    let mut world = at(&data, 5, 4, Facing::North);
    hurt(&mut world, &data, "chest_1", true);
    give(&mut world, &data, FIGHTER, TOOLS, 1);
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("chest_1")),
        Err(Rejection::NeedsSupplies {
            material: Material::Wood,
            have: 0,
            need: 1
        })
    );
    give(&mut world, &data, FIGHTER, TIMBER, 1);
    let (after, _) = outcome(&world, &data, &thing("chest_1"), true);
    assert_eq!(carried(&after, &data, TIMBER), 0);
    assert_eq!(state(&after, &data, "chest_1"), (0, false, None));
}

/// What a repair used up, in the order it was taken: each item's id, its count and its place.
fn used(events: &[Event]) -> Vec<(&str, u16, &ItemPlace)> {
    let spent = events.iter().filter_map(|e| match e {
        Event::ItemSpent { item, count, from } => Some((item.as_str(), *count, from)),
        _ => None,
    });
    spent.collect()
}

#[test]
fn a_repair_kit_is_supplies_for_any_material_and_is_used_after_the_materials_own() {
    let data = yard_data(0);
    let mut world = at(&data, 2, 1, Facing::West);
    hurt(&mut world, &data, "crate_1", true);
    let crate_1 = thing("crate_1");
    // A kit is supplies, not a tool.
    give(&mut world, &data, FIGHTER, KIT, 3);
    let was = world.clone();
    assert_eq!(
        repair(&mut world, &data, FIGHTER, crate_1.clone()),
        Err(Rejection::NeedsTool {
            material: Material::Wood
        })
    );
    assert_eq!(world, was);
    // With the tool, a kit goes for each supply the size takes, pass or fail.
    give(&mut world, &data, 3, TOOLS, 1);
    let fighter = ItemPlace::Member(world.party.members[FIGHTER].id);
    for pass in [true, false] {
        let (after, events) = outcome(&world, &data, &crate_1, pass);
        assert_eq!(used(&events), [(KIT, 2, &fighter)], "pass {pass}");
        assert_eq!(carried(&after, &data, KIT), 1);
        assert_eq!(state(&after, &data, "crate_1").1, !pass);
    }
    // One kit short of the two is not enough.
    let mut short = world.clone();
    short.party.members[FIGHTER].equipment.pop();
    give(&mut short, &data, FIGHTER, KIT, 1);
    assert_eq!(
        repair(&mut short, &data, FIGHTER, crate_1.clone()),
        Err(Rejection::NeedsSupplies {
            material: Material::Wood,
            have: 1,
            need: 2
        })
    );
    // The material's own supply goes first, though it lies in the stores and the kits in the
    // first pack; the kits make up the rest.
    world.party.inventory.push((item(&data, TIMBER), 1));
    let (after, events) = outcome(&world, &data, &crate_1, true);
    assert_eq!(
        used(&events),
        [(TIMBER, 1, &ItemPlace::Stores), (KIT, 1, &fighter)]
    );
    assert_eq!(carried(&after, &data, KIT), 2);
    // With timber enough no kit is touched.
    world.party.inventory.last_mut().unwrap().1 = 2;
    let (after, events) = outcome(&world, &data, &crate_1, true);
    assert_eq!(used(&events), [(TIMBER, 2, &ItemPlace::Stores)]);
    assert_eq!(carried(&after, &data, KIT), 3);
    assert_eq!(state(&after, &data, "crate_1"), (0, false, None));
}

#[test]
fn a_repair_kit_serves_metal_and_stone_with_their_own_tools() {
    let mut data = yard_data(0);
    let mut world = at(&data, 2, 1, Facing::West);
    hurt(&mut world, &data, "crate_1", true);
    give(&mut world, &data, FIGHTER, KIT, 2);
    give(&mut world, &data, FIGHTER, TOOLS, 1);
    let kind = data.registry.objects.get("objects:object:crate").unwrap();
    for (material, tools) in [
        (Material::Metal, "base:item:smiths_tools"),
        (Material::Stone, "base:item:masons_tools"),
    ] {
        data.objects.get_mut(&kind).unwrap().attributes.material = Some(material);
        // The carpenter's tools are not this material's.
        assert_eq!(
            repair(&mut world.clone(), &data, FIGHTER, thing("crate_1")),
            Err(Rejection::NeedsTool { material })
        );
        let mut world = world.clone();
        give(&mut world, &data, 4, tools, 1);
        let (after, _) = outcome(&world, &data, &thing("crate_1"), true);
        assert_eq!(carried(&after, &data, KIT), 0, "{material:?}");
        assert_eq!(state(&after, &data, "crate_1"), (0, false, None));
    }
}

#[test]
fn a_broken_door_is_rebuilt_shut_and_a_fallen_doorway_is_cleared_first() {
    let data = yard_data(0);
    let mut world = at(&data, 6, 3, Facing::South);
    // Worn only: the tool and no supplies.
    hurt_door(&mut world, &data, false);
    give(&mut world, &data, FIGHTER, TOOLS, 1);
    let (fixed, events) = outcome(&world, &data, &door(TORCH), true);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::DoorRepaired { mended: false, .. })),
        "{events:?}"
    );
    assert_eq!(wear(&fixed, &data), (0, false, false));
    // Broken, opened before it broke: two timber, and it is hung shut.
    apply(&mut world, &data, Command::Door).expect("it opens");
    hurt_door(&mut world, &data, true);
    assert_eq!(
        repair(&mut world, &data, FIGHTER, door(TORCH)),
        Err(Rejection::NeedsSupplies {
            material: Material::Wood,
            have: 0,
            need: 2
        })
    );
    give(&mut world, &data, WIZARD, TIMBER, 2);
    let before = world.party_clock().elapsed;
    let (mut fixed, _) = outcome(&world, &data, &door(TORCH), true);
    assert_eq!(fixed.party_clock().elapsed, before + 60);
    assert_eq!(wear(&fixed, &data), (0, false, false));
    let shut = EdgeView::Door {
        open: false,
        damage: 0,
        broken: false,
        collapsed: false,
    };
    assert_eq!(query::viewport(&fixed, &data).unwrap().tiles[0].front, shut);
    let events = apply(&mut fixed, &data, Command::Step(Direction::Forward)).unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Blocked {
                reason: BlockReason::ClosedDoor,
                ..
            }
        )),
        "{events:?}"
    );
    // The game's own seed, for the save: tried until it passes, with timber to spare.
    let mut saved = world.clone();
    give(&mut saved, &data, FIGHTER, TIMBER, 40);
    while wear(&saved, &data).1 {
        repair(&mut saved, &data, FIGHTER, door(TORCH)).expect("a repair");
    }
    crate::disk::round_trip(&saved, &data, "a rebuilt door");
    // It is a door again: it opens, and it can be struck.
    apply(&mut fixed, &data, Command::Door).expect("it opens");
    apply(&mut fixed, &data, Command::Door).expect("it shuts");
    let command = attack(&fixed, FIGHTER, TORCH);
    apply(&mut fixed, &data, command).expect("a blow");
    // A failed check leaves it broken and takes the timber.
    let (failed, _) = outcome(&world, &data, &door(TORCH), false);
    assert!(wear(&failed, &data).1);
    assert_eq!(carried(&failed, &data, TIMBER), 0);
    // A doorway that fell in is cleared before it is rebuilt.
    let falls = yard_data(100);
    let mut world = at(&falls, 6, 3, Facing::South);
    hurt_door(&mut world, &falls, true);
    assert!(wear(&world, &falls).2);
    give(&mut world, &falls, FIGHTER, TOOLS, 1);
    give(&mut world, &falls, FIGHTER, TIMBER, 2);
    let was = world.clone();
    assert_eq!(
        repair(&mut world, &falls, FIGHTER, door(TORCH)),
        Err(Rejection::Buried)
    );
    assert_eq!(world, was);
    apply(&mut world, &falls, Command::Clear).expect("rubble to clear");
    let (fixed, _) = outcome(&world, &falls, &door(TORCH), true);
    assert_eq!(wear(&fixed, &falls), (0, false, false));
}

#[test]
fn what_cannot_be_repaired_says_why() {
    let data = yard_data(0);
    let mut world = at(&data, 2, 1, Facing::West);
    give(&mut world, &data, FIGHTER, TOOLS, 1);
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("crate_1")),
        Err(Rejection::NothingToRepair)
    );
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("barrel_9")),
        Err(Rejection::NoSuchObject {
            object: "barrel_9".into()
        })
    );
    assert_eq!(
        repair(&mut world, &data, FIGHTER, Target::Stack(0)),
        Err(Rejection::WrongTarget)
    );
    // One who is down repairs nothing.
    hurt(&mut world, &data, "crate_1", false);
    world.party.members[FIGHTER].hp = 0;
    let fighter = world.party.members[FIGHTER].id;
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("crate_1")),
        Err(Rejection::MemberDown { member: fighter })
    );
    // The statue has no durability; a whole door has nothing to repair.
    let mut world = at(&data, 3, 1, Facing::North);
    assert_eq!(
        repair(&mut world, &data, FIGHTER, thing("statue_1")),
        Err(Rejection::CannotBeDamaged)
    );
    let mut world = at(&data, 6, 3, Facing::South);
    assert_eq!(
        repair(&mut world, &data, FIGHTER, door(TORCH)),
        Err(Rejection::NothingToRepair)
    );
}

#[test]
fn the_targets_of_a_repair_are_the_harmed_things_in_touch_each_with_its_cost() {
    let data = yard_data(0);
    let mut world = at(&data, 2, 1, Facing::West);
    let fighter = world.party.members[FIGHTER].id;
    let rows = targets(&world, &data, fighter, &Chosen::Repair).unwrap();
    assert!(rows.is_empty(), "nothing is harmed: {rows:?}");
    hurt(&mut world, &data, "crate_1", true);
    let rows = targets(&world, &data, fighter, &Chosen::Repair).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].target, thing("crate_1"));
    assert_eq!(
        rows[0].refusal,
        Some(Rejection::NeedsTool {
            material: Material::Wood
        })
    );
    let quote = query::repair_quote(&world, &data, &thing("crate_1")).expect("a quote");
    assert_eq!(
        (quote.minutes, quote.supplies, quote.material),
        (60, 2, Material::Wood)
    );
    give(&mut world, &data, FIGHTER, TOOLS, 1);
    give(&mut world, &data, FIGHTER, TIMBER, 2);
    let rows = targets(&world, &data, fighter, &Chosen::Repair).unwrap();
    assert_eq!(rows[0].refusal, None);
    // Damaged only: no supplies in the quote. Nothing to repair: no quote.
    let mut world = at(&data, 5, 4, Facing::North);
    assert_eq!(
        query::repair_quote(&world, &data, &thing("chest_1")).err(),
        Some(Rejection::NothingToRepair)
    );
    hurt(&mut world, &data, "chest_1", false);
    let quote = query::repair_quote(&world, &data, &thing("chest_1")).expect("a quote");
    assert_eq!((quote.minutes, quote.supplies), (30, 0));
    // A spell that mends lists the same things.
    mender(&mut world, &data);
    let wizard = world.party.members[WIZARD].id;
    let chosen = Chosen::Spell {
        spell: MENDING.into(),
        pay: omnis_sim::Pay::default(),
    };
    let rows = targets(&world, &data, wizard, &chosen).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        (&rows[0].target, &rows[0].refusal),
        (&thing("chest_1"), &None)
    );
    let _ = BOLT;
}
