//! The authoring proof (M5.10): every map `crates/omnis-sim/tests/packs/test` ships is built from blank with the
//! editor's requests alone, derived from the map's own definition, and the file the save then
//! writes is the file the pack ships, byte for byte. So the request vocabulary covers every
//! field the maps use, and the content round-trips through the editor with no loss (PRD §13,
//! Phase 1's close). The ignored test is the rewrite itself: it put the shipped maps through the
//! editor once (their comments moved to `notes/`, the pack sealed at 0.2.0, the owner's choices
//! of 2026-10-25) and stays for the next time the maps need the writer's layout again.

use crate::common::{base_pack, copy_pack, load_test_packs, test_pack};
use omnis_core::{Facing, Level};
use omnis_data::edit::{
    Answer, Brush, EdgeKind, Edit, Grid, NewMap, Place, Request, Session, answer,
};
use omnis_data::{Data, Join, MapDef, load_packs, seal};
use std::collections::BTreeMap;
use std::path::Path;

/// The shipped maps, by the name in their id and file.
const MAPS: [&str; 4] = ["town", "dungeon", "meadow", "depths"];

fn def_of(data: &Data, name: &str) -> MapDef {
    let id = format!("test:map:{name}");
    let map = data
        .registry
        .maps
        .get(&id)
        .unwrap_or_else(|| panic!("{id}"));
    data.maps[&map].def.clone()
}

/// The requests that build `def` from blank, in an order the editor accepts: the terrains and
/// levels first, then every tile, height and inner edge that differs from a blank map, the
/// stairs in the list's order (a stair paint appends to it), then every placement.
fn requests_for(def: &MapDef) -> (NewMap, Vec<Request>) {
    let new = NewMap {
        id: def.id.clone(),
        name: def.name.clone(),
        kind: def.kind,
        tileset: def.tileset.clone(),
        width: def.width,
        height: def.height,
        wall: def.wall.clone(),
        door: def.door.clone(),
        door_open: def.door_open.clone(),
        terrain: def.terrains[0].clone(),
    };
    let mut out = Vec::new();
    for terrain in &def.terrains[1..] {
        out.push(Request::Edit {
            edit: Edit::TerrainAdd(terrain.clone()),
        });
    }
    for level in &def.levels {
        out.push(Request::Edit {
            edit: Edit::LevelAdd {
                level: level.level,
                name: level.name.clone(),
                base: level.base,
            },
        });
    }
    let glyphs: BTreeMap<char, u8> = def
        .terrains
        .iter()
        .enumerate()
        .map(|(i, t)| (t.glyph, u8::try_from(i).unwrap()))
        .collect();
    let mut grids = vec![(Level::GROUND, &def.layout, &def.heights)];
    grids.extend(
        def.levels
            .iter()
            .map(|l| (Level(l.level), &l.layout, &l.heights)),
    );
    let grids: BTreeMap<Level, Grid> = grids
        .into_iter()
        .map(|(level, layout, heights)| {
            let grid = Grid::from_text(def.width, def.height, layout, heights, &glyphs)
                .unwrap_or_else(|e| panic!("{}: level {level}: {e}", def.id));
            (level, grid)
        })
        .collect();
    for (&level, grid) in &grids {
        for y in 0..def.height {
            for x in 0..def.width {
                let paint = |brush| Request::Paint {
                    level,
                    x,
                    y,
                    brush,
                    stroke: false,
                };
                match grid.tile(x, y).unwrap() {
                    // A blank map's own level is all its first terrain; an added level is all
                    // no ground.
                    None if level == Level::GROUND => out.push(paint(Brush::Clear)),
                    None => {}
                    Some(index) => {
                        let terrain = &def.terrains[usize::from(index)];
                        let stair = matches!(terrain.join, Some(Join::Up | Join::Down));
                        if !stair && (index != 0 || level != Level::GROUND) {
                            out.push(paint(Brush::Tile {
                                glyph: terrain.glyph,
                                rise: None,
                            }));
                        }
                    }
                }
                let eighths = grid.height_at(x, y).unwrap();
                if eighths != 0 {
                    out.push(paint(Brush::Height { eighths }));
                }
                for side in [Facing::East, Facing::South] {
                    let boundary = match side {
                        Facing::East => x + 1 == def.width,
                        _ => y + 1 == def.height,
                    };
                    let kind = grid.edge(x, y, side).unwrap();
                    if !boundary && kind != EdgeKind::Open {
                        out.push(paint(Brush::Edge { side, kind }));
                    }
                }
            }
        }
    }
    for stair in &def.stairs {
        let level = Level(stair.level);
        let index = grids[&level].tile(stair.x, stair.y).unwrap().unwrap();
        out.push(Request::Paint {
            level,
            x: stair.x,
            y: stair.y,
            brush: Brush::Tile {
                glyph: def.terrains[usize::from(index)].glyph,
                rise: Some(stair.rise),
            },
            stroke: false,
        });
    }
    let (x, y, facing) = def.start;
    let mut places = vec![Place::Start { x, y, facing }];
    places.extend(def.portals.iter().cloned().map(Place::Portal));
    places.extend(def.encounters.iter().cloned().map(Place::Encounter));
    if def.random.is_some() {
        places.push(Place::Random(def.random.clone()));
    }
    places.extend(def.sites.iter().cloned().map(Place::Site));
    places.extend(def.rest_events.iter().cloned().map(Place::RestEvent));
    places.extend(def.objects.iter().cloned().map(Place::Object));
    places.extend(def.ground.iter().cloned().map(Place::Ground));
    places.extend(def.lore.iter().cloned().map(Place::Lore));
    out.extend(places.into_iter().map(|place| Request::Place { place }));
    (new, out)
}

