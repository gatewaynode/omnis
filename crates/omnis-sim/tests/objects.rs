//! Objects in cells (M8.5 step 2): kinds of every shape and interaction, played on the fixture
//! pack `tests/packs/objects`, loaded after the base and test packs.

use crate::common::{base_pack, party_of, test_pack};
use omnis_core::{Direction, Facing, Level, MapId, Position};
use omnis_data::{Data, ObjectEffect, Requirement, Shape, Verb, load_packs};
use omnis_sim::objects::ObjectState;
use omnis_sim::{BlockReason, Command, Event, Settings, World, apply, query};
use std::path::PathBuf;

/// The base and test packs with the object fixtures on top.
pub fn objects() -> Data {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/packs/objects");
    load_packs(&[&base_pack(), &test_pack(), &fixture]).unwrap_or_else(|r| panic!("{r}"))
}

#[test]
fn the_fixture_s_kinds_load_with_their_states_and_flags() {
    let data = objects();
    let names: Vec<&str> = data.registry.objects.names().collect();
    assert_eq!(
        names,
        [
            "objects:object:chest",
            "objects:object:crate",
            "objects:object:fountain",
            "objects:object:gate",
            "objects:object:lever",
            "objects:object:plaque",
            "objects:object:rune",
            "objects:object:statue",
            "objects:object:stone",
            "objects:object:torch",
        ]
    );
    let chest = &data.objects[&data.registry.objects.get("objects:object:chest").unwrap()];
    assert_eq!(chest.initial_state, "closed");
    assert!(chest.states["open"].open && !chest.states["closed"].open);
    assert_eq!(chest.attributes.capacity, 8);
    let opens: Vec<Option<&str>> = chest
        .interactions
        .iter()
        .filter(|i| i.verb == Verb::Open)
        .map(|i| i.label.as_deref())
        .collect();
    assert_eq!(
        opens,
        [
            Some("objects:text:chest.force"),
            Some("objects:text:chest.pick")
        ],
        "two ways to open, told apart by their labels"
    );
    let gate = &data.objects[&data.registry.objects.get("objects:object:gate").unwrap()];
    assert_eq!(gate.look.shape, Shape::Arch);
    assert!(matches!(
        gate.interactions[0].requires[..],
        [Requirement::Flag { min: 1, .. }]
    ));
    assert!(
        data.registry.flags.get("objects:flag:lever").is_some(),
        "a flag an object names is interned at load"
    );
    let stone = &data.objects[&data.registry.objects.get("objects:object:stone").unwrap()];
    assert_eq!(
        stone.interactions[0].effects.last(),
        Some(&ObjectEffect::Remove)
    );
}

#[test]
fn the_shipped_packs_have_no_objects() {
    let data = crate::common::data();
    assert!(data.objects.is_empty() && data.registry.objects.is_empty());
}

/// The ids of the objects whose footprints reach into a cell of the yard.
fn ids_at(data: &Data, level: i8, x: u16, y: u16) -> Vec<String> {
    let yard = &data.maps[&data.registry.maps.get("objects:map:yard").unwrap()];
    yard.objects_at(omnis_core::Level(level), x, y)
        .map(|(_, p)| p.id.clone())
        .collect()
}

#[test]
fn the_yard_places_every_object_in_the_cells_its_footprint_reaches() {
    let data = objects();
    let yard = &data.maps[&data.registry.maps.get("objects:map:yard").unwrap()];
    assert_eq!(yard.objects.len(), 11);
    assert_eq!(ids_at(&data, 0, 1, 1), ["crate_1"]);
    assert_eq!(ids_at(&data, 0, 3, 2), ["gate_1"]);
    assert_eq!(
        ids_at(&data, 0, 3, 3),
        ["gate_1"],
        "the gate stands across the gap, half in the cell south of it"
    );
    assert_eq!(
        ids_at(&data, 0, 4, 1),
        ["plaque_1"],
        "the plaque on the edge"
    );
    assert_eq!(ids_at(&data, 0, 4, 2), ["plaque_1"]);
    assert_eq!(ids_at(&data, 1, 5, 0), ["crate_2"], "the loft's crate");
    assert!(ids_at(&data, 0, 5, 0).is_empty(), "under the loft, nothing");
    assert!(ids_at(&data, 0, 0, 1).is_empty());
    let chest = yard.objects.iter().find(|p| p.id == "chest_1").unwrap();
    assert_eq!(chest.state, "closed", "the kind's initial state");
    let dagger = data.registry.items.get("base:item:dagger").unwrap();
    assert_eq!(chest.contents, [(dagger, 1)]);
    let lever = yard.objects.iter().find(|p| p.id == "lever_1").unwrap();
    assert_eq!((lever.hidden, lever.pos), (Some(12), (-3, -3)));
    let arrows = data.registry.items.get("base:item:arrows").unwrap();
    let pile = &yard.ground[&(omnis_core::Level::GROUND, 2, 2)];
    assert_eq!((pile.pos, &pile.items[..]), ((1, 1), &[(arrows, 10)][..]));
}

#[test]
fn the_shipped_maps_place_nothing() {
    let data = crate::common::data();
    for map in data.maps.values() {
        assert!(map.objects.is_empty() && map.object_cells.is_empty() && map.ground.is_empty());
    }
}

