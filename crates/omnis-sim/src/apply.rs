//! `apply(&mut World, &Data, Command) -> Result<Vec<Event>, Rejection>` (ARCHITECTURE.md §4.2).

use crate::command::{Command, Rejection};
use crate::event::{BlockReason, Event, MessageKey};
use crate::names::{Place, id_of};
use crate::party::{self, PartyCommand};
use crate::service::{self, ServiceState};
use crate::time;
use crate::world::{Known, Mode, World, layer};
use crate::{INTERACT_MINUTES, PARTY};
use crate::{casting, combat, dev, effects, encounter, items, rest, visibility};
use alloc::vec::Vec;
use omnis_core::{Direction, Facing, Position, Rotation, Vertical};
use omnis_data::{Data, Join, MapData};

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
            service::apply(world, data, state, command, &mut events)?;
        }
        (Mode::Explore | Mode::Town(_), Command::Turn(rotation)) => turn(world, *rotation),
        (Mode::Explore, Command::Interact) => interact(world, data, &mut events),
        (Mode::Explore, Command::Climb(way)) => climb(world, data, *way, &mut events)?,
        (Mode::Explore | Mode::Town(_), Command::Party(command))
        | (Mode::Combat(_), Command::Party(command @ PartyCommand::Tactics(_)))
            if !matches!(world.mode, Mode::Combat(_))
                || matches!(command, PartyCommand::Tactics(c) if c.in_fight()) =>
        {
            party::apply(world, data, command, &mut events)?;
        }
        (
            Mode::Explore | Mode::Town(_),
            Command::Cast {
                caster,
                spell,
                target,
            },
        ) => casting::apply(world, data, *caster, spell, *target, &mut events)?,
        (Mode::Explore | Mode::Town(_), Command::Item(command)) => {
            items::apply(world, data, command, &mut events)?;
        }
        (Mode::Explore, Command::Rest(command)) => {
            rest::apply(world, data, command, &mut events)?;
        }
        (Mode::Encounter(_), Command::Encounter(choice)) => {
            encounter::apply_choice(world, data, *choice, &mut events)?;
        }
        (Mode::Combat(_), Command::Combat(command)) => {
            combat::apply(world, data, command.clone(), &mut events)?;
        }
        _ => return Err(Rejection::WrongMode),
    }
    look(world, data, &mut events);
    world.turn += 1;
    world.record(&events);
    Ok(events)
}

/// The party lives `minutes`: its age, its shared time at the company of where it is, and
/// its date; a day rolls on the date.
pub(crate) fn advance(world: &mut World, data: &Data, minutes: u32, events: &mut Vec<Event>) {
    let calendar = data.calendar();
    let day = calendar.day(world.party_time.date);
    let clock = world.clocks.entry(PARTY).or_insert_with(world_clock_origin);
    clock.advance(minutes, calendar.minutes_per_day);
    time::live(world, data, minutes);
    let day_rolled = calendar.day(world.party_time.date) != day;
    events.push(Event::TimeAdvanced {
        holder: crate::time_view::holder_name(PARTY, data),
        minutes,
        day_rolled,
    });
    effects::prune(world, data, events);
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
            service: id_of(&data.registry.services, state.service),
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

/// Where a step lands: the tile, its minutes, and the far end of a portal or a join (a stair, a
/// hole) when the tile has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Landing {
    /// The tile stepped onto.
    pub to: Position,
    /// Its terrain's minutes.
    pub minutes: u32,
    /// Where a portal or a join on it leads, when the far end is ground on a loaded map. Only a
    /// step fires it, so the party arriving there does not go on.
    pub through: Option<Position>,
}

impl Landing {
    /// Where the party ends up.
    pub(crate) fn end(self) -> Position {
        self.through.unwrap_or(self.to)
    }
}

