//! Elevation (M8.5 step 1a): cliffs refuse a step by the terrain entered, ramps cross up to two
//! walls, hills hide what lies behind them, and height adds to how far the party sees. Played
//! on the fixture pack `tests/packs/heights` (one map, `heights:map:hills`), loaded after the
//! base and test packs. On the shipped flat maps sight is exactly what it was.

use crate::common::{base_pack, data, step, test_pack, turn};
use omnis_core::{Facing, Position, Rotation};
use omnis_data::{Data, MapData, load_packs};
use omnis_sim::visibility::{self, edge_open, line_of_sight};
use omnis_sim::world::MapState;
use omnis_sim::{BlockReason, Event, Settings, World, query};
use std::path::PathBuf;

fn hills() -> Data {
    let heights = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/packs/heights");
    load_packs(&[&base_pack(), &test_pack(), &heights]).unwrap_or_else(|r| panic!("{r}"))
}

/// A new world placed on the hills at `(x, y)` facing `facing`.
fn at(data: &Data, x: u16, y: u16, facing: Facing) -> World {
    let mut world = World::new(data, 7, Settings::default()).expect("entry map");
    let map = data.registry.maps.get("heights:map:hills").expect("hills");
    world.position = Position {
        map,
        level: omnis_core::Level::GROUND,
        x,
        y,
        facing,
    };
    world
}

fn moved_or_blocked(events: &[Event]) -> Option<Result<(u16, u16), BlockReason>> {
    events.iter().find_map(|e| match e {
        Event::Moved { to, .. } => Some(Ok((to.x, to.y))),
        Event::Blocked { reason } => Some(Err(*reason)),
        _ => None,
    })
}

#[test]
fn a_slope_within_the_terrain_s_climb_walks_and_a_cliff_refuses() {
    let data = hills();
    let mut world = at(&data, 0, 0, Facing::East);
    assert_eq!(
        moved_or_blocked(&step(&mut world, &data)),
        Some(Ok((1, 0))),
        "0 to 2"
    );
    assert_eq!(
        moved_or_blocked(&step(&mut world, &data)),
        Some(Ok((2, 0))),
        "2 to 4"
    );
    assert_eq!(
        moved_or_blocked(&step(&mut world, &data)),
        Some(Err(BlockReason::Cliff)),
        "4 to 12 is more than grass's 2"
    );
    assert_eq!((world.position.x, world.position.y), (2, 0));
}

#[test]
fn a_cliff_costs_what_any_other_refusal_costs() {
    let data = hills();
    let mut cliff = at(&data, 2, 0, Facing::East);
    let mut wall = at(&data, 8, 0, Facing::East);
    let before = (cliff.party_clock().elapsed, wall.party_clock().elapsed);
    let at_cliff = step(&mut cliff, &data);
    let at_wall = step(&mut wall, &data);
    assert_eq!(moved_or_blocked(&at_cliff), Some(Err(BlockReason::Cliff)));
    assert_eq!(moved_or_blocked(&at_wall), Some(Err(BlockReason::Wall)));
    assert_eq!(
        cliff.party_clock().elapsed - before.0,
        wall.party_clock().elapsed - before.1,
        "the same minutes"
    );
    assert_eq!(cliff.turn, wall.turn, "the same turn count");
}

#[test]
fn the_terrain_entered_decides_so_an_edge_can_go_one_way() {
    let data = hills();
    let mut world = at(&data, 0, 1, Facing::East);
    assert_eq!(
        moved_or_blocked(&step(&mut world, &data)),
        Some(Ok((1, 1))),
        "scree at 0 to stairs at 3: the stairs allow 4"
    );
    turn(&mut world, &data, Rotation::Around);
    assert_eq!(
        moved_or_blocked(&step(&mut world, &data)),
        Some(Err(BlockReason::Cliff)),
        "back onto scree, which allows 1"
    );
}

