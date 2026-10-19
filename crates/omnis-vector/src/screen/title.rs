//! The title (presentation-ARCHITECTURE §6.3, §6.7): where the program starts when no game is
//! asked for on the command line. New game opens the form; Load quick save loads the quick
//! save, dim when there is none; Settings opens the player's settings; Quit stands in its own
//! row, apart from the others (PRD §7).

use super::{Control, ControlId, Ctx, Effect, HostOp, Panel, Row, Screen, ScreenId, Span};

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let mut load = Control::button(ControlId::Load, "Load quick save");
    load.why_not = ctx.save.missing();
    Screen {
        id: ScreenId::Title,
        title: Span::title("OMNIS"),
        panels: vec![Panel {
            rows: vec![
                Row::controls(vec![
                    Control::button(ControlId::NewGame, "New game"),
                    load,
                    Control::button(ControlId::Settings, "Settings"),
                ]),
                Row::controls(vec![Control::button(ControlId::Quit, "Quit")]),
            ],
        }],
        keys: Vec::new(),
        message: None,
    }
}

/// What each control asks of the shell.
#[must_use]
pub fn apply(control: ControlId) -> Effect {
    match control {
        ControlId::NewGame => Effect::Open(ScreenId::NewGame),
        ControlId::Load => Effect::Host(HostOp::Load { migrate: false }),
        ControlId::Settings => Effect::Open(ScreenId::Settings),
        ControlId::Quit => Effect::Quit,
        _ => Effect::None,
    }
}
