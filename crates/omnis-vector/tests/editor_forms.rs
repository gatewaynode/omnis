//! The editor's attribute bar and forms (M5.6, `tasks/plans/m5-editor-v1.md` row 6): the bar
//! over the plan edits the cursor's cell where it stands (terrain, height, a stair's rise), a
//! column whose words change keeps its nodes, and no column takes the keyboard without a field.

use crate::common;

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use common::editor::{arrow, editing, in_editor, in_editor_on, panel, press_edit};
use omnis_sim::omnis_core::Facing;
use omnis_vector::editor::EditAct;
use omnis_vector::shell::editor::{Column, Editing};
use omnis_vector::shell::screen_text::screen_text;

#[test]
fn the_bar_steps_the_cursor_cell_s_terrain_and_height_one_undo_step_each() {
    let mut app = in_editor("test:map:town");
    let start = editing(&app).cursor;
    let tile = editing(&app).cell_terrain().expect("the start has ground");
    let terrains = editing(&app).def().expect("a map").terrains.len();
    let text = screen_text(app.world());
    assert!(
        text.contains(&format!("Cell ({}, {})  level 0", start.0, start.1)),
        "{text}"
    );
    assert!(
        text.contains("Terrain") && text.contains("Height"),
        "{text}"
    );
    // The terrain steps through the map's list and round through no ground.
    arrow(&mut app, EditAct::CellTerrain, true);
    let next = usize::from(tile) + 1;
    let expected = (next < terrains).then(|| u8::try_from(next).unwrap());
    assert_eq!(editing(&app).cell_terrain(), expected);
    assert_eq!(editing(&app).cursor, start, "the cursor stays");
    for _ in next..terrains {
        arrow(&mut app, EditAct::CellTerrain, true);
    }
    assert_eq!(
        editing(&app).cell_terrain(),
        None,
        "past the last terrain lies no ground"
    );
    let text = screen_text(app.world());
    assert!(
        text.contains("No ground") && text.contains("no ground"),
        "{text}"
    );
    arrow(&mut app, EditAct::CellTerrain, true);
    assert_eq!(editing(&app).cell_terrain(), Some(0), "and round");
    // The height steps by an eighth either way.
    arrow(&mut app, EditAct::CellHeight, true);
    assert_eq!(editing(&app).cell_height(), 1);
    arrow(&mut app, EditAct::CellHeight, false);
    arrow(&mut app, EditAct::CellHeight, false);
    assert_eq!(editing(&app).cell_height(), -1);
    let text = screen_text(app.world());
    assert!(text.contains("Height -1 eighths"), "{text}");
    // One undo step each: the three heights, then the terrains one by one.
    for _ in 0..3 {
        press_edit(&mut app, EditAct::Undo);
    }
    assert_eq!(editing(&app).cell_height(), 0);
    assert_eq!(editing(&app).cell_terrain(), Some(0));
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(editing(&app).cell_terrain(), None);
}

#[test]
fn rise_is_offered_on_a_stair_cell_only_and_turns_the_way_it_climbs() {
    let mut app = in_editor_on(&["crates/omnis-sim/tests/packs/levels"], "levels:map:tower");
    // The ground level's stair up at (2, 0) rises west.
    app.world_mut().resource_mut::<Editing>().point((2, 0));
    app.update();
    assert_eq!(editing(&app).cell_rise(), Some(Facing::West));
    let text = screen_text(app.world());
    assert!(text.contains("Rise") && text.contains("west"), "{text}");
    arrow(&mut app, EditAct::CellRise, true);
    assert_eq!(editing(&app).cell_rise(), Some(Facing::North));
    let def = editing(&app).def().expect("a map");
    assert!(
        def.stairs
            .iter()
            .any(|s| s.level == 0 && s.x == 2 && s.y == 0 && s.rise == Facing::North),
        "{:?}",
        def.stairs
    );
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(
        editing(&app).cell_rise(),
        Some(Facing::West),
        "one undo step"
    );
    // Off a stair, the bar offers no rise.
    app.world_mut().resource_mut::<Editing>().point((0, 0));
    app.update();
    assert_eq!(editing(&app).cell_rise(), None);
    let text = screen_text(app.world());
    assert!(!text.contains("Rise"), "{text}");
}

#[test]
fn a_column_whose_words_change_keeps_its_nodes_and_no_column_takes_the_keyboard() {
    let mut app = in_editor("test:map:town");
    // The first step makes the map dirty, which wakes Undo: the tools column's shape changes.
    arrow(&mut app, EditAct::CellHeight, true);
    let (bar, _) = panel(&mut app, Column::Bar);
    let (tools, _) = panel(&mut app, Column::Tools);
    let (inspector, _) = panel(&mut app, Column::Inspector);
    // The second changes words only: the bar's height, the inspector's height line.
    arrow(&mut app, EditAct::CellHeight, true);
    assert_eq!(editing(&app).cell_height(), 2);
    assert_eq!(
        panel(&mut app, Column::Bar).0,
        bar,
        "the bar's words changed, not its shape"
    );
    assert_eq!(panel(&mut app, Column::Tools).0, tools);
    assert_eq!(panel(&mut app, Column::Inspector).0, inspector);
    let text = screen_text(app.world());
    assert!(text.contains("Height 2 eighths"), "{text}");
    // Each column's words are its own: the bar's title overwrote no other column's.
    assert!(
        text.contains("EDITOR  ") && text.contains("INSPECTOR") && text.contains("Cell ("),
        "{text}"
    );
    assert!(
        app.world()
            .get_resource::<InputFocus>()
            .is_none_or(|f| f.get().is_none()),
        "no field, so nothing takes the keyboard"
    );
}

