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

/// Place a `kind` with its palette stamp by a click on `cell` (the editor's column, step 3): the
/// stamp chosen by its position in the palette, the detail as it stands.
fn stamp(app: &mut App, kind: omnis_vector::editor::forms::Placing, cell: (u16, u16)) {
    use bevy::input::ButtonState;
    use common::editor::{mouse, point_at};
    use omnis_vector::editor::brush::{self, Entry};
    let def = editing(app).def().expect("a map");
    let i = brush::palette(&def)
        .iter()
        .position(|e| *e == Entry::Stamp(kind))
        .expect("the stamp");
    press_edit(app, EditAct::Brush(u8::try_from(i).unwrap()));
    point_at(app, cell, None);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

#[test]
fn a_portal_stamp_lands_at_the_chosen_map_s_start_then_its_form_edits_and_removes_it() {
    use common::editor::field_of;
    use omnis_vector::editor::form::FormKind;
    use omnis_vector::editor::forms::Placing;
    use omnis_vector::shell::session::Packs;
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    app.world_mut()
        .resource_mut::<Editing>()
        .stamps
        .set(Placing::Portal, "test:map:meadow".to_owned());
    stamp(&mut app, Placing::Portal, (3, 4));
    assert!(editing(&app).form.is_none(), "a stamp opens no form");
    let def = editing(&app).def().unwrap();
    let portal = def
        .portals
        .iter()
        .find(|p| p.x == 3 && p.y == 4)
        .expect("the portal");
    assert_eq!(portal.to_map, "test:map:meadow");
    let meadow = data
        .maps
        .values()
        .find(|m| m.def.id == "test:map:meadow")
        .unwrap();
    assert_eq!(
        (portal.to_x, portal.to_y, portal.to_facing),
        meadow.def.start,
        "a new portal lands at the map's start"
    );
    let marker = portal.marker.clone().expect("a marker of the tileset");
    // The inspector lists it at its cell, with Edit and Remove; its form holds what landed.
    let text = form_text(&app);
    assert!(text.contains("Portal to test:map:meadow"), "{text}");
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().expect("the portal's form");
    assert_eq!(form.kind, FormKind::Portal);
    assert_eq!(form.choice_of("to map"), "test:map:meadow");
    assert_eq!(form.choice_of("marker"), marker);
    assert!(form.remove.is_some(), "Remove is offered");
    let text = form_text(&app);
    assert!(
        text.contains("Portal at (3, 4)") && text.contains("to map") && text.contains("marker"),
        "{text}"
    );
    // The choice and the numbers step with the arrows; Apply changes it in one step.
    let to_map = EditAct::Field(field_of(&form, "to map"));
    while editing(&app).form.as_ref().unwrap().choice_of("to map") != "test:map:town" {
        arrow(&mut app, to_map, true);
    }
    let to_x = EditAct::Field(field_of(&form, "to x"));
    for _ in 0..3 {
        arrow(&mut app, to_x, true);
    }
    let steps = editing(&app).status.undo;
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    let def = editing(&app).def().unwrap();
    let portal = def
        .portals
        .iter()
        .find(|p| p.x == 3 && p.y == 4)
        .expect("the portal");
    assert_eq!(portal.to_map, "test:map:town");
    assert_eq!(portal.to_x, meadow.def.start.0 + 3);
    assert_eq!(editing(&app).status.undo, steps + 1);
    press_edit(&mut app, EditAct::RemoveAt(0));
    assert!(editing(&app).here().is_empty());
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(editing(&app).here().len(), 1, "the removal undone");
    press_edit(&mut app, EditAct::Undo);
    press_edit(&mut app, EditAct::Undo);
    assert!(
        editing(&app).here().is_empty(),
        "the edit and the placing undone"
    );
}

/// The first text field of a form takes the keyboard when the form opens; typing reaches the
/// form without rebuilding the field; a plan click takes the keyboard back and the form stands.
#[test]
fn the_first_text_field_takes_the_keyboard_and_a_plan_click_takes_it_back() {
    use bevy::input::ButtonState;
    use common::app::{keyboard, keyboard_on};
    use common::editor::{field_entity, field_of, mouse, point_at};
    use omnis_vector::editor::NoteOf;
    use omnis_vector::shell::screen::Pressable;
    let (mut app, window) = keyboard_on(in_editor("test:map:town"));
    app.update();
    app.world_mut().resource_mut::<Editing>().point((3, 4));
    app.update();
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    let form = editing(&app).form.clone().expect("the note's form");
    let note = EditAct::Field(field_of(&form, "note"));
    let field = field_entity(&mut app, note).expect("the note field");
    let focused = app.world().resource::<InputFocus>().get();
    assert_eq!(focused, Some(field), "the first field takes the keyboard");
    assert!(
        app.world().get::<Pressable>(field).is_none(),
        "an editor field"
    );
    keyboard(&mut app, window, "signpost");
    app.update();
    assert_eq!(
        editing(&app).form.as_ref().unwrap().text_of("note"),
        "signpost"
    );
    assert_eq!(
        field_entity(&mut app, note),
        Some(field),
        "the field stands"
    );
    point_at(&mut app, (5, 3), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(editing(&app).cursor, (5, 3));
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert!(editing(&app).form.is_some(), "the form stands");
    assert_eq!(field_entity(&mut app, note), Some(field), "unrebuilt");
    press_edit(&mut app, EditAct::Cancel);
}

#[test]
fn an_encounter_s_stacks_are_rows_and_a_bad_id_is_refused_with_the_form_standing() {
    use common::editor::{field_of, type_into};
    use omnis_sim::omnis_data::Disposition;
    use omnis_vector::editor::forms::Placing;
    let mut app = in_editor("test:map:town");
    stamp(&mut app, Placing::Encounter, (3, 4));
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().expect("a form opened");
    let text = form_text(&app);
    assert!(
        text.contains("Encounter encounter_3_4") && text.contains("Stacks"),
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
    type_into(
        &mut app,
        EditAct::Field(field_of(&form, "id")),
        "encounter_3_4",
    );
    // An encounter starts with one stack; two rows added, then the last two removed.
    press_edit(&mut app, EditAct::ListAdd(0));
    press_edit(&mut app, EditAct::ListAdd(0));
    assert_eq!(editing(&app).form.as_ref().unwrap().rows_of(0).len(), 3);
    press_edit(&mut app, EditAct::ListRemove(0, 2));
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
    assert_eq!(
        def.encounters
            .iter()
            .filter(|e| (e.x, e.y) == (3, 4))
            .count(),
        1,
        "the one encounter, changed"
    );
    let e = def
        .encounters
        .iter()
        .find(|e| e.id == "encounter_3_4")
        .expect("placed");
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

// ---- 6c: terrains, levels and notes ----

#[test]
fn a_terrain_is_added_edited_and_removed_through_the_form_and_the_rules_hold() {
    use common::editor::{field_of, type_into};
    use omnis_vector::editor::brush::Entry;
    let mut app = in_editor("test:map:town");
    let before = editing(&app).def().unwrap().terrains.len();
    press_edit(&mut app, EditAct::TerrainAdd);
    let form = editing(&app).form.clone().expect("the terrain form");
    assert!(form_text(&app).contains("New terrain"));
    type_into(&mut app, EditAct::Field(field_of(&form, "glyph")), "m");
    type_into(&mut app, EditAct::Field(field_of(&form, "name")), "mud");
    type_into(&mut app, EditAct::Field(field_of(&form, "floor")), "grass");
    // A colour set outright, as the slider would.
    app.world_mut()
        .resource_mut::<Editing>()
        .set_field(field_of(&form, "red"), 90);
    arrow(&mut app, EditAct::Field(field_of(&form, "join")), true);
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    let def = editing(&app).def().unwrap();
    assert_eq!(def.terrains.len(), before + 1);
    let mud = def.terrains.last().unwrap();
    assert_eq!(
        (mud.glyph, mud.name.as_str(), mud.floor.as_str()),
        ('m', "mud", "grass")
    );
    assert_eq!(mud.color, (90, 128, 128));
    assert_eq!(mud.join, Some(omnis_sim::omnis_data::map::Join::Up));
    assert!(mud.passable);
    let data = app
        .world()
        .resource::<omnis_vector::shell::session::Packs>()
        .data
        .clone();
    let swatch = omnis_vector::editor::view::tools(editing(&app), &data)
        .control(omnis_vector::screen::ControlId::Edit(EditAct::Brush(
            u8::try_from(before).unwrap(),
        )))
        .map(|c| c.kind.clone());
    assert!(
        matches!(
            swatch,
            Some(omnis_vector::screen::Kind::Swatch {
                glyph: 'm',
                rgb: (90, 128, 128),
                ..
            })
        ),
        "the new swatch: {swatch:?}"
    );
    // Edit terrain works on the brush's terrain; a taken glyph is refused by the document.
    press_edit(&mut app, EditAct::Brush(u8::try_from(before).unwrap()));
    assert_eq!(
        editing(&app).entry,
        Entry::Terrain(u8::try_from(before).unwrap())
    );
    press_edit(&mut app, EditAct::TerrainEdit);
    let form = editing(&app).form.clone().expect("the mud form");
    assert!(form_text(&app).contains("Terrain m mud"));
    assert_eq!(form.text_of("name"), "mud");
    type_into(&mut app, EditAct::Field(field_of(&form, "glyph")), "r");
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_some(), "refused: the form stands");
    assert!(editing(&app).message.is_some());
    type_into(&mut app, EditAct::Field(field_of(&form, "glyph")), "m");
    type_into(&mut app, EditAct::Field(field_of(&form, "name")), "marsh");
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    assert_eq!(
        editing(&app).def().unwrap().terrains.last().unwrap().name,
        "marsh"
    );
    // Remove: an unused terrain goes; one in use is refused in the document's words.
    press_edit(&mut app, EditAct::TerrainEdit);
    press_edit(&mut app, EditAct::Discard);
    assert!(editing(&app).form.is_none());
    assert_eq!(editing(&app).def().unwrap().terrains.len(), before);
    press_edit(&mut app, EditAct::Brush(0));
    press_edit(&mut app, EditAct::TerrainEdit);
    press_edit(&mut app, EditAct::Discard);
    assert!(editing(&app).form.is_some(), "the street is in use");
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("use")),
        "{:?}",
        editing(&app).message
    );
    press_edit(&mut app, EditAct::Cancel);
    // Each change was one undo step: the removal, the rename, the addition.
    for _ in 0..3 {
        press_edit(&mut app, EditAct::Undo);
    }
    assert_eq!(editing(&app).def().unwrap().terrains.len(), before);
    assert!(!form_text(&app).contains("mud"));
}

#[test]
fn a_level_is_added_named_and_removed_and_the_ground_has_no_form() {
    use common::editor::{field_of, type_into};
    use omnis_sim::omnis_core::Level;
    let mut app = in_editor("test:map:town");
    // The ground level is the map itself: its button is dim with the reason (and takes no press).
    let text = form_text(&app);
    assert!(text.contains("the ground level is the map"), "{text}");
    press_edit(&mut app, EditAct::LevelAdd);
    let form = editing(&app).form.clone().expect("the level form");
    assert_eq!(form.number_of("level"), 1, "the first free level above");
    type_into(
        &mut app,
        EditAct::Field(field_of(&form, "name")),
        "test:text:map.town.loft",
    );
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    assert_eq!(editing(&app).levels(), vec![Level::GROUND, Level(1)]);
    arrow(&mut app, EditAct::Level, true);
    assert_eq!(editing(&app).level, Level(1));
    let text = form_text(&app);
    assert!(
        text.contains("Level 1") || text.contains("level 1"),
        "{text}"
    );
    press_edit(&mut app, EditAct::LevelEdit);
    let form = editing(&app).form.clone().expect("level 1's form");
    assert_eq!(form.text_of("name"), "test:text:map.town.loft");
    arrow(&mut app, EditAct::Field(field_of(&form, "base")), true);
    press_edit(&mut app, EditAct::Apply);
    let def = editing(&app).def().unwrap();
    assert_eq!(def.levels[0].base, 1);
    press_edit(&mut app, EditAct::LevelEdit);
    press_edit(&mut app, EditAct::Discard);
    assert_eq!(editing(&app).levels(), vec![Level::GROUND]);
    assert_eq!(editing(&app).level, Level::GROUND, "shown again");
}

/// A form replaced by another of the same shape (a note's form at another cell, with the
/// message already standing) draws the new form's values: the kit rewrites words in place when
/// the shape stands, and a field's text is not a word (2026-10-24, with the RON editor's report).
#[test]
fn a_form_replaced_by_one_of_the_same_shape_shows_the_new_form_s_values() {
    use bevy::text::EditableText;
    use common::editor::{field_entity, field_of, type_into};
    use omnis_vector::editor::NoteOf;
    let mut app = in_editor("test:map:town");
    let shown = |app: &mut App| {
        let form = editing(app).form.clone().expect("a note form");
        let field = EditAct::Field(field_of(&form, "note"));
        let entity = field_entity(app, field).expect("the note field");
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .value()
            .to_string()
    };
    app.world_mut().resource_mut::<Editing>().point((2, 2));
    app.update();
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    let form = editing(&app).form.clone().unwrap();
    type_into(
        &mut app,
        EditAct::Field(field_of(&form, "note")),
        "First cell.",
    );
    assert_eq!(shown(&mut app), "First cell.");
    app.world_mut().resource_mut::<Editing>().point((3, 2));
    app.update();
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("the open form was replaced")
    );
    assert_eq!(editing(&app).form.as_ref().unwrap().text_of("note"), "");
    assert_eq!(shown(&mut app), "", "the second cell's form is empty");
    let form = editing(&app).form.clone().unwrap();
    type_into(
        &mut app,
        EditAct::Field(field_of(&form, "note")),
        "Second cell.",
    );
    // A third, with the message standing: the same shape again.
    app.world_mut().resource_mut::<Editing>().point((4, 2));
    app.update();
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    assert_eq!(editing(&app).form.as_ref().unwrap().text_of("note"), "");
    assert_eq!(shown(&mut app), "", "the third cell's form is empty");
}

