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

pub fn yard(data: &Data) -> MapId {
    data.registry.maps.get("objects:map:yard").expect("yard")
}

/// A party of one in the yard at `(x, y)` of `level`, facing `facing`.
pub fn in_yard(data: &Data, level: i8, x: u16, y: u16, facing: Facing) -> World {
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

/// Whether `to` is in sight from `from` on the yard's ground, objects as `world` has them.
fn sees(data: &Data, world: &World, from: (u16, u16), to: (u16, u16)) -> bool {
    let map = &data.maps[&yard(data)];
    omnis_sim::visibility::line_of_sight(
        map,
        &data.objects,
        world.maps.get(&yard(data)),
        Level::GROUND,
        from,
        to,
    )
}

#[test]
fn a_tall_object_hides_what_lies_squarely_behind_it_but_not_its_own_cell() {
    let data = objects();
    let mut world = in_yard(&data, 0, 1, 0, Facing::East);
    assert!(sees(&data, &world, (1, 0), (3, 0)), "the statue's own cell");
    assert!(
        sees(&data, &world, (1, 0), (5, 0)),
        "a corner of (5, 0) shows past the statue"
    );
    assert!(!sees(&data, &world, (1, 0), (6, 0)), "squarely behind it");
    let map = &data.maps[&yard(&data)];
    assert_eq!(
        omnis_sim::visibility::ray(map, &data.objects, None, world.position, 8),
        [(2, 0), (3, 0), (4, 0), (5, 0)],
        "the spyglass stops where the statue hides the rest"
    );
    world.maps.entry(yard(&data)).or_default().objects.insert(
        "statue_1".into(),
        ObjectState {
            removed: true,
            ..ObjectState::default()
        },
    );
    assert!(sees(&data, &world, (1, 0), (6, 0)), "the statue gone");
}

#[test]
fn an_object_hides_only_where_its_top_rises_above_the_line_of_sight() {
    let mut data = objects();
    let world = in_yard(&data, 0, 0, 1, Facing::East);
    assert!(
        !sees(&data, &world, (0, 1), (6, 1)),
        "the crate, six eighths tall, rises above the eye at four"
    );
    let crate_kind = data.registry.objects.get("objects:object:crate").unwrap();
    data.objects.get_mut(&crate_kind).unwrap().look.height = 2;
    assert!(
        sees(&data, &world, (0, 1), (6, 1)),
        "two eighths tall and close to the eye, the line passes over it"
    );
    data.objects
        .get_mut(&crate_kind)
        .unwrap()
        .attributes
        .blocks_sight = false;
    data.objects.get_mut(&crate_kind).unwrap().look.height = 6;
    assert!(
        sees(&data, &world, (0, 1), (6, 1)),
        "a kind that does not block sight hides nothing"
    );
}

#[test]
fn the_top_is_tested_where_the_line_leaves_the_footprint_too() {
    let mut data = objects();
    let world = in_yard(&data, 0, 0, 1, Facing::East);
    let crate_kind = data.registry.objects.get("objects:object:crate").unwrap();
    data.objects.get_mut(&crate_kind).unwrap().look.height = 3;
    assert!(
        !sees(&data, &world, (0, 1), (3, 1)),
        "three eighths tall: under the line where it enters, above it where it leaves"
    );
}

#[test]
fn an_object_in_the_viewer_s_own_cell_can_hide_what_lies_beyond() {
    let data = objects();
    let world = in_yard(&data, 0, 1, 1, Facing::East);
    assert!(
        !sees(&data, &world, (1, 1), (6, 1)),
        "looking out from inside the crate's cell"
    );
}

#[test]
fn an_object_spilling_into_a_cell_does_not_hide_its_own_cell_behind_the_spill() {
    let mut data = objects();
    let gate = data.registry.objects.get("objects:object:gate").unwrap();
    data.objects.get_mut(&gate).unwrap().attributes.blocks_sight = true;
    let world = in_yard(&data, 0, 3, 1, Facing::South);
    assert!(sees(&data, &world, (3, 1), (3, 2)), "the gate's cell");
    assert!(!sees(&data, &world, (3, 1), (3, 4)), "beyond the gate");
    assert!(
        sees(&data, &world, (3, 4), (3, 2)),
        "from the south, through the cell its spill reaches, the gate's own cell is seen"
    );
}

#[test]
fn the_yard_s_lore_resolves_by_cell_by_kind_and_by_placement() {
    let data = objects();
    let map = &data.maps[&yard(&data)];
    let at = |x: u16, y: u16| {
        map.lore
            .get(&(Level::GROUND, x, y))
            .cloned()
            .unwrap_or_default()
    };
    let bootprints = at(2, 2);
    assert_eq!(bootprints.len(), 1);
    assert_eq!(
        (
            bootprints[0].id.as_str(),
            bootprints[0].text.as_str(),
            bootprints[0].dc,
            bootprints[0].once
        ),
        ("bootprints", "objects:text:lore.bootprints", None, false)
    );
    let carving = at(4, 3);
    assert_eq!((carving[0].dc, carving[0].once), (Some(10), true));
    assert!(at(3, 3).is_empty());
    // The crate's lore is its kind's; the statue's its placement's.
    let crate_kind = data.registry.objects.get("objects:object:crate").unwrap();
    let stencil = data.objects[&crate_kind]
        .lore
        .as_ref()
        .expect("the crate's lore");
    assert_eq!(
        (stencil.text.as_str(), stencil.dc, stencil.once),
        ("objects:text:crate.lore", None, false)
    );
    let statue = map.objects.iter().find(|p| p.id == "statue_1").unwrap();
    let name = statue.lore.as_ref().expect("the statue's lore");
    assert_eq!(
        (name.text.as_str(), name.dc, name.once),
        ("objects:text:statue.lore", Some(12), true)
    );
    assert!(
        map.objects
            .iter()
            .find(|p| p.id == "crate_1")
            .unwrap()
            .lore
            .is_none()
    );
}
