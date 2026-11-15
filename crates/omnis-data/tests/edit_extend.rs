//! Growing and cutting a map on any side (owner, 2026-11-14; `tasks/plans/editor-resize.md`):
//! everything on the map moves together, a side that grows keeps its old outer wall, and what a
//! cut leaves outside is dropped.

use crate::common::{base_pack, fixture_pack, load_test_packs, test_pack};
use omnis_core::{Facing, Level};
use omnis_data::edit::{Brush, Document, EdgeKind, Edit, EditError, words};
use omnis_data::{Data, MapDef, load_packs};
use std::path::PathBuf;

fn with_fixture(name: &str) -> Data {
    let roots = [base_pack(), test_pack(), fixture_pack(name)];
    let roots: Vec<&std::path::Path> = roots.iter().map(PathBuf::as_path).collect();
    load_packs(&roots).unwrap_or_else(|report| panic!("{name}: {report}"))
}

fn open(data: &Data, id: &str) -> Document {
    let map = data.registry.maps.get(id).unwrap_or_else(|| panic!("{id}"));
    let def = data.maps[&map].def.clone();
    let name = id.rsplit(':').next().unwrap();
    Document::from_def(&format!("data/maps/{name}.ron"), def).unwrap_or_else(|e| panic!("{e:?}"))
}

/// The yard as these tests know it, seven by five: the fixture's, with the hall that was grown
/// east of it (2026-11-15) cut off again.
fn yard(data: &Data) -> Document {
    let mut doc = open(data, "objects:map:yard");
    doc.edit(extend(0, -30, 0, 0)).unwrap();
    assert_eq!((doc.def().width, doc.def().height), (7, 5));
    doc
}

fn extend(north: i16, east: i16, south: i16, west: i16) -> Edit {
    Edit::Extend {
        north,
        east,
        south,
        west,
    }
}

/// Every thing placed by cell, as `(what, level, x, y)`.
fn placed(def: &MapDef) -> Vec<(String, i8, u16, u16)> {
    let mut all = Vec::new();
    let mut push = |what: String, level: i8, x: u16, y: u16| all.push((what, level, x, y));
    for s in &def.stairs {
        push("stair".into(), s.level, s.x, s.y);
    }
    for p in &def.portals {
        push(format!("portal {}", p.id), p.level.0, p.x, p.y);
    }
    for e in &def.encounters {
        push(format!("encounter {}", e.id), e.level.0, e.x, e.y);
    }
    for s in &def.sites {
        push("site".into(), s.level.0, s.x, s.y);
    }
    for o in &def.objects {
        push(format!("object {}", o.id), o.level.0, o.x, o.y);
    }
    for g in &def.ground {
        push("pile".into(), g.level.0, g.x, g.y);
    }
    for l in &def.lore {
        push(format!("lore {}", l.id), l.level.0, l.x, l.y);
    }
    all
}

/// The old layout rows sit whole inside the new ones, `(west, north)` cells in: every tile and
/// every edge, the old outer wall among them.
fn holds(new: &[String], old: &[String], (west, north): (usize, usize)) {
    for (r, row) in old.iter().enumerate() {
        let within = &new[r + 2 * north][2 * west..2 * west + row.len()];
        assert_eq!(within, row, "row {r}");
    }
}

/// A map grown on all four sides at once keeps every cell, edge and placed thing, each moved by
/// the cells added to the west and north; the old outer wall stands where the map used to end,
/// the new cells are plain ground on the ground level and no ground above, and the loader
/// takes the result.
#[test]
fn a_grow_moves_everything_together_and_keeps_the_old_wall() {
    let data = with_fixture("objects");
    let mut doc = yard(&data);
    let before = doc.def();
    doc.edit(extend(1, 2, 3, 4)).unwrap();
    let after = doc.def();
    assert_eq!((after.width, after.height), (7 + 2 + 4, 5 + 1 + 3));
    holds(&after.layout, &before.layout, (4, 1));
    holds(&after.levels[0].layout, &before.levels[0].layout, (4, 1));
    let moved: Vec<_> = placed(&before)
        .into_iter()
        .map(|(what, level, x, y)| (what, level, x + 4, y + 1))
        .collect();
    assert_eq!(placed(&after), moved);
    assert_eq!(
        (after.start.0, after.start.1),
        (before.start.0 + 4, before.start.1 + 1)
    );
    // A new cell: grass on the ground (the first terrain that joins nothing), none on the loft.
    let tile = |layout: &[String], x: usize, y: usize| layout[2 * y + 1].as_bytes()[2 * x + 1];
    assert_eq!(tile(&after.layout, 0, 0), b'g');
    assert_eq!(tile(&after.layout, 12, 8), b'g');
    assert_eq!(tile(&after.levels[0].layout, 0, 0), b'_');
    // The new outer edge is walled all round.
    assert!(after.layout[0].chars().all(|c| c == '+' || c == '-'));
    assert!(
        after
            .layout
            .iter()
            .skip(1)
            .step_by(2)
            .all(|r| r.starts_with('|') && r.ends_with('|'))
    );
    assert_eq!(doc.validate(Some(&data)), vec![]);
    doc.undo().unwrap();
    assert_eq!(doc.def(), before);
    doc.redo().unwrap();
    assert_eq!(doc.def(), after);
}

