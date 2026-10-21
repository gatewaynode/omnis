//! `edit::Grid` (M5.2): a layout read into the grid and written again is the file's rows byte for
//! byte, on every map the repository ships or tests with; heights too. The grid's edits keep the
//! rules the loader enforces at the boundary.

use crate::common::{base_pack, fixture_pack, load_test_packs, test_pack};
use omnis_core::Facing;
use omnis_data::edit::{EdgeKind, EditError, Grid};
use omnis_data::{Data, MapDef, load_packs};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn with_fixture(name: &str) -> Data {
    let roots = [base_pack(), test_pack(), fixture_pack(name)];
    let roots: Vec<&std::path::Path> = roots.iter().map(PathBuf::as_path).collect();
    load_packs(&roots).unwrap_or_else(|report| panic!("{name}: {report}"))
}

fn glyphs(def: &MapDef) -> (BTreeMap<char, u8>, Vec<char>) {
    let by_glyph = def
        .terrains
        .iter()
        .enumerate()
        .map(|(i, t)| (t.glyph, u8::try_from(i).expect("few terrains")))
        .collect();
    (by_glyph, def.terrains.iter().map(|t| t.glyph).collect())
}

/// Every level of every map: the rows come back as written.
fn round_trips(data: &Data) -> usize {
    let mut levels = 0;
    for map in data.maps.values() {
        let def = &map.def;
        let (by_glyph, order) = glyphs(def);
        let mut grids = vec![(0, &def.layout, &def.heights)];
        grids.extend(def.levels.iter().map(|l| (l.level, &l.layout, &l.heights)));
        for (level, layout, heights) in grids {
            let grid = Grid::from_text(def.width, def.height, layout, heights, &by_glyph)
                .unwrap_or_else(|e| panic!("{} level {level}: {e}", def.id));
            assert_eq!(&grid.layout(&order), layout, "{} level {level}", def.id);
            assert_eq!(&grid.heights(), heights, "{} level {level} heights", def.id);
            levels += 1;
        }
    }
    levels
}

#[test]
fn the_test_packs_maps_read_and_write_byte_for_byte() {
    assert_eq!(round_trips(&load_test_packs()), 4);
}

#[test]
fn the_fixture_maps_with_levels_heights_and_objects_too() {
    // tower: four levels; hills: heights rows; yard: two levels with objects.
    let counts: Vec<usize> = ["levels", "heights", "objects"]
        .iter()
        .map(|name| round_trips(&with_fixture(name)) - 4)
        .collect();
    assert_eq!(counts, [4, 1, 2]);
}

#[test]
fn a_blank_grid_is_walled_round_and_open_inside() {
    let grid = Grid::blank(3, 2, Some(0));
    let rows = grid.layout(&['.']);
    assert_eq!(
        rows,
        ["+-+-+-+", "|. . .|", "+ + + +", "|. . .|", "+-+-+-+"]
    );
    assert_eq!(grid.heights(), Vec::<String>::new());
    assert_eq!(grid.edge(1, 0, Facing::North), Some(EdgeKind::Wall));
    assert_eq!(grid.edge(1, 0, Facing::South), Some(EdgeKind::Open));
}

#[test]
fn a_boundary_edge_refuses_anything_but_a_wall() {
    let mut grid = Grid::blank(2, 2, Some(0));
    assert_eq!(
        grid.set_edge(0, 0, Facing::West, EdgeKind::Door),
        Err(EditError::BoundaryEdge)
    );
    assert_eq!(grid.set_edge(0, 0, Facing::East, EdgeKind::Door), Ok(()));
    assert_eq!(
        grid.edge(1, 0, Facing::West),
        Some(EdgeKind::Door),
        "one edge, two tiles"
    );
    assert_eq!(
        grid.set_tile(2, 0, Some(0)),
        Err(EditError::Outside { x: 2, y: 0 })
    );
    assert_eq!(grid.set_height(0, 0, 4097), Err(EditError::OutOfRange));
}

#[test]
fn resize_keeps_the_top_left_and_walls_the_new_boundary() {
    let data = load_test_packs();
    let id = data
        .registry
        .maps
        .get("test:map:dungeon")
        .expect("the dungeon");
    let def = &data.maps[&id].def;
    let (by_glyph, order) = glyphs(def);
    let mut grid =
        Grid::from_text(def.width, def.height, &def.layout, &def.heights, &by_glyph).unwrap();
    let before = grid.layout(&order);
    grid.resize(3, 2, Some(0));
    let after = grid.layout(&order);
    assert_eq!(after.len(), 5);
    for (row, full) in after.iter().enumerate() {
        let kept: String = before[row].chars().take(6).collect();
        if row == 0 || row == 4 {
            assert_eq!(full, "+-+-+-+", "boundary row {row}");
        } else if row % 2 == 1 {
            assert_eq!(&full[..6], kept, "tile row {row}");
            assert!(full.ends_with('|'));
        }
    }
    grid.resize(4, 3, None);
    assert_eq!(grid.tile(3, 2), Some(None), "new tiles are no ground");
    assert_eq!(grid.edge(3, 2, Facing::East), Some(EdgeKind::Wall));
}

#[test]
fn heights_are_written_one_way_and_a_flat_level_writes_none() {
    let mut grid = Grid::blank(3, 1, Some(0));
    grid.set_height(1, 0, 12).unwrap();
    grid.set_height(2, 0, -8).unwrap();
    assert_eq!(grid.heights(), ["0 12 -8"]);
    grid.set_height(1, 0, 0).unwrap();
    grid.set_height(2, 0, 0).unwrap();
    assert_eq!(grid.heights(), Vec::<String>::new());
    let meadow = load_test_packs();
    let id = meadow
        .registry
        .maps
        .get("test:map:meadow")
        .expect("the meadow");
    let def = &meadow.maps[&id].def;
    let (by_glyph, _) = glyphs(def);
    let grid =
        Grid::from_text(def.width, def.height, &def.layout, &def.heights, &by_glyph).unwrap();
    assert!(grid.bytes() < 16 * 1024, "{} bytes", grid.bytes());
}

#[test]
fn removing_a_terrain_reindexes_the_tiles_above_it() {
    let mut grid = Grid::blank(3, 1, Some(2));
    grid.set_tile(0, 0, Some(0)).unwrap();
    grid.set_tile(1, 0, Some(1)).unwrap();
    assert!(grid.uses(1));
    grid.reindex_above(1);
    assert_eq!(grid.layout(&['a', 'c']), ["+-+-+-+", "|a c c|", "+-+-+-+"]);
}
