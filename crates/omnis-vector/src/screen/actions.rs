//! The square's actions (presentation-ARCHITECTURE §6, §10.1): what the party can do on its
//! square, facing the way it faces. Space opens the screen; Shift+Space runs the default action,
//! the first one available, or opens the screen when there is none. An action is listed only
//! when the simulation, tried on a copy of the world, would do something with it, so the
//! mechanics decide what appears; this module only names them. The square's actions are
//! `Interact` (a door on the faced edge of the party's cell) and `Climb` up or down a ladder
//! (M8.5 step 1b).

use super::{Control, ControlId, Effect, Game, Key, Panel, Row, Screen, ScreenId, Span};
use crate::sim::SimWorld;
use omnis_sim::omnis_core::{Facing, Vertical};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Event, MessageKey};

/// One action the square offers.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    /// Its label.
    pub label: String,
    /// What it sends.
    pub command: Command,
}

/// Whether the faced edge of the party's cell holds a door, as the simulation's `Interact`
/// looks for it, and whether that door is open.
fn faced_door(world: &SimWorld, data: &Data) -> Option<bool> {
    let p = world.position(data)?;
    let map = data.maps.get(&p.map)?;
    if !map.cell(p.level, p.x, p.y)?.doors.has(p.facing) {
        return None;
    }
    Some(world.door_open(p.map, p.level, p.x, p.y, p.facing))
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

const fn facing_name(facing: Facing) -> &'static str {
    match facing {
        Facing::North => "north",
        Facing::East => "east",
        Facing::South => "south",
        Facing::West => "west",
    }
}

/// The actions the simulation would accept here, in menu order: the faced door or site, then
/// a ladder up and down.
#[must_use]
pub fn available(world: &SimWorld, data: &Data) -> Vec<Offer> {
    let mut offers = Vec::new();
    let interact = Command::Interact;
    if does_something(world, data, &interact) {
        let label = match faced_door(world, data) {
            Some(false) => "Open the door",
            Some(true) => "Close the door",
            None => "Use",
        };
        offers.push(Offer {
            label: label.to_owned(),
            command: interact,
        });
    }
    for (way, label) in [(Vertical::Up, "Climb up"), (Vertical::Down, "Climb down")] {
        let climb = Command::Climb(way);
        if does_something(world, data, &climb) {
            offers.push(Offer {
                label: label.to_owned(),
                command: climb,
            });
        }
    }
    offers
}

/// The command Shift+Space runs, if there is one: the first available action.
#[must_use]
pub fn default_action(world: &SimWorld, data: &Data) -> Option<Command> {
    available(world, data).into_iter().next().map(|o| o.command)
}

/// The screen: the square, its actions, and Close last.
#[must_use]
pub fn content(data: &Data, game: Game) -> Screen {
    let world = game.world;
    let p = &game.views.here.position;
    let offers = available(world, data);
    let mut rows = Vec::new();
    if offers.is_empty() {
        rows.push(Row::text(Span::body("Nothing to do here")));
    }
    let mut controls: Vec<Control> = (0u8..)
        .zip(offers)
        .map(|(i, o)| Control::button(ControlId::Action(i), o.label))
        .collect();
    controls.push(Control::button(ControlId::Close, "Close"));
    let keys = (1u8..=9)
        .zip(controls.iter().map(|c| c.id))
        .map(|(n, id)| (Key::Digit(n), id))
        .collect();
    rows.push(Row::controls(controls));
    Screen {
        id: ScreenId::Actions,
        title: Span::title(format!(
            "Here: ({}, {}) facing {}",
            p.x,
            p.y,
            facing_name(p.facing)
        )),
        panels: vec![Panel { rows }],
        keys,
        message: None,
    }
}

/// An action sends its command and closes the screen; Close only closes it.
#[must_use]
pub fn apply(data: &Data, game: Game, control: ControlId) -> Effect {
    match control {
        ControlId::Action(i) => available(game.world, data)
            .into_iter()
            .nth(usize::from(i))
            .map_or(Effect::None, |o| Effect::Command {
                command: o.command,
                close: true,
            }),
        _ => Effect::Back,
    }
}