/// The same under the window's churn (every field changed every frame): the first cell's typed
/// note never flows into the second cell's fresh form.
#[test]
fn a_form_replaced_under_the_window_s_churn_keeps_the_new_form_s_values() {
    use common::app::churn_fields;
    use common::editor::{field_of, type_into};
    use omnis_vector::editor::NoteOf;
    let mut app = in_editor("test:map:town");
    churn_fields(&mut app);
    app.update();
    app.world_mut().resource_mut::<Editing>().point((2, 2));
    app.update();
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    let form = editing(&app).form.clone().unwrap();
    type_into(
        &mut app,
        EditAct::Field(field_of(&form, "note")),
        "First cell.",
    );
    assert_eq!(
        editing(&app).form.as_ref().unwrap().text_of("note"),
        "First cell."
    );
    app.world_mut().resource_mut::<Editing>().point((3, 2));
    app.update();
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        editing(&app).form.as_ref().unwrap().text_of("note"),
        "",
        "the second cell's form stays empty"
    );
}

#[test]
fn a_note_is_written_to_the_pack_at_once_outside_the_history_and_cleared() {
    use bevy::text::EditableText;
    use common::editor::{field_entity, field_of, in_editor_scratch, type_into};
    use omnis_vector::editor::NoteOf;
    let (mut app, scratch) = in_editor_scratch("editor-forms-notes", "test:map:town");
    let undo = editing(&app).status.undo;
    let text = form_text(&app);
    assert!(
        text.contains("Note: none") && text.contains("Map note: A 12x5 town"),
        "{text}"
    );
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    let form = editing(&app).form.clone().expect("the note form");
    assert!(form.remove.is_none(), "nothing to clear yet");
    let field = EditAct::Field(field_of(&form, "note"));
    let entity = field_entity(&mut app, field).expect("the note field");
    assert!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .allow_newlines,
        "several lines"
    );
    type_into(&mut app, field, "A quiet corner.\nThe well is dry.");
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    assert_eq!(
        editing(&app).note(NoteOf::Cell),
        Some("A quiet corner.\nThe well is dry.")
    );
    assert_eq!(
        editing(&app).note_key(NoteOf::Cell).as_deref(),
        Some("test:map:town/0/10,2")
    );
    let file = scratch.join("notes/town.ron");
    let written = std::fs::read_to_string(&file).expect("the note file");
    assert!(written.contains("quiet corner"), "{written}");
    assert_eq!(editing(&app).status.undo, undo, "outside the history");
    assert!(!editing(&app).status.dirty, "not a change to the map");
    let text = form_text(&app);
    assert!(text.contains("Note: A quiet corner."), "{text}");
    // The map's and the level's notes have their own keys.
    press_edit(&mut app, EditAct::Note(NoteOf::Map));
    let form = editing(&app).form.clone().unwrap();
    type_into(
        &mut app,
        EditAct::Field(field_of(&form, "note")),
        "The town.",
    );
    press_edit(&mut app, EditAct::Apply);
    assert_eq!(editing(&app).note(NoteOf::Map), Some("The town."));
    assert_eq!(
        editing(&app).note_key(NoteOf::Level).as_deref(),
        Some("test:map:town/level/0")
    );
    // Clear note takes the cell's away; the file keeps the map's.
    press_edit(&mut app, EditAct::Note(NoteOf::Cell));
    assert!(editing(&app).form.as_ref().unwrap().remove.is_some());
    press_edit(&mut app, EditAct::Discard);
    assert_eq!(editing(&app).note(NoteOf::Cell), None);
    let written = std::fs::read_to_string(&file).expect("the note file");
    assert!(
        !written.contains("quiet corner") && written.contains("The town."),
        "{written}"
    );
}

