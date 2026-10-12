//! The collision mirror agrees with the simulation on every edge of every test map
//! (presentation-ARCHITECTURE.md §5.4, §10.2): for each cell and heading, `collide::blocks` names a
//! reason exactly when `apply(Step)` reports `Blocked` with that reason, closed doors and open.

use crate::common;

use common::{SEED, data, heights_data};
use omnis_sim::omnis_core::{Direction, Facing, Position};
use omnis_sim::{BlockReason, Command, Event, Settings, World, apply};
use omnis_vector::collide::blocks;
use omnis_vector::sim::SimWorld;

fn simulated(world: &World, data: &omnis_sim::omnis_data::Data) -> Option<BlockReason> {
    let mut trial = world.clone();
    let events =
        apply(&mut trial, data, Command::Step(Direction::Forward)).expect("explore accepts a step");
    events.iter().find_map(|e| match e {
        Event::Blocked { reason } => Some(*reason),
        _ => None,
    })
}

/// What `agree` met: every heading checked, doors opened, service doors, cliffs.
#[derive(Default)]
struct Met {
    checked: usize,
    doors: usize,
    services: usize,
    cliffs: usize,
}

/// Every map, cell and heading of `data`: the mirror names the simulation's reason.
fn agree(data: &omnis_sim::omnis_data::Data) -> Met {
    let base = SimWorld::start(data, SEED, Settings::default()).expect("an entry map");
    let mut met = Met::default();
    let Met {
        checked,
        doors,
        services,
        cliffs,
    } = &mut met;
    for (&map_id, map) in &data.maps {
        for y in 0..map.def.height {
            for x in 0..map.def.width {
                for facing in Facing::ALL {
                    let mut world = base.clone();
                    world.position = Position {
                        map: map_id,
                        level: omnis_sim::omnis_core::Level::GROUND,
                        x,
                        y,
                        facing,
                    };
                    let mirror = blocks(
                        data,
                        &world,
                        map_id,
                        omnis_sim::omnis_core::Level::GROUND,
                        x,
                        y,
                        facing,
                    );
                    assert_eq!(
                        mirror,
                        simulated(&world, data),
                        "{} ({x}, {y}) {facing:?}",
                        map.def.id
                    );
                    *checked += 1;
                    if mirror == Some(BlockReason::Cliff) {
                        *cliffs += 1;
                    }
                    if mirror == Some(BlockReason::ClosedDoor) {
                        let events = apply(&mut world, data, Command::Interact)
                            .expect("explore accepts interact");
                        // On a service's site Interact enters the service, not the door.
                        if events
                            .iter()
                            .any(|e| matches!(e, Event::ServiceEntered { .. }))
                        {
                            *services += 1;
                            continue;
                        }
                        let opened = blocks(
                            data,
                            &world,
                            map_id,
                            omnis_sim::omnis_core::Level::GROUND,
                            x,
                            y,
                            facing,
                        );
                        assert_ne!(opened, Some(BlockReason::ClosedDoor), "the door opened");
                        assert_eq!(
                            opened,
                            simulated(&world, data),
                            "open door {} ({x}, {y}) {facing:?}",
                            map.def.id
                        );
                        *doors += 1;
                    }
                }
            }
        }
    }
    // Every map, every cell, four headings.
    let cells: usize = data
        .maps
        .values()
        .map(|m| usize::from(m.def.width) * usize::from(m.def.height))
        .sum();
    assert_eq!(*checked, 4 * cells);
    met
}

#[test]
fn the_mirror_agrees_with_the_simulation_everywhere() {
    let met = agree(&data());
    assert!(met.doors > 0, "the dungeon's doors were exercised");
    assert!(met.services > 0, "the town's service doors were met");
}

/// The engine's elevation fixture (M8.5 step 1a): slopes, cliffs, a one-way edge and a ramp.
#[test]
fn the_mirror_agrees_on_cliffs_and_ramps() {
    let met = agree(&heights_data());
    assert!(met.cliffs > 10, "{} cliffs met", met.cliffs);
}
