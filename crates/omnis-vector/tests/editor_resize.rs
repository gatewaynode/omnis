//! The map's resize in the editor (owner, 2026-11-14: "plan the Resize form, all four sides.
//! Shrink is destructive, a warning and y/n accept buttons should be thrown when shrinking a map
//! destroys things."): a button on the inspector's map line opens a modal with a field a side;
//! a grow is made at once; a cut that destroys anything shows the list with Yes and No.
//! Headless on the town (12 x 5; the gate's portal at (11, 2), the start at (10, 2)).

use crate::common;
use bevy::prelude::*;
use common::app::press_ui;
use common::editor::{editing, in_editor, press_edit, top_screen, type_ui};
use omnis_vector::editor::EditAct;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::screen::Pressable;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;

const NORTH: u8 = 0;
const EAST: u8 = 1;
const WEST: u8 = 3;

fn size(app: &App) -> (u16, u16) {
    let def = editing(app).def().expect("a map");
    (def.width, def.height)
}

fn side(app: &mut App, side: u8, text: &str) {
    type_ui(app, ControlId::Edit(EditAct::ResizeSide(side)), text);
    app.update();
}

fn press(app: &mut App, act: EditAct) {
    press_ui(app, Pressable(ControlId::Edit(act)));
}

fn message(app: &App) -> String {
    editing(app).message.clone().unwrap_or_default()
}

/// The inspector's map line carries Resize...; it opens the modal on the map's size, the
/// fields follow what is typed, and a grow is made at once as one step, the modal closed.
#[test]
fn a_grow_is_asked_for_by_side_and_made_at_once() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let inspector = omnis_vector::editor::view::inspector(editing(&app), &data);
    let button = inspector.control(ControlId::Edit(EditAct::Resize));
    assert_eq!(button.expect("on the map's line").label, "Resize...");
    press_edit(&mut app, EditAct::Resize);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorResize));
    let text = screen_text(app.world());
    assert!(text.contains("The map is 12 x 5."), "{text}");
    for label in ["North", "East", "South", "West"] {
        assert!(text.contains(label), "{label} in {text}");
    }
    // Nothing asked for: Apply is dim and says why.
    let apply = |app: &App| {
        let ctx = app
            .world()
            .resource::<omnis_vector::shell::screen::Screens>();
        let _ = ctx;
        let resizing = editing(app).resizing.clone().expect("the form");
        resizing.becomes()
    };
    assert_eq!(apply(&app), Err("nothing to add or cut".to_owned()));
    side(&mut app, WEST, "3");
    side(&mut app, NORTH, "1");
    assert_eq!(apply(&app), Ok((15, 6)));
    let text = screen_text(app.world());
    assert!(text.contains("It becomes 15 x 6."), "{text}");
    assert!(
        text.contains("Every cell moves 3 east and 1 south"),
        "{text}"
    );
    let steps = editing(&app).history().expect("a map").0.len();
    press(&mut app, EditAct::ResizeApply);
    assert_eq!(top_screen(&app), None, "a grow closes the modal");
    assert_eq!(size(&app), (15, 6));
    let def = editing(&app).def().unwrap();
    let gate = def.portals.iter().find(|p| p.id == "town-gate").unwrap();
    assert_eq!((gate.x, gate.y), (14, 3), "the gate went with its cell");
    assert_eq!(
        editing(&app).history().unwrap().0.len(),
        steps + 1,
        "one step"
    );
    assert!(
        message(&app).starts_with("grew 1 north, 3 west"),
        "{}",
        message(&app)
    );
    assert_eq!(editing(&app).resizing, None);
    // Undo puts the map back.
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(size(&app), (12, 5));
}

