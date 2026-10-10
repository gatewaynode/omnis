//! The action menu's content (presentation-ARCHITECTURE.md §8): what the party can do on its square,
//! facing the way it faces. Space opens the menu; Shift+Space runs the default action, the first
//! one available, or opens the menu when there is none. An action is listed only when the
//! simulation, tried on a copy of the world, would do something with it, so the mechanics decide
//! what appears; this module only names them. Today the simulation's one square action is
//! `Interact` (a door on the faced edge of the party's cell).

use super::notice::{Choice, Notice, Order};
use crate::sim::SimWorld;
use bevy::prelude::Resource;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Event, MessageKey};

/// Whether the action menu is open.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Menu {
    /// Open.
    pub open: bool,
}

/// Whether the faced edge of the party's cell holds a door, as the simulation's `Interact`
/// looks for it, and whether that door is open.
fn faced_door(world: &SimWorld, data: &Data) -> Option<bool> {
    let p = world.position(data)?;
    let map = data.maps.get(&p.map)?;
    if !map.cell(p.x, p.y)?.doors.has(p.facing) {
        return None;
    }
    Some(world.door_open(p.map, p.x, p.y, p.facing))
}

/// Whether the simulation would do something with `command` here: it accepts it, and does not
/// answer "nothing here" (every accepted command also reports what the party sees, so the
/// events alone are never empty).
fn does_something(world: &SimWorld, data: &Data, command: &Command) -> bool {
    world.trial(data, command).is_ok_and(|events| {
        !events.iter().any(|e| {
            matches!(
                e,
                Event::Message {
                    key: MessageKey::NothingHere
                }
            )
        })
    })
}

fn facing_name(facing: Facing) -> &'static str {
    match facing {
        Facing::North => "north",
        Facing::East => "east",
        Facing::South => "south",
        Facing::West => "west",
    }
}

/// The actions the simulation would accept here, in menu order.
#[must_use]
pub fn available(world: &SimWorld, data: &Data) -> Vec<Choice> {
    let interact = Command::Interact;
    if !does_something(world, data, &interact) {
        return Vec::new();
    }
    let label = match faced_door(world, data) {
        Some(false) => "Open the door",
        Some(true) => "Close the door",
        None => "Use",
    };
    vec![Choice {
        label: label.to_owned(),
        order: Order::Command(interact),
        blocked: None,
    }]
}

/// The command Shift+Space runs, if there is one: the first available action.
#[must_use]
pub fn default_action(world: &SimWorld, data: &Data) -> Option<Command> {
    available(world, data)
        .into_iter()
        .find_map(|c| match c.order {
            Order::Command(command) => Some(command),
            _ => None,
        })
}

/// The menu: the square, the actions, and Close last.
#[must_use]
pub fn menu(world: &SimWorld, data: &Data) -> Notice {
    let p = world.here(data).position;
    let mut choices = available(world, data);
    let lines = if choices.is_empty() {
        vec!["Nothing to do here".to_owned()]
    } else {
        Vec::new()
    };
    choices.push(Choice {
        label: "Close".to_owned(),
        order: Order::Close,
        blocked: None,
    });
    Notice {
        title: format!("Here: ({}, {}) facing {}", p.x, p.y, facing_name(p.facing)),
        lines,
        choices,
    }
}