#[test]
fn every_shipped_map_is_rebuilt_from_blank_with_the_requests_alone() {
    let data = load_test_packs();
    for name in MAPS {
        let shipped = def_of(&data, name);
        let file = format!("data/maps/{name}.ron");
        let scratch = copy_pack(&test_pack(), &format!("edit-authoring-{name}"));
        std::fs::remove_file(scratch.join(&file)).unwrap();
        let mut session = Session::new(vec![base_pack(), scratch.clone()]).unwrap();
        let (new, requests) = requests_for(&shipped);
        let count = requests.len();
        answer(&mut session, &data, Request::New { map: new }).unwrap();
        for request in requests {
            answer(&mut session, &data, request.clone())
                .unwrap_or_else(|e| panic!("{name}: {request:?}: {e}"));
        }
        assert_eq!(
            session.document().unwrap().def(),
            shipped,
            "{name}: the document after {count} requests"
        );
        match answer(&mut session, &data, Request::Save).unwrap() {
            Answer::Written { path, version } => {
                assert_eq!(path, file);
                assert!(version.is_some(), "{name}: the save reseals");
            }
            other => panic!("{name}: {other:?}"),
        }
        let again = load_packs(&[&base_pack(), &scratch]).unwrap_or_else(|r| panic!("{r}"));
        assert_eq!(def_of(&again, name), shipped, "{name}: the saved map loads");
        assert_eq!(
            std::fs::read_to_string(scratch.join(&file)).unwrap(),
            std::fs::read_to_string(test_pack().join(&file)).unwrap(),
            "{name}: the file the editor writes is the file the pack ships"
        );
    }
}

/// The map notes the four maps' RON comments became (2026-10-25), keyed as the notes store
/// keys them: the map's id for the header and the list comments, a cell for the one comment
/// that described a placement.
const NOTES: [(&str, &str); 5] = [
    (
        "test:map:town",
        "A 12x5 town: a street with seven services behind doors, four to the north and three to \
         the south, a green at the south-east corner, and the road out to the meadow at the east \
         end. Each service's rooms have their own terrain so the automap tells them apart; the \
         site is the tile just inside the door.\n\nWhat may happen while the party rests here: \
         placeholders at chance 0 (per mille) until the owner's ideas fill them.",
    ),
    (
        "test:map:dungeon",
        "A 24x24 dungeon: sixteen 6x6 rooms, doors between most of them, a few pillars.\n\n\
         Placed groups in the second row of rooms, rats only until the fight systems have been \
         played (owner, 2026-09-13): the pair the M4 acceptance clears (from the start: forward \
         x5, turn left, forward x2, turn right, use, forward x3), a wary one that comes back, and \
         a friendly one. A small random table of rats on top. Goblins and skeletons in the lower \
         two rows of rooms (M7b, measured by `clear_over_seeds` before the owner plays).",
    ),
    (
        "test:map:dungeon/0/23,23",
        "The way down to the depths (M7b), in the far corner. The tileset has no stairs-down art \
         yet, so the up stairs stand in (pixel art is placeholder, PRD D26).",
    ),
    (
        "test:map:meadow",
        "A 32x32 meadow: a road north to the dungeon entrance and south to the town, a pond, tree \
         clumps, a hedge all round.\n\nA random table that never fires: every step still rolls \
         its check on the encounter stream, which the stream-isolation tests count.\n\nWhat may \
         happen while the party rests here: placeholders at chance 0 (per mille) until the \
         owner's ideas fill them.",
    ),
    (
        "test:map:depths",
        "The depths below the test dungeon (M7b): four 6x6 rooms of goblins and skeletons, the \
         stairs back up in the first; east of them (M7c, 2026-10-04) the throne room of Bob the \
         Rat King, a homebrew caster behind two stacks of giant rats (two front stacks: Bob \
         stands in the back row until a rat stack falls), and an empty antechamber north of it. \
         Measured by `clear_over_seeds` before the owner plays.",
    ),
];

/// Rewrites the shipped maps through the editor: each opened and saved, so the file is the
/// writer's; the bank's two cells the checkpoint of 2026-10-25 had painted street put back; the
/// notes above written; the pack sealed at `REWRITE_VERSION`. Run deliberately, from the
/// repository: `cargo test -p omnis-data --test integration rewrite_the_shipped -- --ignored`.
#[test]
#[ignore = "rewrites the shipped maps and reseals crates/omnis-sim/tests/packs/test; run deliberately"]
fn rewrite_the_shipped_maps_through_the_editor() {
    const REWRITE_VERSION: &str = "0.2.0";
    let data = load_test_packs();
    let root = test_pack();
    let mut session = Session::new(vec![base_pack(), root.clone()]).unwrap();
    for name in MAPS {
        session.open(&format!("test:map:{name}")).unwrap();
        if name == "town" {
            let doc = session.document_mut().unwrap();
            for y in [3, 4] {
                doc.paint(
                    Level::GROUND,
                    7,
                    y,
                    &Brush::Tile {
                        glyph: 'B',
                        rise: None,
                    },
                    false,
                )
                .unwrap();
            }
        }
        session.save(&data).unwrap();
    }
    for (key, text) in NOTES {
        session.set_note(key, Some(text)).unwrap();
    }
    let sealed = seal::seal(Path::new(&root), Some(REWRITE_VERSION)).unwrap();
    assert_eq!(sealed.version, REWRITE_VERSION);
}
