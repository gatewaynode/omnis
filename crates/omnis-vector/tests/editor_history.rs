//! The history as a list (the editor's column, step 5, owner 2026-10-31): a History icon beside
//! Undo and Redo opens a modal over the editor naming every step oldest first, the current one
//! marked; a row jumps the map there with the modal still up; Undo, Redo and Close at the
//! bottom; a change after a jump drops the steps undone. Headless on the town.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::press_ui;
use common::editor::{editing, in_editor, mouse, point_at, press_edit, top_screen};
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::{self, Entry, Tool};
use omnis_vector::screen::{ControlId, Kind, ScreenId};
use omnis_vector::shell::screen::Pressable;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;

fn history(app: &App) -> (Vec<String>, usize) {
    let (steps, at) = editing(app).history().expect("a map");
    (steps.to_vec(), at)
}

fn message(app: &App) -> String {
    editing(app).message.clone().unwrap_or_default()
}

fn choose(app: &mut App, entry: Entry) {
    let def = editing(app).def().expect("a map");
    let i = brush::palette(&def)
        .iter()
        .position(|e| *e == entry)
        .expect("in the palette");
    press_edit(app, EditAct::Brush(u8::try_from(i).unwrap()));
    assert_eq!(editing(app).entry, entry);
}

fn click(app: &mut App, cell: (u16, u16)) {
    point_at(app, cell, None);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

fn drag(app: &mut App, from: (u16, u16), through: &[(u16, u16)]) {
    point_at(app, from, None);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    for &cell in through {
        point_at(app, cell, None);
        app.update();
    }
    mouse(app, MouseButton::Left, ButtonState::Released);
}

fn row(app: &mut App, at: u16) {
    press_ui(app, Pressable(ControlId::Edit(EditAct::HistoryTo(at))));
}

#[test]
fn the_history_icon_opens_a_list_of_the_steps_and_a_row_jumps_there() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let opened = editing(&app).def().expect("a map");
    // The icon stands in the tool grid after Redo, pressable with no steps.
    let tools = omnis_vector::editor::view::tools(editing(&app), &data);
    let icon = tools
        .control(ControlId::Edit(EditAct::History))
        .expect("the history icon");
    assert!(matches!(
        icon.kind,
        Kind::Icon {
            name: "rotate-ccw-clock",
            ..
        }
    ));
    assert!(icon.why_not.is_none(), "open with no steps too");
    // A fresh map: the list holds the map as opened and nothing else.
    assert_eq!(history(&app), (Vec::new(), 0));
    press_edit(&mut app, EditAct::History);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorHistory));
    let text = screen_text(app.world());
    assert!(text.contains("> the map as opened"), "{text}");
    assert!(text.contains("no steps yet"), "{text}");
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_ne!(top_screen(&app), Some(ScreenId::EditorHistory));
    // Three steps: a stroke of three cells (the document's words), the eraser on a cell (its
    // words), a cell run (the editor's own words, as the message said them).
    choose(&mut app, Entry::Terrain(0));
    press_edit(&mut app, EditAct::Tool(Tool::Brush));
    drag(&mut app, (4, 3), &[(5, 3), (6, 3)]);
    let after_stroke = editing(&app).def().expect("a map");
    press_edit(&mut app, EditAct::Tool(Tool::Eraser));
    click(&mut app, (9, 1));
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    click(&mut app, (1, 4));
    click(&mut app, (6, 4));
    press_edit(&mut app, EditAct::TakeRun);
    let run_words = message(&app);
    assert!(run_words.starts_with("6 cells painted"), "{run_words}");
    let after_all = editing(&app).def().expect("a map");
    let stroke = "a stroke of 3 cells: painted street at (4, 3)";
    assert_eq!(
        history(&app),
        (
            vec![
                stroke.to_owned(),
                "cleared (9, 1)".to_owned(),
                run_words.clone()
            ],
            3
        )
    );
    // The list names them, the last marked.
    press_edit(&mut app, EditAct::History);
    let text = screen_text(app.world());
    assert!(text.contains(&format!("1  {stroke}")), "{text}");
    assert!(text.contains("2  cleared (9, 1)"), "{text}");
    assert!(text.contains(&format!("> 3  {run_words}")), "{text}");
    // Row 1 jumps there: the map as it stood after the stroke, the modal still up, row 1
    // marked, the later steps still listed.
    row(&mut app, 1);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorHistory));
    assert_eq!(editing(&app).def().expect("a map"), after_stroke);
    assert_eq!(history(&app).1, 1);
    assert_eq!(history(&app).0.len(), 3);
    assert_eq!(message(&app), "at step 1 of 3");
    let text = screen_text(app.world());
    assert!(text.contains(&format!("> 1  {stroke}")), "{text}");
    assert!(text.contains(&format!("3  {run_words}")), "{text}");
    // The modal's Undo and Redo walk one step; the last row redoes all.
    press_ui(&mut app, Pressable(ControlId::Edit(EditAct::Redo)));
    assert_eq!(history(&app).1, 2);
    press_ui(&mut app, Pressable(ControlId::Edit(EditAct::Undo)));
    assert_eq!(history(&app).1, 1);
    row(&mut app, 3);
    assert_eq!(history(&app).1, 3);
    assert_eq!(editing(&app).def().expect("a map"), after_all);
    // The map as opened: everything undone, nothing to save.
    row(&mut app, 0);
    assert_eq!(editing(&app).def().expect("a map"), opened);
    assert!(!editing(&app).status.dirty);
    assert_eq!(message(&app), "the map as opened");
    // A change after a jump drops the steps undone.
    row(&mut app, 1);
    press_ui(&mut app, Pressable(ControlId::Resume));
    press_edit(&mut app, EditAct::Tool(Tool::Brush));
    click(&mut app, (9, 4));
    assert_eq!(
        history(&app),
        (
            vec![stroke.to_owned(), "painted street at (9, 4)".to_owned()],
            2
        )
    );
    // A pending run is dropped by a jump.
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    click(&mut app, (1, 1));
    assert!(editing(&app).run.is_some());
    press_edit(&mut app, EditAct::History);
    row(&mut app, 1);
    assert!(editing(&app).run.is_none(), "a jump drops the run");
    assert_eq!(history(&app).1, 1);
}
