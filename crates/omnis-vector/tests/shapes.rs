//! Objects and piles in the view (M8.5 step 2b, item 1): every look at every facing takes the
//! engine's footprint as its plan and stands on its home floor; the yard is drawn as the engine
//! lists it, a hidden object only once found and a taken one never; every object has fills; a
//! shut gate is drawn crossed and an open one is not.

use crate::common;

use common::{heights_data, map, objects_data};
use omnis_sim::api::{ObjectState, PileView, SeenObject};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::footprint::Footprint;
use omnis_sim::omnis_data::{Data, Look, Shape};
use omnis_vector::fill::Tri;
use omnis_vector::geometry::{SegKind, Segment};
use omnis_vector::ground::Ground;
use omnis_vector::scene::Scene;
use omnis_vector::shapes::{PILE_LOOK, shapes};
use omnis_vector::sim::SimWorld;

const SHAPES: [Shape; 7] = [
    Shape::Box,
    Shape::Post,
    Shape::Slab,
    Shape::Arch,
    Shape::Sign,
    Shape::Heap,
    Shape::Figure,
];

fn object(x: u16, y: u16, pos: (i8, i8), facing: Facing, look: Look, blocks: bool) -> SeenObject {
    SeenObject {
        object: "it".into(),
        object_kind: "objects:object:crate".into(),
        x,
        y,
        pos,
        facing,
        state: String::new(),
        look,
        blocks,
    }
}

/// The smallest box holding every point: west, north, east, south, low, high.
fn bounds(points: impl Iterator<Item = [f32; 3]>) -> [f32; 6] {
    points.fold(
        [f32::MAX, f32::MAX, f32::MIN, f32::MIN, f32::MAX, f32::MIN],
        |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[2]),
                b[2].max(p[0]),
                b[3].max(p[2]),
                b[4].min(p[1]),
                b[5].max(p[1]),
            ]
        },
    )
}

fn of_lines(lines: &[Segment]) -> [f32; 6] {
    bounds(lines.iter().flat_map(|s| [s.a, s.b]))
}

fn of_fills(fills: &[Tri]) -> [f32; 6] {
    bounds(fills.iter().flatten().copied())
}

/// A footprint in cells, with a floor and a top.
fn expected(f: &Footprint, floor: f32, top: f32) -> [f32; 6] {
    let cells = |v: i32| f32::from(i16::try_from(v).expect("small")) / 16.0;
    [
        cells(f.x0),
        cells(f.y0),
        cells(f.x1),
        cells(f.y1),
        floor,
        top,
    ]
}

fn close(a: [f32; 6], b: [f32; 6]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5)
}

#[test]
fn every_shape_s_plan_is_the_engine_s_footprint_at_every_facing() {
    let ground = Ground::default();
    for shape in SHAPES {
        for facing in Facing::ALL {
            for blocks in [false, true] {
                let look = Look {
                    shape,
                    width: 6,
                    depth: 2,
                    height: 5,
                };
                let o = object(2, 1, (3, -2), facing, look, blocks);
                let (lines, fills) = shapes(&[o], &[], &ground);
                let footprint = Footprint::of(2, 1, (3, -2), facing, 6, 2);
                let want = expected(&footprint, 0.0, 5.0 / 8.0);
                let what = format!("{shape:?} facing {facing:?}");
                assert!(
                    close(of_lines(&lines), want),
                    "{what}: lines {:?}",
                    of_lines(&lines)
                );
                assert!(
                    close(of_fills(&fills), want),
                    "{what}: fills {:?}",
                    of_fills(&fills)
                );
                assert!(lines.iter().all(|s| s.kind == SegKind::Object), "{what}");
            }
        }
    }
}

#[test]
fn a_pile_is_a_small_heap_at_its_place() {
    let pile = PileView {
        x: 2,
        y: 2,
        pos: (1, 1),
        items: Vec::new(),
    };
    let (lines, fills) = shapes(&[], &[pile], &Ground::default());
    let footprint = Footprint::of(
        2,
        2,
        (1, 1),
        Facing::North,
        PILE_LOOK.width,
        PILE_LOOK.depth,
    );
    let want = expected(&footprint, 0.0, f32::from(PILE_LOOK.height) / 8.0);
    assert!(close(of_lines(&lines), want), "{:?}", of_lines(&lines));
    assert!(close(of_fills(&fills), want));
    assert!(lines.iter().all(|s| s.kind == SegKind::Pile));
}

#[test]
fn a_shape_stands_on_its_home_cell_s_floor() {
    let data = heights_data();
    let id = map(&data, "heights:map:hills");
    let ground = Ground::from_map(&data.maps[&id], Level::GROUND);
    let m = &data.maps[&id];
    let mut raised = 0;
    for y in 0..m.def.height {
        for x in 0..m.def.width {
            let Some(patch) = ground.patch(x, y) else {
                continue;
            };
            let look = Look {
                shape: Shape::Box,
                width: 4,
                depth: 4,
                height: 8,
            };
            let (lines, _) = shapes(
                &[object(x, y, (0, 0), Facing::North, look, true)],
                &[],
                &ground,
            );
            let b = of_lines(&lines);
            assert!((b[4] - patch.centre).abs() < 1e-5, "({x}, {y})");
            assert!((b[5] - patch.centre - 1.0).abs() < 1e-5, "({x}, {y})");
            raised += usize::from(patch.centre > 0.0);
        }
    }
    assert!(raised > 3, "{raised} raised cells");
}

