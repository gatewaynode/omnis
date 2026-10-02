//! The question before a step into or out of a town service, or the service panel's Leave
//! (owner, 2026-09-27: always ask, always log). `ask` says whether a command needs one and what
//! it says; `answer` says what each button sends. The command is held until the answer: Go
//! sends it, Stay sends nothing.
//! `input.rs` asks, `feathers_confirm.rs` draws the panel. Bevy-free. "Remember my choice"
//! is a horizon.

use omnis_sim::omnis_core::{Direction, ServiceId};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Mode, ServiceCommand, World, query};

/// A held step and its question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    /// The step, sent on Go.
    pub command: Command,
    /// "Enter the Inn?", "Leave the Inn?".
    pub question: String,
}

/// The panel's two buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ConfirmId {
    /// Take the step (Enter).
    #[default]
    Go,
    /// Stay where the party is (Escape).
    Stay,
}

/// The question a command needs first, if any: a step that would go into a service, a step
/// out of one that would go somewhere (a wall keeps the party inside, so it asks nothing), or
/// leaving from inside one.
#[must_use]
pub fn ask(world: &World, data: &Data, command: &Command) -> Option<Confirm> {
    let question = match command {
        Command::Step(direction) => question(world, data, *direction)?,
        Command::Service(ServiceCommand::Leave) => {
            let Mode::Town(state) = &world.mode else {
                return None;
            };
            format!("Leave the {}?", service_name(data, state.service))
        }
        _ => return None,
    };
    Some(Confirm {
        command: command.clone(),
        question,
    })
}

fn service_name(data: &Data, id: ServiceId) -> &str {
    data.services
        .get(&id)
        .map_or("?", |s| data.label("en", &s.name))
}

fn question(world: &World, data: &Data, direction: Direction) -> Option<String> {
    let name = |id| service_name(data, id);
    match &world.mode {
        Mode::Town(state) => {
            query::step_lands(world, data, direction)?;
            Some(format!("Leave the {}?", name(state.service)))
        }
        Mode::Explore => {
            let service = query::site_ahead(world, data, direction)?;
            Some(format!("Enter the {}?", name(service)))
        }
        Mode::Encounter(_) | Mode::Combat(_) => None,
    }
}

/// What an answer sends: the held step on Go, nothing on Stay.
#[must_use]
pub fn answer(held: Confirm, id: ConfirmId) -> Option<Command> {
    match id {
        ConfirmId::Go => Some(held.command),
        ConfirmId::Stay => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_core::{Facing, Rotation};
    use omnis_sim::omnis_data::load_packs;
    use omnis_sim::{Settings, apply};
    use std::path::PathBuf;

    fn packs() -> Data {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")]).unwrap()
    }

    /// A new game at the inn's door, the door open, facing it.
    fn at_the_inn_door(data: &Data) -> World {
        let mut world = World::new(data, 3, Settings::default()).unwrap();
        (world.position.x, world.position.y, world.position.facing) = (1, 2, Facing::North);
        apply(&mut world, data, Command::Interact).unwrap();
        world
    }

    #[test]
    fn a_step_into_or_out_of_a_service_asks_and_nothing_else_does() {
        let data = packs();
        let mut world = at_the_inn_door(&data);
        let forward = Command::Step(Direction::Forward);
        let held = ask(&world, &data, &forward).expect("a step into the inn asks");
        assert_eq!(held.question, "Enter the Inn?");
        assert_eq!(held.command, forward);
        assert_eq!(ask(&world, &data, &Command::Step(Direction::Right)), None);
        assert_eq!(ask(&world, &data, &Command::Turn(Rotation::Left)), None);
        assert_eq!(
            ask(&world, &data, &Command::Interact),
            None,
            "use goes in unasked"
        );
        let leave = Command::Service(ServiceCommand::Leave);
        assert_eq!(
            ask(&world, &data, &leave),
            None,
            "outside, nothing to leave"
        );

        apply(&mut world, &data, forward.clone()).unwrap();
        let back = Command::Step(Direction::Back);
        assert_eq!(
            ask(&world, &data, &back).map(|c| c.question),
            Some("Leave the Inn?".to_owned())
        );
        assert_eq!(
            ask(&world, &data, &Command::Step(Direction::Right)).map(|c| c.question),
            Some("Leave the Inn?".to_owned()),
            "any step off the site leaves"
        );
        let held = ask(&world, &data, &leave).expect("the panel's Leave asks");
        assert_eq!(
            (held.question.as_str(), &held.command),
            ("Leave the Inn?", &leave)
        );
        assert_eq!(
            ask(&world, &data, &Command::Service(ServiceCommand::Room)),
            None,
            "only leaving asks"
        );
        let map = world.position.map;
        world.map_state(map).open_doors.clear();
        assert_eq!(
            ask(&world, &data, &back),
            None,
            "a shut door keeps the party in"
        );
    }

    #[test]
    fn go_sends_the_step_and_stay_sends_nothing() {
        let held = Confirm {
            command: Command::Step(Direction::Left),
            question: "Enter the Bank?".to_owned(),
        };
        assert_eq!(
            answer(held.clone(), ConfirmId::Go),
            Some(Command::Step(Direction::Left))
        );
        assert_eq!(answer(held, ConfirmId::Stay), None);
    }
}
