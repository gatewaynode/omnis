//! The scene of a map with levels (M8.5 step 1b), on the engine's tower fixture: the party's
//! level, the levels just above and below at their own heights, the joins drawn, and no ceiling
//! grid under ground of the level above. A map without levels draws exactly its extract.

use crate::common;

use common::{SEED, data, levels_data, map};
use omnis_sim::Settings;
use omnis_sim::omnis_core::Level;
use omnis_sim::omnis_data::Data;
use omnis_vector::geometry::{SegKind, Segment, extract};
use omnis_vector::ground::Ground;
use omnis_vector::scene::{NEIGHBOUR_DIM, Scene};
use omnis_vector::sim::SimWorld;

const CELLAR: Level = Level(-1);
const UPPER: Level = Level(1);
const ROOF: Level = Level(2);

fn scene(data: &Data, level: Level) -> Scene {
    let world = SimWorld::start(data, SEED, Settings::default()).expect("an entry map");
    let id = map(data, "levels:map:tower");
    Scene::build(&data.maps[&id], id, level, &world)
}

/// Whether `lines` hold a floor-grid line starting at the north-west corner of `(x, y)` at
/// height `h`.
fn floor_at(lines: &[Segment], x: f32, y: f32, h: f32) -> bool {
    lines
        .iter()
        .any(|s| s.kind == SegKind::Floor && s.a == [x, h, y])
}

/// The lines of `kind` within cell `(x, y)` (both ends inside it, its sides included).
fn within(lines: &[Segment], kind: SegKind, x: f32, y: f32) -> Vec<Segment> {
    let inside = |p: [f32; 3]| (x..=x + 1.0).contains(&p[0]) && (y..=y + 1.0).contains(&p[2]);
    lines
        .iter()
        .filter(|s| s.kind == kind && inside(s.a) && inside(s.b))
        .copied()
        .collect()
}

fn heights(lines: &[Segment]) -> (f32, f32) {
    lines
        .iter()
        .flat_map(|s| [s.a[1], s.b[1]])
        .fold((f32::MAX, f32::MIN), |(lo, hi), h| (lo.min(h), hi.max(h)))
}

#[test]
fn from_the_ground_the_bridge_and_the_cellar_are_drawn_beside_it_at_their_heights() {
    let data = levels_data();
    let ground = scene(&data, Level::GROUND);
    assert!(
        floor_at(&ground.own, 4.0, 2.0, 0.0),
        "the road under the bridge"
    );
    assert!(
        floor_at(&ground.neighbours, 4.0, 2.0, 1.0),
        "the bridge over the road, a level up"
    );
    assert!(
        floor_at(&ground.neighbours, 1.0, 0.0, -1.0),
        "the cellar under the tower, a level down"
    );
    assert!(
        !ground.own.iter().any(|s| s.a[1] < 0.0 || s.b[1] < 0.0),
        "the party's own level does not reach into the cellar"
    );
    assert!(
        !floor_at(&ground.neighbours, 1.0, 0.0, 2.0),
        "the roof is two levels up: not drawn"
    );
}

#[test]
fn the_top_level_has_only_the_level_below_beside_it() {
    let data = levels_data();
    let roof = scene(&data, ROOF);
    assert!(
        floor_at(&roof.neighbours, 4.0, 2.0, 1.0),
        "the bridge below"
    );
    assert!(
        !floor_at(&roof.neighbours, 4.0, 2.0, 0.0),
        "the road is two levels down: not drawn"
    );
    let cellar = scene(&data, CELLAR);
    assert!(
        floor_at(&cellar.neighbours, 4.0, 2.0, 0.0),
        "the road above"
    );
    assert!(
        !floor_at(&cellar.neighbours, 4.0, 2.0, 1.0),
        "the bridge is two levels up: not drawn"
    );
}

#[test]
fn a_stair_climbs_its_cell_to_the_floor_above_and_down_to_the_floor_below() {
    let data = levels_data();
    let up = within(&scene(&data, Level::GROUND).own, SegKind::Stair, 2.0, 0.0);
    assert_eq!(heights(&up), (0.0, 1.0), "{up:?}");
    assert_eq!(up.len(), 6, "four treads on two stringers");
    let mut treads: Vec<f32> = up
        .iter()
        .filter(|s| s.a[1] == s.b[1])
        .map(|s| s.a[1])
        .collect();
    treads.sort_by(f32::total_cmp);
    assert_eq!(
        treads,
        [0.25, 0.5, 0.75, 1.0],
        "each tread a quarter higher"
    );
    let down = within(&scene(&data, UPPER).own, SegKind::Stair, 4.0, 3.0);
    assert_eq!(
        heights(&down),
        (0.0, 1.0),
        "from the bridge at 1 to the courtyard at 0"
    );
}

