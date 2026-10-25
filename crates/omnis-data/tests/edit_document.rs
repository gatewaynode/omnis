//! `edit::Document` (M5.2): every change lands whole or not at all and undoes as one step, a
//! stroke of paints as one; the stairs list follows the stair tiles so the file always loads;
//! terrains and levels keep the loader's rules; snapshots share the grids a change left alone.

use crate::common::{base_pack, fixture_pack, load_test_packs, test_pack};
use omnis_core::{Facing, Level};
use omnis_data::edit::{Brush, Document, EdgeKind, Edit, EditError, Place, Target, UNDO_CAP};
use omnis_data::{Data, MapDef, Site, Terrain, load_packs};
use std::path::PathBuf;

fn with_fixture(name: &str) -> Data {
    let roots = [base_pack(), test_pack(), fixture_pack(name)];
    let roots: Vec<&std::path::Path> = roots.iter().map(PathBuf::as_path).collect();
    load_packs(&roots).unwrap_or_else(|report| panic!("{name}: {report}"))
}

fn def_of(data: &Data, id: &str) -> MapDef {
    let map = data.registry.maps.get(id).unwrap_or_else(|| panic!("{id}"));
    data.maps[&map].def.clone()
}

fn open(data: &Data, id: &str) -> Document {
    let def = def_of(data, id);
    let name = id.rsplit(':').next().unwrap();
    Document::from_def(&format!("data/maps/{name}.ron"), def).unwrap_or_else(|e| panic!("{e:?}"))
}

const G: Level = Level::GROUND;

#[test]
fn every_fixture_map_opens_unchanged_and_valid() {
    for (data, ids) in [
        (
            load_test_packs(),
            vec![
                "test:map:town",
                "test:map:meadow",
                "test:map:dungeon",
                "test:map:depths",
            ],
        ),
        (with_fixture("levels"), vec!["levels:map:tower"]),
        (with_fixture("heights"), vec!["heights:map:hills"]),
        (with_fixture("objects"), vec!["objects:map:yard"]),
    ] {
        for id in ids {
            let doc = open(&data, id);
            assert_eq!(doc.def(), def_of(&data, id), "{id}");
            assert!(!doc.dirty(), "{id}");
            assert_eq!(doc.validate(Some(&data)), vec![], "{id}");
        }
    }
}

#[test]
fn a_paint_undoes_exactly_and_redoes() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:dungeon");
    let before = doc.def();
    let wall = Brush::Edge {
        side: Facing::East,
        kind: EdgeKind::Wall,
    };
    doc.paint(G, 1, 1, &wall, false).unwrap();
    assert!(doc.dirty());
    assert_eq!(
        doc.grid(G).unwrap().edge(1, 1, Facing::East),
        Some(EdgeKind::Wall)
    );
    assert_eq!(
        doc.grid(G).unwrap().edge(2, 1, Facing::West),
        Some(EdgeKind::Wall)
    );
    assert_ne!(doc.def(), before);
    doc.undo().unwrap();
    assert_eq!(doc.def(), before);
    assert!(!doc.dirty(), "back at the file's state");
    doc.redo().unwrap();
    assert_ne!(doc.def(), before);
    assert_eq!(doc.undo(), Ok(()));
    assert_eq!(doc.undo(), Err(EditError::NothingToUndo));
}

