//! Objects in cells (M8.5 step 2): kinds of every shape and interaction, played on the fixture
//! pack `tests/packs/objects`, loaded after the base and test packs.

use crate::common::{base_pack, test_pack};
use omnis_data::{Data, ObjectEffect, Requirement, Shape, Verb, load_packs};
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
