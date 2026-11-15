//! The map's resize (owner, 2026-11-14; `tasks/plans/editor-resize.md`): a modal over the
//! editor asking how many cells to add on each side, a negative number cutting them; what the
//! map becomes; and, when a cut would destroy something, the list of it with Yes and No.

use super::{Control, ControlId, Ctx, Effect, Kind, Panel, Payload, Row, Screen, ScreenId, Span};
use crate::editor::EditAct;
use crate::editor::resize::{Resizing, SHOWN, SIDES};

/// A side's field holds at most a sign and three digits.
const FIELD_MAX: usize = 4;

/// The warning: what the cut destroys, then Yes and No.
fn asking(lost: &[String]) -> Vec<Row> {
    let mut rows = vec![Row::text(Span::alert("This cut destroys:"))];
    rows.extend(lost.iter().take(SHOWN).map(|l| Row::text(Span::body(l))));
    if lost.len() > SHOWN {
        let more = lost.len() - SHOWN;
        rows.push(Row::text(Span::body(format!("and {more} more"))));
    }
    rows.push(Row::text(Span::body(
        "Undo brings it back until the map is closed.",
    )));
    rows.push(Row::controls(vec![
        Control::button(ControlId::Edit(EditAct::ResizeYes), "Yes, resize"),
        Control::button(ControlId::Edit(EditAct::ResizeNo), "No"),
    ]));
    rows
}

/// The fields: the size now, a field a side, what it becomes, Apply and Cancel.
fn fields(resizing: &Resizing) -> Vec<Row> {
    let (w, h) = resizing.size;
    let mut rows = vec![
        Row::text(Span::body(format!("The map is {w} x {h}."))),
        Row::text(Span::body(
            "Cells to add on each side; a negative number cuts them.",
        )),
    ];
    for (side, (label, typed)) in (0u8..).zip(SIDES.iter().zip(&resizing.typed)) {
        let field = Control {
            kind: Kind::Text {
                value: typed.clone(),
                max: FIELD_MAX,
                lines: 1,
            },
            ..Control::button(ControlId::Edit(EditAct::ResizeSide(side)), *label)
        };
        rows.push(Row::field(*label, field));
    }
    let becomes = resizing.becomes();
    if let Ok((w, h)) = becomes {
        rows.push(Row::text(Span::body(format!("It becomes {w} x {h}."))));
    }
    if let Ok(sides) = resizing.sides()
        && (sides.west != 0 || sides.north != 0)
    {
        rows.push(Row::text(Span::body(format!(
            "Every cell moves {} east and {} south: a saved game's party on this map will \
             stand where the old cell was.",
            sides.west, sides.north
        ))));
    }
    rows.push(Row::controls(vec![
        Control {
            why_not: becomes.err(),
            ..Control::button(ControlId::Edit(EditAct::ResizeApply), "Apply")
        },
        Control::button(ControlId::Resume, "Cancel"),
    ]));
    rows
}

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let rows = match ctx.editing.and_then(|e| e.resize) {
        Some(resizing) => match &resizing.asking {
            Some(lost) => asking(lost),
            None => fields(resizing),
        },
        None => vec![Row::controls(vec![Control::button(
            ControlId::Resume,
            "Close",
        )])],
    };
    Screen {
        id: ScreenId::EditorResize,
        title: Span::title("RESIZE"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// A control used: Cancel goes back; the fields, Apply, Yes and No are the editor's acts.
#[must_use]
pub fn apply(control: ControlId, payload: &Payload) -> Effect {
    match control {
        ControlId::Resume => Effect::Back,
        ControlId::Edit(act) => Effect::Editor(act, payload.clone()),
        _ => Effect::None,
    }
}
