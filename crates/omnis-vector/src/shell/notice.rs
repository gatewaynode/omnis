//! What the panel shows: a heading, lines, and choices that become orders
//! (alt-ARCHITECTURE.md §8, §9). Shared by the fight notice and the action menu.

use crate::cinema::Scene;
use bevy::prelude::Component;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, World, apply};

/// What a notice button does.
#[derive(Component, Debug, Clone, PartialEq)]
pub enum Order {
    /// Send a command to the simulation.
    Command(Command),
    /// Start a fresh world and party.
    Restart,
    /// Close the action menu.
    Close,
}

/// One button of the notice.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// The label.
    pub label: String,
    /// What it does.
    pub order: Order,
    /// Why it cannot be chosen now, if it cannot.
    pub blocked: Option<String>,
}

/// The panel's content.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    /// The heading.
    pub title: String,
    /// Lines under the heading: the monsters, one a stack, or what is here.
    pub lines: Vec<String>,
    /// The buttons, in order; the number keys pick them.
    pub choices: Vec<Choice>,
    /// The picture window's scene, above everything else, in a fight.
    pub scene: Option<Scene>,
}

/// Why the simulation would refuse a command now, found on a copy of the world.
pub fn refusal(world: &World, data: &Data, command: &Command) -> Option<String> {
    apply(&mut world.clone(), data, command.clone())
        .err()
        .map(|r| r.to_string())
}

/// A choice for `command`, blocked with the simulation's reason if it would be refused.
pub fn choice(world: &World, data: &Data, label: String, command: Command) -> Choice {
    Choice {
        blocked: refusal(world, data, &command),
        label,
        order: Order::Command(command),
    }
}