#[test]
fn a_stroke_of_paints_is_one_step_and_a_refusal_changes_nothing() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:meadow");
    let glyph = doc.def().terrains[1].glyph;
    let brush = Brush::Tile { glyph, rise: None };
    doc.paint(G, 2, 2, &brush, false).unwrap();
    doc.paint(G, 3, 2, &brush, true).unwrap();
    doc.paint(G, 4, 2, &brush, true).unwrap();
    assert_eq!(doc.undo_depth(), 1, "three paints, one step");
    assert_eq!(
        doc.paint(G, 999, 2, &brush, true),
        Err(EditError::Outside { x: 999, y: 2 })
    );
    let unused = "0123456789@#%&"
        .chars()
        .find(|c| doc.def().terrains.iter().all(|t| t.glyph != *c))
        .unwrap();
    assert_eq!(
        doc.paint(
            G,
            2,
            3,
            &Brush::Tile {
                glyph: unused,
                rise: None
            },
            false
        ),
        Err(EditError::UnknownGlyph(unused))
    );
    assert_eq!(doc.undo_depth(), 1);
    doc.paint(G, 5, 2, &brush, false).unwrap();
    assert_eq!(doc.undo_depth(), 2, "a new stroke is a new step");
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert!(!doc.dirty());
}

#[test]
fn many_paints_are_one_step_and_fail_together() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:meadow");
    let wall = |side| Brush::Edge {
        side,
        kind: EdgeKind::Wall,
    };
    let run = [
        ((2, 2), wall(Facing::North)),
        ((3, 2), wall(Facing::North)),
        ((4, 2), wall(Facing::West)),
    ];
    assert_eq!(doc.paint_many(G, &run), Ok(()));
    assert_eq!(doc.undo_depth(), 1, "three paints, one step");
    let grid = doc.grid(G).unwrap();
    assert_eq!(grid.edge(2, 2, Facing::North), Some(EdgeKind::Wall));
    assert_eq!(grid.edge(3, 2, Facing::North), Some(EdgeKind::Wall));
    assert_eq!(grid.edge(4, 2, Facing::West), Some(EdgeKind::Wall));
    // A refusal in the middle lands nothing: the first paint is not kept.
    let mixed = [
        ((5, 2), wall(Facing::North)),
        (
            (0, 2),
            Brush::Edge {
                side: Facing::West,
                kind: EdgeKind::Door,
            },
        ),
        ((6, 2), wall(Facing::North)),
    ];
    assert_eq!(doc.paint_many(G, &mixed), Err(EditError::BoundaryEdge));
    assert_eq!(doc.undo_depth(), 1);
    let grid = doc.grid(G).unwrap();
    assert_eq!(grid.edge(5, 2, Facing::North), Some(EdgeKind::Open));
    assert_eq!(grid.edge(6, 2, Facing::North), Some(EdgeKind::Open));
    // It closes an open stroke: a joined paint after it is a new step.
    let glyph = doc.def().terrains[1].glyph;
    let tile = Brush::Tile { glyph, rise: None };
    doc.paint(G, 2, 3, &tile, false).unwrap();
    assert_eq!(doc.undo_depth(), 2);
    assert_eq!(doc.paint_many(G, &[((3, 3), tile.clone())]), Ok(()));
    assert_eq!(doc.undo_depth(), 3);
    doc.paint(G, 4, 3, &tile, true).unwrap();
    assert_eq!(doc.undo_depth(), 4, "the run closed the stroke");
    // An empty list changes nothing and is no step.
    assert_eq!(doc.paint_many(G, &[]), Ok(()));
    assert_eq!(doc.undo_depth(), 4);
    for _ in 0..4 {
        doc.undo().unwrap();
    }
    assert!(!doc.dirty());
}