#[test]
fn a_ramp_crosses_two_walls_both_ways_and_the_edge_beside_it_is_a_cliff() {
    let data = hills();
    let mut world = at(&data, 0, 2, Facing::East);
    assert_eq!(
        moved_or_blocked(&step(&mut world, &data)),
        Some(Ok((1, 2))),
        "up 16"
    );
    turn(&mut world, &data, Rotation::Around);
    assert_eq!(
        moved_or_blocked(&step(&mut world, &data)),
        Some(Ok((0, 2))),
        "down 16"
    );
    let mut beside = at(&data, 1, 3, Facing::North);
    assert_eq!(
        moved_or_blocked(&step(&mut beside, &data)),
        Some(Err(BlockReason::Cliff)),
        "the terrace's face, with no ramp"
    );
}

fn seen(data: &Data, world: &World) -> Vec<(u16, u16)> {
    query::viewport(world, data)
        .expect("viewport")
        .tiles
        .iter()
        .map(|t| (t.x, t.y))
        .collect()
}

#[test]
fn a_hill_hides_the_ground_behind_it_and_the_hilltop_sees_over_a_wall() {
    let data = hills();
    let below = at(&data, 0, 4, Facing::East);
    let view = seen(&data, &below);
    assert!(view.contains(&(2, 4)), "the hill itself is seen");
    for hidden in [(3, 4), (5, 4), (7, 4)] {
        assert!(!view.contains(&hidden), "{hidden:?} lies behind the hill");
    }
    let top = at(&data, 2, 4, Facing::East);
    let view = seen(&data, &top);
    assert!(
        !view.contains(&(3, 4)),
        "the foot of a two-wall drop is under the hilltop's own edge"
    );
    for over in [(6, 4), (7, 4), (8, 4)] {
        assert!(view.contains(&over), "{over:?} is seen over the wall");
    }
    let viewport = query::viewport(&top, &data).expect("viewport");
    let below = viewport.tiles.iter().find(|t| (t.x, t.y) == (6, 4));
    assert_eq!(below.map(|t| t.height), Some(0));
    assert_eq!(viewport.tiles[0].height, 16, "the party's own tile");
}

#[test]
fn standing_high_sees_farther_by_the_rule_s_rise() {
    let data = hills();
    assert_eq!(data.rules.value("sight_rise_per_tile"), Some(8));
    let depth = |world: &World| query::viewport(world, &data).expect("vp").visibility_depth;
    assert_eq!(depth(&at(&data, 0, 4, Facing::East)), 6, "grass's depth");
    assert_eq!(
        depth(&at(&data, 2, 4, Facing::East)),
        8,
        "16 above the lowest tile around: two more"
    );
    let map = &data.maps[&data.registry.maps.get("heights:map:hills").expect("hills")];
    let place = |x, y| Position {
        map: data.registry.maps.get("heights:map:hills").expect("hills"),
        level: omnis_core::Level::GROUND,
        x,
        y,
        facing: Facing::East,
    };
    assert_eq!(visibility::above(map, place(2, 4)), 16);
    assert_eq!(
        visibility::above(map, place(0, 4)),
        0,
        "nothing lower around"
    );
    assert_eq!(
        visibility::above(map, place(1, 1)),
        3,
        "stairs at 3 beside scree at 0"
    );
}

#[test]
fn the_spyglass_stops_where_the_hill_hides_the_ground() {
    let data = hills();
    let world = at(&data, 0, 4, Facing::East);
    let map = &data.maps[&world.position.map];
    assert_eq!(
        visibility::ray(map, None, world.position, 8),
        [(1, 4), (2, 4)]
    );
}

#[test]
fn map_text_draws_a_ramp() {
    let data = hills();
    let world = at(&data, 0, 4, Facing::East);
    let text = query::map_text(&world, &data, world.position.map, omnis_core::Level::GROUND)
        .expect("text");
    assert_eq!(text.lines().nth(5), Some("|g/g g g g g g g g|"));
}

