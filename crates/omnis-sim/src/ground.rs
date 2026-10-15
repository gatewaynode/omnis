//! Items lying on the ground (M8.5 step 2): one pile per cell, placed by the map or laid down by
//! the party. The save keeps only the piles that differ from the map's, an emptied placed pile
//! with no items, so a map whose ground was never touched saves nothing new.

use crate::command::Rejection;
use crate::event::{Event, ItemPlace};
use crate::items::{actor, add_to, from_kit, from_list, out_of_kit, take_from};
use crate::names::id_of;
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, Facing, ItemId, Level, MapId};
use omnis_data::Data;
use omnis_data::limits::MAX_PILE_STACKS;
use omnis_data::placement::Pile;

/// Eighths ahead of the party a pile it lays down lies, when the cell has none.
const DROP_AHEAD: i8 = 2;

/// The pile on a cell now: the saved one, else the map's; `None` when there is none or it is
/// empty.
#[must_use]
pub fn pile_at<'a>(
    world: &'a World,
    data: &'a Data,
    map: MapId,
    cell: (Level, u16, u16),
) -> Option<&'a Pile> {
    let saved = world.maps.get(&map).and_then(|s| s.ground.get(&cell));
    saved
        .or_else(|| data.maps.get(&map).and_then(|m| m.ground.get(&cell)))
        .filter(|p| !p.items.is_empty())
}

/// Lay `count` of a kit's item on the ground of the party's cell.
pub(crate) fn drop(
    world: &mut World,
    data: &Data,
    member: CharacterId,
    item: &str,
    count: u16,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let (map, cell) = here(world)?;
    let own = actor(world, data, member)?;
    let id = from_kit(world, data, own, item, count)?;
    let mut pile = pile_at(world, data, map, cell).cloned().unwrap_or(Pile {
        pos: ahead(world.position.facing),
        items: Vec::new(),
    });
    if pile.items.len() >= MAX_PILE_STACKS && !pile.items.iter().any(|(i, _)| *i == id) {
        return Err(Rejection::PileFull);
    }
    out_of_kit(world, data, own, id, count, events);
    add_to(&mut pile.items, id, count);
    keep(world, data, map, cell, pile);
    events.push(moved(
        data,
        id,
        count,
        ItemPlace::Member(member),
        ItemPlace::Ground,
    ));
    Ok(())
}

/// Pick `count` of an item up from the ground of the party's cell into a member's kit.
pub(crate) fn pick_up(
    world: &mut World,
    data: &Data,
    member: CharacterId,
    item: &str,
    count: u16,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let (map, cell) = here(world)?;
    let own = actor(world, data, member)?;
    let mut pile = pile_at(world, data, map, cell).cloned().unwrap_or_default();
    let id = from_list(data, &pile.items, item, count, |item| {
        Rejection::NotOnGround {
            item: String::from(item),
        }
    })?;
    if take_from(&mut pile.items, id, count).is_err() {
        return Err(Rejection::NotOnGround { item: item.into() });
    }
    keep(world, data, map, cell, pile);
    add_to(&mut world.party.members[own].equipment, id, count);
    events.push(moved(
        data,
        id,
        count,
        ItemPlace::Ground,
        ItemPlace::Member(member),
    ));
    Ok(())
}

/// Lay items on the ground of the party's cell, where a pile already lies or at its centre: the
/// spoils of a fight won there.
pub(crate) fn lay(world: &mut World, data: &Data, items: &[(ItemId, u16)]) {
    if items.is_empty() {
        return;
    }
    let pos = world.position;
    let cell = (pos.level, pos.x, pos.y);
    let mut pile = pile_at(world, data, pos.map, cell)
        .cloned()
        .unwrap_or_default();
    for &(item, count) in items {
        add_to(&mut pile.items, item, count);
    }
    keep(world, data, pos.map, cell, pile);
}

/// The party's map and cell, while exploring.
fn here(world: &World) -> Result<(MapId, (Level, u16, u16)), Rejection> {
    if !matches!(world.mode, Mode::Explore) {
        return Err(Rejection::WrongMode);
    }
    let pos = world.position;
    Ok((pos.map, (pos.level, pos.x, pos.y)))
}

/// Where a new pile lies: a little ahead of the party.
fn ahead(facing: Facing) -> (i8, i8) {
    match facing {
        Facing::North => (0, -DROP_AHEAD),
        Facing::South => (0, DROP_AHEAD),
        Facing::East => (DROP_AHEAD, 0),
        Facing::West => (-DROP_AHEAD, 0),
    }
}

/// Remember a cell's pile, unless it is the map's own or, where the map has none, empty.
fn keep(world: &mut World, data: &Data, map: MapId, cell: (Level, u16, u16), pile: Pile) {
    let placed = data.maps.get(&map).and_then(|m| m.ground.get(&cell));
    let as_placed = match placed {
        Some(p) => *p == pile,
        None => pile.items.is_empty(),
    };
    let ground = &mut world.map_state(map).ground;
    if as_placed {
        ground.remove(&cell);
    } else {
        ground.insert(cell, pile);
    }
}

/// The event of items changing place.
pub(crate) fn moved(
    data: &Data,
    item: ItemId,
    count: u16,
    from: ItemPlace,
    to: ItemPlace,
) -> Event {
    Event::ItemMoved {
        item: id_of(&data.registry.items, item),
        count,
        from,
        to,
    }
}
