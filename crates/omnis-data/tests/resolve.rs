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
