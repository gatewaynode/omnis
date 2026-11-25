//! The smith's repair service (PRD §8.3; `tasks/plans/repair-service.md`, step 11 of combat in
//! feet): the smith lists every damaged or broken thing on the maps of its region and makes
//! one whole for coin, at once, with no tool, check or supplies. The owner's choices,
//! 2026-11-25: a thing only damaged costs half; a buried doorway is listed and refused until
//! it is cleared; the price is ten times the supplies the job would use, 2 gp for a Tiny one.
//! In the fixture the forge and the yard are one region; the test town is another.

use crate::common::{inside, party_of};
use crate::objects::{objects, yard};
use omnis_core::{Facing, Level, MapId, Position};
use omnis_data::{Data, Size};
use omnis_sim::world::DoorWear;
use omnis_sim::{
    Command, Event, Job, ModeKind, OfferView, Rejection, ServiceCommand, Settings, World, apply,
    objects as sim_objects, service_view,
};

const FORGE: &str = "objects:map:forge";
const YARD: &str = "objects:map:yard";
/// The forge's door, named from the tile west of it.
const DOOR: (u16, u16, Facing) = (1, 0, Facing::East);

fn forge(data: &Data) -> MapId {
    data.registry.maps.get(FORGE).expect("forge")
}

/// A party of two with 50 gp inside the forge's smith.
fn at_forge(data: &Data) -> World {
    let mut world = World::new(data, 11, Settings::default()).expect("entry map");
    party_of(&mut world, data, 2);
    world.party.gold = 5000;
    world.position = Position {
        map: forge(data),
        level: Level::GROUND,
        x: 0,
        y: 0,
        facing: Facing::North,
    };
    apply(&mut world, data, Command::Interact).expect("the smith's door");
    assert_eq!(world.mode.kind(), ModeKind::Town, "inside the smith");
    world
}

fn thing(map: &str, object: &str) -> Job {
    Job::Object {
        map: map.into(),
        object: object.into(),
    }
}

fn door(map: &str, (x, y, facing): (u16, u16, Facing)) -> Job {
    Job::Door {
        map: map.into(),
        level: Level::GROUND,
        x,
        y,
        facing,
    }
}

/// The object has taken `damage` and is broken or not, as a blow would leave it.
fn harm(world: &mut World, map: MapId, id: &str, damage: u16, broken: bool) {
    let state = world.map_state(map).objects.entry(id.into()).or_default();
    state.damage = damage;
    state.broken = broken;
}

fn wear(world: &mut World, map: MapId, at: (u16, u16, Facing), wear: DoorWear) {
    world
        .map_state(map)
        .wear_door(Level::GROUND, at, |w| *w = wear);
}

const BROKEN: DoorWear = DoorWear {
    damage: 18,
    broken: true,
    collapsed: false,
};

fn repair(job: Job) -> ServiceCommand {
    ServiceCommand::Repair { job }
}

fn ask(world: &mut World, data: &Data, job: Job) -> Vec<Event> {
    apply(world, data, Command::Service(repair(job.clone())))
        .unwrap_or_else(|r| panic!("{job:?}: {r}"))
}

/// The repair is refused with `why`, and nothing changed.
fn refused(world: &mut World, data: &Data, job: Job, why: Rejection) {
    let before = world.clone();
    assert_eq!(
        apply(world, data, Command::Service(repair(job.clone()))),
        Err(why),
        "{job:?}"
    );
    assert_eq!(*world, before, "{job:?} changed the world");
}

/// The repairs the smith offers, in the order they are listed.
fn offers(world: &World, data: &Data) -> Vec<OfferView> {
    let view = service_view(world, data).expect("inside a service");
    let repairs = view.offers.into_iter();
    repairs
        .filter(|o| matches!(o.command, ServiceCommand::Repair { .. }))
        .collect()
}

fn jobs(world: &World, data: &Data) -> Vec<Job> {
    let jobs = offers(world, data).into_iter().map(|o| match o.command {
        ServiceCommand::Repair { job } => job,
        _ => unreachable!(),
    });
    jobs.collect()
}

fn minutes(world: &World) -> i64 {
    world.party_clock().elapsed
}

#[test]
fn the_smith_lists_what_is_harmed_in_its_region_and_nothing_else() {
    let data = objects();
    let mut world = at_forge(&data);
    assert_eq!(jobs(&world, &data), [], "nothing is harmed");

    let (forge, yard) = (forge(&data), yard(&data));
    harm(&mut world, yard, "crate_1", 3, false);
    harm(&mut world, yard, "chest_1", 10, true);
    harm(&mut world, forge, "crate_f", 18, true);
    wear(&mut world, forge, DOOR, BROKEN);
    // An object the party changed without harming it is no job.
    world
        .map_state(yard)
        .objects
        .entry("torch_1".into())
        .or_default()
        .state = Some("out".into());
    assert_eq!(
        jobs(&world, &data),
        [
            thing(FORGE, "crate_f"),
            door(FORGE, DOOR),
            thing(YARD, "chest_1"),
            thing(YARD, "crate_1"),
        ],
        "by map, objects by id and then doors"
    );
    let before = world.clone();
    let _ = service_view(&world, &data);
    assert_eq!(world, before, "the view changes nothing");

    // The test town's smith is in another region: the yard's things are not its work.
    let mut town = inside(&data, "smith");
    harm(&mut town, yard, "crate_1", 18, true);
    assert_eq!(jobs(&town, &data), []);
    refused(
        &mut town,
        &data,
        thing(YARD, "crate_1"),
        Rejection::NotOffered,
    );
}

