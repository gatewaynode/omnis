//! The fixture pack loads after the base pack, and what it contains is what the maps need.

mod common;

use omnis_core::{Edges, Facing};
use omnis_data::{MapKind, ServiceKind, SlotKind, load_packs};

#[test]
fn test_pack_loads_with_its_maps() {
    let data = common::load_test_packs();
    assert_eq!(data.packs.len(), 2);
    assert_eq!(data.packs[1].id, "test");
    assert_eq!(data.packs[1].depends, ["base"]);
    assert_eq!(data.fingerprints[1].id, "test");
    assert_eq!(data.maps.len(), 3);
    assert_ne!(data.fingerprints[1].hash, 0);

    let dungeon_id = data
        .registry
        .maps
        .get("test:map:dungeon")
        .expect("dungeon interned");
    let meadow_id = data
        .registry
        .maps
        .get("test:map:meadow")
        .expect("meadow interned");
    let dungeon = &data.maps[&dungeon_id];
    let meadow = &data.maps[&meadow_id];
    assert_eq!(
        (dungeon.def.width, dungeon.def.height, dungeon.def.kind),
        (24, 24, MapKind::Dungeon)
    );
    assert_eq!(
        (meadow.def.width, meadow.def.height, meadow.def.kind),
        (32, 32, MapKind::Outdoor)
    );
    assert_eq!(dungeon.cells.len(), 24 * 24);
    assert_eq!(meadow.cells.len(), 32 * 32);
    assert_eq!(data.text("en", dungeon.name), "Test Dungeon");
    assert_eq!(
        data.text("de", meadow.name),
        "test:text:map.meadow.name",
        "missing language falls back to the key"
    );

    // The dungeon's first room: walls on the boundary, a door east at row 3.
    let corner = dungeon.cell(0, 0).unwrap();
    assert_eq!(
        corner.walls,
        Edges::NONE.with(Facing::North).with(Facing::West)
    );
    let door_tile = dungeon.cell(5, 3).unwrap();
    assert!(
        door_tile.doors.has(Facing::East),
        "door at (5,3) east: {door_tile:?}"
    );
    assert!(
        dungeon.cell(6, 3).unwrap().doors.has(Facing::West),
        "same door from the other side"
    );
    assert!(
        dungeon.cell(5, 2).unwrap().walls.has(Facing::East),
        "wall where there is no door"
    );
    let pillar = dungeon.terrain(dungeon.cell(3, 3).unwrap());
    assert!(!pillar.passable && pillar.opaque, "pillar at (3,3)");
    let water = meadow.terrain(meadow.cell(8, 22).unwrap());
    assert!(!water.passable && !water.opaque, "water can be seen across");

    // The meadow is open inside the hedge; the road runs north to the portal.
    assert_eq!(meadow.cell(10, 10).unwrap().walls, Edges::NONE);
    assert_eq!(
        meadow.cell(0, 0).unwrap().walls,
        Edges::NONE.with(Facing::North).with(Facing::West)
    );
    assert_eq!(meadow.terrain(meadow.cell(16, 20).unwrap()).name, "road");
    assert_eq!(
        meadow
            .terrain(meadow.cell(16, 20).unwrap())
            .visibility_depth,
        12
    );
    assert_eq!(
        dungeon
            .terrain(dungeon.cell(1, 1).unwrap())
            .visibility_depth,
        6
    );
}

#[test]
fn the_game_starts_in_a_town_of_seven_services_behind_doors() {
    let data = common::load_test_packs();
    let town_id = data.registry.maps.get("test:map:town").expect("town");
    assert_eq!(data.entry, Some(town_id));
    let town = &data.maps[&town_id];
    assert_eq!(
        (
            town.def.width,
            town.def.height,
            town.def.kind,
            town.def.start
        ),
        (12, 5, MapKind::Town, (10, 2, Facing::West))
    );
    assert_eq!(data.text("en", town.name), "Test Town");
    let kinds: Vec<ServiceKind> = town
        .sites
        .iter()
        .map(|s| data.services[&s.service].kind)
        .collect();
    assert_eq!(
        kinds,
        [
            ServiceKind::Inn,
            ServiceKind::Temple,
            ServiceKind::Trainer,
            ServiceKind::Guild,
            ServiceKind::Smith,
            ServiceKind::Tavern,
            ServiceKind::Bank,
        ]
    );
    for site in &town.sites {
        let (x, y) = (site.x, site.y);
        let door = if y == 1 { Facing::South } else { Facing::North };
        let cell = town.cell(x, y).unwrap();
        assert!(
            cell.doors.has(door),
            "the site at ({x}, {y}) is behind a door"
        );
        let name = &town.terrain(cell).name;
        let def = &data.services[&site.service];
        assert_eq!(
            data.registry.services.name(site.service),
            Some(format!("base:service:{name}").as_str()),
            "each service has its own terrain"
        );
        assert!(!data.label("en", &def.name).is_empty());
    }
    assert_eq!(
        town.site_at(1, 1),
        data.registry.services.get("base:service:inn")
    );
    assert_eq!(town.site_at(1, 2), None, "the street is no service");
    let colours: std::collections::BTreeSet<_> =
        town.def.terrains.iter().map(|t| t.color).collect();
    assert_eq!(
        colours.len(),
        town.def.terrains.len(),
        "the automap tells them apart"
    );
}

