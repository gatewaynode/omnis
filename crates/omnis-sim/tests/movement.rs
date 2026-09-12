//! Walking the test maps: steps, walls, doors, pillars, portals, time, and the automap.

mod common;

use common::{data, interact, step, turn, without_visible, world};
use omnis_core::{Direction, Facing, Position, Rotation};
use omnis_sim::world::layer;
use omnis_sim::{BlockReason, Command, Event, MessageKey, PARTY, apply, query};

#[test]
fn a_new_game_starts_on_the_entry_map_and_sees_the_meadow() {
    let data = data();
    let world = world(&data);
    let meadow = data.registry.maps.get("test:map:meadow").unwrap();
    assert_eq!(
        world.position,
        Position {
            map: meadow,
            x: 16,
            y: 16,
            facing: Facing::North
        }
    );
    assert_eq!(world.party_clock().elapsed, 0);
    assert!(
        world.automap.maps.is_empty(),
        "nothing seen until the first command"
    );
    let view = query::viewport(&world, &data).expect("map loaded");
    assert_eq!((view.detail_depth, view.visibility_depth), (4, 12));
    // Each row is one tile wider than the diagonal on both sides.
    let near: usize = (0..=5).map(|d| 2 * (d + 1) + 1).sum();
    assert!(
        view.tiles[..near].iter().all(|t| t.depth <= 5),
        "nearest rows first"
    );
    assert_eq!(
        view.tiles.iter().filter(|t| t.depth <= 5).count(),
        near,
        "nothing hides the first six rows"
    );
    assert!(
        view.tiles.len() < 169,
        "the tree clumps hide some of the far rows"
    );
    assert_eq!(
        view.tiles[0],
        query::ViewTile {
            depth: 0,
            offset: 0,
            x: 16,
            y: 16,
            terrain: 1,
            front: query::EdgeView::Open,
            left: query::EdgeView::Open,
            right: query::EdgeView::Open
        }
    );
}

#[test]
fn steps_take_terrain_time_and_fill_the_automap() {
    let data = data();
    let mut world = world(&data);
    let meadow = world.position.map;
    let events = step(&mut world, &data);
    assert_eq!(
        without_visible(events.clone()),
        [
            Event::Moved {
                from: Position {
                    map: meadow,
                    x: 16,
                    y: 16,
                    facing: Facing::North
                },
                to: Position {
                    map: meadow,
                    x: 16,
                    y: 15,
                    facing: Facing::North
                }
            },
            Event::TimeAdvanced {
                holder: PARTY,
                minutes: 1,
                day_rolled: false
            },
        ]
    );
    let Some(Event::Visible { tiles }) = events.last() else {
        panic!("every command ends with Visible")
    };
    assert_eq!(tiles[0].depth, 0);
    let known = query::automap(&world, meadow).unwrap();
    assert_eq!(
        known[&(16, 15)].layers,
        layer::VISITED | layer::TERRAIN | layer::STRUCTURE
    );
    assert_eq!(
        known[&(16, 10)].layers,
        layer::TERRAIN | layer::STRUCTURE,
        "seen, not visited"
    );
    assert_eq!(known[&(16, 10)].seen_at, 1);
    assert!(
        !known.contains_key(&(16, 17)),
        "behind the party is not in the cone"
    );

    // Sidestep onto grass costs two minutes; turning costs none.
    let events = apply(&mut world, &data, Command::Step(Direction::Right)).unwrap();
    assert!(
        matches!(events[1], Event::TimeAdvanced { minutes: 2, .. }),
        "{events:?}"
    );
    assert_eq!(world.position.x, 17);
    assert_eq!(world.position.facing, Facing::North);
    let events = turn(&mut world, &data, Rotation::Left);
    assert_eq!(without_visible(events), []);
    assert_eq!(world.position.facing, Facing::West);
    assert_eq!(world.party_clock().elapsed, 3);
    assert_eq!(world.turn, 3);
}

