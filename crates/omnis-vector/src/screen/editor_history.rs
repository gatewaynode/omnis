//! The history as a list (the editor's column, step 5, owner 2026-10-31): a modal over the
//! editor naming every undo step oldest first, the current one marked; a row jumps the map to
//! where it stood after that step, the plan beneath showing it; Undo, Redo and Close at the
//! bottom. Built from `EditingCtx::history`, the document's own steps (`Document::steps`, `at`).

use super::{Control, ControlId, Ctx, Effect, Panel, Payload, Row, Screen, ScreenId, Span};
use crate::editor::EditAct;

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let edit = |act| ControlId::Edit(act);
    let close = Control::button(ControlId::Resume, "Close");
    let screen = |rows| Screen {
        id: ScreenId::EditorHistory,
        title: Span::title("HISTORY"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    };
    let Some((steps, at)) = ctx.editing.and_then(|e| e.history) else {
        return screen(vec![
            Row::text(Span::body("Open a map first.")),
            Row::controls(vec![close]),
        ]);
    };
    // A row a step, the one the map stands after marked and not pressable.
    let step = |i: usize, words: String| {
        let mut row = Control::button(
            edit(EditAct::HistoryTo(u16::try_from(i).unwrap_or(u16::MAX))),
            if i == at { format!("> {words}") } else { words },
        );
        if i == at {
            row.why_not = Some("where the map stands".to_owned());
        }
        Row::controls(vec![row])
    };
    let mut rows = vec![step(0, "the map as opened".to_owned())];
    for (i, words) in steps.iter().enumerate() {
        rows.push(step(i + 1, format!("{}  {words}", i + 1)));
    }
    if steps.is_empty() {
        rows.push(Row::text(Span::body("no steps yet")));
    }
    let mut undo = Control::button(edit(EditAct::Undo), "Undo");
    if at == 0 {
        undo.why_not = Some("nothing to undo".to_owned());
    }
    let mut redo = Control::button(edit(EditAct::Redo), "Redo");
    if at == steps.len() {
        redo.why_not = Some("nothing to redo".to_owned());
    }
    rows.push(Row::controls(vec![undo, redo, close]));
    screen(rows)
}

/// A control used: Close goes back; the rows and the buttons are the editor's acts.
#[must_use]
pub fn apply(control: ControlId, payload: &Payload) -> Effect {
    match control {
        ControlId::Resume => Effect::Back,
        ControlId::Edit(act) => Effect::Editor(act, payload.clone()),
        _ => Effect::None,
    }
}