// ---- marks on the plan; encounters that start valid (owner, 2026-10-24) ----

/// How many pixels of `colour` lie in the plan's cell `cell` of `raster`.
fn in_plan_cell(
    app: &App,
    raster: &omnis_vector::minimap::Raster,
    cell: (u16, u16),
    colour: [u8; 4],
) -> usize {
    let (left, top, side) = editing(app).view.cell_rect(cell);
    let mut n = 0;
    for y in top..top + i64::from(side) {
        for x in left..left + i64::from(side) {
            if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y))
                && raster.pixel(x, y) == Some(colour)
            {
                n += 1;
            }
        }
    }
    n
}

/// The plan painted as the layer paints it, with the placements' marks.
fn painted(app: &App) -> omnis_vector::minimap::Raster {
    use omnis_vector::editor::plan::{self, Source};
    let e = editing(app);
    let def = e.def().expect("a map");
    let data = app
        .world()
        .resource::<omnis_vector::shell::session::Packs>()
        .data
        .clone();
    let marks = plan::marks(&def, &data, e.level);
    plan::paint(
        Source {
            grid: e.grid().expect("a grid"),
            terrains: &def.terrains,
            size: (def.width, def.height),
            level: e.level,
            resolved: e.resolved.as_ref(),
            start: None,
            marks: &marks,
            preview: None,
        },
        e.view,
        e.pane,
    )
}