#[test]
fn water_blocks_movement_but_not_sight_and_the_hedge_blocks_both() {
    let data = data();
    let mut world = world(&data);
    let meadow = world.position.map;
    // Pond occupies x 6..=10, y 20..=24. Stand at (11, 22) facing west.
    world.position = Position {
        map: meadow,
        x: 11,
        y: 22,
        facing: Facing::West,
    };
    let events = step(&mut world, &data);
    assert_eq!(
        without_visible(events),
        [Event::Blocked {
            reason: BlockReason::Impassable
        }]
    );
    let known = query::automap(&world, meadow).unwrap();
    assert!(
        known.contains_key(&(5, 22)),
        "the far shore is visible across the water"
    );
    // The hedge: at (0, 22) facing west there is nothing to step into.
    world.position = Position {
        map: meadow,
        x: 0,
        y: 22,
        facing: Facing::West,
    };
    let events = step(&mut world, &data);
    assert_eq!(
        without_visible(events),
        [Event::Blocked {
            reason: BlockReason::Wall
        }]
    );
    let view = query::viewport(&world, &data).unwrap();
    assert!(
        view.tiles.iter().all(|t| t.depth == 0),
        "a wall at depth 0 ends the cone"
    );
    assert_eq!(
        view.tiles.len(),
        3,
        "the party's tile and the two beside it"
    );
    assert_eq!(view.tiles[0].front, query::EdgeView::Wall);
}

#[test]
fn the_party_sees_the_tiles_beside_it() {
    let data = data();
    let mut world = world(&data);
    let view = query::viewport(&world, &data).unwrap();
    let row0: Vec<i8> = view
        .tiles
        .iter()
        .filter(|t| t.depth == 0)
        .map(|t| t.offset)
        .collect();
    assert_eq!(
        row0,
        [0, 1, -1],
        "open ground: both neighbours, centre first"
    );

    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    world.position = Position {
        map: dungeon,
        x: 5,
        y: 0,
        facing: Facing::North,
    };
    let view = query::viewport(&world, &data).unwrap();
    let row0: Vec<i8> = view.tiles.iter().map(|t| t.offset).collect();
    assert_eq!(
        row0,
        [0, -1],
        "the room wall on the right hides that neighbour; the map edge ends the cone"
    );
}

#[test]
fn trees_are_opaque() {
    let data = data();
    let mut world = world(&data);
    let meadow = world.position.map;
    // The clump around (24, 10) with radius 3 lies to the north-east of (20, 15).
    world.position = Position {
        map: meadow,
        x: 20,
        y: 15,
        facing: Facing::North,
    };
    let view = query::viewport(&world, &data).unwrap();
    let at = |x: u16, y: u16| view.tiles.iter().any(|t| t.x == x && t.y == y);
    assert!(
        at(20, 8) && at(18, 8),
        "open ground straight ahead and to the left"
    );
    assert!(at(21, 10), "the nearest tree is seen");
    assert!(
        at(21, 9),
        "the tile behind a lone tree shows past its corner"
    );
    assert!(!at(22, 9), "but nothing behind the clump's edge");
    assert!(!at(23, 9) && !at(24, 8), "nor anything past the clump");
    assert!(at(19, 8), "but the line that skirts the clump is clear");
}

#[test]
fn portals_link_the_maps_both_ways() {
    let data = data();
    let mut world = world(&data);
    let meadow = world.position.map;
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    for _ in 0..10 {
        step(&mut world, &data);
    }
    assert_eq!((world.position.x, world.position.y), (16, 6));
    let events = without_visible(step(&mut world, &data));
    assert_eq!(events.len(), 3, "{events:?}");
    assert_eq!(
        events[2],
        Event::Moved {
            from: Position {
                map: meadow,
                x: 16,
                y: 5,
                facing: Facing::North
            },
            to: Position {
                map: dungeon,
                x: 1,
                y: 0,
                facing: Facing::South
            }
        }
    );
    assert_eq!(world.position.map, dungeon);
    assert_eq!(world.party_clock().elapsed, 11, "eleven road steps");
    assert!(
        query::automap(&world, dungeon)
            .unwrap()
            .contains_key(&(1, 0))
    );
    assert!(
        query::automap(&world, meadow)
            .unwrap()
            .contains_key(&(16, 5)),
        "the portal tile was visited too"
    );

    // Back out: the exit is the dungeon's (0, 0).
    world.position = Position {
        map: dungeon,
        x: 1,
        y: 0,
        facing: Facing::West,
    };
    step(&mut world, &data);
    assert_eq!(
        world.position,
        Position {
            map: meadow,
            x: 16,
            y: 6,
            facing: Facing::South
        }
    );
}

