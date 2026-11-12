//! The editor's menu (M5.4, presentation-ARCHITECTURE §6.3): Esc over the editor. Resume,
//! Settings, Save (M5.5: dim with the loader's first refusal while the map would not load),
//! Back to title (which closes the editor, and says so when changes would be lost), and Quit
//! in its own row, apart from the rest (PRD §7).

use super::{Control, ControlId, Ctx, Effect, Panel, Payload, Row, Screen, ScreenId, Span};
use crate::editor::EditAct;

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let editing = ctx.editing;
    let dirty = editing.is_some_and(|e| e.dirty);
    let save_why = match editing {
        Some(e) if !e.open => Some("open a map first".to_owned()),
        Some(e) => e.errors.first().map(|first| {
            format!(
                "{} error{}: {first}",
                e.errors.len(),
                if e.errors.len() == 1 { "" } else { "s" }
            )
        }),
        None => Some("the editor is not open".to_owned()),
    };
    let leave = if dirty {
        "Discard changes and leave"
    } else {
        "Back to title"
    };
    Screen {
        id: ScreenId::EditorMenu,
        title: Span::title("EDITOR"),
        panels: vec![Panel {
            rows: vec![
                Row::controls(vec![
                    Control::button(ControlId::Resume, "Resume"),
                    Control::button(ControlId::Settings, "Settings"),
                ]),
                Row::controls(vec![Control {
                    why_not: save_why,
                    ..Control::button(ControlId::Edit(EditAct::Save), "Save")
                }]),
                Row::controls(vec![Control::button(ControlId::QuitToTitle, leave)]),
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
        ControlId::Edit(act) => Effect::Editor(act, Payload::Press),
        _ => Effect::None,
    }
}