#[test]
fn the_plan_marks_placements_on_their_cells() {
    use omnis_sim::omnis_core::Level;
    use omnis_vector::editor::plan::{self, Mark, START};
    use omnis_vector::minimap::DOOR;
    let mut app = in_editor("test:map:town");
    let def = editing(&app).def().unwrap();
    let data = app
        .world()
        .resource::<omnis_vector::shell::session::Packs>()
        .data
        .clone();
    let marks = plan::marks(&def, &data, Level::GROUND);
    assert!(marks.contains(&((11, 2), Mark::Portal)), "{marks:?}");
    // The inn's site is an activity of the inn's kind: its icon, a bed.
    let site = def.sites[0].clone();
    assert_eq!(site.service, "base:service:inn");
    assert!(
        marks.contains(&(
            (site.x, site.y),
            Mark::Activity(Some(omnis_sim::omnis_data::ServiceKind::Inn))
        )),
        "{marks:?}"
    );
    let raster = painted(&app);
    assert!(
        in_plan_cell(&app, &raster, (11, 2), DOOR) > 0,
        "the portal's rectangle and circle"
    );
    assert!(
        in_plan_cell(&app, &raster, (site.x, site.y), START) > 0,
        "the site's icon"
    );
    assert_eq!(
        in_plan_cell(&app, &raster, (3, 4), START),
        0,
        "a plain cell"
    );
    // An encounter stamped marks its cell at once.
    stamp(
        &mut app,
        omnis_vector::editor::forms::Placing::Encounter,
        (3, 4),
    );
    assert!(
        editing(&app)
            .def()
            .unwrap()
            .encounters
            .iter()
            .any(|e| (e.x, e.y) == (3, 4)),
        "{:?}",
        editing(&app).message
    );
    let raster = painted(&app);
    assert!(
        in_plan_cell(&app, &raster, (3, 4), START) > 0,
        "the encounter's triangle"
    );
}