/// Heights go with their cells.
#[test]
fn heights_move_with_their_cells() {
    let data = with_fixture("heights");
    let mut doc = open(&data, "heights:map:hills");
    let before = doc.def();
    assert!(!before.heights.is_empty(), "the hills have heights");
    doc.edit(extend(2, 0, 0, 1)).unwrap();
    let after = doc.def();
    let row = |text: &str| -> Vec<String> { text.split_whitespace().map(str::to_owned).collect() };
    for (r, old) in before.heights.iter().enumerate() {
        let new = row(&after.heights[r + 2]);
        assert_eq!(new[1..], row(old)[..], "row {r}");
        assert_eq!(new[0], "0", "a new cell is flat");
    }
    assert!(row(&after.heights[0]).iter().all(|h| h == "0"));
}

/// A cut drops what it leaves outside and moves the rest; the start left outside stands on
/// the nearest cell inside.
#[test]
fn a_cut_drops_what_falls_outside_on_any_side() {
    let data = with_fixture("objects");
    let before = yard(&data).def();
    let kept = |by: (i32, i32), inside: &dyn Fn(u16, u16) -> bool| -> Vec<_> {
        placed(&before)
            .into_iter()
            .filter(|(_, _, x, y)| inside(*x, *y))
            .map(|(what, level, x, y)| {
                let x = u16::try_from(i32::from(x) + by.0).unwrap();
                let y = u16::try_from(i32::from(y) + by.1).unwrap();
                (what, level, x, y)
            })
            .collect()
    };
    // Two columns off the east: the chest, the torch, the loft's crate and the rats go.
    let mut doc = yard(&data);
    doc.edit(extend(0, -2, 0, 0)).unwrap();
    let east = doc.def();
    assert_eq!((east.width, east.height), (5, 5));
    assert_eq!(placed(&east), kept((0, 0), &|x, _| x < 5));
    assert!(placed(&east).len() < placed(&before).len());
    assert!(east.encounters.is_empty(), "the rats stood at (5, 1)");
    assert!(east.layout.iter().all(|row| row.len() == 11));
    assert!(east.layout[1].ends_with('|'), "the new edge is walled");
    // One column off the west and one row off the north: everything else moves up and left.
    let mut doc = yard(&data);
    doc.edit(extend(-1, 0, 0, -1)).unwrap();
    let corner = doc.def();
    assert_eq!((corner.width, corner.height), (6, 4));
    assert_eq!(placed(&corner), kept((-1, -1), &|x, y| x >= 1 && y >= 1));
    assert_eq!(
        (corner.start.0, corner.start.1),
        (0, 1),
        "the start at (0, 2) is put on the nearest cell inside"
    );
    // Two rows off the south.
    let mut doc = yard(&data);
    doc.edit(extend(0, 0, -2, 0)).unwrap();
    assert_eq!(placed(&doc.def()), kept((0, 0), &|_, y| y < 3));
}

/// Growing a side and cutting it off again is the map it started as.
#[test]
fn a_grow_and_the_same_cut_is_the_map_as_it_was() {
    for (fixture, id) in [
        ("objects", "objects:map:yard"),
        ("levels", "levels:map:tower"),
    ] {
        let data = with_fixture(fixture);
        let mut doc = open(&data, id);
        let before = doc.def();
        doc.edit(extend(2, 1, 4, 3)).unwrap();
        doc.edit(extend(-2, -1, -4, -3)).unwrap();
        assert_eq!(doc.def(), before, "{id}");
    }
}

/// A side may not leave the map under one cell or over the limit; nothing changes when refused.
#[test]
fn a_size_outside_the_limits_is_refused() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:town");
    let before = doc.def();
    for edit in [
        extend(0, 0, 0, -12),
        extend(-3, 0, -2, 0),
        extend(0, 245, 0, 0),
        extend(126, 0, 126, 0),
    ] {
        assert_eq!(
            doc.edit(edit.clone()),
            Err(EditError::OutOfRange),
            "{edit:?}"
        );
        assert_eq!(doc.def(), before);
    }
    doc.edit(extend(0, 244, 0, 0)).unwrap();
    assert_eq!(doc.def().width, 256);
}

