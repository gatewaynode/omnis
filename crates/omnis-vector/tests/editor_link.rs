//! Linking portals in the editor (the entrances and exits enhancement, W1 step 2b, the owner's
//! direction of 2026-11-01): a portal stamped with nothing chosen is unlinked; the portal
//! picker lists the portals nothing links to, the open map's unsaved ones among them, each with
//! its map and cell, searched by any of them; a stamp with one chosen places a portal linked to
//! it (one side stating the link is enough, so the far map's file is never written) and lets
//! the choice go; a portal form's Link... picks for its link field and Unlink clears it; the
//! inspector names where a portal leads; removing a portal another names is the loader's error
//! on this map, with the other map's id.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{press_ui, set};
use common::editor::field_entity;
use common::editor::{
    arrow, editing, held, in_editor, in_editor_scratch, mouse, point_at, press_edit, top_screen,
    type_ui,
};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::Data;
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::{self, Entry};
use omnis_vector::editor::forms::Placing;
use omnis_vector::editor::plan::{self, Mark};
use omnis_vector::labels;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::editor::Editing;
use omnis_vector::shell::screen::Pressable;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;
use std::sync::Arc;

fn data(app: &App) -> Arc<Data> {
    app.world().resource::<Packs>().data.clone()
}

fn edit(act: EditAct) -> ControlId {
    ControlId::Edit(act)
}

/// Choose `entry` in the palette by its position, as its icon's press would.
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

/// The cursor turned to face `facing` (a fresh portal faces the opposite way, toward the author).
fn face(app: &mut App, facing: Facing) {
    app.world_mut().resource_mut::<Editing>().facing = facing;
    app.update();
}

/// The cursor put on `cell` without a click (the inspector lists what stands there).
fn cursor_to(app: &mut App, cell: (u16, u16)) {
    app.world_mut().resource_mut::<Editing>().cursor = cell;
    app.update();
}

/// Step the map choice to `id` and Open it.
fn open_map(app: &mut App, id: &str) {
    for _ in 0..8 {
        let e = editing(app);
        if e.maps[e.chosen].id == id {
            break;
        }
        arrow(app, EditAct::Map, true);
    }
    assert_eq!(editing(app).maps[editing(app).chosen].id, id);
    press_edit(app, EditAct::Open);
    assert_eq!(editing(app).def().expect("opened").id, id);
}

fn valid(app: &App) {
    assert!(
        editing(app).status.errors.is_empty(),
        "{:?}",
        editing(app).status.errors
    );
}

fn message(app: &App) -> String {
    editing(app).message.clone().unwrap_or_default()
}

/// The plan's mark on `cell` of the ground level.
fn mark_at(app: &App, cell: (u16, u16)) -> Option<Mark> {
    let def = editing(app).def().expect("a map");
    plan::marks(&def, &data(app), Level::GROUND)
        .into_iter()
        .find(|(c, _)| *c == cell)
        .map(|(_, m)| m)
}

/// What the window does every frame to a drawn text field: Bevy's layout takes it mutably, so
/// it reads as changed, and the column pushes its text into the model when the two differ (the
/// owner's report, 2026-11-01: a link chosen in the picker was gone by Apply). The drawn field
/// of the form's `label` touched that way, then a frame; its drawn text.
fn window_touches(app: &mut App, label: &str) -> String {
    let form = editing(app).form.clone().expect("a form");
    let i = form
        .fields
        .iter()
        .position(|f| f.label == label)
        .expect("the field");
    let entity = field_entity(app, EditAct::Field(u8::try_from(i).unwrap())).expect("drawn");
    let text = {
        let mut field = app
            .world_mut()
            .get_mut::<bevy::text::EditableText>(entity)
            .expect("a text field");
        field.set_changed();
        field.value().to_string()
    };
    app.update();
    text
}

fn portal_link(app: &App, cell: (u16, u16)) -> Option<String> {
    let def = editing(app).def().expect("a map");
    def.portals
        .iter()
        .find(|p| (p.x, p.y) == cell)
        .unwrap_or_else(|| panic!("a portal at {cell:?}"))
        .link
        .clone()
}

