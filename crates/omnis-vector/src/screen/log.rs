//! The full log (owner, 2026-10-17): every line of the session, stamped with the party's clock,
//! newest at the foot, filtered by every word typed. A filter beside the lines with how many
//! match, and Close; the lines in one text, so typing rewrites it in place and the field keeps
//! its focus. Opened by L, the "Log  L" button or a click on the running log.

use super::{
    Control, ControlId, Effect, Forms, Game, Kind, Panel, Payload, Row, Screen, ScreenId, Span,
};
use crate::style::Role;

/// The most lines shown: the newest that match.
pub const SHOWN: usize = 500;

/// The longest filter, in bytes.
pub const FILTER_MAX: usize = 64;

/// The screen.
#[must_use]
pub fn content(game: Game, forms: &Forms) -> Screen {
    let history = game.history;
    let found = history.matching(&forms.log_filter);
    let first = found.len().saturating_sub(SHOWN);
    let count = if first > 0 {
        format!("The newest {SHOWN} of {} lines", found.len())
    } else {
        format!("{} of {} lines", found.len(), history.len())
    };
    let lines = if found.is_empty() {
        let why = if history.is_empty() {
            "Nothing has been said yet"
        } else {
            "No line holds every word"
        };
        Span {
            role: Role::Dim,
            ..Span::body(why)
        }
    } else {
        let text: Vec<String> = found[first..].iter().map(|e| e.stamped()).collect();
        Span::body(text.join("\n"))
    };
    let filter = Control {
        id: ControlId::LogFilter,
        label: "Filter".to_owned(),
        kind: Kind::Text {
            value: forms.log_filter.clone(),
            max: FILTER_MAX,
        },
        why_not: None,
    };
    Screen {
        id: ScreenId::Log,
        title: Span::title("Log"),
        panels: vec![
            Panel {
                rows: vec![
                    Row::controls(vec![filter]),
                    Row::text(Span {
                        role: Role::Dim,
                        ..Span::body(count)
                    }),
                    Row::controls(vec![Control::button(ControlId::Close, "Close")]),
                ],
            },
            Panel {
                rows: vec![Row::text(lines)],
            },
        ],
        keys: Vec::new(),
        message: None,
    }
}

/// The filter as typed, or Close.
pub fn apply(forms: &mut Forms, control: ControlId, payload: &Payload) -> Effect {
    match (control, payload) {
        (ControlId::LogFilter, Payload::Text(text)) => {
            forms.log_filter.clone_from(text);
            Effect::None
        }
        (ControlId::Close, _) => Effect::Back,
        _ => Effect::None,
    }
}
