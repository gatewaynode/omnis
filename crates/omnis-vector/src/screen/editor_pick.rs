//! The stamp's detail picker (the editor's column, step 3b, owner 2026-10-31): a modal over the
//! editor listing what the loaded packs have of the chosen stamp's kind (monsters, services,
//! object kinds, items, text keys; for a portal, the portals nothing links to, as "id  -  map
//! (x, y)"; `editor::pick::options`), searched by a text field and chosen several at once, up to
//! what one cell takes; Finish holds the choice in the stamp (and goes on to the properties
//! modal for the kinds that have them) or writes it into a form's field (a portal's link),
//! Cancel keeps what was held. Built from `EditingCtx::pick`, the model's own state
//! (`editor::pick::Picking`), which carries its options.

use super::{Control, ControlId, Ctx, Effect, Kind, Panel, Payload, Row, Screen, ScreenId, Span};
use crate::editor::EditAct;
use crate::editor::stamp;

/// The most bytes the search takes.
const FILTER_MAX: usize = 64;

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let edit = |act| ControlId::Edit(act);
    let cancel = Control::button(ControlId::Resume, "Cancel");
    let Some(p) = ctx.editing.and_then(|e| e.pick) else {
        return Screen {
            id: ScreenId::EditorPick,
            title: Span::title("CHOOSE"),
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
    let kind = p.kind;
    let what = kind.what();
    let limit = stamp::limit(kind);
    let mut rows = vec![
        Row::controls(vec![Control {
            kind: Kind::Text {
                value: p.filter.clone(),
                max: FILTER_MAX,
                lines: 1,
            },
            ..Control::button(edit(EditAct::PickFilter), "search")
        }]),
        Row::text(Span::body(format!("{} of {limit} chosen", p.chosen.len()))),
    ];
    let shown = p.shown();
    if p.options.is_empty() {
        rows.push(Row::text(Span::body(format!("The packs have no {what}."))));
    } else if shown.is_empty() {
        rows.push(Row::text(Span::body(format!("No {what} matches."))));
    }
    let full = limit > 1 && p.chosen.len() >= limit;
    for i in shown {
        let (id, label) = (&p.options[i], &p.labels[i]);
        let chosen = p.is_chosen(id);
        let mark = if chosen { "[x]" } else { "[ ]" };
        let mut row = Control::button(
            edit(EditAct::PickToggle(u16::try_from(i).unwrap_or(u16::MAX))),
            format!("{mark} {label}"),
        );
        if full && !chosen {
            row.why_not = Some(format!("at most {limit} {what}s"));
        }
        rows.push(Row::controls(vec![row]));
    }
    let mut finish = Control::button(edit(EditAct::PickFinish), "Finish");
    if p.chosen.is_empty() {
        finish.why_not = Some("nothing chosen".to_owned());
    }
    rows.push(Row::controls(vec![finish, cancel]));
    Screen {
        id: ScreenId::EditorPick,
        title: Span::title(format!("CHOOSE {}S", what.to_uppercase())),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// What each control asks of the shell: an act goes to the editor with what was pressed, typed
/// or set; Cancel closes the modal.
#[must_use]
pub fn apply(control: ControlId, payload: &Payload) -> Effect {
    match control {
        ControlId::Resume => Effect::Back,
        ControlId::Edit(act) => Effect::Editor(act, payload.clone()),
        _ => Effect::None,
    }
}