/// Sight as it was before height: the reference the flat maps must still match.
mod flat {
    use super::{MapData, MapState, edge_open};
    use omnis_core::Facing;

    pub fn line_of_sight(
        map: &MapData,
        state: Option<&MapState>,
        from: (u16, u16),
        to: (u16, u16),
    ) -> bool {
        let start = (2 * i32::from(from.0) + 1, 2 * i32::from(from.1) + 1);
        let (tx, ty) = (2 * i32::from(to.0), 2 * i32::from(to.1));
        let ends = [
            (tx + 1, ty + 1),
            (tx, ty),
            (tx + 2, ty),
            (tx, ty + 2),
            (tx + 2, ty + 2),
        ];
        ends.iter().any(|&end| {
            walk(map, state, from, to, start, end, true)
                || walk(map, state, from, to, start, end, false)
        })
    }

    fn walk(
        map: &MapData,
        state: Option<&MapState>,
        from: (u16, u16),
        target: (u16, u16),
        start: (i32, i32),
        end: (i32, i32),
        x_first: bool,
    ) -> bool {
        let (dx, dy) = (end.0 - start.0, end.1 - start.1);
        let (nx, ny) = (dx.abs(), dy.abs());
        let (crossings_x, crossings_y) = ((nx + 1) / 2, (ny + 1) / 2);
        let step_x = if dx > 0 { Facing::East } else { Facing::West };
        let step_y = if dy > 0 { Facing::South } else { Facing::North };
        let (mut x, mut y) = from;
        let (mut ix, mut iy) = (0, 0);
        if (x, y) == target {
            return true;
        }
        while ix < crossings_x || iy < crossings_y {
            let toward_x = (2 * ix + 1) * ny;
            let toward_y = (2 * iy + 1) * nx;
            let step_x_now = iy >= crossings_y
                || (ix < crossings_x && (toward_x < toward_y || (toward_x == toward_y && x_first)));
            let facing = if step_x_now {
                ix += 1;
                step_x
            } else {
                iy += 1;
                step_y
            };
            if (x, y) != from
                && map
                    .cell(omnis_core::Level::GROUND, x, y)
                    .is_some_and(|c| map.terrain(c).opaque)
            {
                return false;
            }
            if !edge_open(map, state, omnis_core::Level::GROUND, x, y, facing) {
                return false;
            }
            let (fx, fy) = facing.delta();
            x = u16::try_from(i32::from(x) + fx).unwrap_or(u16::MAX);
            y = u16::try_from(i32::from(y) + fy).unwrap_or(u16::MAX);
            if map.cell(omnis_core::Level::GROUND, x, y).is_none() {
                return false;
            }
            if (x, y) == target {
                return true;
            }
        }
        true
    }
}

#[test]
fn on_the_flat_maps_sight_is_what_it_was_for_every_pair_of_tiles() {
    let data = data();
    let mut pairs = 0_u32;
    for map in data.maps.values() {
        let (w, h) = (map.def.width, map.def.height);
        assert!(
            map.levels
                .iter()
                .flat_map(|l| &l.cells)
                .flatten()
                .all(|c| c.height == 0),
            "a shipped map is flat"
        );
        for fy in 0..h {
            for fx in 0..w {
                for ty in fy.saturating_sub(8)..h.min(fy + 9) {
                    for tx in fx.saturating_sub(8)..w.min(fx + 9) {
                        assert_eq!(
                            line_of_sight(map, None, omnis_core::Level::GROUND, (fx, fy), (tx, ty)),
                            flat::line_of_sight(map, None, (fx, fy), (tx, ty)),
                            "{} from ({fx}, {fy}) to ({tx}, {ty})",
                            map.def.id
                        );
                        pairs += 1;
                    }
                }
            }
        }
    }
    assert!(pairs > 100_000, "{pairs} pairs");
}
