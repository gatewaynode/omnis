//! `MapData::resolve` (M5.1): a map definition resolved in memory against loaded packs gives
//! exactly what the loader built from its file, and refuses what the loader would refuse, in the
//! loader's words. The editor, the preview and Playtest stand on it.

use crate::common::{base_pack, fixture_pack, load_test_packs, test_pack};
use omnis_data::{Data, MapData, Portal, load_packs, partner_of, unlinked_portals};

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
fn a_portal_whose_link_names_nothing_loaded_is_refused() {
    let data = load_test_packs();
    let mut def = town(&data);
    def.portals[0].link = Some("nowhere".to_owned());
    assert_eq!(
        messages(MapData::resolve(&def, &data)),
        ["portal 'town-gate' at (11, 2) links to 'nowhere', which no loaded map has"]
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
    // The town's gate now links to the stairs down, which link to the dungeon: the meadow hears
    // of its stairs' two claimants, and of the gate's three-way link.
    data.maps.get_mut(&town_id).expect("the town").def.portals[0].link =
        Some("meadow-stairs".to_owned());
    let mut got = messages(MapData::resolve(&meadow, &data));
    got.sort();
    assert_eq!(
        got,
        [
            "portal 'meadow-stairs' at (16, 5) is linked with both 'dungeon-stairs' on 'test:map:dungeon' and 'town-gate' on 'test:map:town'",
            "test:map:town: portal 'town-gate' links to 'meadow-stairs', which links to 'dungeon-stairs'",
        ]
    );
}

// ---- The entrances and exits enhancement (W1, 2026-11-01; step 2 on links) ----

fn def_of(data: &Data, id: &str) -> omnis_data::MapDef {
    let id = data.registry.maps.get(id).unwrap_or_else(|| panic!("{id}"));
    data.maps[&id].def.clone()
}

fn portal(id: &str, (x, y): (u16, u16), facing: omnis_core::Facing, link: Option<&str>) -> Portal {
    Portal {
        level: omnis_core::Level::GROUND,
        x,
        y,
        id: id.to_owned(),
        link: link.map(str::to_owned),
        facing,
        marker: Some("signpost".to_owned()),
    }
}

#[test]
fn an_unlinked_portal_resolves_clean_and_is_a_plain_tile() {
    let data = load_test_packs();
    let mut town = def_of(&data, "test:map:town");
    town.portals
        .push(portal("spare", (5, 2), omnis_core::Facing::South, None));
    let resolved = MapData::resolve(&town, &data).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        resolved.portals.len(),
        1,
        "the gate alone: {:?}",
        resolved.portals
    );
    assert!(
        resolved
            .portal_at(omnis_core::Level::GROUND, 5, 2)
            .is_none()
    );
    assert_eq!(partner_of(&town.portals[1], &town, &data), None);
    let unlinked: Vec<(&str, &str)> = unlinked_portals(&town, &data)
        .into_iter()
        .map(|(map, p)| (map, p.id.as_str()))
        .collect();
    assert_eq!(unlinked, [("test:map:town", "spare")]);
}

#[test]
fn a_link_stated_by_one_side_pairs_both_and_lands_ahead_of_the_partner() {
    let ground = omnis_core::Level::GROUND;
    let mut data = load_test_packs();
    let town_id = data.registry.maps.get("test:map:town").expect("the town");
    let meadow_id = data
        .registry
        .maps
        .get("test:map:meadow")
        .expect("the meadow");
    // The file states the link on the town's side alone.
    let town = def_of(&data, "test:map:town");
    let meadow = def_of(&data, "test:map:meadow");
    assert_eq!(town.portals[0].link.as_deref(), Some("meadow-gate"));
    assert_eq!(meadow.portals[1].link, None);
    let check = |data: &Data| {
        let town = MapData::resolve(&def_of(data, "test:map:town"), data).unwrap();
        let gate = town.portal_at(ground, 11, 2).expect("the gate");
        assert_eq!(
            (gate.to_map, gate.to_x, gate.to_y, gate.to_facing),
            (meadow_id, 16, 30, omnis_core::Facing::North),
            "ahead of the meadow's gate, facing its way"
        );
        let meadow = MapData::resolve(&def_of(data, "test:map:meadow"), data).unwrap();
        let home = meadow.portal_at(ground, 16, 31).expect("the road home");
        assert_eq!(
            (home.to_map, home.to_x, home.to_y, home.to_facing),
            (town_id, 10, 2, omnis_core::Facing::West)
        );
        let (far, partner) = partner_of(&meadow.def.portals[1], &meadow.def, data).expect("linked");
        assert_eq!((far, partner.id.as_str()), ("test:map:town", "town-gate"));
    };
    check(&data);
    // Stated on the meadow's side instead: the same pair.
    data.maps.get_mut(&town_id).unwrap().def.portals[0].link = None;
    data.maps.get_mut(&meadow_id).unwrap().def.portals[1].link = Some("town-gate".to_owned());
    check(&data);
    // Stated on both: the same pair.
    data.maps.get_mut(&town_id).unwrap().def.portals[0].link = Some("meadow-gate".to_owned());
    check(&data);
}

