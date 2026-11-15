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
use omnis_vector::shell::screen::{EditorPress, Pressable};
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

/// The side's field that holds the keyboard.
fn focused_side(app: &App) -> Option<ControlId> {
    let focused = app
        .world()
        .resource::<bevy::input_focus::InputFocus>()
        .get()?;
    app.world().get::<Pressable>(focused).map(|p| p.0)
}

/// The field of side `side`.
fn field(app: &mut App, side: u8) -> Entity {
    let world = app.world_mut();
    let mut fields = world.query::<(Entity, &Pressable, &bevy::text::EditableText)>();
    fields
        .iter(world)
        .find(|(_, p, _)| p.0 == ControlId::Edit(EditAct::ResizeSide(side)))
        .map(|(e, _, _)| e)
        .expect("the side's field")
}

/// The owner's report of 2026-11-15: "I could only write in one direction field, the cursor
/// disappeared if I tried to choose any other field, I couldn't tab or arrow through the form
/// fields." The form opens with the keyboard in North; a field chosen by a click keeps the
/// keyboard through what is typed in it, whatever lines come and go beneath.
#[test]
fn every_side_s_field_takes_the_keyboard_when_chosen_and_keeps_it() {
    use common::app::{keyboard, keyboard_on};
    let (mut app, window) = keyboard_on(in_editor("test:map:town"));
    app.update();
    press_edit(&mut app, EditAct::Resize);
    app.update();
    assert_eq!(
        focused_side(&app),
        Some(ControlId::Edit(EditAct::ResizeSide(NORTH))),
        "the first field takes the keyboard as the form opens"
    );
    keyboard(&mut app, window, "1");
    assert_eq!(editing(&app).resizing.as_ref().unwrap().typed[0], "1");
    for (side, text) in [(EAST, "2"), (2, "3"), (WEST, "4"), (NORTH, "5")] {
        let chosen = field(&mut app, side);
        app.world_mut()
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(chosen, bevy::input_focus::FocusCause::Pressed);
        app.update();
        app.update();
        let id = ControlId::Edit(EditAct::ResizeSide(side));
        assert_eq!(focused_side(&app), Some(id), "side {side} chosen");
        keyboard(&mut app, window, text);
        assert_eq!(focused_side(&app), Some(id), "side {side} after typing");
    }
    assert_eq!(
        editing(&app).resizing.as_ref().unwrap().typed,
        ["15", "2", "3", "4"].map(str::to_owned)
    );
}

/// A key with no text of its own, down and up.
fn key(app: &mut App, window: Entity, code: KeyCode, logical: bevy::input::keyboard::Key) {
    use bevy::input::ButtonState;
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: code,
                logical_key: logical.clone(),
                state,
                text: None,
                repeat: false,
                window,
            });
        app.update();
    }
}

/// The same report: a click as Bevy resolves it (a request for the keyboard on what was
/// pressed, which goes up the tree) gives the keyboard to the field pressed, and to a box's
/// field when the box was; Tab, Shift+Tab, Down and Up step through the four fields and stay
/// among them; none of it writes into a field.
#[test]
fn a_click_gives_a_field_the_keyboard_and_tab_and_the_arrows_step_through_the_fields() {
    use bevy::input::keyboard::Key;
    use bevy::input_focus::AcquireFocus;
    use common::app::keyboard_on;
    let (mut app, window) = keyboard_on(in_editor("test:map:town"));
    // The two observers a window has from Bevy's own plugins.
    app.add_observer(bevy::input_focus::tab_navigation::acquire_focus_tab_index);
    app.add_observer(bevy::input_focus::on_window_acquire_focus_clear);
    app.update();
    // A form open in the inspector beneath: its field is not one of the modal's.
    press_edit(&mut app, EditAct::Note(omnis_vector::editor::NoteOf::Map));
    app.update();
    let world = app.world_mut();
    let mut beneath = world.query::<(&EditorPress, &bevy::text::EditableText)>();
    assert!(beneath.iter(world).next().is_some(), "the note's field");
    press_edit(&mut app, EditAct::Resize);
    app.update();
    let id = |side| Some(ControlId::Edit(EditAct::ResizeSide(side)));
    let click = |app: &mut App, on: Entity| {
        app.world_mut().trigger(AcquireFocus {
            focused_entity: on,
            window,
        });
        app.update();
    };
    // Tab passes over what is hidden, and no plugin here says what is seen.
    let world = app.world_mut();
    let mut seen = world.query::<&mut InheritedVisibility>();
    for mut v in seen.iter_mut(world) {
        *v = InheritedVisibility::VISIBLE;
    }
    let south = field(&mut app, 2);
    click(&mut app, south);
    assert_eq!(focused_side(&app), id(2), "a click on a field");
    let east = field(&mut app, EAST);
    let its_box = app.world().get::<ChildOf>(east).expect("in a box").parent();
    click(&mut app, its_box);
    assert_eq!(focused_side(&app), id(EAST), "a click on a field's box");
    for (code, logical, to) in [
        (KeyCode::Tab, Key::Tab, 2),
        (KeyCode::Tab, Key::Tab, WEST),
        (KeyCode::Tab, Key::Tab, NORTH),
        (KeyCode::ArrowDown, Key::ArrowDown, EAST),
        (KeyCode::ArrowUp, Key::ArrowUp, NORTH),
        (KeyCode::ArrowUp, Key::ArrowUp, WEST),
    ] {
        key(&mut app, window, code, logical);
        assert_eq!(focused_side(&app), id(to), "after {code:?}");
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    key(&mut app, window, KeyCode::Tab, Key::Tab);
    assert_eq!(focused_side(&app), id(2), "Shift+Tab goes back");
    assert_eq!(
        editing(&app).resizing.as_ref().unwrap().typed,
        [const { String::new() }; 4],
        "no key wrote into a field"
    );
    // A click on the panel, which is no field's box, is the way out of the fields.
    let panel = app.world().get::<ChildOf>(its_box).expect("a row").parent();
    click(&mut app, panel);
    assert_eq!(focused_side(&app), None);
}