fn yard_world(data: &Data) -> SimWorld {
    let (mut world, _) = common::town_session(data);
    common::place(
        &mut world,
        map(data, "objects:map:yard"),
        0,
        2,
        Facing::East,
    );
    world
}

/// The lines of `kind` with both ends inside a footprint, its edges included.
fn inside(lines: &[Segment], kind: SegKind, f: &Footprint) -> usize {
    let b = expected(f, 0.0, 0.0);
    let within = |p: [f32; 3]| {
        (b[0] - 1e-5..=b[2] + 1e-5).contains(&p[0]) && (b[1] - 1e-5..=b[3] + 1e-5).contains(&p[2])
    };
    lines
        .iter()
        .filter(|s| s.kind == kind && within(s.a) && within(s.b))
        .count()
}

fn footprint_of(o: &SeenObject) -> Footprint {
    Footprint::of(o.x, o.y, o.pos, o.facing, o.look.width, o.look.depth)
}

#[test]
fn the_yard_draws_every_object_the_engine_lists_each_with_fills() {
    let data = objects_data();
    let world = yard_world(&data);
    let yard = map(&data, "objects:map:yard");
    let scene = Scene::build(&data, yard, Level::GROUND, &world);
    let (objects, piles) = world.objects_on(&data, yard, Level::GROUND);
    assert_eq!(objects.len(), 8, "the yard's ground level, two hidden");
    for o in &objects {
        let f = footprint_of(o);
        assert!(
            inside(&scene.own, SegKind::Object, &f) >= 12,
            "{}",
            o.object
        );
        let top = f32::from(o.look.height) / 8.0;
        let b = expected(&f, 0.0, 0.0);
        let on_top = |t: &Tri| {
            t.iter().all(|p| {
                (p[1] - top).abs() < 1e-5
                    && (b[0] - 1e-5..=b[2] + 1e-5).contains(&p[0])
                    && (b[1] - 1e-5..=b[3] + 1e-5).contains(&p[2])
            })
        };
        assert!(scene.fills.iter().any(on_top), "{} has a lid", o.object);
    }
    assert_eq!(piles.len(), 1);
    let arrows = Footprint::of(2, 2, (1, 1), Facing::North, 2, 2);
    assert_eq!(inside(&scene.own, SegKind::Pile, &arrows), 12);
}

#[test]
fn a_hidden_object_is_drawn_once_found_and_a_taken_one_never() {
    let data = objects_data();
    let mut world = yard_world(&data);
    let yard = map(&data, "objects:map:yard");
    let lever = Footprint::of(0, 0, (-3, -3), Facing::North, 1, 1);
    let stone = Footprint::of(0, 2, (2, -2), Facing::North, 2, 2);
    let drawn = |w: &SimWorld, f: &Footprint| {
        inside(
            &Scene::build(&data, yard, Level::GROUND, w).own,
            SegKind::Object,
            f,
        )
    };
    assert_eq!(drawn(&world, &lever), 0, "hidden");
    assert_eq!(drawn(&world, &stone), 12);
    world.known.notice(yard, "lever_1");
    assert_eq!(drawn(&world, &lever), 12, "found");
    let take = omnis_sim::Command::Act {
        object: "stone_1".into(),
        verb: omnis_sim::omnis_data::Verb::Take,
        option: 0,
    };
    world.apply(&data, take).expect("taken");
    assert_eq!(drawn(&world, &stone), 0, "taken away");
}

#[test]
fn a_shut_gate_is_drawn_crossed_and_an_open_one_is_not() {
    let data = objects_data();
    let mut world = yard_world(&data);
    let yard = map(&data, "objects:map:yard");
    let count = |w: &SimWorld| {
        let scene = Scene::build(&data, yard, Level::GROUND, w);
        let lines = scene
            .own
            .iter()
            .filter(|s| s.kind == SegKind::Object)
            .count();
        (lines, scene.fills.len())
    };
    let shut = count(&world);
    world.map_state(yard).objects.insert(
        "gate_1".into(),
        ObjectState {
            state: Some("open".into()),
            ..ObjectState::default()
        },
    );
    let open = count(&world);
    assert_eq!((shut.0 - open.0, shut.1 - open.1), (2, 2));
}

#[test]
fn the_loft_s_crate_is_drawn_with_its_level() {
    let data = objects_data();
    let world = yard_world(&data);
    let yard = map(&data, "objects:map:yard");
    let crate_2 = Footprint::of(5, 0, (0, 0), Facing::North, 6, 6);
    let below = Scene::build(&data, yard, Level::GROUND, &world);
    assert_eq!(inside(&below.own, SegKind::Object, &crate_2), 0);
    assert_eq!(inside(&below.neighbours, SegKind::Object, &crate_2), 12);
    let above = Scene::build(&data, yard, Level(1), &world);
    let lines: Vec<&Segment> = above
        .own
        .iter()
        .filter(|s| s.kind == SegKind::Object)
        .collect();
    assert_eq!(lines.len(), 12, "the loft's own crate alone");
    let low = lines
        .iter()
        .map(|s| s.a[1].min(s.b[1]))
        .fold(f32::MAX, f32::min);
    assert!(
        (low - 1.0).abs() < 1e-5,
        "on the loft's floor, a wall up: {low}"
    );
}
