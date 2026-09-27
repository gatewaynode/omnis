//! `apply(&mut World, &Data, Command) -> Result<Vec<Event>, Rejection>` (ARCHITECTURE.md §4.2).

use crate::command::{Command, Rejection};
use crate::event::{BlockReason, Event, MessageKey};
use crate::party::{self, PartyCommand};
use crate::service::{self, ServiceState};
use crate::world::{Known, Mode, World, door_key, layer};
use crate::{INTERACT_MINUTES, MINUTES_PER_DAY, PARTY};
use crate::{casting, combat, dev, effects, encounter, items, visibility};
use alloc::vec::Vec;
use omnis_core::{Direction, Facing, MapId, Position, Rotation};
use omnis_data::Data;

/// Apply one command. A `Rejection` leaves the world unchanged; every `Ok` advances `turn`,
/// appends the events to the log, and ends with what the party now sees.
pub fn apply(world: &mut World, data: &Data, command: Command) -> Result<Vec<Event>, Rejection> {
    let mut events = Vec::new();
    match (&world.mode, &command) {
        (_, Command::Dev(edit)) => dev::apply(world, data, edit, &mut events)?,
        (Mode::Explore, Command::Step(direction)) => step(world, data, *direction, &mut events)?,
        (&Mode::Town(state), Command::Step(direction)) => {
            step_out(world, data, state, *direction, &mut events)?;
        }
        (&Mode::Town(state), Command::Service(command)) => {
            service::apply(world, data, state, *command, &mut events)?;
        }
        (Mode::Explore | Mode::Town(_), Command::Turn(rotation)) => turn(world, *rotation),
        (Mode::Explore, Command::Interact) => interact(world, data, &mut events),
        (Mode::Explore | Mode::Town(_), Command::Party(command))
        | (Mode::Combat(_), Command::Party(command @ PartyCommand::AutoCast { .. })) => {
            party::apply(world, data, command, &mut events)?;
        }
        (
            Mode::Explore | Mode::Town(_),
            Command::Cast {
                caster,
                spell,
                target,
            },
        ) => casting::apply(world, data, *caster, *spell, *target, &mut events)?,
        (Mode::Explore | Mode::Town(_), Command::Item(command)) => {
            items::apply(world, data, *command, &mut events)?;
        }
        (Mode::Encounter(_), Command::Encounter(choice)) => {
            encounter::apply_choice(world, data, *choice, &mut events)?;
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
    effects::prune(world, events);
}

fn world_clock_origin() -> omnis_core::Clock {
    omnis_core::Clock::new(omnis_core::EraId(0))
}

/// A step; when it lands somewhere, the tile's service or encounter may follow.
fn step(
    world: &mut World,
    data: &Data,
    direction: Direction,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let from = world.position;
    let facing = from.facing.toward(direction);
    if !r#move(world, data, direction, events) {
        return Ok(());
    }
    arrive(world, data, from, facing, events)
}

/// A step out of a service: the party leaves only when the step goes somewhere, so a wall
/// keeps it inside. Then the step lands as any other.
fn step_out(
    world: &mut World,
    data: &Data,
    state: ServiceState,
    direction: Direction,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let from = world.position;
    let facing = from.facing.toward(direction);
    let at = events.len();
    if !r#move(world, data, direction, events) {
        return Ok(());
    }
    events.insert(
        at,
        Event::ServiceLeft {
            service: state.service,
        },
    );
    world.mode = Mode::Explore;
    arrive(world, data, from, facing, events)
}

/// Where a step lands: a site takes the party inside with no encounter roll; anywhere else the
/// tile's encounter may follow.
fn arrive(
    world: &mut World,
    data: &Data,
    from: Position,
    facing: Facing,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    if service::enter_here(world, data, events) {
        return Ok(());
    }
    encounter::trigger(world, data, from, facing, events).map_err(Rejection::Rule)
}

/// Where a step lands: the tile, its minutes, and a portal's far end when the tile has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Landing {
    /// The tile stepped onto.
    pub to: Position,
    /// Its terrain's minutes.
    pub minutes: u32,
    /// Where a portal on it leads, when the far end is on a loaded map.
    pub through: Option<Position>,
}

