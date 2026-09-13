//! `apply(&mut World, &Data, Command) -> Result<Vec<Event>, Rejection>` (ARCHITECTURE.md §4.2).

use crate::combat;
use crate::command::{Command, Rejection};
use crate::event::{BlockReason, Event, MessageKey};
use crate::party;
use crate::visibility;
use crate::world::{Known, Mode, World, door_key, layer};
use crate::{INTERACT_MINUTES, MINUTES_PER_DAY, PARTY};
use alloc::vec::Vec;
use omnis_core::{Direction, Position, Rotation};
use omnis_data::Data;

/// Apply one command. A `Rejection` leaves the world unchanged; every `Ok` advances `turn`,
/// appends the events to the log, and ends with what the party now sees.
pub fn apply(world: &mut World, data: &Data, command: Command) -> Result<Vec<Event>, Rejection> {
    let mut events = Vec::new();
    match (&world.mode, &command) {
        (Mode::Explore, Command::Step(direction)) => step(world, data, *direction, &mut events),
        (Mode::Explore, Command::Turn(rotation)) => turn(world, *rotation),
        (Mode::Explore, Command::Interact) => interact(world, data, &mut events),
        (Mode::Explore, Command::Party(command)) => {
            party::apply(world, data, command, &mut events)?;
        }
        (Mode::Combat(_), Command::Combat(command)) => {
            combat::apply(world, data, *command, &mut events)?;
        }
        _ => return Err(Rejection::WrongMode),
    }
    look(world, data, &mut events);
    world.turn += 1;
    world.record(&events);
    Ok(events)
}

pub(crate) fn advance(world: &mut World, minutes: u32, events: &mut Vec<Event>) {
    let clock = world.clocks.entry(PARTY).or_insert_with(world_clock_origin);
    let day_rolled = clock.advance(minutes, MINUTES_PER_DAY);
    events.push(Event::TimeAdvanced {
        holder: PARTY,
        minutes,
        day_rolled,
    });
}

fn world_clock_origin() -> omnis_core::Clock {
    omnis_core::Clock::new(omnis_core::EraId(0))
}

fn step(world: &mut World, data: &Data, direction: Direction, events: &mut Vec<Event>) {
    let pos = world.position;
    let facing = pos.facing.toward(direction);
    let Some(map) = data.maps.get(&pos.map) else {
        events.push(Event::Blocked {
            reason: BlockReason::MapEdge,
        });
        return;
    };
    let Some(cell) = map.cell(pos.x, pos.y) else {
        events.push(Event::Blocked {
            reason: BlockReason::MapEdge,
        });
        return;
    };
    if cell.walls.has(facing) {
        events.push(Event::Blocked {
            reason: BlockReason::Wall,
        });
        return;
    }
    if cell.doors.has(facing)
        && !world
            .maps
            .get(&pos.map)
            .is_some_and(|s| s.door_open(pos.x, pos.y, facing))
    {
        events.push(Event::Blocked {
            reason: BlockReason::ClosedDoor,
        });
        return;
    }
    let target = pos
        .neighbour(facing)
        .and_then(|(x, y)| map.cell(x, y).map(|c| (x, y, c)));
    let Some((x, y, target_cell)) = target else {
        events.push(Event::Blocked {
            reason: BlockReason::MapEdge,
        });
        return;
    };
    let terrain = map.terrain(target_cell);
    if !terrain.passable {
        events.push(Event::Blocked {
            reason: BlockReason::Impassable,
        });
        return;
    }
    let to = pos.at(x, y);
    world.position = to;
    events.push(Event::Moved { from: pos, to });
    advance(world, terrain.step_minutes, events);
    visit(world, data);
    if let Some(portal) = map.portal_at(x, y) {
        let dest = Position {
            map: portal.to_map,
            x: portal.to_x,
            y: portal.to_y,
            facing: portal.to_facing,
        };
        if data
            .maps
            .get(&dest.map)
            .is_some_and(|m| m.cell(dest.x, dest.y).is_some())
        {
            world.position = dest;
            events.push(Event::Moved { from: to, to: dest });
            visit(world, data);
        }
    }
}

fn turn(world: &mut World, rotation: Rotation) {
    world.position = world.position.turned(rotation);
}

fn interact(world: &mut World, data: &Data, events: &mut Vec<Event>) {
    let pos = world.position;
    let has_door = data
        .maps
        .get(&pos.map)
        .and_then(|m| m.cell(pos.x, pos.y))
        .is_some_and(|c| c.doors.has(pos.facing));
    if !has_door {
        events.push(Event::Message {
            key: MessageKey::NothingHere,
        });
        return;
    }
    let key = door_key(pos.x, pos.y, pos.facing);
    let state = world.map_state(pos.map);
    let open = if state.open_doors.remove(&key) {
        false
    } else {
        state.open_doors.insert(key);
        true
    };
    events.push(Event::Door {
        map: pos.map,
        x: pos.x,
        y: pos.y,
        facing: pos.facing,
        open,
    });
    advance(world, INTERACT_MINUTES, events);
}

/// Record the party's own tile as visited.
pub(crate) fn visit(world: &mut World, data: &Data) {
    let pos = world.position;
    let Some(cell) = data.maps.get(&pos.map).and_then(|m| m.cell(pos.x, pos.y)) else {
        return;
    };
    let seen_at = world.party_clock().elapsed;
    world.automap.record(
        pos.map,
        pos.x,
        pos.y,
        Known {
            terrain: cell.terrain,
            walls: cell.walls,
            doors: cell.doors,
            layers: layer::VISITED | layer::TERRAIN | layer::STRUCTURE,
            seen_at,
        },
    );
}

/// Perceive the cone, record it, and emit `Visible`.
fn look(world: &mut World, data: &Data, events: &mut Vec<Event>) {
    let tiles = visibility::cone(world, data);
    let map_id = world.position.map;
    let seen_at = world.party_clock().elapsed;
    if let Some(map) = data.maps.get(&map_id) {
        for tile in &tiles {
            if let Some(cell) = map.cell(tile.x, tile.y) {
                world.automap.record(
                    map_id,
                    tile.x,
                    tile.y,
                    Known {
                        terrain: cell.terrain,
                        walls: cell.walls,
                        doors: cell.doors,
                        layers: layer::TERRAIN | layer::STRUCTURE,
                        seen_at,
                    },
                );
            }
        }
    }
    events.push(Event::Visible { tiles });
}
