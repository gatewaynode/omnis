//! Repair as the player meets it (`tasks/plans/repair.md`, step 10b): the square offers it by
//! name when something in touch is harmed, never as the default action; the target step lists
//! each thing with what it takes; the work and its outcome are told, and the view is drawn
//! anew with the door back in its frame.

use crate::common;
use omnis_sim::api::Chosen;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::{Data, Material};
use omnis_sim::{Command, Event, Rejection, Target};
use omnis_vector::rolllog::Names;
use omnis_vector::screen::actions::{self, Does};
use omnis_vector::shell::session::reshapes;
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;
use omnis_vector::targeting::{self, Aim};

const TORCH: Target = Target::Door {
    x: 6,
    y: 3,
    facing: Facing::South,
};

/// The yard with doorways that never fall in.
fn data() -> Data {
    let mut data = common::objects_data();
    let yard = data.registry.maps.get("objects:map:yard").unwrap();
    let map = data.maps.get_mut(&yard).unwrap();
    map.def.door_make.collapse_percent = 0;
    data
}

/// Before the torch's door, which the fighter has broken.
fn broken(data: &Data) -> SimWorld {
    let mut world = crate::loot::yard(data, 6, 3, Facing::South);
    let fighter = common::views(&world, data).party.members[0].member;
    let command = Command::Attack {
        member: fighter,
        target: TORCH,
    };
    for _ in 0..200 {
        let events = world.apply(data, command.clone()).expect("a blow");
        if events.iter().any(|e| matches!(e, Event::DoorBroken { .. })) {
            return world;
        }
    }
    panic!("the door never broke");
}

/// `count` of an item into the party's stores.
fn give(world: &mut SimWorld, data: &Data, item: &str, count: u16) {
    let item = data.registry.items.get(item).expect("a base item");
    world.party.inventory.push((item, count));
}

fn offered(world: &SimWorld, data: &Data) -> bool {
    let offers = actions::available(world, data);
    let repair = offers.iter().find(|o| o.does == Does::Repair);
    if let Some(offer) = repair {
        assert_eq!(offer.label, "Repair...");
    }
    repair.is_some()
}

/// The lines of the torch's doorway itself.
fn door_lines(world: &SimWorld, data: &Data) -> usize {
    use omnis_vector::geometry::SegKind;
    let map = data.registry.maps.get("objects:map:yard").unwrap();
    let scene = omnis_vector::scene::Scene::build(data, map, Level::GROUND, world);
    let near = |p: [f32; 3]| (p[2] - 4.0).abs() < 0.3 && (6.0..=7.0).contains(&p[0]);
    let own = scene.own.iter();
    own.filter(|s| s.kind == SegKind::Door && near(s.a) && near(s.b))
        .count()
}

#[test]
fn the_square_offers_a_repair_by_name_when_something_in_touch_is_harmed() {
    let data = data();
    let whole = crate::loot::yard(&data, 6, 3, Facing::South);
    assert!(!offered(&whole, &data), "nothing is harmed");
    let world = broken(&data);
    assert!(offered(&world, &data));
    // Shift+Space never starts the work: hours are asked for by name.
    assert!(!actions::runs_by_default(&Does::Repair));
    assert!(!actions::runs_by_default(&Does::Attack));
    assert!(!actions::runs_by_default(&Does::Command(Command::Clear)));
    assert!(actions::runs_by_default(&Does::Command(Command::Interact)));
    assert_ne!(actions::default_action(&world, &data), Some(Does::Repair));
    // The target step: the door, refused for want of a tool until one is carried.
    let fighter = common::views(&world, &data).party.members[0].member;
    let rows = world.targets(&data, fighter, &Chosen::Repair);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].refusal,
        Some(Rejection::NeedsTool {
            material: Material::Wood
        })
    );
    let aim = Aim::repair(fighter);
    assert_eq!(
        aim.command(TORCH),
        Some(Command::Repair {
            member: fighter,
            target: TORCH
        })
    );
    assert_eq!(aim.command(Target::Member(fighter)), None);
}

#[test]
fn a_target_s_row_says_what_the_repair_takes() {
    let data = data();
    let world = broken(&data);
    let quote = world.repair_quote(&data, &TORCH).expect("a quote");
    assert_eq!(targeting::repair_note(&quote), "60 min, 2 supplies (wood)");
    let patch = omnis_sim::repair::RepairQuote {
        minutes: 30,
        supplies: 0,
        material: Material::Metal,
    };
    assert_eq!(targeting::repair_note(&patch), "30 min");
    let one = omnis_sim::repair::RepairQuote {
        supplies: 1,
        ..patch
    };
    assert_eq!(targeting::repair_note(&one), "30 min, 1 supply (metal)");
    let whole = crate::loot::yard(&data, 6, 3, Facing::South);
    assert_eq!(
        whole.repair_quote(&data, &TORCH),
        Err(Rejection::NothingToRepair)
    );
}

#[test]
fn a_repair_is_told_and_the_door_is_drawn_back_in_its_frame() {
    let data = data();
    let shut = door_lines(&crate::loot::yard(&data, 6, 3, Facing::South), &data);
    let mut world = broken(&data);
    assert_ne!(door_lines(&world, &data), shut);
    give(&mut world, &data, "base:item:carpenters_tools", 1);
    give(&mut world, &data, "base:item:timber", 60);
    let views = common::views(&world, &data);
    let fighter = views.party.members[0].member;
    let mut names = Names::default();
    names.observe(&views.party);
    names.locate(views.position.map);
    let who = views.party.members[0].name.clone();
    let command = Command::Repair {
        member: fighter,
        target: TORCH,
    };
    let mut told = Vec::new();
    for _ in 0..30 {
        let events = world.apply(&data, command.clone()).expect("a repair");
        if let Some(done) = events
            .iter()
            .find(|e| matches!(e, Event::DoorRepaired { .. }))
        {
            assert!(reshapes(done), "the view is drawn anew");
            told.push(text::describe(done, &names, &data).expect("told"));
            break;
        }
    }
    assert_eq!(told, [format!("{who} repairs the door")]);
    assert_eq!(door_lines(&world, &data), shut, "a door in its frame, shut");
    assert!(!offered(&world, &data), "nothing more to repair");
    // An object, by the spell and by tools.
    let mended = Event::ObjectRepaired {
        object: "crate_1".into(),
        by: fighter,
        mended: true,
    };
    let line = text::describe(&mended, &names, &data).expect("told");
    assert!(line.starts_with(&format!("{who} mends ")), "{line}");
    assert!(reshapes(&mended));
    let repaired = Event::ObjectRepaired {
        object: "crate_1".into(),
        by: fighter,
        mended: false,
    };
    let line = text::describe(&repaired, &names, &data).expect("told");
    assert!(line.starts_with(&format!("{who} repairs ")), "{line}");
}
