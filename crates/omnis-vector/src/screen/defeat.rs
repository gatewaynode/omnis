//! The fallen party (presentation-ARCHITECTURE §6.3): when no member can fight, the party walks no
//! further and this screen offers a way on, so the game never dead-ends. Monsters met or fought
//! have the fight screen (`shell/combat.rs`).

use super::{Control, ControlId, Effect, HostOp, Panel, Row, Screen, ScreenId, Span};

/// The screen.
#[must_use]
pub fn content() -> Screen {
    Screen {
        id: ScreenId::Defeat,
        title: Span::title("The party has fallen"),
        panels: vec![Panel {
            rows: vec![Row::controls(vec![Control::button(
                ControlId::StartAgain,
                "Start again",
            )])],
        }],
        keys: Vec::new(),
    }
}

/// Start again starts the same game over.
#[must_use]
pub fn apply(control: ControlId) -> Effect {
    match control {
        ControlId::StartAgain => Effect::Host(HostOp::Restart),
        _ => Effect::None,
    }
}