// ---- 6b: the forms and the placements ----

fn form_text(app: &App) -> String {
    screen_text(app.world())
}

#[test]
fn a_portal_is_placed_from_the_form_then_edited_and_removed() {
    use bevy::input::ButtonState;
    use common::app::{keyboard, keyboard_on};
    use common::editor::{field_entity, field_of, mouse, point_at};
    use omnis_vector::editor::form::FormKind;
    use omnis_vector::shell::screen::Pressable;
    let (mut app, window) = keyboard_on(in_editor("test:map:town"));
    app.update();
    app.world_mut().resource_mut::<Editing>().point((3, 4));
    app.update();
    // Add places the chosen kind; a portal is the first.
    assert_eq!(editing(&app).place_kind, 0);
    press_edit(&mut app, EditAct::Add);
    let form = editing(&app).form.clone().expect("a form opened");
    assert_eq!(form.kind, FormKind::Portal);
    let text = form_text(&app);
    assert!(
        text.contains("Portal at (3, 4)") && text.contains("to map") && text.contains("marker"),
        "{text}"
    );
    // Its first text field has the keyboard, and the game's router never sees it.
    let marker = EditAct::Field(field_of(&form, "marker"));
    let field = field_entity(&mut app, marker).expect("the marker field");
    let focused = app.world().resource::<InputFocus>().get();
    assert_eq!(focused, Some(field), "the first field takes the keyboard");
    assert!(
        app.world().get::<Pressable>(field).is_none(),
        "an editor field"
    );
    // Typing on the keyboard reaches the form without rebuilding the field.
    keyboard(&mut app, window, "signpost");
    app.update();
    assert_eq!(
        editing(&app).form.as_ref().unwrap().text_of("marker"),
        "signpost"
    );
    assert_eq!(
        field_entity(&mut app, marker),
        Some(field),
        "the field stands"
    );
    // The choice and the numbers step with the arrows.
    let to_map = EditAct::Field(field_of(&form, "to map"));
    while editing(&app).form.as_ref().unwrap().choice_of("to map") != "test:map:meadow" {
        arrow(&mut app, to_map, true);
    }
    let to_x = EditAct::Field(field_of(&form, "to x"));
    for _ in 0..3 {
        arrow(&mut app, to_x, true);
    }
    // A plan click takes the keyboard from the field; the form stands.
    point_at(&mut app, (5, 3), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(editing(&app).cursor, (5, 3));
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert!(editing(&app).form.is_some(), "the form stands");
    assert_eq!(field_entity(&mut app, marker), Some(field), "unrebuilt");
    // Apply places it where the form was opened, one undo step.
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    let def = editing(&app).def().unwrap();
    let portal = def
        .portals
        .iter()
        .find(|p| p.x == 3 && p.y == 4)
        .expect("the portal");
    assert_eq!(portal.to_map, "test:map:meadow");
    assert_eq!((portal.to_x, portal.to_y), (3, 0));
    assert_eq!(portal.marker.as_deref(), Some("signpost"));
    // The inspector lists it at its cell, with Edit and Remove.
    app.world_mut().resource_mut::<Editing>().point((3, 4));
    app.update();
    let text = form_text(&app);
    assert!(text.contains("Portal to test:map:meadow"), "{text}");
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().expect("the portal's form");
    assert_eq!(form.text_of("marker"), "signpost");
    assert_eq!(form.choice_of("to map"), "test:map:meadow");
    assert!(form.remove.is_some(), "Remove is offered");
    press_edit(&mut app, EditAct::Cancel);
    assert!(editing(&app).form.is_none());
    press_edit(&mut app, EditAct::RemoveAt(0));
    assert!(editing(&app).here().is_empty());
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(editing(&app).here().len(), 1, "the removal undone");
    press_edit(&mut app, EditAct::Undo);
    assert!(editing(&app).here().is_empty(), "the placing undone");
}

#[test]
fn an_encounter_s_stacks_are_rows_and_a_bad_id_is_refused_with_the_form_standing() {
    use common::editor::{field_of, type_into};
    use omnis_sim::omnis_data::Disposition;
    let mut app = in_editor("test:map:town");
    app.world_mut().resource_mut::<Editing>().point((3, 4));
    app.update();
    arrow(&mut app, EditAct::PlaceKind, true);
    press_edit(&mut app, EditAct::Add);
    let form = editing(&app).form.clone().expect("a form opened");
    let text = form_text(&app);
    assert!(
        text.contains("Encounter at (3, 4)") && text.contains("Stacks"),
        "{text}"
    );
    type_into(&mut app, EditAct::Field(field_of(&form, "id")), "two words");
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_some(), "the form stands");
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("one word")),
        "{:?}",
        editing(&app).message
    );
    type_into(&mut app, EditAct::Field(field_of(&form, "id")), "rats");
    // Two rows added, the second removed.
    press_edit(&mut app, EditAct::ListAdd(0));
    press_edit(&mut app, EditAct::ListAdd(0));
    assert_eq!(editing(&app).form.as_ref().unwrap().rows_of(0).len(), 2);
    press_edit(&mut app, EditAct::ListRemove(0, 1));
    let rows = editing(&app).form.as_ref().unwrap().rows_of(0);
    assert_eq!(rows.len(), 1);
    let (creature, count) = (rows[0][0], rows[0][1]);
    type_into(
        &mut app,
        EditAct::Field(u8::try_from(creature).unwrap()),
        "test:monster:giant_rat",
    );
    arrow(&mut app, EditAct::Field(u8::try_from(count).unwrap()), true);
    // The disposition steps round; once is a flag.
    arrow(
        &mut app,
        EditAct::Field(field_of(&form, "disposition")),
        true,
    );
    arrow(&mut app, EditAct::Field(field_of(&form, "once")), true);
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    let def = editing(&app).def().unwrap();
    let e = def
        .encounters
        .iter()
        .find(|e| e.id == "rats")
        .expect("placed");
    assert_eq!((e.x, e.y), (3, 4));
    assert_eq!(e.stacks, vec![("test:monster:giant_rat".to_owned(), 2)]);
    assert_eq!(e.disposition, Disposition::Wary);
    assert!(e.once && e.stand.is_none());
    // The loader's verdict follows: the monster is real, so the map stays valid.
    assert!(
        editing(&app).status.errors.is_empty(),
        "{:?}",
        editing(&app).status.errors
    );
}

