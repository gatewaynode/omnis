//! The town notice: a stopgap until the services screen (tasks/TODO.md, parity P4). Stepping
//! onto a service's site puts the party in the service; this client has no screen for it yet,
//! so the panel names the service and offers Leave, and the party is never stuck inside.
//! `shell/panel.rs` shows it.

use super::notice::{Choice, Notice, Order};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Mode, ServiceCommand, World};

/// The notice while the party is in a service, or `None` outside one.
#[must_use]
pub fn notice(world: &World, data: &Data) -> Option<Notice> {
    let Mode::Town(state) = &world.mode else {
        return None;
    };
    let name = data
        .services
        .get(&state.service)
        .map_or("A service", |s| data.label("en", &s.name));
    Some(Notice {
        title: name.to_owned(),
        lines: vec![
            "Town services are not in this client yet.".to_owned(),
            "Use the other client for them: just run-app.".to_owned(),
        ],
        choices: vec![Choice {
            label: "Leave".to_owned(),
            order: Order::Command(Command::Service(ServiceCommand::Leave)),
            blocked: None,
        }],
    })
}