#[test]
fn a_portal_links_to_another_map_s_unlinked_portal_through_the_picker_and_both_maps_show_the_pair()
{
    let (mut app, scratch) = in_editor_scratch("editor-link", "test:map:town");
    // The town: an unlinked portal on the street at (5, 2), facing east (the cursor faces west),
    // so what comes through lands on (6, 2). Saved: the picker reads the maps as the files hold
    // them for the other maps.
    face(&mut app, Facing::West);
    choose(&mut app, Entry::Stamp(Placing::Portal));
    click(&mut app, (5, 2));
    assert_eq!(message(&app), "Portal 'portal_5_2' placed at (5, 2)");
    valid(&app);
    assert_eq!(mark_at(&app, (5, 2)), Some(Mark::Portal { linked: false }));
    press_edit(&mut app, EditAct::Save);
    assert!(message(&app).starts_with("Saved"), "{}", message(&app));
    let town_file = scratch.join("data/maps/town.ron");
    let town_text = std::fs::read_to_string(&town_file).unwrap();
    assert!(town_text.contains("portal_5_2"));
    // The meadow: the portal picker offers the town's new portal with its map and cell, found by
    // any of them.
    open_map(&mut app, "test:map:meadow");
    face(&mut app, Facing::West);
    choose(&mut app, Entry::Stamp(Placing::Portal));
    press_edit(&mut app, EditAct::Pick);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorPick));
    let data = data(&app);
    let town_name = labels::map(&data, "test:map:town").to_owned();
    let label = format!("portal_5_2  -  {town_name} (5, 2)");
    // The fixture may hold the owner's own unlinked portals too: the row is found by its id.
    let row = {
        let p = editing(&app).picking.clone().expect("the picker");
        let row = p
            .options
            .iter()
            .position(|o| o == "portal_5_2")
            .unwrap_or_else(|| panic!("{:?}", p.options));
        assert_eq!(p.labels[row], label);
        row
    };
    let row = u16::try_from(row).unwrap();
    let text = screen_text(app.world());
    assert!(text.contains(&format!("[ ] {label}")), "{text}");
    for needle in [town_name.to_uppercase(), "(5, 2)".to_owned()] {
        type_ui(&mut app, edit(EditAct::PickFilter), &needle);
        let text = screen_text(app.world());
        assert!(text.contains(&format!("[ ] {label}")), "{needle}: {text}");
    }
    type_ui(&mut app, edit(EditAct::PickFilter), "nothing of the kind");
    assert!(screen_text(app.world()).contains("No portal matches."));
    type_ui(&mut app, edit(EditAct::PickFilter), "");
    press_ui(&mut app, Pressable(edit(EditAct::PickToggle(row))));
    assert!(screen_text(app.world()).contains(&format!("[x] {label}")));
    press_ui(&mut app, Pressable(edit(EditAct::PickFinish)));
    assert_eq!(top_screen(&app), None);
    assert_eq!(held(&app, Placing::Portal), ["portal_5_2"]);
    // A click on the road places a portal linked to it, facing east: the town's portal lands on
    // (17, 20) and this one on (6, 2). The choice is let go: the partner is taken.
    click(&mut app, (16, 20));
    assert_eq!(
        message(&app),
        "Portal 'portal_16_20' to 'portal_5_2' placed at (16, 20)"
    );
    valid(&app);
    assert_eq!(portal_link(&app, (16, 20)).as_deref(), Some("portal_5_2"));
    assert_eq!(mark_at(&app, (16, 20)), Some(Mark::Portal { linked: true }));
    assert!(held(&app, Placing::Portal).is_empty(), "the choice let go");
    assert!(
        editing(&app).stamps.held(Placing::Portal).is_empty(),
        "the stamp drops unlinked portals again"
    );
    // The inspector names where it leads.
    let text = screen_text(app.world());
    assert!(
        text.contains(&format!("to {town_name} 'portal_5_2'")),
        "{text}"
    );
    // Saved: the meadow's file states the link; the town's file is as it was.
    press_edit(&mut app, EditAct::Save);
    assert!(message(&app).starts_with("Saved"), "{}", message(&app));
    let meadow_text = std::fs::read_to_string(scratch.join("data/maps/meadow.ron")).unwrap();
    assert!(
        meadow_text.contains("link: Some(\"portal_5_2\")"),
        "{meadow_text}"
    );
    assert_eq!(
        std::fs::read_to_string(&town_file).unwrap(),
        town_text,
        "one side states the link"
    );
    // The town, opened again: its portal is linked by the meadow's, drawn full, and leads there.
    open_map(&mut app, "test:map:town");
    assert_eq!(mark_at(&app, (5, 2)), Some(Mark::Portal { linked: true }));
    assert_eq!(portal_link(&app, (5, 2)), None, "its own file says nothing");
    cursor_to(&mut app, (5, 2));
    let data = self::data(&app);
    let meadow_name = labels::map(&data, "test:map:meadow").to_owned();
    let text = screen_text(app.world());
    assert!(
        text.contains("Portal 'portal_5_2'")
            && text.contains(&format!("to {meadow_name} 'portal_16_20'")),
        "{text}"
    );
    // Removing it leaves the meadow's portal naming nothing: the loader's error, here, with the
    // meadow's id; Undo puts it back.
    press_edit(&mut app, EditAct::RemoveAt(0));
    let errors = editing(&app).status.errors.clone();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains(
            "test:map:meadow: portal 'portal_16_20' at (16, 20) links to 'portal_5_2', which no loaded map has"
        ),
        "{errors:?}"
    );
    press_edit(&mut app, EditAct::Undo);
    valid(&app);
}