#[test]
fn a_ladder_rises_to_the_floor_above_with_a_rung_every_quarter() {
    let data = levels_data();
    let cellar = within(&scene(&data, CELLAR).own, SegKind::Ladder, 1.0, 1.0);
    assert_eq!(
        heights(&cellar),
        (-1.0, 0.0),
        "from the cellar to the ground"
    );
    let upper = within(&scene(&data, UPPER).own, SegKind::Ladder, 1.0, 1.0);
    assert_eq!(
        heights(&upper),
        (1.0, 2.0),
        "from the upper floor to the roof"
    );
    // Two ladders crossed, each two rails and three rungs between the floors.
    assert_eq!(upper.len(), 10, "{upper:?}");
}

#[test]
fn a_hole_is_its_opening_hatched_to_the_cell_s_corners() {
    let data = levels_data();
    let hole = within(&scene(&data, UPPER).own, SegKind::Hole, 5.0, 2.0);
    assert_eq!(hole.len(), 8, "four sides and four hatches");
    assert_eq!(heights(&hole), (1.0, 1.0), "on the upper floor");
}

#[test]
fn no_ceiling_grid_is_drawn_under_ground_of_the_level_above() {
    let mut data = levels_data();
    let id = map(&data, "levels:map:tower");
    let tower = data.maps.get_mut(&id).unwrap();
    let grass = tower
        .def
        .terrains
        .iter()
        .position(|t| t.name == "grass")
        .unwrap();
    tower.def.terrains[grass].ceiling = Some("grass".into());
    let own = scene(&data, Level::GROUND).own;
    let ceiling = |x: f32, y: f32| {
        own.iter()
            .any(|s| s.kind == SegKind::Ceiling && s.a[0] == x && s.a[2] == y)
    };
    assert!(
        ceiling(0.0, 0.0),
        "open sky above (0, 0): its ceiling is drawn"
    );
    assert!(
        !ceiling(3.0, 1.0),
        "the bridge is above (3, 1): no ceiling there"
    );
}

#[test]
fn a_map_without_levels_draws_its_extract_and_nothing_beside_it() {
    let data = data();
    let world = SimWorld::start(&data, SEED, Settings::default()).expect("an entry map");
    for (&id, m) in &data.maps {
        let scene = Scene::build(m, id, Level::GROUND, &world);
        let ground = Ground::from_map(m, Level::GROUND);
        assert_eq!(
            scene.own,
            extract(m, id, Level::GROUND, &world, &ground),
            "{}",
            m.def.id
        );
        assert!(scene.neighbours.is_empty(), "{}", m.def.id);
    }
}

#[test]
fn the_neighbours_are_stroked_dimmer_than_the_party_s_level() {
    let data = levels_data();
    let ground = scene(&data, Level::GROUND);
    let strokes: Vec<(&Segment, f32)> = ground.strokes(true).collect();
    assert_eq!(strokes.len(), ground.own.len() + ground.neighbours.len());
    let bridge = strokes
        .iter()
        .find(|(s, _)| s.kind == SegKind::Floor && s.a == [4.0, 1.0, 2.0])
        .expect("the bridge is stroked");
    assert_eq!(bridge.1, NEIGHBOUR_DIM);
    let road = strokes
        .iter()
        .find(|(s, _)| s.kind == SegKind::Floor && s.a == [4.0, 0.0, 2.0])
        .expect("the road is stroked");
    assert_eq!(road.1, 1.0);
    assert!(bridge.1 < road.1, "the bridge is dimmer than the road");
    assert!(
        ground.strokes(false).all(|(s, _)| s.kind != SegKind::Floor),
        "the grid setting hides the neighbours' grids too"
    );
}

#[test]
fn a_wall_beside_no_ground_stands_on_the_floor_it_bounds() {
    let data = levels_data();
    let upper = scene(&data, UPPER);
    // The bridge's west wall, between no ground at (3, 2) and the bridge at (4, 2): from the
    // bridge's floor (base 8) a wall high, not down to the ground under the bridge.
    let wall: Vec<Segment> = upper
        .own
        .iter()
        .filter(|s| s.kind == SegKind::Wall && s.a[0] == 4.0 && s.b[0] == 4.0)
        .filter(|s| (2.0..=3.0).contains(&s.a[2]) && (2.0..=3.0).contains(&s.b[2]))
        .copied()
        .collect();
    assert!(!wall.is_empty(), "the bridge's west wall");
    assert_eq!(heights(&wall), (1.0, 2.0), "{wall:?}");
}

#[test]
fn a_stair_pair_draws_one_flight_from_either_end() {
    let data = levels_data();
    let key = |s: &Segment| (s.a.map(f32::to_bits), s.b.map(f32::to_bits));
    let flight = |level: Level| {
        let mut lines: Vec<_> = within(&scene(&data, level).own, SegKind::Stair, 4.0, 3.0)
            .iter()
            .map(key)
            .collect();
        lines.sort_unstable();
        lines
    };
    // The courtyard's stair up and the bridge's stair down are the same flight.
    assert_eq!(flight(Level::GROUND), flight(UPPER));
}
