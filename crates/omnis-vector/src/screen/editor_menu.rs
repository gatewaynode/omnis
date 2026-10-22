//! The editor's menu (M5.4, presentation-ARCHITECTURE §6.3): Esc over the editor. Resume,
//! Settings, Back to title (which closes the editor; from M5.5 it says when changes would be
//! lost), and Quit in its own row, apart from the rest (PRD §7).

use super::{Control, ControlId, Effect, Panel, Row, Screen, ScreenId, Span};

/// The screen.
#[must_use]
pub fn content() -> Screen {
    Screen {
        id: ScreenId::EditorMenu,
        title: Span::title("EDITOR"),
        panels: vec![Panel {
            rows: vec![
                Row::controls(vec![
                    Control::button(ControlId::Resume, "Resume"),
                    Control::button(ControlId::Settings, "Settings"),
                ]),
                Row::controls(vec![Control::button(
                    ControlId::QuitToTitle,
                    "Back to title",
                )]),
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
        ControlId::Resume => Effect::Back,
        ControlId::Settings => Effect::Open(ScreenId::Settings),
        ControlId::QuitToTitle => Effect::ToTitle,
        ControlId::Quit => Effect::Quit,
        _ => Effect::None,
    }
}