#[test]
fn many_places_are_one_step_and_fail_together() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:town");
    let site_at = |doc: &Document, x: u16, y: u16| {
        doc.def()
            .sites
            .iter()
            .find(|s| (s.level, s.x, s.y) == (G, x, y))
            .cloned()
    };
    let inn = site_at(&doc, 1, 1).expect("the inn's site");
    let mut moved = inn.clone();
    moved.x = 2;
    let places = vec![
        Place::Remove(Target::Site {
            level: G,
            x: 1,
            y: 1,
        }),
        Place::Site(moved),
    ];
    assert_eq!(doc.place_many(places), Ok(()));
    assert_eq!(doc.undo_depth(), 1, "a removal and a placement, one step");
    assert!(site_at(&doc, 1, 1).is_none());
    assert_eq!(
        site_at(&doc, 2, 1).map(|s| s.service),
        Some(inn.service.clone())
    );
    // A refusal in the list lands nothing: the placement before it is not kept.
    let mut temple = site_at(&doc, 4, 1).expect("the temple's site");
    temple.x = 5;
    let mixed = vec![
        Place::Site(temple),
        Place::Remove(Target::Site {
            level: G,
            x: 9,
            y: 9,
        }),
    ];
    assert!(matches!(
        doc.place_many(mixed),
        Err(EditError::NoSuchTarget(_))
    ));
    assert_eq!(doc.undo_depth(), 1);
    assert!(site_at(&doc, 5, 1).is_none());
    assert!(site_at(&doc, 4, 1).is_some());
    // It closes an open stroke, and an empty list is no step.
    let glyph = doc.def().terrains[1].glyph;
    let tile = Brush::Tile { glyph, rise: None };
    doc.paint(G, 2, 2, &tile, false).unwrap();
    assert_eq!(doc.undo_depth(), 2);
    assert_eq!(doc.place_many(Vec::new()), Ok(()));
    assert_eq!(doc.undo_depth(), 2);
    doc.paint(G, 3, 2, &tile, true).unwrap();
    assert_eq!(doc.undo_depth(), 3, "the list closed the stroke");
    // One undo returns the site.
    doc.undo().unwrap();
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(site_at(&doc, 1, 1), Some(inn));
}

#[test]
fn the_undo_cap_holds() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:town");
    let brush = Brush::Height { eighths: 1 };
    for i in 0..(UNDO_CAP + 20) {
        let h = i16::try_from(i % 8).unwrap() + 1;
        doc.paint(G, 1, 1, &Brush::Height { eighths: h }, false)
            .unwrap();
    }
    assert_eq!(doc.undo_depth(), UNDO_CAP);
    let _ = brush;
}

#[test]
fn stairs_follow_the_stair_tiles() {
    let data = with_fixture("levels");
    let mut doc = open(&data, "levels:map:tower");
    let def = doc.def();
    let up = def
        .terrains
        .iter()
        .find(|t| t.join == Some(omnis_data::Join::Up))
        .unwrap()
        .glyph;
    let floor = def.terrains[0].glyph;
    let stairs_before = def.stairs.len();
    assert_eq!(
        doc.paint(
            G,
            1,
            1,
            &Brush::Tile {
                glyph: up,
                rise: None
            },
            false
        ),
        Err(EditError::NeedsRise)
    );
    doc.paint(
        G,
        1,
        1,
        &Brush::Tile {
            glyph: up,
            rise: Some(Facing::East),
        },
        false,
    )
    .unwrap();
    assert_eq!(doc.def().stairs.len(), stairs_before + 1);
    doc.edit(Edit::Rise {
        level: G,
        x: 1,
        y: 1,
        rise: Facing::South,
    })
    .unwrap();
    assert!(
        doc.def()
            .stairs
            .iter()
            .any(|s| s.x == 1 && s.y == 1 && s.rise == Facing::South)
    );
    doc.paint(
        G,
        1,
        1,
        &Brush::Tile {
            glyph: floor,
            rise: None,
        },
        false,
    )
    .unwrap();
    assert_eq!(
        doc.def().stairs.len(),
        stairs_before,
        "painting over a stair drops its entry"
    );
    assert_eq!(
        doc.edit(Edit::Rise {
            level: G,
            x: 1,
            y: 1,
            rise: Facing::South
        }),
        Err(EditError::NotAStair)
    );
    // The existing stairs: clearing one tile drops its entry too.
    let s = def.stairs[0];
    doc.paint(Level(s.level), s.x, s.y, &Brush::Clear, false)
        .unwrap();
    assert_eq!(doc.def().stairs.len(), stairs_before - 1);
}