/// The town's gate moves with the map, so its landing does: the pair still fits.
#[test]
fn a_portal_moves_with_its_cell() {
    let data = load_test_packs();
    let mut doc = open(&data, "test:map:town");
    doc.edit(extend(0, 0, 0, 2)).unwrap();
    let def = doc.def();
    let gate = def.portals.iter().find(|p| p.id == "town-gate").unwrap();
    assert_eq!((gate.x, gate.y), (13, 2));
    assert_eq!((def.start.0, def.start.1), (12, 2));
}

/// `Resize` is an extend on the east and south, and keeps the old wall as any grow does.
#[test]
fn resize_is_an_extend_on_the_east_and_south() {
    let data = with_fixture("objects");
    let mut resized = yard(&data);
    resized
        .edit(Edit::Resize {
            width: 9,
            height: 4,
        })
        .unwrap();
    let mut extended = yard(&data);
    extended.edit(extend(0, 2, -1, 0)).unwrap();
    assert_eq!(resized.def(), extended.def());
    let before = yard(&data).def();
    let row = &resized.def().layout[1];
    assert_eq!(
        &row[..15],
        &before.layout[1][..],
        "the old east wall still stands"
    );
}

/// The step is named for what it did, side by side.
#[test]
fn the_step_says_which_sides_grew_and_which_were_cut() {
    assert_eq!(words::edit(&extend(0, 0, 2, 3)), "grew 2 south, 3 west");
    assert_eq!(words::edit(&extend(0, -2, 0, 0)), "cut 2 east");
    assert_eq!(
        words::edit(&extend(1, -2, 0, 0)),
        "grew 1 north; cut 2 east"
    );
    assert_eq!(words::edit(&extend(0, 0, 0, 0)), "size kept");
}

/// Stairs are kept by cell too and move with theirs; and a new cell is never a stair or a
/// ladder, whatever terrain the map lists first.
#[test]
fn stairs_move_and_a_new_cell_joins_nothing() {
    let data = with_fixture("levels");
    let mut doc = open(&data, "levels:map:tower");
    let before = doc.def();
    assert!(!before.stairs.is_empty(), "the tower has stairs");
    doc.edit(extend(0, 0, 0, 2)).unwrap();
    let moved: Vec<_> = placed(&before)
        .into_iter()
        .map(|(what, level, x, y)| (what, level, x + 2, y))
        .collect();
    assert_eq!(placed(&doc.def()), moved);
    assert_eq!(doc.validate(Some(&data)), vec![]);

    // The yard with its ladder listed first: the new ground is the next terrain, not a ladder.
    let data = with_fixture("objects");
    let mut def = yard(&data).def();
    def.terrains.swap(0, 2);
    assert!(def.terrains[0].join.is_some());
    let mut doc = Document::from_def("data/maps/yard.ron", def).unwrap();
    doc.edit(extend(0, 1, 0, 0)).unwrap();
    let glyph = doc.def().layout[1].as_bytes()[2 * 7 + 1];
    assert_eq!(glyph, b'f', "the loft floor, which joins nothing");
}

