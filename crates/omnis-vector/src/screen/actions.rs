//! The square's actions (presentation-ARCHITECTURE §6, §10.1): what the party can do on its
//! square, facing the way it faces. Space opens the screen; Shift+Space runs the default action,
//! the first one available, or opens the screen when there is none. An action is listed only
//! when the simulation, tried on a copy of the world, would do something with it, so the
//! mechanics decide what appears; this module only names them. The square's actions are
//! `Interact` (a door on the faced edge of the party's cell) and `Climb` up or down a ladder
//! (M8.5 step 1b), and `Search` (step 2b). Where `Interact` would ask which thing to use
//! (`ChooseVerb`) or refuse the one thing there is, Use opens the verbs screen instead, so the
//! choice or the reason shows and nothing refused is sent.

use super::inventory::{owner, owner_choice, step_owner};
use super::{
    Control, ControlId, Effect, Forms, Game, Key, Panel, Payload, Row, Screen, ScreenId, Span,
};
use crate::labels;
use crate::sim::SimWorld;
use crate::targeting::Aim;
use omnis_sim::omnis_core::{Facing, Vertical};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Chosen, Command, Event, ItemPlace, MessageKey, Rejection};

/// What an action does.
#[derive(Debug, Clone, PartialEq)]
pub enum Does {
    /// Sends a command.
    Command(Command),
    /// Opens the verbs screen: the engine would ask which thing to use, or refuse the one there is.
    Choose,
    /// Opens the loot window on a place within reach: the ground, or an open container.
    Loot(ItemPlace),
    /// Opens the target step for a weapon attack at a group in sight (the opening strike).
    Attack,
}

/// One action the square offers.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    /// Its label.
    pub label: String,
    /// What it does.
    pub does: Does,
}

/// Whether the faced edge of the party's cell holds a door, as the simulation's `Interact`
/// looks for it, and whether that door is open.
fn faced_door(world: &SimWorld, data: &Data) -> Option<bool> {
    let p = world.position(data)?;
    let map = data.maps.get(&p.map)?;
    if !map.cell(p.level, p.x, p.y)?.doors.has(p.facing) {
        return None;
    }
    // A broken door is nothing to open or shut.
    if world.door_broken(p.map, p.level, p.x, p.y, p.facing) {
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

/// "Approach: Pack rat x2" for a Use that does something with a group standing in reach (M8.5
/// step 2c): with anything else in reach Use would ask which, so the group is the one thing. By
/// its front monster's name and how many stand there.
fn approached(world: &SimWorld, data: &Data) -> Option<String> {
    let objects = world.objects(data);
    let being = objects.beings.first()?;
    let name = labels::count_label(labels::monster(data, &being.monster), being.count);
    Some(format!("Approach: {name}"))
}

const fn facing_name(facing: Facing) -> &'static str {
    match facing {
        Facing::North => "north",
        Facing::East => "east",
        Facing::South => "south",
        Facing::West => "west",
    }
}

/// The actions the simulation would accept here, in menu order: the faced door, site or object
/// (or the verbs, to choose among several), the ground within reach when something lies on it,
/// a ladder up and down, then Search.
#[must_use]
pub fn available(world: &SimWorld, data: &Data) -> Vec<Offer> {
    let mut offers = Vec::new();
    let interact = Command::Interact;
    if does_something(world, data, &interact) {
        let label = match faced_door(world, data) {
            Some(false) => "Open the door".to_owned(),
            Some(true) => "Close the door".to_owned(),
            None => approached(world, data).unwrap_or_else(|| "Use".to_owned()),
        };
        offers.push(Offer {
            label,
            does: Does::Command(interact),
        });
    } else if matches!(
        world.trial(data, &interact),
        Err(Rejection::ChooseVerb { .. } | Rejection::VerbRefused { .. })
    ) {
        offers.push(Offer {
            label: "Use...".to_owned(),
            does: Does::Choose,
        });
    }
    if !world.objects(data).ground.is_empty() {
        offers.push(Offer {
            label: "Pick up...".to_owned(),
            does: Does::Loot(ItemPlace::Ground),
        });
    }
    // An open container in reach that holds something: the way back to what it opened on.
    for object in &world.objects(data).objects {
        if object.contents.as_ref().is_some_and(|c| !c.is_empty()) {
            offers.push(Offer {
                label: format!("Take from {}...", labels::text(data, &object.name)),
                does: Does::Loot(ItemPlace::Object(object.object.clone())),
            });
        }
    }
    // A group in sight may be attacked from here: who attacks and at what is asked next.
    let anyone = world.party(data).members.first().map(|m| m.member);
    if anyone.is_some_and(|id| !world.targets(data, id, &Chosen::Attack).is_empty()) {
        offers.push(Offer {
            label: "Attack...".to_owned(),
            does: Does::Attack,
        });
    }
    let search = Command::Search;
    let mut commands: Vec<(Command, &str)> =
        [(Vertical::Up, "Climb up"), (Vertical::Down, "Climb down")]
            .map(|(way, label)| (Command::Climb(way), label))
            .into();
    commands.push((search, "Search here"));
    for (command, label) in commands {
        if does_something(world, data, &command) {
            offers.push(Offer {
                label: label.to_owned(),
                does: Does::Command(command),
            });
        }
    }
    offers
}

/// What Shift+Space does, if anything: the first available action, never a search (it spends
/// ten minutes, so it is asked for by name).
#[must_use]
pub fn default_action(world: &SimWorld, data: &Data) -> Option<Does> {
    available(world, data)
        .into_iter()
        .map(|o| o.does)
        .find(|d| *d != Does::Command(Command::Search) && *d != Does::Attack)
}

/// The screen: the square, its actions, and Close last.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    if let Some(aim) = forms.aim.as_ref().filter(|_| game.views.combat.is_none()) {
        // The attack's target step, with who attacks to choose beside Back.
        let mut screen = crate::targeting::content(data, game, aim, ScreenId::Actions);
        let back = screen.panels.first_mut().and_then(|p| p.rows.first_mut());
        if let Some(row) = back {
            row.controls
                .push(owner_choice(game, aim.member, "Attacker"));
        }
        return screen;
    }
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

/// An action sends its command and closes the screen, or opens the verbs over it; Close only
/// closes it.
#[must_use]
pub fn apply(
    data: &Data,
    game: Game,
    forms: &mut Forms,
    control: ControlId,
    payload: &Payload,
) -> Effect {
    match control {
        ControlId::Back if forms.aim.is_some() => {
            forms.aim = None;
            Effect::None
        }
        ControlId::Owner => {
            step_owner(game, forms, payload);
            let now = owner(game, forms).map(|m| m.member);
            if let (Some(aim), Some(member)) = (forms.aim.as_mut(), now) {
                aim.member = member;
            }
            Effect::None
        }
        ControlId::Target(at) => {
            let command = forms.aim.take().and_then(|aim| aim.chosen(data, game, at));
            command.map_or(Effect::None, |command| Effect::Command {
                command,
                close: true,
            })
        }
        ControlId::Action(i) => available(game.world, data)
            .into_iter()
            .nth(usize::from(i))
            .map_or(Effect::None, |o| match o.does {
                Does::Command(command) => Effect::Command {
                    command,
                    close: true,
                },
                Does::Choose => Effect::Open(ScreenId::Verbs),
                Does::Loot(from) => Effect::Loot(from),
                Does::Attack => {
                    forms.aim = owner(game, forms).map(|m| Aim::attack(m.member));
                    Effect::None
                }
            }),
        _ => Effect::Back,
    }
}