#[test]
fn a_portal_form_links_and_unlinks_through_the_picker_on_the_same_map() {
    let mut app = in_editor("test:map:meadow");
    face(&mut app, Facing::West);
    choose(&mut app, Entry::Stamp(Placing::Portal));
    click(&mut app, (16, 20));
    click(&mut app, (16, 24));
    valid(&app);
    for cell in [(16, 20), (16, 24)] {
        assert_eq!(mark_at(&app, cell), Some(Mark::Portal { linked: false }));
    }
    // The form of the second: Link... offers the first (this one is left out; the fixture's own
    // unlinked portals may be there too); Finish fills the link field; Apply links them.
    press_edit(&mut app, EditAct::EditAt(0));
    assert!(editing(&app).form.is_some(), "the portal's form");
    press_edit(&mut app, EditAct::LinkPick);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorPick));
    let options = editing(&app)
        .picking
        .as_ref()
        .map(|p| p.options.clone())
        .expect("the picker");
    assert!(
        options.contains(&"portal_16_20".to_owned())
            && !options.contains(&"portal_16_24".to_owned()),
        "the first offered, this one left out: {options:?}"
    );
    let row = options.iter().position(|o| o == "portal_16_20").unwrap();
    press_ui(
        &mut app,
        Pressable(edit(EditAct::PickToggle(u16::try_from(row).unwrap()))),
    );
    press_ui(&mut app, Pressable(edit(EditAct::PickFinish)));
    assert_eq!(top_screen(&app), None);
    let link = |app: &App| {
        editing(app)
            .form
            .as_ref()
            .map(|f| f.text_of("link").to_owned())
    };
    assert_eq!(link(&app), Some("portal_16_20".to_owned()));
    // The window's frame: the drawn field follows the model, and the model keeps the choice.
    assert_eq!(window_touches(&mut app, "link"), "portal_16_20");
    assert_eq!(
        link(&app),
        Some("portal_16_20".to_owned()),
        "the choice survives the window's frame"
    );
    let steps = editing(&app).status.undo;
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{}", message(&app));
    assert_eq!(editing(&app).status.undo, steps + 1);
    assert_eq!(portal_link(&app, (16, 24)).as_deref(), Some("portal_16_20"));
    valid(&app);
    for cell in [(16, 20), (16, 24)] {
        assert_eq!(mark_at(&app, cell), Some(Mark::Portal { linked: true }));
    }
    let data = data(&app);
    let meadow_name = labels::map(&data, "test:map:meadow").to_owned();
    let text = screen_text(app.world());
    assert!(
        text.contains(&format!("to {meadow_name} 'portal_16_20'")),
        "{text}"
    );
    // Edited again: Link... offers the partner, shown chosen; Cancel keeps it. Unlink clears the
    // field; Apply unlinks both.
    press_edit(&mut app, EditAct::EditAt(0));
    press_edit(&mut app, EditAct::LinkPick);
    {
        let p = editing(&app).picking.clone().expect("the picker");
        assert!(
            p.options.contains(&"portal_16_20".to_owned()),
            "{:?}",
            p.options
        );
        assert!(p.is_chosen("portal_16_20"));
    }
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_eq!(top_screen(&app), None);
    assert_eq!(
        editing(&app)
            .form
            .as_ref()
            .map(|f| f.text_of("link").to_owned()),
        Some("portal_16_20".to_owned()),
        "Cancel keeps the field"
    );
    press_edit(&mut app, EditAct::Unlink);
    assert_eq!(link(&app), Some(String::new()));
    assert_eq!(window_touches(&mut app, "link"), "");
    assert_eq!(
        link(&app),
        Some(String::new()),
        "Unlink survives the window's frame"
    );
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{}", message(&app));
    assert_eq!(portal_link(&app, (16, 24)), None);
    valid(&app);
    for cell in [(16, 20), (16, 24)] {
        assert_eq!(mark_at(&app, cell), Some(Mark::Portal { linked: false }));
    }
    assert!(screen_text(app.world()).contains("unlinked"));
}

