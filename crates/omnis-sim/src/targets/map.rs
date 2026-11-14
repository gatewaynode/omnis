//! The things of the map as the range rule sees them (`tasks/plans/targeting.md` step 6): a
//! door or an object of the party's level, how far it is as a king moves, whether the line to
//! it is clear, and whether the party can touch it. Sight is asked in every direction from the
//! party's tile, not only the way it faces. What the party neither sees nor knows of is not
//! there: a hidden object not found, a thing on an unexplored tile behind a wall.

use super::Sight;
use crate::combat::cast::Target;
use crate::command::Rejection;
use crate::objects::{found_in, reachable, removed_in};
use crate::visibility;
use crate::world::{World, door_key};
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use omnis_core::Facing;
use omnis_data::placement::ResolvedPlacement;
use omnis_data::{Data, Range, Targeting};

/// Where a thing of the map stands to the party.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Spot {
    /// Tiles from the party, as a king moves.
    pub distance: u16,
    /// Whether the party sees it now.
    pub sight: Sight,
    /// Whether it is within the party's reach: what `Use` reaches.
    pub touch: bool,
}

fn distance(world: &World, (x, y): (u16, u16)) -> u16 {
    let pos = world.position;
    pos.x.abs_diff(x).max(pos.y.abs_diff(y))
}

/// Whether the party sees the tile: within its sight's depth, by a clear line.
fn sees(world: &World, data: &Data, tile: (u16, u16)) -> bool {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return false;
    };
    let state = world.maps.get(&pos.map);
    let from = (pos.x, pos.y);
    distance(world, tile) <= u16::from(visibility::depth(world, data, map))
        && visibility::line_of_sight(map, &data.objects, state, pos.level, from, tile)
}

fn sight(clear: bool) -> Sight {
    if clear { Sight::Clear } else { Sight::Obscured }
}

/// The door on the `facing` edge of `(x, y)` of the party's level, when there is one the party
/// sees or has on its map. The nearer of the edge's two tiles gives the distance, and seeing
/// either is seeing the door.
pub(crate) fn door(world: &World, data: &Data, x: u16, y: u16, facing: Facing) -> Option<Spot> {
    let pos = world.position;
    let map = data.maps.get(&pos.map)?;
    if !map.cell(pos.level, x, y)?.doors.has(facing) {
        return None;
    }
    let far = pos
        .at(x, y)
        .neighbour(facing)
        .map(|t| (t, facing.opposite()));
    let sides: Vec<((u16, u16), Facing)> = [Some(((x, y), facing)), far]
        .into_iter()
        .flatten()
        .filter(|((x, y), _)| map.cell(pos.level, *x, *y).is_some())
        .collect();
    let clear = sides.iter().any(|(tile, _)| sees(world, data, *tile));
    let mapped = world.automap.tiles(pos.map, pos.level);
    let known = sides.iter().any(|(tile, edge)| {
        mapped
            .and_then(|m| m.get(tile))
            .is_some_and(|k| k.doors.has(*edge))
    });
    (clear || known).then(|| Spot {
        distance: sides
            .iter()
            .map(|(t, _)| distance(world, *t))
            .min()
            .unwrap_or(0),
        sight: sight(clear),
        touch: door_key(x, y, facing) == door_key(pos.x, pos.y, pos.facing),
    })
}

/// The object `id` of the party's level, when it stands there still, is not hidden from the
/// party, and the party sees it or has its tile on its map.
pub(crate) fn object<'d>(
    world: &World,
    data: &'d Data,
    id: &str,
) -> Option<(&'d ResolvedPlacement, Spot)> {
    let pos = world.position;
    let map = data.maps.get(&pos.map)?;
    let placement = map
        .objects
        .iter()
        .find(|p| p.id == id && p.level == pos.level)?;
    if removed_in(world.maps.get(&pos.map), placement)
        || !found_in(&world.known, pos.map, placement)
    {
        return None;
    }
    let tile = (placement.x, placement.y);
    let clear = sees(world, data, tile);
    let mapped = world.automap.tiles(pos.map, pos.level);
    let known = mapped.is_some_and(|m| m.contains_key(&tile));
    (clear || known).then(|| {
        let spot = Spot {
            distance: distance(world, tile),
            sight: sight(clear),
            touch: reachable(world, data).any(|p| p.id == id),
        };
        (placement, spot)
    })
}

/// How far `targeting` looks for things of the map: nothing its range cannot reach is farther.
fn radius(world: &World, data: &Data, targeting: Targeting) -> Option<u16> {
    let map = data.maps.get(&world.position.map)?;
    match targeting.range {
        Range::Own | Range::Link => None,
        Range::Touch => Some(1),
        Range::Tiles(n) => Some(u16::from(n)),
        Range::Sight => Some(u16::from(visibility::depth(world, data, map))),
    }
}

/// Whether a thing standing at `spot` is within what `targeting` reaches.
pub(crate) fn within(
    world: &World,
    data: &Data,
    targeting: Targeting,
    spot: Spot,
) -> Result<(), Rejection> {
    let near = match targeting.range {
        Range::Own | Range::Link => false,
        Range::Touch => spot.touch,
        Range::Tiles(_) | Range::Sight => {
            radius(world, data, targeting).is_some_and(|r| spot.distance <= r)
        }
    };
    if !near {
        return Err(Rejection::BeyondRange);
    }
    if targeting.needs_sight() && spot.sight == Sight::Obscured {
        return Err(Rejection::OutOfSight);
    }
    Ok(())
}

/// The doors and objects `targeting` reaches or would reach with a clear line, nearest first,
/// doors before objects at one distance.
pub(crate) fn things(world: &World, data: &Data, targeting: Targeting) -> Vec<(Target, Spot)> {
    let pos = world.position;
    let (Some(map), Some(radius)) = (data.maps.get(&pos.map), radius(world, data, targeting))
    else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut doors = BTreeSet::new();
    let span = |at: u16| at.saturating_sub(radius)..=at.saturating_add(radius);
    for y in span(pos.y) {
        for x in span(pos.x) {
            let Some(cell) = map.cell(pos.level, x, y) else {
                continue;
            };
            for facing in Facing::ALL {
                if !cell.doors.has(facing) || !doors.insert(door_key(x, y, facing)) {
                    continue;
                }
                let (x, y, facing) = door_key(x, y, facing);
                if let Some(spot) = door(world, data, x, y, facing) {
                    found.push((Target::Door { x, y, facing }, spot));
                }
            }
        }
    }
    for placement in map.objects.iter().filter(|p| p.level == pos.level) {
        if let Some((_, spot)) = object(world, data, &placement.id) {
            let object = placement.id.clone();
            found.push((Target::Object { object }, spot));
        }
    }
    found.retain(|(_, spot)| {
        !matches!(
            within(world, data, targeting, *spot),
            Err(Rejection::BeyondRange)
        )
    });
    found.sort_by(|a, b| (a.1.distance, &a.0).cmp(&(b.1.distance, &b.0)));
    found
}