#[test]
fn portals_and_tileset_slots_resolve() {
    let data = common::load_test_packs();
    let dungeon_id = data
        .registry
        .maps
        .get("test:map:dungeon")
        .expect("dungeon interned");
    let meadow_id = data
        .registry
        .maps
        .get("test:map:meadow")
        .expect("meadow interned");
    let dungeon = &data.maps[&dungeon_id];
    let meadow = &data.maps[&meadow_id];

    // Portals link the maps both ways.
    let down = meadow.portal_at(16, 5).expect("entrance");
    assert_eq!(
        (down.to_map, down.to_x, down.to_y, down.to_facing),
        (dungeon_id, 1, 0, Facing::South)
    );
    assert_eq!(dungeon.encounters.len(), 3);
    let (index, rats) = dungeon.encounter_at(3, 8).expect("the rats");
    assert_eq!(index, 0);
    assert_eq!(rats.stacks.len(), 1);
    assert_eq!(
        (
            data.registry.monsters.name(rats.stacks[0].0),
            rats.stacks[0].1
        ),
        (Some("test:monster:giant_rat"), 2)
    );
    assert!(rats.once);
    assert_eq!(dungeon.encounter_at(0, 0), None);
    assert_eq!(dungeon.random.as_ref().map(|r| r.chance_percent), Some(3));
    assert_eq!(dungeon.random.as_ref().map(|r| r.total_weight()), Some(4));
    assert_eq!(meadow.random.as_ref().map(|r| r.chance_percent), Some(0));
    let up = dungeon.portal_at(0, 0).expect("exit");
    assert_eq!((up.to_map, up.to_x, up.to_y), (meadow_id, 16, 6));
    let town_id = data.registry.maps.get("test:map:town").unwrap();
    let home = meadow.portal_at(16, 31).expect("the road south");
    assert_eq!(
        (home.to_map, home.to_x, home.to_y, home.to_facing),
        (town_id, 10, 2, Facing::West)
    );
    let out = data.maps[&town_id].portal_at(11, 2).expect("the town gate");
    assert_eq!(
        (out.to_map, out.to_x, out.to_y, out.to_facing),
        (meadow_id, 16, 16, Facing::North),
        "leaving town lands on the meadow's start"
    );
    // Every portal stands visible under an Object surface of its map's tileset.
    let town = &data.maps[&town_id];
    let markers = [
        (meadow, 16, 5, "stairs.down"),
        (meadow, 16, 31, "signpost"),
        (dungeon, 0, 0, "stairs.up"),
        (town, 11, 2, "signpost"),
    ];
    for (map, x, y, marker) in markers {
        assert_eq!(map.marker_at(x, y), Some(marker));
        let surface = &data.tilesets[&map.tileset].surfaces[marker];
        assert_eq!(surface.kind, SlotKind::Object);
        assert!(!surface.slots.is_empty(), "{marker} was baked");
    }
    assert_eq!(meadow.marker_at(16, 6), None, "no portal, no marker");

    // Tileset slots resolve to the baked sprite paths.
    let tileset = &data.tilesets[&dungeon.tileset];
    assert_eq!((tileset.detail_depth, tileset.width), (6, 5));
    assert_eq!(
        tileset.slot("wall", 2, -1),
        Some("assets/tilesets/dungeon/wall_d2_o-1.png")
    );
    assert_eq!(
        tileset.slot("wall.left", 1, 1),
        None,
        "left walls have no right-side slots"
    );
    assert_eq!(tileset.surfaces["door"].kind, SlotKind::Door);
    assert!(
        tileset.slot("floor", 3, 4).is_some(),
        "one tile beyond the diagonal"
    );
    assert_eq!(tileset.slot("floor", 5, 6), None, "beyond width");
}

#[test]
fn fingerprint_depends_on_content_only() {
    let [base, test] = common::test_packs();
    let a = load_packs(&[&base, &test]).unwrap();
    let b = load_packs(&[&base, &test]).unwrap();
    assert_eq!(a.fingerprints, b.fingerprints);
    assert_eq!(a, b, "loading is a pure function of the files");
}
