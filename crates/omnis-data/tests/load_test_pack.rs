//! The fixture pack loads, and what it contains is what the M1 maps need.

mod common;

use omnis_core::{Edges, Facing};
use omnis_data::{MapKind, SlotKind, load_packs};

#[test]
fn test_pack_loads_with_both_maps() {
    let data = load_packs(&[&common::test_pack()]).unwrap_or_else(|report| panic!("{report}"));
    assert_eq!(data.packs.len(), 1);
    assert_eq!(data.packs[0].id, "test");
    assert_eq!(data.fingerprints[0].id, "test");
    assert_eq!(data.entry, data.registry.maps.get("test:map:meadow"));
    assert_ne!(data.fingerprints[0].hash, 0);

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
fn portals_and_tileset_slots_resolve() {
    let data = load_packs(&[&common::test_pack()]).unwrap_or_else(|report| panic!("{report}"));
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
    let up = dungeon.portal_at(0, 0).expect("exit");
    assert_eq!((up.to_map, up.to_x, up.to_y), (meadow_id, 16, 6));

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
    assert_eq!(tileset.slot("floor", 3, 4), None, "beyond width");
}

#[test]
fn fingerprint_depends_on_content_only() {
    let a = load_packs(&[&common::test_pack()]).unwrap();
    let b = load_packs(&[&common::test_pack()]).unwrap();
    assert_eq!(a.fingerprints, b.fingerprints);
    assert_eq!(a, b, "loading is a pure function of the files");
}