#[test]
fn two_claimants_of_one_portal_are_refused_on_the_claimed_side() {
    let data = load_test_packs();
    let mut town = def_of(&data, "test:map:town");
    town.portals.push(portal(
        "spare",
        (5, 2),
        omnis_core::Facing::South,
        Some("meadow-gate"),
    ));
    assert_eq!(
        messages(MapData::resolve(&town, &data)),
        [
            "test:map:meadow: portal 'meadow-gate' at (16, 31) is linked with both 'town-gate' on 'test:map:town' and 'spare' on 'test:map:town'"
        ]
    );
}

#[test]
fn a_portal_id_is_one_across_the_loaded_maps_and_within_a_map() {
    let data = load_test_packs();
    let mut town = def_of(&data, "test:map:town");
    town.portals.push(portal(
        "meadow-gate",
        (5, 2),
        omnis_core::Facing::South,
        None,
    ));
    assert_eq!(
        messages(MapData::resolve(&town, &data)),
        ["portal id 'meadow-gate' is also on 'test:map:meadow'"]
    );
    town.portals[1].id = "spare".to_owned();
    town.portals
        .push(portal("spare", (6, 2), omnis_core::Facing::South, None));
    assert_eq!(
        messages(MapData::resolve(&town, &data)),
        ["portals at (5, 2) and (6, 2) share the id 'spare'"]
    );
    town.portals.pop();
    town.portals[1].id = String::new();
    assert_eq!(
        messages(MapData::resolve(&town, &data)),
        ["portal at (5, 2) has no id"]
    );
    town.portals[1].id = " spare".to_owned();
    assert_eq!(
        messages(MapData::resolve(&town, &data)),
        ["portal at (5, 2) has an id that is not 1 to 32 printable ASCII characters"]
    );
}

#[test]
fn a_landing_ahead_of_the_partner_must_be_inside_the_map_and_free_of_portals() {
    let mut data = load_test_packs();
    let dungeon_id = data
        .registry
        .maps
        .get("test:map:dungeon")
        .expect("the dungeon");
    // The stairs up at (0, 0) turned west: the meadow's stairs down would land off the map.
    let mut dungeon = def_of(&data, "test:map:dungeon");
    dungeon.portals[0].facing = omnis_core::Facing::West;
    let words = "portal 'meadow-stairs' at (16, 5) lands ahead of 'dungeon-stairs' on 'test:map:dungeon', outside the map";
    assert_eq!(
        messages(MapData::resolve(&dungeon, &data)),
        [format!("test:map:meadow: {words}")]
    );
    data.maps.get_mut(&dungeon_id).unwrap().def.portals[0].facing = omnis_core::Facing::West;
    let meadow = def_of(&data, "test:map:meadow");
    assert_eq!(messages(MapData::resolve(&meadow, &data)), [words]);
    // A portal standing on the landing, (1, 0) ahead of the stairs up facing east.
    let mut dungeon = def_of(&data, "test:map:dungeon");
    dungeon.portals[0].facing = omnis_core::Facing::East;
    let mut block = dungeon.portals[0].clone();
    (block.id, block.x) = ("block".to_owned(), 1);
    dungeon.portals.push(block);
    assert_eq!(
        messages(MapData::resolve(&dungeon, &data)),
        [
            "test:map:meadow: portal 'meadow-stairs' at (16, 5) lands ahead of 'dungeon-stairs' on 'test:map:dungeon' at (1, 0), where a portal stands"
        ]
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
fn maps_the_entry_no_longer_reaches_through_linked_portals_are_refused() {
    let data = load_test_packs();
    let mut town = def_of(&data, "test:map:town");
    // The gate unlinked: the meadow's road home is unlinked too, and the meadow, the dungeon and
    // the depths, still linked among themselves, are cut off from the entry.
    town.portals[0].link = None;
    let mut got = messages(MapData::resolve(&town, &data));
    got.sort();
    assert_eq!(
        got,
        [
            "map 'test:map:depths' cannot be reached from the entry map 'test:map:town' through portals",
            "map 'test:map:dungeon' cannot be reached from the entry map 'test:map:town' through portals",
            "map 'test:map:meadow' cannot be reached from the entry map 'test:map:town' through portals",
        ]
    );
    // The stairs into the depths removed: the depths' stairs up are unlinked, and a map with no
    // linked portal is unfinished, not unreached.
    let mut dungeon = def_of(&data, "test:map:dungeon");
    dungeon.portals.retain(|p| p.id != "dungeon-depths");
    assert!(MapData::resolve(&dungeon, &data).is_ok());
}