#[test]
fn levels_come_and_go_with_what_stands_on_them() {
    let data = with_fixture("levels");
    let mut doc = open(&data, "levels:map:tower");
    let levels = doc.levels();
    assert_eq!(levels, [Level(-1), G, Level(1), Level(2)]);
    assert_eq!(
        doc.edit(Edit::LevelAdd {
            level: 0,
            name: "x".into(),
            base: 0
        }),
        Err(EditError::LevelZero)
    );
    assert_eq!(
        doc.edit(Edit::LevelAdd {
            level: 1,
            name: "x".into(),
            base: 0
        }),
        Err(EditError::LevelExists(1))
    );
    assert_eq!(
        doc.edit(Edit::LevelRemove { level: 0 }),
        Err(EditError::LevelZero)
    );
    assert_eq!(
        doc.edit(Edit::LevelRemove { level: 7 }),
        Err(EditError::NoLevel(7))
    );
    let stairs_on_1 = doc.def().stairs.iter().filter(|s| s.level == 1).count();
    assert!(stairs_on_1 > 0);
    doc.edit(Edit::LevelRemove { level: 1 }).unwrap();
    assert_eq!(doc.levels(), [Level(-1), G, Level(2)]);
    assert!(doc.def().stairs.iter().all(|s| s.level != 1));
    doc.edit(Edit::LevelAdd {
        level: 3,
        name: "levels:text:map.tower.roof".into(),
        base: 24,
    })
    .unwrap();
    let def = doc.def();
    let added = def.levels.iter().find(|l| l.level == 3).unwrap();
    assert_eq!(added.base, 24);
    assert!(added.layout[1].contains('_'), "a new level is no ground");
    assert_eq!(added.layout.len(), usize::from(def.height) * 2 + 1);
    doc.edit(Edit::LevelSet {
        level: 3,
        name: None,
        base: Some(32),
    })
    .unwrap();
    assert_eq!(
        doc.def().levels.iter().find(|l| l.level == 3).unwrap().base,
        32
    );
}

#[test]
fn terrains_keep_the_loaders_rules() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:town");
    let street = doc.def().terrains[0].clone();
    let mut pond = street.clone();
    pond.glyph = '~';
    pond.name = "pond".into();
    assert_eq!(
        doc.edit(Edit::TerrainAdd(Terrain {
            glyph: '-',
            ..pond.clone()
        })),
        Err(EditError::Reserved('-'))
    );
    assert_eq!(
        doc.edit(Edit::TerrainAdd(Terrain {
            glyph: 'r',
            ..pond.clone()
        })),
        Err(EditError::GlyphTaken('r'))
    );
    doc.edit(Edit::TerrainAdd(pond.clone())).unwrap();
    assert_eq!(
        doc.edit(Edit::TerrainRemove { glyph: 'r' }),
        Err(EditError::TerrainInUse('r'))
    );
    let mut stair = street.clone();
    stair.join = Some(omnis_data::Join::Up);
    assert_eq!(
        doc.edit(Edit::TerrainSet {
            glyph: 'r',
            terrain: stair
        }),
        Err(EditError::TerrainInUse('r')),
        "a join change while in use would orphan the stairs list"
    );
    // Renaming a terrain carries its rest events along.
    let mut avenue = street.clone();
    avenue.name = "avenue".into();
    doc.edit(Edit::TerrainSet {
        glyph: 'r',
        terrain: avenue,
    })
    .unwrap();
    assert!(doc.def().rest_events.iter().any(|r| r.terrain == "avenue"));
    assert!(doc.def().rest_events.iter().all(|r| r.terrain != "street"));
    // Removing the unused pond reindexes nothing above it (it is last) and the map still loads.
    doc.edit(Edit::TerrainRemove { glyph: '~' }).unwrap();
    assert_eq!(doc.validate(Some(&data)), vec![]);
    // Removing a used-nowhere terrain in the middle reindexes the tiles above it.
    let mut doc = open(&data, "test:map:town");
    let unused: Vec<char> = {
        let def = doc.def();
        let grid = doc.grid(G).unwrap();
        def.terrains
            .iter()
            .enumerate()
            .filter(|(i, _)| !grid.uses(u8::try_from(*i).unwrap()))
            .map(|(_, t)| t.glyph)
            .collect()
    };
    if let Some(&glyph) = unused.first() {
        let before = doc.def();
        doc.edit(Edit::TerrainRemove { glyph }).unwrap();
        assert_eq!(doc.def().layout, before.layout, "the picture is unchanged");
        assert_eq!(doc.validate(Some(&data)), vec![]);
    }
}

