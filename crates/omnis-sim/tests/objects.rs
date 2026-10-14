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
