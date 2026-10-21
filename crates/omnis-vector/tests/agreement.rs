//! The collision mirror agrees with the simulation on every edge of every test map
//! (presentation-ARCHITECTURE.md §5.4, §10.2): for each cell and heading, `collide::blocks` names a
//! reason exactly when `apply(Step)` reports `Blocked` with that reason, closed doors and open.

use crate::common;

use common::{SEED, data, heights_data, levels_data, map, objects_data};
use omnis_sim::api::ObjectState;
use omnis_sim::omnis_core::{Direction, Facing, Position};
use omnis_sim::{BlockReason, Command, Event, Rejection, Settings, World, apply};
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

/// What `agree` met: every heading checked, doors opened, service doors, cliffs, edges onto no
/// ground.
#[derive(Default)]
struct Met {
    checked: usize,
    doors: usize,
    services: usize,
    cliffs: usize,
    no_ground: usize,
    objects: usize,
}

/// Every map, level, cell and heading of `data`: the mirror names the simulation's reason.
fn agree(data: &omnis_sim::omnis_data::Data) -> Met {
    agree_in(
        data,
        &SimWorld::start(data, SEED, Settings::default()).expect("an entry map"),
    )
}

/// `agree` from the world `base`, its maps' state as it has them.
fn agree_in(data: &omnis_sim::omnis_data::Data, base: &SimWorld) -> Met {
    let mut met = Met::default();
    let Met {
        checked,
        doors,
        services,
        cliffs,
        no_ground,
        objects,
    } = &mut met;
    for (&map_id, map) in &data.maps {
        for level in map.levels.iter().map(|l| l.level) {
            for y in 0..map.def.height {
                for x in 0..map.def.width {
                    for facing in Facing::ALL {
                        let mut world = base.clone();
                        world.position = Position {
                            map: map_id,
                            level,
                            x,
                            y,
                            facing,
                        };
                        let mirror = blocks(data, &world, map_id, level, x, y, facing);
                        assert_eq!(
                            mirror,
                            simulated(&world, data),
                            "{} level {level} ({x}, {y}) {facing:?}",
                            map.def.id
                        );
                        *checked += 1;
                        if mirror == Some(BlockReason::Cliff) {
                            *cliffs += 1;
                        }
                        if mirror == Some(BlockReason::NoGround) {
                            *no_ground += 1;
                        }
                        if mirror == Some(BlockReason::Object) {
                            *objects += 1;
                        }
                        if mirror == Some(BlockReason::ClosedDoor) {
                            // Where an object is in reach too, Use asks which: the door.
                            let events = match apply(&mut world, data, Command::Interact) {
                                Err(Rejection::ChooseVerb { .. }) => {
                                    apply(&mut world, data, Command::Door).expect("the door")
                                }
                                other => other.expect("explore accepts interact"),
                            };
                            // On a service's site Interact enters the service, not the door.
                            if events
                                .iter()
                                .any(|e| matches!(e, Event::ServiceEntered { .. }))
                            {
                                *services += 1;
                                continue;
                            }
                            let opened = blocks(data, &world, map_id, level, x, y, facing);
                            assert_ne!(opened, Some(BlockReason::ClosedDoor), "the door opened");
                            assert_eq!(
                                opened,
                                simulated(&world, data),
                                "open door {} level {level} ({x}, {y}) {facing:?}",
                                map.def.id
                            );
                            *doors += 1;
                        }
                    }
                }
            }
        }
    }
    // Every map, every level, every cell, four headings.
    let cells: usize = data
        .maps
        .values()
        .map(|m| m.levels.len() * usize::from(m.def.width) * usize::from(m.def.height))
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

/// The engine's levels fixture (M8.5 step 1b): four levels, each checked on its own cells, with
/// edges onto no ground.
#[test]
fn the_mirror_agrees_on_every_level() {
    let met = agree(&levels_data());
    assert!(
        met.no_ground > 0,
        "{} edges onto no ground met",
        met.no_ground
    );
}

/// The engine's objects fixture (M8.5 step 2): the yard's footprints block as the engine's do,
/// the shut gate across the hedge's gap among them, and once the gate is open it no longer does.
#[test]
fn the_mirror_agrees_on_the_yard_s_objects() {
    let data = objects_data();
    let mut world = SimWorld::start(&data, SEED, Settings::default()).expect("an entry map");
    let shut = agree_in(&data, &world);
    assert!(shut.objects > 10, "{} steps into objects met", shut.objects);
    let yard = map(&data, "objects:map:yard");
    let gap = |w: &SimWorld| {
        blocks(
            &data,
            w,
            yard,
            omnis_sim::omnis_core::Level::GROUND,
            3,
            2,
            Facing::South,
        )
    };
    assert_eq!(gap(&world), Some(BlockReason::Object), "the shut gate");
    world.map_state(yard).objects.insert(
        "gate_1".into(),
        ObjectState {
            state: Some("open".into()),
            ..ObjectState::default()
        },
    );
    let open = agree_in(&data, &world);
    assert_eq!(gap(&world), None, "through the open gate");
    assert!(open.objects < shut.objects);
}
