//! The fight notice's content (alt-ARCHITECTURE.md §9): while monsters are met the 3D view stays
//! up and the panel offers the encounter's choices, so the viewer never dead-ends. In a fight it
//! offers a placeholder Attack, Dodge and Flee until the combat screen (Phase B) replaces it;
//! when the party falls it offers a restart. `shell/panel.rs` shows it.

use super::notice::{Choice, Notice, Order, choice};
use crate::cinema::opening;
use omnis_sim::combat::state::can_fight;
use omnis_sim::event::ActorRef;
use omnis_sim::omnis_data::Data;
use omnis_sim::{
    CombatCommand, CombatView, Command, EncounterChoice, ModeKind, World, bribe_cost, combat_view,
};

/// Whether the party can no longer fight: every member is down, dead or incapacitated.
#[must_use]
pub fn fallen(world: &World, data: &Data) -> bool {
    !world.party.members.iter().any(|m| can_fight(m, data))
}

/// The notice for the world as it stands, or `None` while exploring with a party that stands.
#[must_use]
pub fn notice(world: &World, data: &Data) -> Option<Notice> {
    let Some(view) = combat_view(world, data) else {
        return fallen(world, data).then(|| Notice {
            title: "The party has fallen".to_owned(),
            lines: Vec::new(),
            choices: vec![Choice {
                label: "Start again".to_owned(),
                order: Order::Restart,
                blocked: None,
            }],
            scene: None,
        });
    };
    let lines = view
        .stacks
        .iter()
        .filter(|s| s.alive)
        .map(|s| {
            let hp: Vec<String> = s.hp.iter().map(ToString::to_string).collect();
            format!(
                "{} x{}  hp {}{}",
                data.label("en", &s.name),
                s.hp.len(),
                hp.join(" "),
                if s.front { "  (front)" } else { "" }
            )
        })
        .collect();
    let (title, choices) = if view.phase == ModeKind::Encounter {
        encounter_choices(world, data, &view)
    } else {
        combat_choices(world, data, &view)
    };
    Some(Notice {
        title,
        lines,
        choices,
        scene: opening(world),
    })
}

/// The heading and choices before a fight: the four encounter choices.
fn encounter_choices(world: &World, data: &Data, view: &CombatView) -> (String, Vec<Choice>) {
    let bribe = bribe_cost(world, data).map_or_else(
        |_| "Bribe".to_owned(),
        |cost| format!("Bribe ({cost} gold)"),
    );
    let options = [
        ("Fight".to_owned(), EncounterChoice::Attack),
        (bribe, EncounterChoice::Bribe),
        ("Hide".to_owned(), EncounterChoice::Hide),
        ("Run".to_owned(), EncounterChoice::Run),
    ];
    (
        format!("Monsters ahead ({:?})", view.disposition),
        options
            .into_iter()
            .map(|(label, c)| choice(world, data, label, Command::Encounter(c)))
            .collect(),
    )
}

/// The heading and choices in a fight, the placeholder until Phase B: attack a stack the
/// acting member reaches, dodge, or flee.
fn combat_choices(world: &World, data: &Data, view: &CombatView) -> (String, Vec<Choice>) {
    let actor = match view.current {
        Some(ActorRef::Member(id)) => world
            .party
            .members
            .iter()
            .find(|m| m.id == id)
            .map_or("?", |m| m.name.as_str()),
        _ => "the monsters",
    };
    let attacks = view
        .stacks
        .iter()
        .filter(|s| s.alive && s.reachable)
        .map(|s| {
            let label = format!("Attack {}", data.label("en", &s.name));
            (label, CombatCommand::Attack { stack: s.index })
        });
    let others = [
        ("Dodge".to_owned(), CombatCommand::Dodge),
        ("Flee".to_owned(), CombatCommand::Run),
    ];
    let choices = attacks
        .chain(others)
        .map(|(label, c)| choice(world, data, label, Command::Combat(c)))
        .collect();
    (format!("Round {}: {actor}'s turn", view.round), choices)
}