/// A cut that destroys something does not happen on Apply: the modal lists what goes, with
/// Yes and No. No leaves the map and the numbers as they were; Yes makes the cut as one step.
#[test]
fn a_cut_that_destroys_asks_first() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Resize);
    side(&mut app, EAST, "-2");
    press(&mut app, EditAct::ResizeApply);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorResize), "it asks");
    assert_eq!(size(&app), (12, 5), "nothing cut yet");
    let text = screen_text(app.world());
    assert!(text.contains("This cut destroys:"), "{text}");
    assert!(text.contains("10 drawn cells"), "{text}");
    assert!(text.contains("portal town-gate at (11, 2)"), "{text}");
    assert!(text.contains("the start's cell"), "{text}");
    assert!(
        text.contains("\"Yes, resize\"") && text.contains("\"No\""),
        "{text}"
    );
    assert!(
        !text.contains("\"Apply\""),
        "the fields give way to the question"
    );
    // (A Yes that no question is waiting on cuts nothing: checked below.)
    // No: the fields again, as they were.
    press(&mut app, EditAct::ResizeNo);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorResize));
    assert_eq!(size(&app), (12, 5));
    let resizing = editing(&app).resizing.clone().expect("still open");
    assert_eq!((resizing.asking, resizing.typed[1].as_str()), (None, "-2"));
    // Yes: cut, one step, the modal closed, the loader's word on the map as it now stands.
    press(&mut app, EditAct::ResizeApply);
    let steps = editing(&app).history().expect("a map").0.len();
    press(&mut app, EditAct::ResizeYes);
    assert_eq!(top_screen(&app), None);
    assert_eq!(size(&app), (10, 5));
    assert_eq!(editing(&app).history().unwrap().0.len(), steps + 1);
    assert!(
        message(&app).starts_with("cut 2 east ("),
        "{}",
        message(&app)
    );
    let def = editing(&app).def().unwrap();
    assert!(
        !def.portals.iter().any(|p| p.id == "town-gate"),
        "the gate is gone"
    );
    assert!(
        !editing(&app).status.errors.is_empty(),
        "the meadow's gate has no partner now: Save is dim with the count"
    );
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(size(&app), (12, 5));
    assert!(editing(&app).status.errors.is_empty());
}

/// What cannot be applied says why, and Cancel changes nothing.
#[test]
fn a_bad_number_or_size_dims_apply_and_cancel_leaves_the_map() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Resize);
    let why = |app: &App| editing(app).resizing.clone().unwrap().becomes();
    side(&mut app, EAST, "x");
    assert_eq!(why(&app), Err("East is not a whole number".to_owned()));
    side(&mut app, EAST, "-12");
    assert_eq!(why(&app), Err("a side must stay 1 to 256 cells".to_owned()));
    side(&mut app, EAST, "245");
    assert_eq!(why(&app), Err("a side must stay 1 to 256 cells".to_owned()));
    // The dim Apply says why on the screen, and is no button to press.
    let text = screen_text(app.world());
    assert!(text.contains("a side must stay 1 to 256 cells"), "{text}");
    assert!(!text.contains("It becomes"), "{text}");
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_eq!(top_screen(&app), None);
    assert_eq!(size(&app), (12, 5));
    // Opened again, the fields are empty.
    press_edit(&mut app, EditAct::Resize);
    assert!(
        editing(&app)
            .resizing
            .clone()
            .unwrap()
            .typed
            .iter()
            .all(String::is_empty)
    );
}

/// Yes is the answer to the warning and nothing else: sent while only the fields are up (over
/// the socket, say), it cuts nothing.
#[test]
fn a_yes_with_no_question_waiting_cuts_nothing() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Resize);
    side(&mut app, EAST, "-2");
    let data = app.world().resource::<Packs>().data.clone();
    app.world_mut()
        .resource_mut::<omnis_vector::shell::editor::Editing>()
        .apply(&data, EditAct::ResizeYes, 0);
    app.update();
    assert_eq!(size(&app), (12, 5));
    assert_eq!(top_screen(&app), Some(ScreenId::EditorResize));
}