#[test]
fn the_price_is_ten_times_the_supplies_and_half_for_a_thing_only_damaged() {
    let mut data = objects();
    let mut world = at_forge(&data);
    world.party.gold = 1_000_000;
    let (forge, yard) = (forge(&data), yard(&data));
    harm(&mut world, forge, "crate_f", 18, true);
    harm(&mut world, yard, "crate_1", 3, false);
    harm(&mut world, yard, "chest_1", 10, true);
    wear(
        &mut world,
        forge,
        DOOR,
        DoorWear {
            damage: 4,
            ..DoorWear::default()
        },
    );
    let price = |world: &World, data: &Data, job: Job| {
        let all = offers(world, data);
        let offer = all.iter().find(|o| o.command == repair(job.clone()));
        let offer = offer.unwrap_or_else(|| panic!("no offer {job:?}"));
        assert_eq!(offer.refusal, None, "{job:?}");
        assert_eq!(offer.pays, None);
        offer.price.expect("a price")
    };
    assert_eq!(
        price(&world, &data, thing(FORGE, "crate_f")),
        1000,
        "Medium"
    );
    assert_eq!(price(&world, &data, thing(YARD, "crate_1")), 500, "half");
    assert_eq!(price(&world, &data, thing(YARD, "chest_1")), 500, "Small");
    assert_eq!(
        price(&world, &data, door(FORGE, DOOR)),
        500,
        "a damaged door"
    );

    // The view's price is what the command charges, for every job.
    for offer in offers(&world, &data) {
        let mut after = world.clone();
        apply(&mut after, &data, Command::Service(offer.command.clone())).unwrap();
        assert_eq!(
            Some(world.party.gold - after.party.gold),
            offer.price,
            "{offer:?}"
        );
    }

    // By size: a Tiny thing has no supplies to count and costs 2 gp; a Large one 20 gp.
    let chest = data.registry.objects.get("objects:object:chest").unwrap();
    for (size, broken, damaged) in [
        (Size::Tiny, 200, 100),
        (Size::Large, 2000, 1000),
        (Size::Gargantuan, 8000, 4000),
    ] {
        let kind = data.objects.get_mut(&chest).unwrap();
        kind.attributes.size = Some(size);
        // A thing over Large has its numbers written.
        kind.attributes.durability.as_mut().unwrap().hit_points = Some(40);
        assert_eq!(price(&world, &data, thing(YARD, "chest_1")), broken);
        let mut dented = world.clone();
        harm(&mut dented, yard, "chest_1", 1, false);
        assert_eq!(price(&dented, &data, thing(YARD, "chest_1")), damaged);
    }
}

#[test]
fn a_bought_repair_takes_the_coin_and_ten_minutes_and_makes_the_thing_whole() {
    let data = objects();
    let mut world = at_forge(&data);
    let (forge, yard) = (forge(&data), yard(&data));
    harm(&mut world, yard, "chest_1", 10, true);
    world
        .map_state(yard)
        .objects
        .get_mut("chest_1")
        .unwrap()
        .state = Some("smashed".into());
    wear(&mut world, forge, DOOR, BROKEN);
    let packs = world.party.clone();
    let clock = minutes(&world);

    let events = ask(&mut world, &data, thing(YARD, "chest_1"));
    assert_eq!(
        events[..2],
        [
            Event::RepairBought {
                service: "base:service:smith".into(),
                cost: 500
            },
            Event::ObjectRepaired {
                map: YARD.into(),
                object: "chest_1".into(),
                by: None,
                mended: false,
            },
        ],
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Check { .. } | Event::ItemSpent { .. })),
        "no check and no supplies: {events:?}"
    );
    assert_eq!(world.party.gold, 4500);
    assert_eq!(minutes(&world), clock + 10, "a transaction's time");
    let chest = sim_objects::saved(&world, yard, "chest_1");
    assert_eq!(
        chest.map_or((0, false, None), |s| (s.damage, s.broken, s.state.clone())),
        (0, false, None),
        "whole, and out of its broken state: as it was placed"
    );
    assert_eq!(world.mode.kind(), ModeKind::Town, "still at the smith");

    let events = ask(&mut world, &data, door(FORGE, DOOR));
    let (x, y, facing) = DOOR;
    assert!(
        events.contains(&Event::DoorRepaired {
            map: FORGE.into(),
            level: Level::GROUND,
            x,
            y,
            facing,
            by: None,
            mended: false,
        }),
        "{events:?}"
    );
    assert_eq!(world.party.gold, 3500, "a Medium door, broken: 10 gp");
    let state = world.maps.get(&forge).cloned().unwrap_or_default();
    assert_eq!(
        state.door_wear(Level::GROUND, x, y, facing),
        DoorWear::default()
    );
    assert!(!state.door_open(Level::GROUND, x, y, facing), "hung shut");
    assert_eq!(world.party.members, packs.members, "no pack is touched");
    assert_eq!(world.party.inventory, packs.inventory);
    assert_eq!(jobs(&world, &data), [], "nothing is left to repair");

    crate::disk::round_trip(&world, &data, "a bought repair");
}

