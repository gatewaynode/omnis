//! `MapData::resolve` (M5.1): a map definition resolved in memory against loaded packs gives
//! exactly what the loader built from its file, and refuses what the loader would refuse, in the
//! loader's words. The editor, the preview and Playtest stand on it.

use crate::common::{base_pack, fixture_pack, load_test_packs, test_pack};
use omnis_data::{Data, MapData, load_packs};

fn with_fixture(name: &str) -> Data {
    let roots = [base_pack(), test_pack(), fixture_pack(name)];
    let roots: Vec<&std::path::Path> = roots.iter().map(PathBuf::as_path).collect();
    load_packs(&roots).unwrap_or_else(|report| panic!("{name}: {report}"))
}
use std::path::PathBuf;

fn every_map_resolves_as_loaded(data: &Data) {
    assert!(!data.maps.is_empty());
    for map in data.maps.values() {
        let resolved = MapData::resolve(&map.def, data)
            .unwrap_or_else(|errors| panic!("{}: {errors:?}", map.def.id));
        assert_eq!(&resolved, map, "{}", map.def.id);
    }
}

#[test]
fn the_test_packs_maps_resolve_as_loaded() {
    every_map_resolves_as_loaded(&load_test_packs());
}

#[test]
fn the_fixture_packs_maps_resolve_as_loaded() {
    for name in ["levels", "heights", "objects"] {
        every_map_resolves_as_loaded(&with_fixture(name));
    }
}

fn messages(result: Result<MapData, Vec<omnis_data::DataError>>) -> Vec<String> {
    match result {
        Ok(_) => panic!("resolved"),
        Err(errors) => errors.into_iter().map(|e| e.message).collect(),
    }
}

fn town(data: &Data) -> omnis_data::MapDef {
    let id = data.registry.maps.get("test:map:town").expect("the town");
    data.maps[&id].def.clone()
}

#[test]
fn a_missing_name_key_is_refused_in_the_loaders_words() {
    let data = load_test_packs();
    let mut def = town(&data);
    def.name = "test:text:map.nowhere.name".into();
    assert_eq!(
        messages(MapData::resolve(&def, &data)),
        ["name text key 'test:text:map.nowhere.name' is not defined in any language"]
    );
}

#[test]
fn a_new_map_belongs_to_no_region_until_a_region_names_it() {
    let data = load_test_packs();
    let mut def = town(&data);
    def.id = "test:map:hamlet".into();
    def.portals.clear();
    assert_eq!(
        messages(MapData::resolve(&def, &data)),
        ["map 'test:map:hamlet' belongs to no region"]
    );
}

#[test]
fn shape_errors_come_first_and_alone() {
    let data = load_test_packs();
    let mut def = town(&data);
    def.width = 0;
    def.layout[1].push('x');
    let got = messages(MapData::resolve(&def, &data));
    assert!(got[0].starts_with("size 0x5 must be within"), "{got:?}");
    assert!(got.iter().any(|m| m.contains("row 1")), "{got:?}");
    assert!(got.iter().all(|m| !m.contains("region")), "{got:?}");
}

#[test]
fn a_portal_landing_outside_its_destination_is_refused() {
    let data = load_test_packs();
    let mut def = town(&data);
    def.portals[0].to_x = 999;
    assert_eq!(
        messages(MapData::resolve(&def, &data)),
        ["portal at (11, 2) lands outside map 'test:map:meadow'"]
    );
}

#[test]
fn another_maps_portal_landing_on_this_one_is_checked_too() {
    let mut data = load_test_packs();
    let town_id = data.registry.maps.get("test:map:town").expect("the town");
    let meadow_id = data
        .registry
        .maps
        .get("test:map:meadow")
        .expect("the meadow");
    let meadow = data.maps[&meadow_id].def.clone();
    assert!(MapData::resolve(&meadow, &data).is_ok());
    data.maps.get_mut(&town_id).expect("the town").def.portals[0].to_x = 999;
    assert_eq!(
        messages(MapData::resolve(&meadow, &data)),
        ["test:map:town: portal at (11, 2) lands outside map 'test:map:meadow'"]
    );
}

// ---- The entrances and exits enhancement (W1, 2026-11-01) ----

fn def_of(data: &Data, id: &str) -> omnis_data::MapDef {
    let id = data.registry.maps.get(id).unwrap_or_else(|| panic!("{id}"));
    data.maps[&id].def.clone()
}

#[test]
fn a_portal_landing_on_rock_or_on_a_portal_is_refused() {
    let data = load_test_packs();
    let mut meadow = def_of(&data, "test:map:meadow");
    assert_eq!(meadow.portals[0].to_map, "test:map:dungeon");
    // (3, 3) of the dungeon is rock.
    (meadow.portals[0].to_x, meadow.portals[0].to_y) = (3, 3);
    assert_eq!(
        messages(MapData::resolve(&meadow, &data)),
        ["portal at (16, 5) lands on (3, 3) of 'test:map:dungeon', which the party cannot enter"]
    );
    // (0, 0) of the dungeon holds the stairs back up.
    (meadow.portals[0].to_x, meadow.portals[0].to_y) = (0, 0);
    assert_eq!(
        messages(MapData::resolve(&meadow, &data)),
        ["portal at (16, 5) lands on (0, 0) of 'test:map:dungeon', where a portal stands"]
    );
}

#[test]
fn a_portal_without_a_way_back_behind_its_landing_is_refused_from_both_maps() {
    let mut data = load_test_packs();
    let mut town = def_of(&data, "test:map:town");
    // The gate lands on (16, 30) of the meadow facing north, the road home at (16, 31) behind.
    // Facing south, the way back would have to stand on (16, 29), where nothing does.
    town.portals[0].to_facing = omnis_core::Facing::South;
    let words = "portal at (11, 2) has no way back: no portal behind its landing (16, 30) on \
                 'test:map:meadow' leads to 'test:map:town'";
    assert_eq!(messages(MapData::resolve(&town, &data)), [words]);
    let meadow = def_of(&data, "test:map:meadow");
    let town_id = data.registry.maps.get("test:map:town").expect("the town");
    data.maps.get_mut(&town_id).expect("the town").def.portals[0].to_facing =
        omnis_core::Facing::South;
    assert_eq!(
        messages(MapData::resolve(&meadow, &data)),
        [format!("test:map:town: {words}")]
    );
}

#[test]
fn a_start_the_party_cannot_stand_on_is_refused() {
    let data = load_test_packs();
    let mut dungeon = def_of(&data, "test:map:dungeon");
    dungeon.start = (3, 3, omnis_core::Facing::North);
    assert_eq!(
        messages(MapData::resolve(&dungeon, &data)),
        ["start (3, 3) is on a tile the party cannot enter"]
    );
}

#[test]
fn a_map_the_entry_no_longer_reaches_is_refused_with_its_orphaned_way_back() {
    let data = load_test_packs();
    let mut dungeon = def_of(&data, "test:map:dungeon");
    let to_depths = dungeon
        .portals
        .iter()
        .position(|p| p.to_map == "test:map:depths")
        .expect("the stairs down");
    dungeon.portals.remove(to_depths);
    let mut got = messages(MapData::resolve(&dungeon, &data));
    got.sort();
    assert_eq!(
        got,
        [
            "map 'test:map:depths' cannot be reached from the entry map 'test:map:town' through portals",
            "test:map:depths: portal at (0, 0) has no way back: no portal behind its landing (22, 23) on 'test:map:dungeon' leads to 'test:map:depths'",
        ]
    );
}
