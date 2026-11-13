//! The form's layout in the editor's right column (owner, 2026-11-13): a field is a dim label
//! in a column of its own and an input in its own box, every input starting at one place; the
//! fields stand in named groups; a yes or no is a tick box.

use crate::common;

use common::app::ui_editor_app;
use common::editor::{editing, press_edit};
use omnis_vector::editor::EditAct;
use omnis_vector::shell::screen_text::screen_text;

/// A line of `screen_text` as `(x, y, width, height, rest)`.
fn rect(line: &str) -> Option<(f32, f32, f32, f32, &str)> {
    let line = line.trim_start();
    let (at, rest) = line.split_once(' ')?;
    let (size, rest) = rest.split_once(' ')?;
    let (x, y) = at.split_once(',')?;
    let (w, h) = size.split_once('x')?;
    Some((
        x.parse().ok()?,
        y.parse().ok()?,
        w.parse().ok()?,
        h.parse().ok()?,
        rest,
    ))
}

/// The rectangle of the line whose text is exactly `words` (a label), as drawn.
fn label(text: &str, words: &str) -> (f32, f32, f32, f32) {
    let quoted = format!("{words:?}");
    let found: Vec<_> = text
        .lines()
        .filter_map(rect)
        .filter(|r| r.4 == quoted)
        .collect();
    assert_eq!(found.len(), 1, "one {words:?} in\n{text}");
    (found[0].0, found[0].1, found[0].2, found[0].3)
}

/// The rectangle of the first drawn thing after the label `words` on its row: the input's.
fn input_after(text: &str, words: &str) -> (f32, f32, f32, f32) {
    let quoted = format!("{words:?}");
    let mut lines = text.lines().filter_map(rect);
    lines
        .by_ref()
        .find(|r| r.4 == quoted)
        .unwrap_or_else(|| panic!("{words:?} in\n{text}"));
    let next = lines.next().expect("something after the label");
    (next.0, next.1, next.2, next.3)
}

#[test]
fn a_form_s_labels_stand_in_one_column_and_its_inputs_start_at_one_place() {
    let (mut app, window) = ui_editor_app();
    press_edit(&mut app, EditAct::TerrainAdd);
    app.update();
    let text = screen_text(app.world());
    #[allow(clippy::cast_precision_loss)]
    let edge = app
        .world()
        .get::<bevy::prelude::Window>(window)
        .expect("the window")
        .resolution
        .physical_width() as f32;
    let names = [
        "glyph",
        "name",
        "floor",
        "block",
        "step minutes",
        "climb",
        "join",
        "role",
    ];
    let first = label(&text, names[0]);
    for name in names {
        let at = label(&text, name);
        assert!(
            (at.0 - first.0).abs() < 1.0,
            "{name} starts with the labels\n{text}"
        );
        let input = input_after(&text, name);
        assert!(
            input.0 >= at.0 + at.2,
            "{name}'s input is right of its label, not around it\n{text}"
        );
        assert!(
            (input.1 - at.1).abs() < 30.0,
            "{name}'s input is on its row\n{text}"
        );
    }
    // Every input's box starts at one place. The reader gives the words' rectangles, not the
    // boxes': a text field's words start at its box's padding and a choice's or a number's
    // first words are its arrow's glyph, centred in the arrow. So inputs of one kind start at
    // exactly one x, and the kinds within an arrow's width of each other.
    let xs =
        |names: &[&str]| -> Vec<f32> { names.iter().map(|n| input_after(&text, n).0).collect() };
    let fields = xs(&["glyph", "name", "floor", "block"]);
    let arrows = xs(&["step minutes", "climb", "join", "role"]);
    for x in &fields {
        assert!(
            (x - fields[0]).abs() < 1.0,
            "the fields start at one x: {fields:?}"
        );
    }
    for x in &arrows {
        assert!(
            (x - arrows[0]).abs() < 1.0,
            "the arrows start at one x: {arrows:?}"
        );
    }
    assert!(
        (arrows[0] - fields[0]).abs() < 20.0,
        "{fields:?} {arrows:?}"
    );
    // Nothing of the form runs past the column's right edge, which is the window's.
    let right = text
        .lines()
        .filter_map(rect)
        .map(|r| r.0 + r.2)
        .fold(0.0, f32::max);
    assert!(right <= edge + 0.5, "drawn to {right} of {edge}\n{text}");
}

#[test]
fn a_form_s_fields_stand_under_their_groups() {
    let (mut app, _window) = ui_editor_app();
    press_edit(&mut app, EditAct::TerrainAdd);
    app.update();
    let text = screen_text(app.world());
    let y = |words: &str| label(&text, words).1;
    assert!(y("IDENTITY") < y("glyph"), "{text}");
    assert!(y("name") < y("TILES") && y("TILES") < y("floor"), "{text}");
    assert!(
        y("block") < y("MOVEMENT") && y("MOVEMENT") < y("passable"),
        "{text}"
    );
    assert!(y("join") < y("LOOK") && y("LOOK") < y("role"), "{text}");
    assert!(
        y("face") < y("MAP COLOUR") && y("MAP COLOUR") < y("red"),
        "{text}"
    );
}

#[test]
fn a_yes_or_no_is_a_tick_box_one_press_turns() {
    let (mut app, _window) = ui_editor_app();
    press_edit(&mut app, EditAct::TerrainAdd);
    app.update();
    let form = editing(&app).form.clone().expect("the terrain form");
    let i = form
        .fields
        .iter()
        .position(|f| f.label == "opaque")
        .map(|i| u8::try_from(i).unwrap())
        .unwrap();
    assert!(!form.flag_of("opaque"));
    let text = screen_text(app.world());
    assert!(
        !text.contains("\"no\"") && !text.contains("\"yes\""),
        "no stepper\n{text}"
    );
    let shape = editing(&app).form.as_ref().map(|f| f.fields.len());
    press_edit(&mut app, EditAct::Field(i));
    assert!(editing(&app).form.as_ref().unwrap().flag_of("opaque"));
    let text = screen_text(app.world());
    let (_, _, w, h) = input_after(&text, "opaque");
    assert!(w < 40.0 && h < 40.0, "a small box: {w} by {h}\n{text}");
    assert!(text.contains("[button] \"x\""), "ticked\n{text}");
    press_edit(&mut app, EditAct::Field(i));
    assert!(!editing(&app).form.as_ref().unwrap().flag_of("opaque"));
    assert_eq!(shape, editing(&app).form.as_ref().map(|f| f.fields.len()));
}
