//! The dark solid fills under the lines (M8.5 step 3), on the shipped maps and the engine's
//! fixtures: what is solid, where the levels open onto each other, and what an eye standing in
//! the scene can and cannot see through them.

use crate::common;

use common::{SEED, data, heights_data, levels_data, map};
use omnis_sim::omnis_core::{Facing, Level, Position};
use omnis_sim::omnis_data::{Data, MapData};
use omnis_sim::{Command, Event, Settings};
use omnis_vector::fill::{Tri, fills};
use omnis_vector::geom::wall_height;
use omnis_vector::geometry::{SegKind, extract};
use omnis_vector::ground::Ground;
use omnis_vector::scene::Scene;
use omnis_vector::sim::SimWorld;

const CELLAR: Level = Level(-1);
const UPPER: Level = Level(1);
const ROOF: Level = Level(2);

fn world(data: &Data) -> SimWorld {
    SimWorld::start(data, SEED, Settings::default()).expect("an entry map")
}

/// The fills of `level` of `map` alone, doors as `world` has them.
fn level_fills(data: &Data, world: &SimWorld, name: &str, level: Level) -> Vec<Tri> {
    let id = map(data, name);
    let m = &data.maps[&id];
    fills(m, id, level, world, &Ground::from_map(m, level))
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Where the ray from `o` along `d` meets `tri`, as a fraction of `d` (Möller–Trumbore).
fn meets(tri: &Tri, o: [f32; 3], d: [f32; 3]) -> Option<f32> {
    let (e1, e2) = (sub(tri[1], tri[0]), sub(tri[2], tri[0]));
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-9 {
        return None;
    }
    let s = sub(o, tri[0]);
    let u = dot(s, p) / det;
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    if !(0.0..=1.0).contains(&u) || v < 0.0 || u + v > 1.0 {
        return None;
    }
    Some(dot(e2, q) / det)
}

/// Whether a fill stands between the eye and `point`. A fill within a hundredth of a cell of
/// the point is the surface the point lies on, not in front of it.
fn hidden(fills: &[Tri], eye: [f32; 3], point: [f32; 3]) -> bool {
    let d = sub(point, eye);
    let near = 0.01 / dot(d, d).sqrt();
    fills
        .iter()
        .filter_map(|t| meets(t, eye, d))
        .any(|t| t > 1e-4 && t < 1.0 - near)
}

/// Whether `point` lies on a fill: a short vertical probe through it meets one, or, for a
/// vertical face, a short probe along x or z does.
fn solid(fills: &[Tri], point: [f32; 3]) -> bool {
    let probes = [[0.0, 0.02, 0.0], [0.02, 0.0, 0.0], [0.0, 0.0, 0.02]];
    probes.iter().any(|&d| {
        let o = sub(point, d);
        let d2 = [d[0] * 2.0, d[1] * 2.0, d[2] * 2.0];
        fills
            .iter()
            .filter_map(|t| meets(t, o, d2))
            .any(|t| (0.0..=1.0).contains(&t))
    })
}

/// The middle of the `facing` side of cell `(x, y)` at height `h`.
fn edge_mid(x: u16, y: u16, facing: Facing, h: f32) -> [f32; 3] {
    let (dx, dz) = match facing {
        Facing::North => (0.0, -0.5),
        Facing::South => (0.0, 0.5),
        Facing::West => (-0.5, 0.0),
        Facing::East => (0.5, 0.0),
    };
    [f32::from(x) + 0.5 + dx, h, f32::from(y) + 0.5 + dz]
}

/// The middle of the cell `(x, y)` at height `h`.
fn mid(x: u16, y: u16, h: f32) -> [f32; 3] {
    [f32::from(x) + 0.5, h, f32::from(y) + 0.5]
}

#[test]
fn every_cell_with_ground_is_filled_on_every_shipped_map() {
    let data = data();
    let world = world(&data);
    let mut checked = 0;
    for (&id, m) in &data.maps {
        let ground = Ground::from_map(m, Level::GROUND);
        let all = fills(m, id, Level::GROUND, &world, &ground);
        for y in 0..m.def.height {
            for x in 0..m.def.width {
                let cell = m
                    .cell(Level::GROUND, x, y)
                    .expect("shipped maps are one level");
                assert!(m.terrain(cell).join.is_none(), "no joins on shipped maps");
                let p = ground.patch(x, y).expect("ground");
                assert!(solid(&all, mid(x, y, p.centre)), "{} ({x}, {y})", m.def.id);
                checked += 1;
            }
        }
    }
    assert!(checked > 1000, "{checked}");
}

#[test]
fn a_tile_with_no_ground_has_no_fill() {
    let data = levels_data();
    let world = world(&data);
    let id = map(&data, "levels:map:tower");
    let m = &data.maps[&id];
    for level in [CELLAR, UPPER, ROOF] {
        let all = level_fills(&data, &world, "levels:map:tower", level);
        let inside = |t: &Tri, x: u16, y: u16| {
            let c = [0, 2].map(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0);
            let (x0, z0) = (f32::from(x), f32::from(y));
            c[0] > x0 + 1e-3 && c[0] < x0 + 1.0 - 1e-3 && c[1] > z0 + 1e-3 && c[1] < z0 + 1.0 - 1e-3
        };
        let mut empty = 0;
        for y in 0..m.def.height {
            for x in 0..m.def.width {
                if m.cell(level, x, y).is_none() {
                    assert!(!all.iter().any(|t| inside(t, x, y)), "{level} ({x}, {y})");
                    empty += 1;
                }
            }
        }
        assert!(empty > 10, "{level}: {empty}");
    }
}

#[test]
fn a_wall_is_solid_from_its_feet_to_its_top() {
    let data = data();
    let world = world(&data);
    let id = map(&data, "test:map:town");
    let m = &data.maps[&id];
    let all = level_fills(&data, &world, "test:map:town", Level::GROUND);
    let mut walls = 0;
    for y in 0..m.def.height {
        for x in 0..m.def.width {
            let cell = m.cell(Level::GROUND, x, y).expect("ground");
            let h = wall_height(m.terrain(cell));
            for facing in Facing::ALL {
                if !cell.walls.has(facing) || cell.doors.has(facing) {
                    continue;
                }
                let at = |h: f32| edge_mid(x, y, facing, h);
                assert!(solid(&all, at(h * 0.5)), "({x}, {y}) {facing:?}");
                assert!(!solid(&all, at(h + 0.1)), "above ({x}, {y}) {facing:?}");
                walls += 1;
            }
        }
    }
    assert!(walls > 20, "{walls}");
}

#[test]
fn a_closed_door_is_solid_and_an_open_one_leaves_its_opening_under_the_lintel() {
    let data = data();
    let (mut world, _) = common::town_session(&data);
    let id = map(&data, "test:map:town");
    let m = &data.maps[&id];
    let mut opened = 0;
    for y in 0..m.def.height {
        for x in 0..m.def.width {
            let cell = m.cell(Level::GROUND, x, y).expect("ground");
            let h = wall_height(m.terrain(cell));
            for facing in Facing::ALL.into_iter().filter(|&f| cell.doors.has(f)) {
                let at = |h: f32| edge_mid(x, y, facing, h);
                let (opening, lintel) = (at(h * 0.4), at(h * 0.95));
                let before = level_fills(&data, &world, "test:map:town", Level::GROUND);
                if world.door_open(id, Level::GROUND, x, y, facing) {
                    continue;
                }
                assert!(solid(&before, opening), "closed ({x}, {y}) {facing:?}");
                world.position = Position {
                    map: id,
                    level: Level::GROUND,
                    x,
                    y,
                    facing,
                };
                let events = world.apply(&data, Command::Interact).expect("interact");
                if events
                    .iter()
                    .any(|e| matches!(e, Event::ServiceEntered { .. }))
                {
                    // A service's site: Interact entered it. Start again outside.
                    world = common::town_session(&data).0;
                    continue;
                }
                if !world.door_open(id, Level::GROUND, x, y, facing) {
                    continue;
                }
                let after = level_fills(&data, &world, "test:map:town", Level::GROUND);
                assert!(!solid(&after, opening), "open ({x}, {y}) {facing:?}");
                assert!(solid(&after, lintel), "the lintel ({x}, {y}) {facing:?}");
                opened += 1;
            }
        }
    }
    assert!(opened > 0, "a door was opened");
}

#[test]
fn a_block_is_a_solid_box() {
    let data = data();
    let world = world(&data);
    let mut boxes = 0;
    for (&id, m) in &data.maps {
        let ground = Ground::from_map(m, Level::GROUND);
        let all = fills(m, id, Level::GROUND, &world, &ground);
        for y in 0..m.def.height {
            for x in 0..m.def.width {
                let cell = m.cell(Level::GROUND, x, y).expect("ground");
                let terrain = m.terrain(cell);
                if terrain.block.is_none() {
                    continue;
                }
                let (base, h) = (
                    ground.patch(x, y).expect("ground").centre,
                    wall_height(terrain),
                );
                let top = mid(x, y, base + h);
                let side = [f32::from(x) + 0.1, base + h * 0.5, f32::from(y) + 0.5];
                assert!(solid(&all, top), "{} top ({x}, {y})", m.def.id);
                assert!(solid(&all, side), "{} side ({x}, {y})", m.def.id);
                boxes += 1;
            }
        }
    }
    assert!(boxes > 0, "the shipped maps have blocks");
}

#[test]
fn a_cliff_face_is_solid_between_its_rims() {
    let data = heights_data();
    let world = world(&data);
    let id = map(&data, "heights:map:hills");
    let m = &data.maps[&id];
    let ground = Ground::from_map(m, Level::GROUND);
    let all = fills(m, id, Level::GROUND, &world, &ground);
    let hatches: Vec<_> = extract(m, id, Level::GROUND, &world, &ground)
        .into_iter()
        .filter(|s| s.kind == SegKind::Cliff)
        .collect();
    assert!(hatches.len() > 4, "{}", hatches.len());
    for s in hatches {
        let middle = [0, 1, 2].map(|k| (s.a[k] + s.b[k]) / 2.0);
        assert!(solid(&all, middle), "{s:?}");
    }
}

#[test]
fn a_hole_is_open_in_its_middle_and_solid_round_it() {
    let data = levels_data();
    let world = world(&data);
    let upper = level_fills(&data, &world, "levels:map:tower", UPPER);
    assert!(!solid(&upper, mid(5, 2, 1.0)), "the opening");
    assert!(solid(&upper, [5.07, 1.0, 2.5]), "the rim");
}

#[test]
fn stairwells_and_ladder_shafts_are_open_through_the_floors_they_join() {
    let data = levels_data();
    let world = world(&data);
    let fills = |level| level_fills(&data, &world, "levels:map:tower", level);
    let (cellar, ground, upper) = (fills(CELLAR), fills(Level::GROUND), fills(UPPER));
    assert!(
        !solid(&upper, mid(2, 0, 1.0)),
        "the stair down's floor is open"
    );
    assert!(
        solid(&ground, mid(2, 0, 0.0)),
        "the stair up stands on its floor"
    );
    assert!(
        !solid(&upper, mid(1, 1, 1.0)),
        "the ladder's shaft through the upper floor"
    );
    assert!(
        !solid(&ground, mid(1, 1, 0.0)),
        "the ladder's shaft through the ground"
    );
    assert!(
        solid(&cellar, mid(1, 1, -1.0)),
        "the ladder's foot has a floor"
    );
}

#[test]
fn a_stair_or_ladder_up_has_no_ceiling() {
    let data = levels_data();
    let world = world(&data);
    let id = map(&data, "levels:map:tower");
    // The tower with every terrain roofed, so each cell has a ceiling to open.
    let mut roofed: MapData = data.maps[&id].clone();
    for terrain in &mut roofed.def.terrains {
        terrain.ceiling = Some("ceiling".to_owned());
    }
    let all = |level| {
        fills(
            &roofed,
            id,
            level,
            &world,
            &Ground::from_map(&roofed, level),
        )
    };
    let (cellar, ground) = (all(CELLAR), all(Level::GROUND));
    assert!(!solid(&ground, mid(2, 0, 1.0)), "over the stair up");
    assert!(!solid(&ground, mid(1, 1, 1.0)), "over the ladder");
    assert!(solid(&ground, mid(1, 0, 1.0)), "over the floor beside it");
    assert!(!solid(&cellar, mid(1, 1, 0.0)), "over the cellar's ladder");
}

/// The eye on `level` of the tower at `(x, z)`, a standing height over its ground.
fn eye(data: &Data, level: Level, x: f32, z: f32) -> ([f32; 3], Scene) {
    let id = map(data, "levels:map:tower");
    let scene = Scene::build(data, id, level, &world(data));
    let y = omnis_vector::ground::eye(&scene.ground, omnis_vector::pose::Pose { x, z, yaw: 0.0 });
    ([x, y, z], scene)
}

#[test]
fn from_the_road_the_bridge_hides_what_is_above_it_and_its_underside_shows() {
    let data = levels_data();
    let (at, scene) = eye(&data, Level::GROUND, 6.5, 2.5);
    // The bridge's floor seen from below, at its north-west corner over the road.
    assert!(
        !hidden(&scene.fills, at, [4.0, 1.0, 2.0]),
        "the bridge's underside"
    );
    // A point a little over the bridge's middle, behind its floor from the road.
    assert!(
        hidden(&scene.fills, at, [4.5, 1.2, 2.5]),
        "above the bridge"
    );
    // The road through the hole, from the bridge's east edge: open; the road under the bridge,
    // behind its floor: hidden.
    let (above, scene) = eye(&data, UPPER, 4.9, 2.5);
    assert!(
        !hidden(&scene.fills, above, [5.75, 0.0, 2.5]),
        "the road through the hole"
    );
    assert!(
        hidden(&scene.fills, above, [4.5, 0.0, 2.5]),
        "the road under the bridge"
    );
}

#[test]
fn from_the_ground_the_cellar_shows_only_down_the_shaft() {
    let data = levels_data();
    // At the shaft's edge, the cellar's floor a little way down it shows; the cellar beside it,
    // under the floor, does not.
    let (at, scene) = eye(&data, Level::GROUND, 2.05, 1.5);
    assert!(
        hidden(&scene.fills, at, [2.5, -1.0, 0.5]),
        "the cellar under the floor"
    );
    assert!(
        !hidden(&scene.fills, at, [1.7, -1.0, 1.5]),
        "the cellar's floor down the shaft"
    );
}

#[test]
fn a_hill_hides_what_lies_behind_it() {
    let data = heights_data();
    let world = world(&data);
    let id = map(&data, "heights:map:hills");
    let m = &data.maps[&id];
    let scene = Scene::build(&data, id, Level::GROUND, &world);
    let ground = &scene.ground;
    // From every cell, at standing height, the ground of some other cell is hidden by the
    // hills between them.
    let mut hidden_cells = 0;
    let at = |x: u16, y: u16| {
        let p = ground.patch(x, y).expect("ground");
        mid(x, y, p.centre)
    };
    let (sx, sy) = (m.def.start.0, m.def.start.1);
    let eye = {
        let p = at(sx, sy);
        [p[0], p[1] + 0.5, p[2]]
    };
    for y in 0..m.def.height {
        for x in 0..m.def.width {
            if hidden(&scene.fills, eye, at(x, y)) {
                hidden_cells += 1;
            }
        }
    }
    assert!(
        hidden_cells > 0,
        "the hills hide some ground from the start"
    );
}

#[test]
fn on_a_flat_map_no_floor_line_is_buried_under_the_ground() {
    let data = data();
    let world = world(&data);
    let id = map(&data, "test:map:town");
    let m = &data.maps[&id];
    let scene = Scene::build(&data, id, Level::GROUND, &world);
    let start = m.def.start;
    let eye = [f32::from(start.0) + 0.5, 0.5, f32::from(start.1) + 0.5];
    // The floor lines round the cell the eye stands on: nothing but that floor is near them.
    let (x0, z0) = (f32::from(start.0), f32::from(start.1));
    let inside = |p: [f32; 3]| (x0..=x0 + 1.0).contains(&p[0]) && (z0..=z0 + 1.0).contains(&p[2]);
    let near: Vec<_> = scene
        .own
        .iter()
        .filter(|s| s.kind == SegKind::Floor && inside(s.a) && inside(s.b))
        .collect();
    assert!(!near.is_empty());
    for s in near {
        let middle = [0, 1, 2].map(|k| (s.a[k] + s.b[k]) / 2.0);
        assert!(!hidden(&scene.fills, eye, middle), "{s:?}");
    }
}

#[test]
fn the_scene_holds_its_neighbours_fills_and_a_map_of_one_level_its_own() {
    let data = levels_data();
    let world = world(&data);
    let id = map(&data, "levels:map:tower");
    let own = |level| level_fills(&data, &world, "levels:map:tower", level).len();
    let scene = Scene::build(&data, id, Level::GROUND, &world);
    assert_eq!(
        scene.fills.len(),
        own(Level::GROUND) + own(CELLAR) + own(UPPER)
    );
    let town = map(&data, "test:map:town");
    let scene = Scene::build(&data, town, Level::GROUND, &world);
    assert_eq!(
        scene.fills.len(),
        level_fills(&data, &world, "test:map:town", Level::GROUND).len()
    );
}

#[test]
fn the_mesh_holds_every_fill_as_its_own_three_vertices() {
    use bevy::mesh::{Mesh, VertexAttributeValues};
    let data = levels_data();
    let world = world(&data);
    let id = map(&data, "levels:map:tower");
    let scene = Scene::build(&data, id, Level::GROUND, &world);
    let mesh = omnis_vector::shell::render::mesh(&scene.fills);
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions");
    };
    let expected: Vec<[f32; 3]> = scene.fills.iter().flatten().copied().collect();
    assert_eq!(positions, &expected);
    assert_eq!(
        mesh.indices().map(bevy::mesh::Indices::len),
        Some(expected.len())
    );
}

#[test]
fn a_stair_is_a_solid_slab_from_its_foot_up_to_its_head_and_its_treads_show_from_the_foot() {
    let data = levels_data();
    let world = world(&data);
    let ground = level_fills(&data, &world, "levels:map:tower", Level::GROUND);
    // The tower's stair at (2, 0) rises west from the ground at 0 to the upper floor at 1.
    assert!(solid(&ground, [2.875, 0.125, 0.5]), "low by the foot");
    assert!(solid(&ground, [2.125, 0.875, 0.5]), "high by the head");
    assert!(
        !solid(&ground, [2.875, 0.875, 0.5]),
        "nothing over the foot"
    );
    let (at, scene) = eye(&data, Level::GROUND, 3.5, 0.5);
    let treads: Vec<_> = scene
        .own
        .iter()
        .filter(|s| s.kind == SegKind::Stair && s.a[1] == s.b[1] && s.a[0] > 2.0 && s.a[0] < 3.0)
        .collect();
    assert_eq!(treads.len(), 4);
    for s in treads {
        let middle = [0, 1, 2].map(|k| (s.a[k] + s.b[k]) / 2.0);
        assert!(!hidden(&scene.fills, at, middle), "{s:?}");
    }
}
