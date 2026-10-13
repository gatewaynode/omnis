//! Levels in one map (M8.5 step 1b): stairs go up and down in pairs, a ladder is climbed one
//! level at a time, a hole drops one way, a step into no ground is refused, and sight and the
//! automap stay on the party's level. Played on the fixture pack `tests/packs/levels` (one map,
//! `levels:map:tower`, four levels), loaded after the base and test packs.

use crate::common::{base_pack, step, test_pack, turn};
use omnis_core::{Direction, Facing, Level, Position, Rotation, Vertical};
use omnis_data::{Data, load_packs};
use omnis_sim::ops::{Op, Reply, dispatch};
use omnis_sim::{BlockReason, Command, Event, Rejection, Settings, World, apply, query};
use std::path::PathBuf;

const CELLAR: Level = Level(-1);
const UPPER: Level = Level(1);
const ROOF: Level = Level(2);

fn tower() -> Data {
    let levels = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/packs/levels");
    load_packs(&[&base_pack(), &test_pack(), &levels]).unwrap_or_else(|r| panic!("{r}"))
}

/// A new world placed in the tower at `(x, y)` of `level`, facing `facing`.
fn at(data: &Data, level: Level, x: u16, y: u16, facing: Facing) -> World {
    let mut world = World::new(data, 7, Settings::default()).expect("entry map");
    let map = data.registry.maps.get("levels:map:tower").expect("tower");
    world.position = Position {
        map,
        level,
        x,
        y,
        facing,
    };
    world
}

/// Where the party is: level, column, row.
fn place(world: &World) -> (Level, u16, u16) {
    (world.position.level, world.position.x, world.position.y)
}

/// Every `Moved` of a command's events, as (level, x, y) of where it ended.
fn moves(events: &[Event]) -> Vec<(Level, u16, u16)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Moved { to, .. } => Some((to.level, to.x, to.y)),
            _ => None,
        })
        .collect()
}

fn blocked(events: &[Event]) -> Option<BlockReason> {
    events.iter().find_map(|e| match e {
        Event::Blocked { reason } => Some(*reason),
        _ => None,
    })
}

/// The tiles a command's `Visible` lists.
fn visible(events: &[Event]) -> Vec<(u16, u16)> {
    events
        .iter()
        .find_map(|e| match e {
            Event::Visible { tiles } => Some(tiles.iter().map(|t| (t.x, t.y)).collect()),
            _ => None,
        })
        .expect("the cone")
}

fn climb(world: &mut World, data: &Data, way: Vertical) -> Result<Vec<Event>, Rejection> {
    apply(world, data, Command::Climb(way))
}

#[test]
fn the_tower_has_its_four_levels_lowest_first() {
    let data = tower();
    let map = &data.maps[&data.registry.maps.get("levels:map:tower").unwrap()];
    let levels: Vec<(Level, &str)> = map
        .levels
        .iter()
        .map(|l| (l.level, l.name.as_str()))
        .collect();
    assert_eq!(
        levels,
        [
            (CELLAR, "levels:text:map.tower.cellar"),
            (Level::GROUND, ""),
            (UPPER, "levels:text:map.tower.upper"),
            (ROOF, "levels:text:map.tower.roof"),
        ]
    );
}

#[test]
fn a_stair_goes_up_and_the_stair_at_the_top_does_not_send_the_party_back() {
    let data = tower();
    // The tower's stair rises west; its foot is the grass east of it.
    let mut world = at(&data, Level::GROUND, 3, 0, Facing::West);
    let events = step(&mut world, &data);
    assert_eq!(
        moves(&events),
        [(Level::GROUND, 2, 0), (UPPER, 2, 0)],
        "onto the stair, then up it"
    );
    assert_eq!(place(&world), (UPPER, 2, 0));
    assert_eq!(world.position.facing, Facing::West, "the facing is kept");
    // Standing on the stair down does nothing; off its head, then back onto it, goes down.
    assert_eq!(moves(&step(&mut world, &data)), [(UPPER, 1, 0)]);
    turn(&mut world, &data, Rotation::Around);
    assert_eq!(
        moves(&step(&mut world, &data)),
        [(UPPER, 2, 0), (Level::GROUND, 2, 0)],
        "and back down"
    );
    assert_eq!(place(&world), (Level::GROUND, 2, 0));
}

#[test]
fn a_stair_up_is_taken_only_from_its_foot() {
    let data = tower();
    // From the side (south) and from the head (west), the stair is floor.
    for (x, y, facing) in [(2, 1, Facing::North), (1, 0, Facing::East)] {
        let mut world = at(&data, Level::GROUND, x, y, facing);
        assert_eq!(
            moves(&step(&mut world, &data)),
            [(Level::GROUND, 2, 0)],
            "from ({x}, {y})"
        );
        assert_eq!(place(&world), (Level::GROUND, 2, 0));
    }
}