/// A click on a modal's control as the window gives it: the control pressed and the mouse
/// button down in one frame, the control let go and the button up in the next (`press_ui`
/// carries no mouse button, so the plan's frame never saw one).
fn real_click(app: &mut App, id: ControlId) {
    set(app, &Pressable(id), Interaction::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    let world = app.world_mut();
    let mut controls = world.query::<(&Pressable, &mut Interaction)>();
    for (c, mut i) in controls.iter_mut(world) {
        if c.0 == id {
            *i = Interaction::None;
        }
    }
    mouse(app, MouseButton::Left, ButtonState::Released);
    app.update();
}

/// The map's portals as "id (x, y) link".
fn portals(app: &App) -> Vec<String> {
    editing(app)
        .def()
        .expect("a map")
        .portals
        .iter()
        .map(|p| format!("{} ({}, {}) {:?}", p.id, p.x, p.y, p.link))
        .collect()
}

/// The owner's report of 2026-11-07: a portal chosen in the stamp's picker never linked
/// anything from the palette. The picker's Finish, pressed with the mouse while the pointer
/// stood over the plan beneath the modal, dropped the stamp on that cell in the same frame
/// (the screens close the modal first, so the plan saw the button go down with nothing over
/// it), spending the choice; the click meant for the plan then placed an unlinked portal. A
/// press on the frame a screen closes is that screen's: Finish and Cancel place nothing, and
/// the next click places the linked portal.
#[test]
fn a_press_that_closes_the_picker_places_nothing_on_the_plan_beneath_it() {
    let (mut app, _scratch) = in_editor_scratch("editor-link-close", "test:map:meadow");
    face(&mut app, Facing::West);
    choose(&mut app, Entry::Stamp(Placing::Portal));
    click(&mut app, (16, 24));
    assert_eq!(message(&app), "Portal 'portal_16_24' placed at (16, 24)");
    let before = portals(&app);
    // The picker, the new portal chosen, the pointer over the road beneath the modal.
    press_edit(&mut app, EditAct::Pick);
    let row = editing(&app)
        .picking
        .clone()
        .expect("the picker")
        .options
        .iter()
        .position(|o| o == "portal_16_24")
        .expect("the new portal offered");
    press_ui(
        &mut app,
        Pressable(edit(EditAct::PickToggle(u16::try_from(row).unwrap()))),
    );
    point_at(&mut app, (16, 20), None);
    real_click(&mut app, edit(EditAct::PickFinish));
    assert_eq!(top_screen(&app), None);
    assert_eq!(portals(&app), before, "Finish placed nothing on the plan");
    assert_eq!(
        held(&app, Placing::Portal),
        ["portal_16_24"],
        "the choice kept"
    );
    // Cancel over the plan likewise.
    press_edit(&mut app, EditAct::Pick);
    point_at(&mut app, (18, 20), None);
    real_click(&mut app, ControlId::Resume);
    assert_eq!(top_screen(&app), None);
    assert_eq!(portals(&app), before, "Cancel placed nothing on the plan");
    assert_eq!(held(&app, Placing::Portal), ["portal_16_24"]);
    // The click meant for the plan places the linked portal.
    click(&mut app, (16, 20));
    assert_eq!(
        message(&app),
        "Portal 'portal_16_20' to 'portal_16_24' placed at (16, 20)"
    );
    valid(&app);
    assert_eq!(mark_at(&app, (16, 20)), Some(Mark::Portal { linked: true }));
    assert_eq!(mark_at(&app, (16, 24)), Some(Mark::Portal { linked: true }));
    assert!(
        held(&app, Placing::Portal).is_empty(),
        "the partner is taken"
    );
}