#[test]
fn a_new_encounter_starts_valid_and_the_form_keeps_the_loader_s_keys() {
    use common::editor::{field_of, type_into};
    use omnis_vector::editor::forms::Placing;
    use omnis_vector::editor::view;
    use omnis_vector::screen::ControlId;
    use omnis_vector::shell::session::Packs;
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    // Stamped with nothing typed: a free id, one stack of a loaded monster, the loader takes
    // the map, Save is live.
    stamp(&mut app, Placing::Encounter, (3, 4));
    let def = editing(&app).def().unwrap();
    let e = def
        .encounters
        .iter()
        .find(|e| (e.x, e.y) == (3, 4))
        .expect("the encounter");
    assert_eq!(e.id, "encounter_3_4");
    assert_eq!(e.stacks.len(), 1, "one stack to start");
    assert!(
        data.monsters.values().any(|m| m.id == e.stacks[0].0),
        "{}",
        e.stacks[0].0
    );
    assert!(
        editing(&app).status.errors.is_empty(),
        "{:?}",
        editing(&app).status.errors
    );
    let save = view::tools(editing(&app), &data)
        .control(ControlId::Edit(EditAct::Save))
        .expect("Save")
        .clone();
    assert_eq!(save.why_not, None, "Save is live");
    // A second encounter on the cell is refused before the document would replace anything.
    let steps = editing(&app).status.undo;
    stamp(&mut app, Placing::Encounter, (3, 4));
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("already has an encounter")),
        "{:?}",
        editing(&app).message
    );
    assert_eq!(editing(&app).status.undo, steps);
    // A taken id on another cell is refused too, the form standing.
    stamp(&mut app, Placing::Encounter, (4, 4));
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().expect("the form");
    assert_eq!(form.text_of("id"), "encounter_4_4", "a free id");
    type_into(
        &mut app,
        EditAct::Field(field_of(&form, "id")),
        "encounter_3_4",
    );
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_some(), "the form stands");
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("already on the map")),
        "{:?}",
        editing(&app).message
    );
    // The stacks stop at the loader's four.
    for _ in 0..3 {
        press_edit(&mut app, EditAct::ListAdd(0));
    }
    assert_eq!(editing(&app).form.as_ref().unwrap().rows_of(0).len(), 4);
    press_edit(&mut app, EditAct::ListAdd(0));
    assert_eq!(editing(&app).form.as_ref().unwrap().rows_of(0).len(), 4);
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("at most 4")),
        "{:?}",
        editing(&app).message
    );
    press_edit(&mut app, EditAct::Cancel);
    // A creature nothing answers to is placed, and the loader's words are said at once.
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().unwrap();
    let rows = form.rows_of(0);
    type_into(
        &mut app,
        EditAct::Field(u8::try_from(rows[0][0]).unwrap()),
        "test:monster:nobody",
    );
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none());
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.starts_with("Placed; the loader says:")),
        "{:?}",
        editing(&app).message
    );
    assert!(!editing(&app).status.errors.is_empty());
    press_edit(&mut app, EditAct::Undo);
    assert!(editing(&app).status.errors.is_empty());
}

