//! The editor's file picker (M5.9b, owner 2026-10-24): a modal over the editor that lists the
//! pack's own files, grouped by folder, and opens one as text. The list is the data layer's
//! (`Session::files`: `pack.ron` and the `.ron` files under the pack's `data`, `text`, `notes`
//! and `migrations`, no symlink followed, nothing outside the root), so nothing is typed and no
//! other path is reachable. While the open file has unsaved text, every other file is dim with
//! the reason, so a pick cannot drop typed work; the open file itself is marked and can be shown
//! again.

use super::{Control, ControlId, Ctx, Effect, Panel, Row, Screen, ScreenId, Span};
use crate::editor::EditAct;

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let (files, open): (&[String], Option<(&str, bool)>) =
        ctx.editing.map_or((&[], None), |e| (e.files, e.file));
    let blocked = open
        .filter(|(_, dirty)| *dirty)
        .map(|(rel, _)| format!("save or revert {rel} first"));
    let mut rows: Vec<Row> = Vec::new();
    let mut folder: Option<&str> = None;
    for (i, rel) in files.iter().enumerate() {
        let (dir, name) = rel.rsplit_once('/').unwrap_or(("", rel.as_str()));
        if !dir.is_empty() && folder != Some(dir) {
            folder = Some(dir);
            rows.push(Row::text(Span::body(format!("{dir}/"))));
        }
        let is_open = open.is_some_and(|(o, _)| o == rel);
        let mark = match (is_open, open) {
            (true, Some((_, true))) => "> * ",
            (true, _) => "> ",
            _ => "",
        };
        let mut button = Control::button(
            ControlId::Edit(EditAct::File(u16::try_from(i).unwrap_or(u16::MAX))),
            format!("{mark}{name}"),
        );
        if !is_open {
            button.why_not.clone_from(&blocked);
        }
        rows.push(Row::controls(vec![button]));
    }
    if files.is_empty() {
        rows.push(Row::text(Span::body("The pack has no files.")));
    }
    rows.push(Row::controls(vec![Control::button(
        ControlId::Resume,
        "Cancel",
    )]));
    Screen {
        id: ScreenId::EditorFiles,
        title: Span::title("OPEN A FILE"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// What each control asks of the shell: a file goes to the editor, Cancel closes the picker.
#[must_use]
pub fn apply(control: ControlId) -> Effect {
    match control {
        ControlId::Resume => Effect::Back,
        ControlId::Edit(act) => Effect::Editor(act),
        _ => Effect::None,
    }
}