/// Where a step in `direction` would land, or why it would not: walls, doors, terrain, the
/// map's edge, a cliff. Nothing changes; `r#move` and the queries share it.
pub(crate) fn landing(
    world: &World,
    data: &Data,
    direction: Direction,
) -> Result<Landing, BlockReason> {
    let pos = world.position;
    let facing = pos.facing.toward(direction);
    let map = data.maps.get(&pos.map).ok_or(BlockReason::MapEdge)?;
    let cell = map
        .cell(pos.level, pos.x, pos.y)
        .ok_or(BlockReason::MapEdge)?;
    if cell.walls.has(facing) {
        return Err(BlockReason::Wall);
    }
    if cell.doors.has(facing)
        && !world
            .maps
            .get(&pos.map)
            .is_some_and(|s| s.door_open(pos.level, pos.x, pos.y, facing))
    {
        return Err(BlockReason::ClosedDoor);
    }
    let (x, y) = pos
        .neighbour(facing)
        .filter(|&(x, y)| map.inside(x, y))
        .ok_or(BlockReason::MapEdge)?;
    let target_cell = map.cell(pos.level, x, y).ok_or(BlockReason::NoGround)?;
    let terrain = map.terrain(target_cell);
    if !terrain.passable {
        return Err(BlockReason::Impassable);
    }
    if cell.cliffs.has(facing) {
        return Err(BlockReason::Cliff);
    }
    let through = map
        .portal_at(pos.level, x, y)
        .map(|portal| Position {
            map: portal.to_map,
            level: portal.to_level,
            x: portal.to_x,
            y: portal.to_y,
            facing: portal.to_facing,
        })
        .or_else(|| joined(map, pos.at(x, y), terrain.join?, facing))
        .filter(|dest| {
            data.maps
                .get(&dest.map)
                .is_some_and(|m| m.cell(dest.level, dest.x, dest.y).is_some())
        });
    Ok(Landing {
        to: pos.at(x, y),
        minutes: terrain.step_minutes,
        through,
    })
}

/// Where a stair or a hole at `at` leads, for a step onto it moving `heading`: the same tile of
/// the next level up or down, the facing kept. A stair up is taken only from its foot (moving the
/// way it rises), a stair down only from its head (moving against it); from any other side the
/// step lands on the stair as on a floor. A ladder leads nowhere by a step; it is climbed.
fn joined(map: &MapData, at: Position, join: Join, heading: Facing) -> Option<Position> {
    let rise = || map.rise(at.level, at.x, at.y);
    let up = match join {
        Join::Up if rise() == Some(heading) => true,
        Join::Down if rise() == Some(heading.opposite()) => false,
        Join::Hole => false,
        Join::Up | Join::Down | Join::Ladder => return None,
    };
    Some(Position {
        level: map.beyond(at.level, up)?,
        ..at
    })
}

/// Climb from the party's tile to the same tile of the next level `way`: a ladder at either end
/// joins them (the top of a ladder stands on the floor above it), and the far end is ground the
/// party may enter. It costs that terrain's minutes.
fn climb(
    world: &mut World,
    data: &Data,
    way: Vertical,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let from = world.position;
    let to = climb_to(data, from, way).ok_or(Rejection::NoLadder)?;
    let minutes = data
        .maps
        .get(&to.map)
        .and_then(|m| {
            m.cell(to.level, to.x, to.y)
                .map(|c| m.terrain(c).step_minutes)
        })
        .unwrap_or(1);
    world.position = to;
    events.push(Event::Moved {
        from: Place::of(from, data),
        to: Place::of(to, data),
    });
    advance(world, data, minutes, events);
    visit(world, data);
    arrive(world, data, from, from.facing, events)
}

