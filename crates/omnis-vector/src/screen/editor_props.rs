//! The stamp's properties (the editor's column, step 3b, owner 2026-10-31): a modal over the
//! editor after the picker's Finish, or from the sliders icon under the palette, for the kinds
//! with something a click does not decide: an encounter's count a monster, its disposition,
//! once and where it stands; an object's place on the cell, facing, state, hidden DC, name and
//! lore, one section a chosen kind; a pile's count an item and its place. The rows are the
//! kinds' own forms' (`Form::props_rows`), held in `editor::pick::Picking`; Finish reads them
//! into the stamp, Cancel keeps what it held.

use super::{Control, ControlId, Ctx, Effect, Panel, Payload, Row, Screen, ScreenId, Span};
use crate::editor::EditAct;
use crate::editor::forms::Placing;

/// The line under the title: what the modal sets for the kind.
const fn hint(kind: Placing) -> &'static str {
    match kind {
        Placing::Encounter => "A count for each, the group's disposition, and where it stands.",
        Placing::Object => "Where each stands on the cell, and whether it is hidden.",
        Placing::Ground => "A count for each, and where the pile lies.",
        Placing::Portal | Placing::Activity | Placing::Lore => "Nothing to set.",
    }
}

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let edit = |act| ControlId::Edit(act);
    let cancel = Control::button(ControlId::Resume, "Cancel");
    let Some(p) = ctx.editing.and_then(|e| e.pick) else {
        return Screen {
            id: ScreenId::EditorProps,
            title: Span::title("PROPERTIES"),
            panels: vec![Panel {
                rows: vec![
                    Row::text(Span::body("Choose a stamp in the palette first.")),
                    Row::controls(vec![cancel]),
                ],
            }],
            keys: Vec::new(),
            message: None,
        };
    };
    let mut rows = vec![Row::text(Span::body(hint(p.kind)))];
    for (n, form) in p.props.iter().enumerate() {
        rows.extend(form.props_rows(u8::try_from(n).unwrap_or(u8::MAX)));
    }
    rows.push(Row::controls(vec![
        Control::button(edit(EditAct::PropsFinish), "Finish"),
        cancel,
    ]));
    Screen {
        id: ScreenId::EditorProps,
        title: Span::title(format!("{} PROPERTIES", p.kind.label().to_uppercase())),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// What each control asks of the shell: as the picker's.
#[must_use]
pub fn apply(control: ControlId, payload: &Payload) -> Effect {
    super::editor_pick::apply(control, payload)
}