#[test]
fn a_stair_down_is_taken_only_from_its_head() {
    let data = tower();
    // From the side (south, the tower floor at (2, 1)) the stair down is floor.
    let mut world = at(&data, UPPER, 2, 1, Facing::North);
    assert_eq!(moves(&step(&mut world, &data)), [(UPPER, 2, 0)]);
    // The bridge's stair down rises north: from its head, the bridge at (4, 2), it goes down.
    let mut world = at(&data, UPPER, 4, 2, Facing::South);
    assert_eq!(
        moves(&step(&mut world, &data)),
        [(UPPER, 4, 3), (Level::GROUND, 4, 3)]
    );
}

#[test]
fn a_step_takes_a_stair_by_the_way_it_moves_not_the_way_the_party_faces() {
    let data = tower();
    // Backing west onto the stair from its foot, facing east, takes it.
    let mut world = at(&data, Level::GROUND, 3, 0, Facing::East);
    let events = apply(&mut world, &data, Command::Step(Direction::Back)).expect("a step");
    assert_eq!(moves(&events), [(Level::GROUND, 2, 0), (UPPER, 2, 0)]);
    // Sidestepping north onto the courtyard stair from (4, 4), facing west, takes it too.
    let mut world = at(&data, Level::GROUND, 4, 4, Facing::West);
    let events = apply(&mut world, &data, Command::Step(Direction::Right)).expect("a step");
    assert_eq!(moves(&events), [(Level::GROUND, 4, 3), (UPPER, 4, 3)]);
}

#[test]
fn a_stair_costs_its_own_minutes_once() {
    let data = tower();
    let mut world = at(&data, Level::GROUND, 3, 0, Facing::West);
    let before = world.party_clock().elapsed;
    step(&mut world, &data);
    assert_eq!(world.party_clock().elapsed - before, 3, "the stair's 3");
}

#[test]
fn the_query_a_client_mirrors_predicts_where_a_stair_leaves_the_party() {
    let data = tower();
    let world = at(&data, Level::GROUND, 3, 0, Facing::West);
    let lands = query::step_lands(&world, &data, Direction::Forward).expect("lands");
    assert_eq!((lands.level, lands.x, lands.y), (UPPER, 2, 0));
    let side = at(&data, Level::GROUND, 2, 1, Facing::North);
    let lands = query::step_lands(&side, &data, Direction::Forward).expect("lands");
    assert_eq!(
        (lands.level, lands.x, lands.y),
        (Level::GROUND, 2, 0),
        "from the side"
    );
}

#[test]
fn a_hole_drops_the_party_onto_the_road_and_does_not_lift_it_back() {
    let data = tower();
    let mut world = at(&data, UPPER, 4, 2, Facing::East);
    assert_eq!(
        moves(&step(&mut world, &data)),
        [(UPPER, 5, 2), (Level::GROUND, 5, 2)]
    );
    assert_eq!(place(&world), (Level::GROUND, 5, 2));
    // The road under the hole is road: stepping off and on again stays on the ground.
    turn(&mut world, &data, Rotation::Around);
    step(&mut world, &data);
    turn(&mut world, &data, Rotation::Around);
    assert_eq!(moves(&step(&mut world, &data)), [(Level::GROUND, 5, 2)]);
    assert_eq!(
        climb(&mut world, &data, Vertical::Up),
        Err(Rejection::NoLadder),
        "nothing climbs up a hole"
    );
}

#[test]
fn a_ladder_climbs_one_level_a_command_from_the_cellar_to_the_roof() {
    let data = tower();
    let mut world = at(&data, CELLAR, 1, 1, Facing::North);
    for level in [Level::GROUND, UPPER, ROOF] {
        let events = climb(&mut world, &data, Vertical::Up).expect("a ladder");
        assert_eq!(moves(&events), [(level, 1, 1)]);
    }
    assert_eq!(
        climb(&mut world, &data, Vertical::Up),
        Err(Rejection::NoLadder),
        "the roof is the top"
    );
    // The roof's tile is floor; the ladder below it is what joins them.
    for level in [UPPER, Level::GROUND, CELLAR] {
        climb(&mut world, &data, Vertical::Down).expect("down the ladder");
        assert_eq!(place(&world), (level, 1, 1));
    }
    assert_eq!(
        climb(&mut world, &data, Vertical::Down),
        Err(Rejection::NoLadder),
        "the cellar is the bottom"
    );
}

