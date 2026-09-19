//! Spells on the world rather than a creature: mage hand opens or closes the first door
//! straight ahead, the crawler's one object.

use crate::apply::toggle_door;
use crate::event::{Event, MessageKey};
use crate::visibility::{edge_open, project};
use crate::world::World;
use alloc::vec::Vec;
use omnis_data::Data;

/// Toggle the first door on the facing edge of a tile within `range` straight ahead; a wall or
/// an opaque tile stops the hand, and nothing found is a message.
pub(crate) fn open_door_ahead(world: &mut World, data: &Data, range: u8, events: &mut Vec<Event>) {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return;
    };
    let state = world.maps.get(&pos.map);
    for k in 0..range {
        let Some((x, y)) = project(pos, k, 0) else {
            break;
        };
        let Some(cell) = map.cell(x, y) else {
            break;
        };
        if k > 0 && map.terrain(cell).opaque {
            break;
        }
        if cell.doors.has(pos.facing) {
            toggle_door(world, pos.map, x, y, pos.facing, events);
            return;
        }
        if !edge_open(map, state, x, y, pos.facing) {
            break;
        }
    }
    events.push(Event::Message {
        key: MessageKey::NothingHere,
    });
}