#[test]
fn the_start_the_random_table_and_the_rest_events_have_their_rows() {
    use common::editor::{field_of, type_into};
    use omnis_sim::omnis_core::rng::Dice;
    let mut app = in_editor("test:map:town");
    app.world_mut().resource_mut::<Editing>().point((3, 4));
    app.update();
    press_edit(&mut app, EditAct::SetStart);
    let def = editing(&app).def().unwrap();
    assert_eq!(def.start, (3, 4, Facing::West));
    let text = form_text(&app);
    assert!(text.contains("Start (3, 4) facing west"), "{text}");
    // The town has no random table: the form starts empty, one entry added.
    assert!(text.contains("No random encounters"), "{text}");
    press_edit(&mut app, EditAct::Random);
    let form = editing(&app).form.clone().expect("the table's form");
    press_edit(&mut app, EditAct::ListAdd(0));
    let rows = editing(&app).form.as_ref().unwrap().rows_of(0);
    type_into(
        &mut app,
        EditAct::Field(u8::try_from(rows[0][2]).unwrap()),
        "test:monster:giant_rat 1d2, test:monster:giant_rat 2d4+1",
    );
    arrow(
        &mut app,
        EditAct::Field(field_of(&form, "chance percent")),
        true,
    );
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    let random = editing(&app).def().unwrap().random.expect("a table");
    assert_eq!(random.chance_percent, 1);
    assert_eq!(random.entries.len(), 1);
    assert_eq!(
        random.entries[0].stacks,
        vec![
            ("test:monster:giant_rat".to_owned(), Dice::new(1, 2)),
            (
                "test:monster:giant_rat".to_owned(),
                Dice {
                    count: 2,
                    sides: 4,
                    modifier: 1
                }
            ),
        ]
    );
    let text = form_text(&app);
    assert!(text.contains("Random encounters: 1 entry at 1%"), "{text}");
    // Editing it again shows the line as written; Remove clears the table.
    press_edit(&mut app, EditAct::Random);
    let rows = editing(&app).form.as_ref().unwrap().rows_of(0);
    assert_eq!(
        editing(&app).form.as_ref().unwrap().text(rows[0][2]),
        "test:monster:giant_rat 1d2, test:monster:giant_rat 2d4+1"
    );
    press_edit(&mut app, EditAct::Discard);
    assert!(editing(&app).def().unwrap().random.is_none());
    // The town's rest events: the first edited (its chance up by one), the second removed.
    let before = editing(&app).def().unwrap().rest_events.clone();
    assert_eq!(before.len(), 2);
    press_edit(&mut app, EditAct::RestEdit(0));
    let form = editing(&app).form.clone().expect("the rest event's form");
    let text = form_text(&app);
    assert!(text.contains("Rest event: street on Any"), "{text}");
    arrow(
        &mut app,
        EditAct::Field(field_of(&form, "chance per mille")),
        true,
    );
    press_edit(&mut app, EditAct::Apply);
    let after = editing(&app).def().unwrap().rest_events.clone();
    assert_eq!(after[0].chance, before[0].chance + 1);
    assert_eq!(after[0].terrain, before[0].terrain);
    press_edit(&mut app, EditAct::RestRemove(1));
    assert_eq!(editing(&app).def().unwrap().rest_events.len(), 1);
}