#[test]
fn doors_block_until_opened_and_pillars_always() {
    let data = data();
    let mut world = world(&data);
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    world.position = Position {
        map: dungeon,
        x: 5,
        y: 3,
        facing: Facing::East,
    };
    let view = query::viewport(&world, &data).unwrap();
    assert_eq!(view.tiles[0].front, query::EdgeView::Door { open: false });
    assert!(
        view.tiles.iter().all(|t| t.depth == 0),
        "a closed door ends the cone"
    );

    assert_eq!(
        without_visible(step(&mut world, &data)),
        [Event::Blocked {
            reason: BlockReason::ClosedDoor
        }]
    );
    let events = without_visible(interact(&mut world, &data));
    assert_eq!(
        events,
        [
            Event::Door {
                map: dungeon,
                x: 5,
                y: 3,
                facing: Facing::East,
                open: true
            },
            Event::TimeAdvanced {
                holder: PARTY,
                minutes: 1,
                day_rolled: false
            }
        ]
    );
    let view = query::viewport(&world, &data).unwrap();
    assert!(
        view.tiles.len() > 1,
        "an open door lets the cone through: {}",
        view.tiles.len()
    );
    assert_eq!(view.tiles[0].front, query::EdgeView::Door { open: true });
    step(&mut world, &data);
    assert_eq!((world.position.x, world.position.y), (6, 3));

    // From the other side the same door is open, and interacting closes it.
    turn(&mut world, &data, Rotation::Around);
    assert_eq!(
        query::viewport(&world, &data).unwrap().tiles[0].front,
        query::EdgeView::Door { open: true }
    );
    let events = without_visible(interact(&mut world, &data));
    assert_eq!(
        events[0],
        Event::Door {
            map: dungeon,
            x: 6,
            y: 3,
            facing: Facing::West,
            open: false
        }
    );
    assert_eq!(
        without_visible(step(&mut world, &data)),
        [Event::Blocked {
            reason: BlockReason::ClosedDoor
        }]
    );
}

#[test]
fn sealed_walls_pillars_and_empty_edges() {
    let data = data();
    let mut world = world(&data);
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    // A sealed room wall and a pillar.
    world.position = Position {
        map: dungeon,
        x: 11,
        y: 3,
        facing: Facing::East,
    };
    assert_eq!(
        without_visible(step(&mut world, &data)),
        [Event::Blocked {
            reason: BlockReason::Wall
        }]
    );
    world.position = Position {
        map: dungeon,
        x: 2,
        y: 3,
        facing: Facing::East,
    };
    assert_eq!(
        without_visible(step(&mut world, &data)),
        [Event::Blocked {
            reason: BlockReason::Impassable
        }]
    );
    let view = query::viewport(&world, &data).unwrap();
    let at = |x: u16, y: u16| view.tiles.iter().any(|t| (t.x, t.y) == (x, y));
    assert!(at(3, 3), "the pillar is seen");
    assert!(!at(4, 3), "but not through");
    assert!(
        at(3, 2) && at(3, 4),
        "the diagonals beside a pillar are seen round the corner"
    );
    assert!(at(5, 0), "and the far corner past them");
    assert!(
        !at(5, 5),
        "the other far corner lies squarely behind the pillar"
    );
    assert!(
        view.tiles.len() >= 10,
        "a lit room is mostly visible: {}",
        view.tiles.len()
    );

    // Interacting with a plain wall says so.
    world.position = Position {
        map: dungeon,
        x: 2,
        y: 2,
        facing: Facing::West,
    };
    turn(&mut world, &data, Rotation::Around);
    assert_eq!(
        without_visible(interact(&mut world, &data)),
        [Event::Message {
            key: MessageKey::NothingHere
        }]
    );
    assert_eq!(
        MessageKey::NothingHere.text_key(),
        "sim:message:nothing_here"
    );
}

#[test]
fn a_day_rolls_after_1440_minutes() {
    let data = data();
    let mut world = world(&data);
    world.clocks.get_mut(&PARTY).unwrap().elapsed = 1439;
    let events = step(&mut world, &data);
    assert!(
        matches!(
            events[1],
            Event::TimeAdvanced {
                minutes: 1,
                day_rolled: true,
                ..
            }
        ),
        "{events:?}"
    );
}

#[test]
fn the_log_is_bounded_and_not_saved() {
    let data = data();
    let mut world = world(&data);
    for _ in 0..(omnis_sim::LOG_CAPACITY + 10) {
        turn(&mut world, &data, Rotation::Left);
    }
    assert_eq!(
        world.log.len(),
        omnis_sim::LOG_CAPACITY,
        "one Visible per turn, capped"
    );
    let text = world.to_ron().unwrap();
    assert!(!text.contains("log"), "the log is not part of the save");
}