/// Where a climb `way` from `from` lands, if a ladder joins the two tiles and the far one can be
/// entered. Nothing changes; the queries share it.
pub(crate) fn climb_to(data: &Data, from: Position, way: Vertical) -> Option<Position> {
    let map = data.maps.get(&from.map)?;
    let level = map.beyond(from.level, way == Vertical::Up)?;
    let to = Position { level, ..from };
    let here = map.cell(from.level, from.x, from.y)?;
    let there = map.cell(to.level, to.x, to.y)?;
    let ladder = |cell| map.terrain(cell).join == Some(Join::Ladder);
    (map.terrain(there).passable && (ladder(here) || ladder(there))).then_some(to)
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
        from: Place::of(from, data),
        to: Place::of(landing.to, data),
    });
    advance(world, data, landing.minutes, events);
    visit(world, data);
    if let Some(dest) = landing.through {
        world.position = dest;
        events.push(Event::Moved {
            from: Place::of(landing.to, data),
            to: Place::of(dest, data),
        });
        visit(world, data);
        know_portals_beside(world, data);
        time::moved(world, data, from.map, events);
    }
    true
}

/// The party steps to `to` (where it came from, facing away): the move, its minutes, the
/// visit. Used by Run before a fight and by flight from one.
pub(crate) fn retreat(world: &mut World, data: &Data, to: Position, events: &mut Vec<Event>) {
    let from = world.position;
    world.position = to;
    events.push(Event::Moved {
        from: Place::of(from, data),
        to: Place::of(to, data),
    });
    let minutes = data
        .maps
        .get(&to.map)
        .and_then(|m| {
            m.cell(to.level, to.x, to.y)
                .map(|c| m.terrain(c).step_minutes)
        })
        .unwrap_or(1);
    advance(world, data, minutes, events);
    visit(world, data);
    time::moved(world, data, from.map, events);
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
        .and_then(|m| m.cell(pos.level, pos.x, pos.y))
        .is_some_and(|c| c.doors.has(pos.facing));
    if !has_door {
        events.push(Event::Message {
            key: MessageKey::NothingHere,
        });
        return;
    }
    toggle_door(world, data, pos, events);
    advance(world, data, INTERACT_MINUTES, events);
}

/// Open a closed door or close an open one on the edge of the tile `at` faces, with the event.
pub(crate) fn toggle_door(world: &mut World, data: &Data, at: Position, events: &mut Vec<Event>) {
    let Position {
        map,
        level,
        x,
        y,
        facing,
    } = at;
    let open = world.map_state(map).toggle_door(level, x, y, facing);
    events.push(Event::Door {
        map: id_of(&data.registry.maps, map),
        level,
        x,
        y,
        facing,
        open,
    });
}

/// Record the party's own tile as visited.
pub(crate) fn visit(world: &mut World, data: &Data) {
    let pos = world.position;
    let Some(cell) = data
        .maps
        .get(&pos.map)
        .and_then(|m| m.cell(pos.level, pos.x, pos.y))
    else {
        return;
    };
    let seen_at = world.party_clock().elapsed;
    world.automap.record(
        pos.map,
        pos.level,
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

/// Record the portals beside the party's tile, north, east, south and west, as seen: a party
/// that has just come through a portal knows the way back, even with its back to it (owner,
/// 2026-09-27, TODO 3c).
fn know_portals_beside(world: &mut World, data: &Data) {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return;
    };
    let seen_at = world.party_clock().elapsed;
    for facing in Facing::ALL {
        let Some((x, y)) = pos.neighbour(facing) else {
            continue;
        };
        let Some(cell) = map
            .cell(pos.level, x, y)
            .filter(|_| map.portal_at(pos.level, x, y).is_some())
        else {
            continue;
        };
        world.automap.record(
            pos.map,
            pos.level,
            x,
            y,
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

/// Perceive the cone, record it, and emit `Visible`.
fn look(world: &mut World, data: &Data, events: &mut Vec<Event>) {
    let tiles = visibility::cone(world, data);
    let (map_id, level) = (world.position.map, world.position.level);
    let seen_at = world.party_clock().elapsed;
    if let Some(map) = data.maps.get(&map_id) {
        for tile in &tiles {
            if let Some(cell) = map.cell(level, tile.x, tile.y) {
                world.automap.record(
                    map_id,
                    level,
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
