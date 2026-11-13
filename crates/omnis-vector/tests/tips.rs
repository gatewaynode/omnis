//! Tooltips (owner, 2026-11-13): a control's tip shows in a box beside it once the pointer has
//! rested on it, and at once while it holds the keyboard; it goes when the pointer leaves or a
//! control is pressed, and it lies inside the window.

use crate::common;

use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::{ComputedNode, Pressed, UiGlobalTransform};
use common::app::ui_editor_app;
use common::editor::{in_editor, press_edit};
use omnis_vector::editor::form::FormKind;
use omnis_vector::editor::{EditAct, tips};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::tips::{REST, Tip, TipBox};

/// The words of the terrain form's field `label`.
fn words(label: &str) -> &'static str {
    tips::field(&FormKind::Terrain(None), None, label).expect("the field has a tip")
}

/// The node that carries the tip `text`.
fn holder(app: &mut App, text: &str) -> Entity {
    let world = app.world_mut();
    let mut tips = world.query::<(Entity, &Tip)>();
    let found: Vec<Entity> = tips
        .iter(world)
        .filter(|(_, t)| t.text == text)
        .map(|(e, _)| e)
        .collect();
    assert_eq!(found.len(), 1, "one node carries {text:?}");
    found[0]
}

/// The tip boxes on screen, each with the node it hangs from.
fn boxes(app: &mut App) -> Vec<(Entity, Entity)> {
    let world = app.world_mut();
    let mut boxes = world.query_filtered::<(Entity, &ChildOf), With<TipBox>>();
    boxes.iter(world).map(|(e, p)| (e, p.parent())).collect()
}

/// The text field drawn inside `node`.
fn field_in(app: &mut App, node: Entity) -> Entity {
    let world = app.world_mut();
    let mut fields = world.query_filtered::<Entity, With<EditableText>>();
    let all: Vec<Entity> = fields.iter(world).collect();
    all.into_iter()
        .find(|f| {
            let mut up = *f;
            loop {
                if up == node {
                    return true;
                }
                match world.get::<ChildOf>(up) {
                    Some(parent) => up = parent.parent(),
                    None => return false,
                }
            }
        })
        .expect("a field in the node")
}

/// The pointer on `node`, or off it, as the window's picking writes it.
fn point(app: &mut App, node: Entity, over: bool) {
    app.world_mut().entity_mut(node).insert(Hovered(over));
}

/// Frames of 16 ms that add up to a little more than `seconds` (three frames more).
fn frames(app: &mut App, seconds: f32) {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    for _ in 0..(seconds / 0.016).ceil() as usize + 4 {
        app.update();
    }
}

/// The editor with the form for a new terrain open and nothing holding the keyboard.
fn with_form() -> App {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::TerrainAdd);
    app.update();
    if let Some(mut focus) = app.world_mut().get_resource_mut::<InputFocus>() {
        focus.clear();
    }
    app.update();
    app
}

#[test]
fn a_tip_shows_once_the_pointer_has_rested_and_goes_when_it_leaves() {
    let mut app = with_form();
    assert!(boxes(&mut app).is_empty(), "nothing rests anywhere");
    let climb = holder(&mut app, words("climb"));
    point(&mut app, climb, true);
    frames(&mut app, REST / 2.0);
    assert!(boxes(&mut app).is_empty(), "not before the rest");
    frames(&mut app, REST);
    let shown = boxes(&mut app);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].1, climb, "the box hangs from the node it explains");
    assert!(
        screen_text(app.world()).contains(&format!("{:?}", words("climb"))),
        "its words are on screen"
    );
    // Held there, it stays one box.
    frames(&mut app, REST);
    assert_eq!(boxes(&mut app).len(), 1);
    // Onto another field: the first box goes at once, the next waits its own rest.
    let join = holder(&mut app, words("join"));
    point(&mut app, climb, false);
    point(&mut app, join, true);
    app.update();
    app.update();
    assert!(boxes(&mut app).is_empty(), "the wait starts again");
    frames(&mut app, REST);
    assert_eq!(boxes(&mut app)[0].1, join);
    point(&mut app, join, false);
    app.update();
    app.update();
    assert!(boxes(&mut app).is_empty(), "gone with the pointer");
    assert!(!screen_text(app.world()).contains(&format!("{:?}", words("join"))));
}

#[test]
fn a_press_takes_the_tip_away() {
    let mut app = with_form();
    let climb = holder(&mut app, words("climb"));
    point(&mut app, climb, true);
    frames(&mut app, REST);
    assert_eq!(boxes(&mut app).len(), 1);
    let pressed = app.world_mut().spawn(Pressed).id();
    app.update();
    app.update();
    assert!(boxes(&mut app).is_empty(), "a press hides it");
    app.world_mut().entity_mut(pressed).despawn();
    frames(&mut app, REST / 2.0);
    assert!(
        boxes(&mut app).is_empty(),
        "and the rest is counted from the press"
    );
    frames(&mut app, REST);
    assert_eq!(
        boxes(&mut app).len(),
        1,
        "still resting there, it comes back"
    );
}

#[test]
fn the_field_that_holds_the_keyboard_shows_its_tip_at_once() {
    let mut app = with_form();
    let name = holder(&mut app, words("name"));
    let field = field_in(&mut app, name);
    app.world_mut()
        .insert_resource(InputFocus::from_entity(field));
    app.update();
    app.update();
    let shown = boxes(&mut app);
    assert_eq!(shown.len(), 1, "no rest for the keyboard's field");
    assert_eq!(shown[0].1, name);
    // The pointer's tip, once due, takes its place; one box at a time.
    let climb = holder(&mut app, words("climb"));
    point(&mut app, climb, true);
    frames(&mut app, REST);
    let shown = boxes(&mut app);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].1, climb);
}