/// What a cut would destroy is listed before it happens: drawn cells counted, each placed
/// thing by name and cell, the start's cell; a grow, and a cut of cells nothing was drawn on,
/// destroys nothing. The step is named with what it took.
#[test]
fn a_cut_s_losses_are_listed_and_empty_cells_are_none() {
    let data = with_fixture("objects");
    let mut doc = yard(&data);
    assert!(
        doc.losses(&extend(3, 3, 3, 3)).unwrap().is_empty(),
        "a grow"
    );
    // Two columns off the east of the yard.
    let lost = doc.losses(&extend(0, -2, 0, 0)).unwrap();
    assert_eq!(lost.cells, 10, "two columns of five drawn cells");
    assert_eq!(
        lost.things,
        [
            "encounter rat at (5, 1)",
            "object chest_1 at (5, 3)",
            "object torch_1 at (6, 4)",
            "object crate_2 at (5, 0)",
        ]
    );
    assert_eq!(lost.start, None);
    assert_eq!(lost.lines()[0], "10 drawn cells");
    assert_eq!(lost.lines().len(), 5);
    // The west column holds the start.
    let lost = doc.losses(&extend(0, 0, 0, -1)).unwrap();
    assert_eq!(lost.start, Some((0, 2)));
    assert!(
        lost.lines().iter().any(|l| l.starts_with("the start")),
        "{:?}",
        lost.lines()
    );
    assert_eq!(doc.losses(&extend(0, 0, 0, -7)), Err(EditError::OutOfRange));
    // A strip grown is ground, so cutting it again counts its cells; cleared to no ground it
    // is empty and costs nothing, the old wall beside it being the map's edge once more.
    doc.edit(extend(0, 2, 0, 0)).unwrap();
    let grown = doc.losses(&extend(0, -2, 0, 0)).unwrap();
    assert_eq!((grown.cells, grown.things.len()), (10, 0));
    for x in 7..9 {
        for y in 0..5 {
            doc.paint(Level::GROUND, x, y, &Brush::Clear, false)
                .unwrap();
        }
    }
    assert!(
        doc.losses(&extend(0, -2, 0, 0)).unwrap().is_empty(),
        "empty cells"
    );
    // A door on the edge of the strip is lost with it, though its cell on the map stays.
    doc.paint(
        Level::GROUND,
        6,
        2,
        &Brush::Edge {
            side: Facing::East,
            kind: EdgeKind::Door,
        },
        false,
    )
    .unwrap();
    assert_eq!(doc.losses(&extend(0, -2, 0, 0)).unwrap().cells, 1);
    // The step's words carry the summary.
    let mut doc = yard(&with_fixture("objects"));
    doc.edit(extend(0, -2, 0, 0)).unwrap();
    assert_eq!(
        doc.steps().last().copied(),
        Some("cut 2 east (10 cells, 4 things lost)")
    );
    doc.edit(extend(0, 1, 0, 0)).unwrap();
    assert_eq!(doc.steps().last().copied(), Some("grew 1 east"));
}

/// An other edit destroys nothing by this measure.
#[test]
fn only_a_change_of_size_has_losses() {
    let data = load_test_packs();
    let doc = open(&data, "test:map:town");
    let fields = Edit::LevelRemove { level: 1 };
    assert!(doc.losses(&fields).unwrap().is_empty());
    let cut = Edit::Resize {
        width: 6,
        height: 3,
    };
    assert!(!doc.losses(&cut).unwrap().is_empty());
}

/// The QA hall inside the yard (owner, 2026-11-15), made with the edit itself: the yard grown
/// thirty cells to the east, one row of them a flagstone hall behind a gate in the old east
/// wall, the rest no ground; four shamblers at its far end. Run deliberately: it rewrites
/// `crates/omnis-sim/tests/packs/objects/data/maps/yard.ron` and reseals the pack.
#[test]
#[ignore = "rewrites the objects fixture's yard and reseals the pack; run deliberately"]
fn put_the_hall_in_the_yard() {
    use omnis_data::edit::{Place, Session};
    use omnis_data::ron_io::from_str;
    let roots = vec![base_pack(), test_pack(), fixture_pack("objects")];
    let data = with_fixture("objects");
    let mut session = Session::new(roots).unwrap();
    session.open("objects:map:yard").unwrap();
    let file = std::path::Path::new("x.ron");
    let flagstones = from_str(
        "(glyph: 'h', name: \"flagstones\", floor: \"road\", ceiling: None, passable: true, \
         visibility_depth: 32, step_minutes: 1)",
        file,
    )
    .unwrap();
    let mob = from_str(
        "(id: \"shamblers\", x: 35, y: 2, stacks: [(\"objects:monster:shambler\", 4)], once: \
         false, stand: Some((pos: (0, 0), facing: West)))",
        file,
    )
    .unwrap();
    let doc = session.document_mut().unwrap();
    assert_eq!(doc.def().width, 7, "the yard as it was");
    doc.edit(Edit::TerrainAdd(flagstones)).unwrap();
    doc.edit(extend(0, 30, 0, 0)).unwrap();
    let edge = |side, kind| Brush::Edge { side, kind };
    for x in 7..37 {
        for y in 0..5 {
            if y == 2 {
                let hall = Brush::Tile {
                    glyph: 'h',
                    rise: None,
                };
                doc.paint(Level::GROUND, x, y, &hall, false).unwrap();
                for side in [Facing::North, Facing::South] {
                    let wall = edge(side, EdgeKind::Wall);
                    doc.paint(Level::GROUND, x, y, &wall, false).unwrap();
                }
            } else {
                doc.paint(Level::GROUND, x, y, &Brush::Clear, false)
                    .unwrap();
            }
        }
    }
    let gate = edge(Facing::East, EdgeKind::Door);
    doc.paint(Level::GROUND, 6, 2, &gate, false).unwrap();
    doc.place(Place::Encounter(mob)).unwrap();
    assert_eq!(doc.validate(Some(&data)), vec![]);
    session.save(&data).unwrap();
}
