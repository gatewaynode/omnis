//! Inside a town service: a stopgap until the services screen (tasks/TODO.md, parity P4).
//! Stepping onto a service's site puts the party in the service; this client has no screen for
//! it yet, so it names the service and offers Leave, and the party is never stuck inside.

use super::{Control, ControlId, Effect, Game, Panel, Row, Screen, ScreenId, Span};
use crate::labels;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, ServiceCommand};

/// The screen; the service's name, or "A service" outside one (the shell shows it only inside).
#[must_use]
pub fn content(data: &Data, game: Game) -> Screen {
    let name = game
        .views
        .here
        .service
        .as_deref()
        .map_or("A service", |id| labels::service(data, id));
    Screen {
        id: ScreenId::Service,
        title: Span::title(name),
        panels: vec![Panel {
            rows: vec![
                Row::text(Span::body("Town services are not in this client yet.")),
                Row::text(Span::body("Use the other client for them: just run-app.")),
                Row::controls(vec![Control::button(ControlId::Leave, "Leave")]),
            ],
        }],
        keys: Vec::new(),
        message: None,
    }
}

/// Leave steps back out to the street.
#[must_use]
pub fn apply(control: ControlId) -> Effect {
    match control {
        ControlId::Leave => Effect::Command {
            command: Command::Service(ServiceCommand::Leave),
            close: false,
        },
        _ => Effect::None,
    }
}