#[test]
fn placements_upsert_by_key_and_remove_by_target() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:town");
    let sites = doc.def().sites.len();
    let moved = Site {
        level: G,
        x: 1,
        y: 1,
        service: "base:service:temple".into(),
    };
    doc.place(Place::Site(moved.clone())).unwrap();
    assert_eq!(doc.def().sites.len(), sites, "replaced the inn on its tile");
    assert_eq!(
        doc.def()
            .sites
            .iter()
            .find(|s| s.x == 1 && s.y == 1)
            .unwrap()
            .service,
        "base:service:temple"
    );
    doc.place(Place::Remove(Target::Site {
        level: G,
        x: 1,
        y: 1,
    }))
    .unwrap();
    assert_eq!(doc.def().sites.len(), sites - 1);
    assert_eq!(
        doc.place(Place::Remove(Target::Site {
            level: G,
            x: 1,
            y: 1
        })),
        Err(EditError::NoSuchTarget("a site at level 0 (1, 1)".into()))
    );
    assert_eq!(
        doc.place(Place::Start {
            x: 50,
            y: 0,
            facing: Facing::North
        }),
        Err(EditError::Outside { x: 50, y: 0 })
    );
    doc.place(Place::Start {
        x: 2,
        y: 2,
        facing: Facing::North,
    })
    .unwrap();
    assert_eq!(doc.def().start, (2, 2, Facing::North));
    assert_eq!(doc.undo_depth(), 3);
    assert!(doc.validate(Some(&data)).is_empty());
}

#[test]
fn a_resize_keeps_the_top_left_and_drops_what_falls_outside() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:town");
    let before = doc.def();
    assert_eq!(
        doc.edit(Edit::Resize {
            width: 0,
            height: 3
        }),
        Err(EditError::OutOfRange)
    );
    doc.edit(Edit::Resize {
        width: 6,
        height: 3,
    })
    .unwrap();
    let after = doc.def();
    assert_eq!((after.width, after.height), (6, 3));
    assert!(after.sites.iter().all(|s| s.x < 6 && s.y < 3));
    assert!(after.sites.len() < before.sites.len());
    assert!(
        after.portals.is_empty(),
        "the portal at (11, 2) fell outside"
    );
    assert_eq!(after.start.0, 5, "the start is clamped inside");
    assert_eq!(after.layout.len(), 7);
    assert_eq!(after.layout[1].len(), 13);
    doc.undo().unwrap();
    assert_eq!(doc.def(), before);
}

#[test]
fn a_snapshot_copies_only_the_level_it_changed() {
    let data = with_fixture("levels");
    let mut doc = open(&data, "levels:map:tower");
    let one = doc.grid(G).unwrap().bytes();
    let before = doc.grid_bytes();
    assert_eq!(before, one * 4, "four levels of one size");
    doc.paint(Level(2), 1, 1, &Brush::Height { eighths: 3 }, false)
        .unwrap();
    assert_eq!(doc.grid_bytes(), one * 5, "one grid copied, three shared");
    doc.paint(Level(2), 1, 2, &Brush::Height { eighths: 3 }, false)
        .unwrap();
    assert_eq!(doc.grid_bytes(), one * 6);
}