#[test]
fn a_tip_s_box_lies_inside_the_window_beside_its_field() {
    let (mut app, window) = ui_editor_app();
    press_edit(&mut app, EditAct::TerrainAdd);
    app.update();
    // The glyph's field holds the keyboard, so its tip shows.
    let glyph = holder(&mut app, words("glyph"));
    let field = field_in(&mut app, glyph);
    app.world_mut()
        .insert_resource(InputFocus::from_entity(field));
    frames(&mut app, 0.1);
    let shown = boxes(&mut app);
    assert_eq!(shown.len(), 1, "the glyph's tip");
    let rect = |app: &App, e: Entity| {
        let node = app.world().get::<ComputedNode>(e).expect("laid out");
        let at = app.world().get::<UiGlobalTransform>(e).expect("placed");
        Rect::from_center_size(at.translation, node.size())
    };
    let (tip, row) = (rect(&app, shown[0].0), rect(&app, shown[0].1));
    let size = app
        .world()
        .get::<Window>(window)
        .expect("the window")
        .resolution
        .physical_size()
        .as_vec2();
    assert!(
        tip.width() > 50.0 && tip.height() > 10.0,
        "it has a size: {tip:?}"
    );
    assert!(
        tip.min.x >= 0.0 && tip.min.y >= 0.0 && tip.max.x <= size.x && tip.max.y <= size.y,
        "inside the window {size:?}: {tip:?}"
    );
    assert!(
        tip.max.x <= row.min.x + 1.0,
        "left of its row, over the plan and off the fields: {tip:?} against {row:?}"
    );
}

/// A tip reads as a sentence: something, in ASCII, ending in a full stop.
fn sentence(tip: Option<&String>, of: &str) {
    let tip = tip.unwrap_or_else(|| panic!("{of} has no tip"));
    assert!(tip.len() > 10 && tip.is_ascii(), "{of}: {tip:?}");
    assert!(tip.ends_with('.'), "{of}: {tip:?}");
    assert!(!tip.contains("  "), "{of} runs its words together: {tip:?}");
}

/// Every field of every form the editor opens has its tip, in the form's own fields and in a
/// row of each of its lists.
#[test]
fn every_field_of_every_form_is_explained() {
    use omnis_sim::omnis_core::{Facing, Level};
    use omnis_vector::editor::form::{At, Form};
    use omnis_vector::editor::forms::Placing;
    use omnis_vector::screen::ControlId;
    let data = common::levels_data();
    let id = common::map(&data, "levels:map:tower");
    let def = &data.maps[&id].def;
    let at = At {
        level: Level::GROUND,
        x: 0,
        y: 0,
        facing: Facing::North,
    };
    let mut forms: Vec<Form> = Placing::ALL
        .iter()
        .map(|kind| Form::placing(*kind, def, &data, at, None))
        .collect();
    forms.push(Form::terrain(def, None, at));
    forms.push(Form::level_add(def, at));
    let level = def.levels.first().expect("the tower has levels").level;
    forms.push(Form::level(def, level, at).expect("its form"));
    forms.push(Form::note("k".to_owned(), "Note".to_owned(), None, at));
    forms.push(Form::random(def, at));
    forms.push(Form::rest_event(def, at));
    let mut fields = 0;
    for mut form in forms {
        for l in 0..form.lists.len() {
            let l = u8::try_from(l).unwrap();
            if form.rows_of(l).is_empty() {
                form.add_row(l).expect("a first row");
            }
        }
        for row in form.rows() {
            for control in &row.controls {
                if let ControlId::Edit(EditAct::Field(_)) = control.id {
                    fields += 1;
                    sentence(
                        control.tip.as_ref(),
                        &format!("{:?} {:?}", form.kind, control.label),
                    );
                }
            }
        }
    }
    assert!(fields > 50, "the forms' fields were walked: {fields}");
}

/// Every control of the editor's three columns has its tip, whatever the palette holds: the
/// tools, the palette's entries and what stands under each, the files' buttons, the bar, the
/// inspector's buttons and an open form's.
#[test]
fn every_control_of_the_editor_s_columns_is_explained() {
    use omnis_vector::editor::brush;
    use omnis_vector::editor::view;
    use omnis_vector::screen::{ControlId, Screen};
    use omnis_vector::shell::editor::Editing;
    use omnis_vector::shell::session::Packs;
    let mut app = in_editor("test:map:town");
    let entries = {
        let def = app.world().resource::<Editing>().def().expect("a map");
        brush::palette(&def).len()
    };
    let mut seen = 0;
    let mut check = |app: &App| {
        let editor = &app.world().resource::<Editing>().0;
        let data = app.world().resource::<Packs>().data.clone();
        let screens: [Screen; 3] = [
            view::tools(editor, &data),
            view::bar(editor, &data),
            view::inspector(editor, &data),
        ];
        for screen in &screens {
            for control in screen.controls() {
                // A row of the list under a portal's link field is a portal's own id.
                if matches!(control.id, ControlId::Edit(EditAct::Link(_))) {
                    continue;
                }
                seen += 1;
                sentence(
                    control.tip.as_ref(),
                    &format!("{:?} {:?}", control.id, control.label),
                );
            }
        }
    };
    check(&app);
    for i in 0..entries {
        press_edit(&mut app, EditAct::Brush(u8::try_from(i).unwrap()));
        check(&app);
    }
    press_edit(&mut app, EditAct::TerrainAdd);
    check(&app);
    press_edit(&mut app, EditAct::Cancel);
    press_edit(&mut app, EditAct::Random);
    check(&app);
    assert!(seen > 500, "the columns were walked: {seen}");
}