#[test]
fn the_pack_rat_names_its_hoard_and_the_hoard_loads() {
    let data = objects();
    let rat = &data.monsters[&data
        .registry
        .monsters
        .get("objects:monster:pack_rat")
        .unwrap()];
    assert_eq!(rat.loot.as_deref(), Some("objects:loot:hoard"));
    let hoard = &data.loot[&data.registry.loot.get("objects:loot:hoard").unwrap()];
    assert_eq!(hoard.picks, 1);
    let weights: Vec<(u16, u8)> = hoard
        .entries
        .iter()
        .map(|e| (e.weight, e.chance_percent))
        .collect();
    assert_eq!(weights, [(3, 100), (1, 50)]);
    let base = crate::common::data();
    assert!(
        base.loot.is_empty() && base.monsters.values().all(|m| m.loot.is_none()),
        "the shipped monsters drop gold only"
    );
}

fn yard(data: &Data) -> MapId {
    data.registry.maps.get("objects:map:yard").expect("yard")
}

/// A party of one in the yard at `(x, y)` of `level`, facing `facing`.
fn in_yard(data: &Data, level: i8, x: u16, y: u16, facing: Facing) -> World {
    let mut world = World::new(data, 11, Settings::default()).expect("entry map");
    party_of(&mut world, data, 1);
    world.position = Position {
        map: yard(data),
        level: Level(level),
        x,
        y,
        facing,
    };
    world
}

/// Where a step forward ends, or why it did not happen.
fn step_result(world: &mut World, data: &Data) -> Result<(u16, u16), BlockReason> {
    let events = apply(world, data, Command::Step(Direction::Forward)).expect("explore");
    events
        .iter()
        .find_map(|e| match e {
            Event::Blocked { reason } => Some(Err(*reason)),
            _ => None,
        })
        .unwrap_or(Ok((world.position.x, world.position.y)))
}

#[test]
fn a_footprint_in_the_path_blocks_a_step_into_or_out_of_its_cell() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 1, Facing::East);
    assert_eq!(
        step_result(&mut world, &data),
        Err(BlockReason::Object),
        "the crate fills the middle of (1, 1)"
    );
    assert_eq!(query::step_lands(&world, &data, Direction::Forward), None);
    let mut world = in_yard(&data, 0, 5, 4, Facing::North);
    assert_eq!(
        step_result(&mut world, &data),
        Ok((5, 3)),
        "the chest stands against the north side, clear of the path from the south"
    );
    world.position.facing = Facing::East;
    assert_eq!(
        step_result(&mut world, &data),
        Err(BlockReason::Object),
        "but it reaches across the path east"
    );
}

#[test]
fn a_footprint_beside_the_path_lets_the_party_pass() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 0, Facing::East);
    assert_eq!(
        step_result(&mut world, &data),
        Ok((1, 0)),
        "north of the crate"
    );
    assert_eq!(step_result(&mut world, &data), Ok((2, 0)));
    assert_eq!(
        step_result(&mut world, &data),
        Err(BlockReason::Object),
        "the statue"
    );
}

#[test]
fn the_gate_across_the_gap_blocks_from_both_sides_and_its_spill_counts() {
    let data = objects();
    let mut world = in_yard(&data, 0, 3, 1, Facing::South);
    assert_eq!(step_result(&mut world, &data), Ok((3, 2)), "up to the gate");
    assert_eq!(step_result(&mut world, &data), Err(BlockReason::Object));
    let mut world = in_yard(&data, 0, 3, 4, Facing::North);
    assert_eq!(
        step_result(&mut world, &data),
        Ok((3, 3)),
        "into the cell the gate spills into"
    );
    assert_eq!(step_result(&mut world, &data), Err(BlockReason::Object));
}

#[test]
fn a_state_that_does_not_block_or_a_removed_object_lets_the_party_through() {
    let data = objects();
    let mut world = in_yard(&data, 0, 3, 2, Facing::South);
    world.maps.entry(yard(&data)).or_default().objects.insert(
        "gate_1".into(),
        ObjectState {
            state: Some("open".into()),
            ..ObjectState::default()
        },
    );
    assert_eq!(step_result(&mut world, &data), Ok((3, 3)), "the gate open");
    let mut world = in_yard(&data, 0, 0, 1, Facing::East);
    world.maps.entry(yard(&data)).or_default().objects.insert(
        "crate_1".into(),
        ObjectState {
            removed: true,
            ..ObjectState::default()
        },
    );
    assert_eq!(step_result(&mut world, &data), Ok((1, 1)), "the crate gone");
}

#[test]
fn an_object_on_another_level_does_not_block() {
    let data = objects();
    let mut world = in_yard(&data, 0, 4, 0, Facing::East);
    assert_eq!(
        step_result(&mut world, &data),
        Ok((5, 0)),
        "under the loft's crate"
    );
    let mut world = in_yard(&data, 1, 6, 0, Facing::West);
    assert_eq!(
        step_result(&mut world, &data),
        Err(BlockReason::Object),
        "on the loft, its crate is in the way"
    );
}

#[test]
fn object_state_is_saved_only_once_something_changes_and_round_trips() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 0, Facing::East);
    step_result(&mut world, &data).expect("a step");
    let plain = world.to_ron().expect("save");
    assert!(
        !plain.contains("objects: {"),
        "no map writes an objects field until one changes: {plain}"
    );
    world.maps.entry(yard(&data)).or_default().objects.insert(
        "torch_1".into(),
        ObjectState {
            state: Some("out".into()),
            used: [0].into(),
            ..ObjectState::default()
        },
    );
    let text = world.to_ron().expect("save");
    assert!(text.contains("objects: {"), "{text}");
    let back = World::from_ron(&text, &data, false).expect("loads");
    assert_eq!(
        back.maps[&yard(&data)].objects,
        world.maps[&yard(&data)].objects
    );
    assert_eq!(back.fingerprint(), world.fingerprint());
}