/// The palette offers "Activity" (owner, 2026-10-24: the category the services fall under) as a
/// stamp; a click places a service's site the inspector lists as a service, and its form names
/// the kind.
#[test]
fn an_activity_stamp_places_a_service_s_site_the_inspector_lists() {
    use omnis_vector::editor::form::FormKind;
    use omnis_vector::editor::forms::Placing;
    use omnis_vector::shell::session::Packs;
    let mut app = in_editor("test:map:town");
    let text = form_text(&app);
    assert!(
        !text.contains("Site") && !text.contains("Add here"),
        "the word Site and the Add row are gone: {text}"
    );
    let data = app.world().resource::<Packs>().data.clone();
    stamp(&mut app, Placing::Activity, (3, 4));
    let def = editing(&app).def().unwrap();
    let placed = def
        .sites
        .iter()
        .find(|s| s.x == 3 && s.y == 4)
        .expect("the site");
    assert_eq!(
        Some(placed.service.clone()),
        editing(&app).stamps.detail(Placing::Activity, &data)
    );
    assert!(placed.service.starts_with("base:service:"));
    let text = form_text(&app);
    assert!(
        text.contains(&format!("Service {}", placed.service)),
        "{text}"
    );
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().expect("the activity form");
    assert_eq!(form.kind, FormKind::Site);
    assert_eq!(form.choice_of("kind"), "Service site");
    assert_eq!(form.choice_of("service"), placed.service);
    assert!(form_text(&app).contains("Activity at (3, 4)"));
    press_edit(&mut app, EditAct::Cancel);
    // Its mark on the plan is its service's icon.
    let kind = data
        .services
        .values()
        .find(|s| s.id == placed.service)
        .map(|s| s.kind);
    assert!(kind.is_some());
    let marks =
        omnis_vector::editor::plan::marks(&def, &data, omnis_sim::omnis_core::Level::GROUND);
    assert!(marks.contains(&((3, 4), omnis_vector::editor::plan::Mark::Activity(kind))));
}