impl Landing {
    /// Where the party ends up.
    pub(crate) fn end(self) -> Position {
        self.through.unwrap_or(self.to)
    }
}

/// Where a step in `direction` would land, or why it would not: walls, doors, terrain, the
/// map's edge. Nothing changes; `r#move` and the queries share it.
pub(crate) fn landing(
    world: &World,
    data: &Data,
    direction: Direction,
) -> Result<Landing, BlockReason> {
    let pos = world.position;
    let facing = pos.facing.toward(direction);
    let map = data.maps.get(&pos.map).ok_or(BlockReason::MapEdge)?;
    let cell = map.cell(pos.x, pos.y).ok_or(BlockReason::MapEdge)?;
    if cell.walls.has(facing) {
        return Err(BlockReason::Wall);
    }
    if cell.doors.has(facing)
        && !world
            .maps
            .get(&pos.map)
            .is_some_and(|s| s.door_open(pos.x, pos.y, facing))
    {
        return Err(BlockReason::ClosedDoor);
    }
    let (x, y, target_cell) = pos
        .neighbour(facing)
        .and_then(|(x, y)| map.cell(x, y).map(|c| (x, y, c)))
        .ok_or(BlockReason::MapEdge)?;
    let terrain = map.terrain(target_cell);
    if !terrain.passable {
        return Err(BlockReason::Impassable);
    }
    let through = map
        .portal_at(x, y)
        .map(|portal| Position {
            map: portal.to_map,
            x: portal.to_x,
            y: portal.to_y,
            facing: portal.to_facing,
        })
        .filter(|dest| {
            data.maps
                .get(&dest.map)
                .is_some_and(|m| m.cell(dest.x, dest.y).is_some())
        });
    Ok(Landing {
        to: pos.at(x, y),
        minutes: terrain.step_minutes,
        through,
    })
}

/// The move itself: the landing, its minutes, the visits, a portal. Whether the party ended up
/// somewhere.
fn r#move(world: &mut World, data: &Data, direction: Direction, events: &mut Vec<Event>) -> bool {
    let from = world.position;
    let landing = match landing(world, data, direction) {
        Ok(landing) => landing,
        Err(reason) => {
            events.push(Event::Blocked { reason });
            return false;
        }
    };
    world.position = landing.to;
    events.push(Event::Moved {
        from,
        to: landing.to,
    });
    advance(world, landing.minutes, events);
    visit(world, data);
    if let Some(dest) = landing.through {
        world.position = dest;
        events.push(Event::Moved {
            from: landing.to,
            to: dest,
        });
        visit(world, data);
    }
    true
}

/// The party steps to `to` (where it came from, facing away): the move, its minutes, the
/// visit. Used by Run before a fight and by flight from one.
pub(crate) fn retreat(world: &mut World, data: &Data, to: Position, events: &mut Vec<Event>) {
    let from = world.position;
    world.position = to;
    events.push(Event::Moved { from, to });
    let minutes = data
        .maps
        .get(&to.map)
        .and_then(|m| m.cell(to.x, to.y).map(|c| m.terrain(c).step_minutes))
        .unwrap_or(1);
    advance(world, minutes, events);
    visit(world, data);
}

fn turn(world: &mut World, rotation: Rotation) {
    world.position = world.position.turned(rotation);
}

/// Go into the service on the party's tile, or else open or close the door ahead.
fn interact(world: &mut World, data: &Data, events: &mut Vec<Event>) {
    if service::enter_here(world, data, events) {
        return;
    }
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
    toggle_door(world, pos.map, pos.x, pos.y, pos.facing, events);
    advance(world, INTERACT_MINUTES, events);
}

/// Open a closed door or close an open one on the facing edge of a tile, with the event.
pub(crate) fn toggle_door(
    world: &mut World,
    map: MapId,
    x: u16,
    y: u16,
    facing: Facing,
    events: &mut Vec<Event>,
) {
    let key = door_key(x, y, facing);
    let state = world.map_state(map);
    let open = if state.open_doors.remove(&key) {
        false
    } else {
        state.open_doors.insert(key);
        true
    };
    events.push(Event::Door {
        map,
        x,
        y,
        facing,
        open,
    });
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