#[test]
fn a_repair_is_refused_with_the_world_as_it_was() {
    let data = objects();
    let mut world = at_forge(&data);
    let (forge, yard) = (forge(&data), yard(&data));
    harm(&mut world, forge, "crate_f", 18, true);

    world.party.gold = 999;
    let short = Rejection::CannotAfford {
        cost: 1000,
        gold: 999,
    };
    refused(&mut world, &data, thing(FORGE, "crate_f"), short.clone());
    let all = offers(&world, &data);
    assert_eq!(all[0].price, Some(1000), "priced all the same");
    assert_eq!(all[0].refusal, Some(short));
    world.party.gold = 5000;

    // A doorway that fell in is listed and refused until the party clears it.
    wear(
        &mut world,
        forge,
        DOOR,
        DoorWear {
            collapsed: true,
            ..BROKEN
        },
    );
    refused(&mut world, &data, door(FORGE, DOOR), Rejection::Buried);
    let all = offers(&world, &data);
    let buried = all.iter().find(|o| o.command == repair(door(FORGE, DOOR)));
    let buried = buried.expect("a buried doorway is listed");
    assert_eq!(
        (buried.price, buried.refusal.clone()),
        (None, Some(Rejection::Buried))
    );
    wear(&mut world, forge, DOOR, BROKEN);
    ask(&mut world, &data, door(FORGE, DOOR));

    refused(
        &mut world,
        &data,
        thing(YARD, "crate_1"),
        Rejection::NothingToRepair,
    );
    refused(
        &mut world,
        &data,
        door(FORGE, DOOR),
        Rejection::NothingToRepair,
    );
    refused(
        &mut world,
        &data,
        thing(YARD, "torch_1"),
        Rejection::CannotBeDamaged,
    );
    refused(
        &mut world,
        &data,
        thing(YARD, "no_such"),
        Rejection::NoSuchObject {
            object: "no_such".into(),
        },
    );
    refused(
        &mut world,
        &data,
        door(FORGE, (3, 0, Facing::East)),
        Rejection::NoDoor,
    );
    refused(
        &mut world,
        &data,
        thing("objects:map:nowhere", "crate_1"),
        Rejection::NotOffered,
    );

    // A thing taken from the map is no job, harmed or not.
    harm(&mut world, yard, "stone_1", 0, false);
    let stone = world.map_state(yard).objects.get_mut("stone_1").unwrap();
    (stone.removed, stone.broken) = (true, true);
    assert_eq!(
        jobs(&world, &data),
        [thing(FORGE, "crate_f")],
        "only the crate that is still broken"
    );
    refused(
        &mut world,
        &data,
        thing(YARD, "stone_1"),
        Rejection::NoSuchObject {
            object: "stone_1".into(),
        },
    );

    // No other service repairs.
    let mut temple = inside(&data, "temple");
    harm(&mut temple, yard, "crate_1", 18, true);
    refused(
        &mut temple,
        &data,
        thing(YARD, "crate_1"),
        Rejection::NotOffered,
    );
}

#[test]
fn the_smith_repairs_a_thing_no_craft_is_named_for() {
    let mut data = objects();
    let crate_kind = data.registry.objects.get("objects:object:crate").unwrap();
    // With no material the armor class is written, or it could not be damaged at all.
    let kind = data.objects.get_mut(&crate_kind).unwrap();
    kind.attributes.material = None;
    kind.attributes.durability.as_mut().unwrap().armor_class = Some(15);
    let mut world = at_forge(&data);
    let forge = forge(&data);
    harm(&mut world, forge, "crate_f", 18, true);
    let events = ask(&mut world, &data, thing(FORGE, "crate_f"));
    assert!(
        events.contains(&Event::RepairBought {
            service: "base:service:smith".into(),
            cost: 1000
        }),
        "{events:?}"
    );
    let state = sim_objects::saved(&world, forge, "crate_f");
    assert!(!state.is_some_and(|s| s.broken));
}

#[test]
fn a_pack_s_rule_sets_the_smith_s_price() {
    let mut data = objects();
    let slot = data.rules.slot("smith.repair_cost").expect("base's slot");
    assert_eq!(slot.inputs(), ["supplies", "broken"]);
    data.rules
        .set_slot("smith.repair_cost", "supplies * 100 + broken")
        .expect("it compiles");
    let mut world = at_forge(&data);
    harm(&mut world, forge(&data), "crate_f", 18, true);
    let events = ask(&mut world, &data, thing(FORGE, "crate_f"));
    assert!(
        events.contains(&Event::RepairBought {
            service: "base:service:smith".into(),
            cost: 201
        }),
        "{events:?}"
    );
}
