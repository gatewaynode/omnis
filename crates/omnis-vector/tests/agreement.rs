//! The collision mirror agrees with the simulation on every edge of every test map
//! (alt-ARCHITECTURE.md §5.4, §10.2): for each cell and heading, `collide::blocks` names a
//! reason exactly when `apply(Step)` reports `Blocked` with that reason, closed doors and open.

mod common;

use common::{SEED, data};
use omnis_sim::omnis_core::{Direction, Facing, Position};
use omnis_sim::{BlockReason, Command, Event, Settings, World, apply};
use omnis_vector::collide::blocks;

fn simulated(world: &World, data: &omnis_sim::omnis_data::Data) -> Option<BlockReason> {
    let mut trial = world.clone();
    let events =
        apply(&mut trial, data, Command::Step(Direction::Forward)).expect("explore accepts a step");
    events.iter().find_map(|e| match e {
        Event::Blocked { reason } => Some(*reason),
        _ => None,
    })
}

#[test]
fn the_mirror_agrees_with_the_simulation_everywhere() {
    let data = data();
    let base = World::new(&data, SEED, Settings::default()).expect("an entry map");
    let mut checked = 0;
    let mut doors = 0;
    for (&map_id, map) in &data.maps {
        for y in 0..map.def.height {
            for x in 0..map.def.width {
                for facing in Facing::ALL {
                    let mut world = base.clone();
                    world.position = Position {
                        map: map_id,
                        x,
                        y,
                        facing,
                    };
                    let mirror = blocks(&data, &world, map_id, x, y, facing);
                    assert_eq!(
                        mirror,
                        simulated(&world, &data),
                        "{} ({x}, {y}) {facing:?}",
                        map.def.id
                    );
                    checked += 1;
                    if mirror == Some(BlockReason::ClosedDoor) {
                        apply(&mut world, &data, Command::Interact)
                            .expect("explore accepts interact");
                        let opened = blocks(&data, &world, map_id, x, y, facing);
                        assert_ne!(opened, Some(BlockReason::ClosedDoor), "the door opened");
                        assert_eq!(
                            opened,
                            simulated(&world, &data),
                            "open door {} ({x}, {y}) {facing:?}",
                            map.def.id
                        );
                        doors += 1;
                    }
                }
            }
        }
    }
    // Both test maps, every cell, four headings: 24² + 32² cells.
    assert_eq!(checked, 4 * (24 * 24 + 32 * 32));
    assert!(doors > 0, "the dungeon's doors were exercised");
}