#[test]
fn a_climb_costs_the_far_tile_s_minutes_and_keeps_the_facing() {
    let data = tower();
    let mut world = at(&data, Level::GROUND, 1, 1, Facing::South);
    let before = world.party_clock().elapsed;
    climb(&mut world, &data, Vertical::Up).expect("a ladder");
    assert_eq!(world.party_clock().elapsed - before, 5, "the ladder's 5");
    let before = world.party_clock().elapsed;
    climb(&mut world, &data, Vertical::Up).expect("to the roof");
    assert_eq!(world.party_clock().elapsed - before, 1, "the roof's floor");
    assert_eq!(world.position.facing, Facing::South);
}

#[test]
fn off_a_ladder_a_climb_is_refused_and_changes_nothing() {
    let data = tower();
    let mut world = at(&data, Level::GROUND, 2, 1, Facing::North);
    let before = world.clone();
    assert_eq!(
        climb(&mut world, &data, Vertical::Up),
        Err(Rejection::NoLadder)
    );
    assert_eq!(
        climb(&mut world, &data, Vertical::Down),
        Err(Rejection::NoLadder)
    );
    assert_eq!(world, before);
}

#[test]
fn a_step_into_no_ground_is_refused() {
    let data = tower();
    let mut world = at(&data, UPPER, 4, 1, Facing::North);
    assert_eq!(
        blocked(&step(&mut world, &data)),
        Some(BlockReason::NoGround)
    );
    assert_eq!(place(&world), (UPPER, 4, 1));
}

#[test]
fn sight_and_the_automap_stay_on_the_party_s_level() {
    let data = tower();
    let map = data.registry.maps.get("levels:map:tower").unwrap();
    let mut world = at(&data, UPPER, 4, 1, Facing::East);
    let events = turn(&mut world, &data, Rotation::Right);
    let tiles = visible(&events);
    assert!(
        tiles.contains(&(4, 3)),
        "the stair at the bridge's end: {tiles:?}"
    );
    // North the edge is open onto no ground, with the courtyard's grass below it.
    let north = visible(&turn(&mut world, &data, Rotation::Around));
    assert!(
        north.contains(&(2, 0)),
        "through the tower's east door, which the ground floor walls up: {north:?}"
    );
    let tower = &data.maps[&map];
    for (x, y) in tiles.into_iter().chain(north) {
        assert!(
            tower.cell(UPPER, x, y).is_some(),
            "({x}, {y}) has no ground on the upper floor"
        );
    }
    let upper = query::automap(&world, map, UPPER).expect("seen on the upper floor");
    assert!(upper.contains_key(&(4, 2)), "the bridge ahead");
    assert!(
        query::automap(&world, map, Level::GROUND).is_none(),
        "the road below is not seen from the bridge"
    );
}

#[test]
fn the_json_tier_climbs_and_draws_a_level() {
    let data = tower();
    let mut world = at(&data, Level::GROUND, 1, 1, Facing::North);
    let reply = dispatch(
        &mut world,
        &data,
        &Op::SimCommand {
            command: Command::Climb(Vertical::Up),
        },
    )
    .expect("climbed");
    let Reply::Events { events } = reply else {
        panic!("{reply:?}")
    };
    assert_eq!(moves(&events), [(UPPER, 1, 1)]);
    let text = |world: &mut World, level| match dispatch(
        world,
        &data,
        &Op::MapText { map: None, level },
    ) {
        Ok(Reply::Text { text }) => text,
        other => panic!("{other:?}"),
    };
    let upper = text(&mut world, None);
    assert!(
        upper.contains('_'),
        "the party's level, with no ground:\n{upper}"
    );
    let ground = text(&mut world, Some(Level::GROUND));
    assert!(!ground.contains('_'), "the ground has ground:\n{ground}");
}

#[test]
fn a_level_is_written_in_the_save_only_off_the_ground() {
    let data = tower();
    let mut world = at(&data, Level::GROUND, 1, 1, Facing::North);
    let ground = world.to_ron().unwrap();
    climb(&mut world, &data, Vertical::Up).unwrap();
    let upper = world.to_ron().unwrap();
    let position = |text: &str| {
        let at = text.find("position:").expect("a position");
        text[at..at + text[at..].find(')').unwrap()].to_owned()
    };
    assert!(
        !position(&ground).contains("level"),
        "{}",
        position(&ground)
    );
    assert!(
        position(&upper).contains("level: 1"),
        "{}",
        position(&upper)
    );
    let back = World::from_ron(&upper, &data, false).expect("loads");
    assert_eq!(place(&back), (UPPER, 1, 1));
}

#[test]
fn script_words_climb() {
    let data = tower();
    let world = at(&data, Level::GROUND, 1, 1, Facing::North);
    assert_eq!(
        crate::common::script(&world, &data, "climb-up climb-down"),
        [Command::Climb(Vertical::Up), Command::Climb(Vertical::Down)]
    );
}
