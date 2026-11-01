//! A stamp's detail chosen in a modal (the editor's column, step 3b, owner 2026-10-31): under the
//! palette the stamp's icon, Select... and, for a kind with properties, the sliders; Select...
//! opens a picker over the editor with a search field, every id of the kind, chosen several at
//! once up to what one cell takes, Finish and Cancel; Finish goes on to the properties (a count
//! a monster or an item, the disposition and stand, an object's place on the cell) for the kinds
//! that have them; a click then drops what was chosen as set, ids minted apart; a right click
//! copies a placement whole into its stamp. Headless on the town and the dungeon; the objects
//! fixture pack's yard for objects.

use crate::common;
use bevy::input::ButtonState;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use common::app::press_ui;
use common::editor::{
    arrow, editing, held, in_editor, in_editor_on, mouse, point_at, press_edit, top_screen, type_ui,
};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::Place;
use omnis_sim::omnis_data::encounter::Disposition;
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::{self, Entry};
use omnis_vector::editor::form::Form;
use omnis_vector::editor::forms::Placing;
use omnis_vector::editor::stamp;
use omnis_vector::screen::{ControlId, Kind, ScreenId};
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

/// The position of `id` among the kind's options: the picker's row.
fn row_of(app: &App, kind: Placing, id: &str) -> EditAct {
    let options = stamp::options(kind, &data(app));
    let i = options
        .iter()
        .position(|o| o == id)
        .unwrap_or_else(|| panic!("{id} among {options:?}"));
    EditAct::PickToggle(u16::try_from(i).unwrap())
}

/// Whether the top screen draws a live control `id` (a dim one is no `Button`).
fn pressable(app: &mut App, id: ControlId) -> bool {
    let world = app.world_mut();
    let mut nodes = world.query::<(&Pressable, Has<Button>)>();
    nodes.iter(world).any(|(p, live)| p.0 == id && live)
}

/// The picker opened for `kind` (chosen in the palette first) and `ids` toggled in it.
fn pick(app: &mut App, kind: Placing, ids: &[&str]) {
    choose(app, Entry::Stamp(kind));
    press_edit(app, EditAct::Pick);
    assert_eq!(top_screen(app), Some(ScreenId::EditorPick));
    for id in ids {
        let row = row_of(app, kind, id);
        press_ui(app, Pressable(edit(row)));
    }
}

/// The properties form `n` as the modal holds it.
fn props(app: &App, n: usize) -> Form {
    editing(app)
        .picking
        .as_ref()
        .and_then(|p| p.props.get(n))
        .cloned()
        .unwrap_or_else(|| panic!("properties form {n}"))
}

/// The count control of row `r` of the first list of properties form `n`.
fn count_of(app: &App, n: u8, r: usize) -> ControlId {
    let form = props(app, usize::from(n));
    let rows = form.rows_of(0);
    let i = rows[r][1];
    assert_eq!(form.fields[i].label, "count");
    edit(EditAct::Prop(n, u8::try_from(i).unwrap()))
}

/// The form's own field `label` of properties form `n`.
fn prop_of(app: &App, n: u8, label: &str) -> ControlId {
    let form = props(app, usize::from(n));
    edit(EditAct::Prop(n, common::editor::field_of(&form, label)))
}

fn click(app: &mut App, cell: (u16, u16)) {
    point_at(app, cell, None);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

fn right_click(app: &mut App, cell: (u16, u16)) {
    point_at(app, cell, None);
    mouse(app, MouseButton::Right, ButtonState::Pressed);
    mouse(app, MouseButton::Right, ButtonState::Released);
}

fn message(app: &App) -> String {
    editing(app).message.clone().unwrap_or_default()
}

fn valid(app: &App) {
    assert!(
        editing(app).status.errors.is_empty(),
        "{:?}",
        editing(app).status.errors
    );
}

/// Cells of the shown level with ground and nothing placed, not the start.
fn bare_cells(app: &App) -> Vec<(u16, u16)> {
    let e = editing(app);
    let def = e.def().expect("a map");
    let grid = e.grid().expect("a grid");
    let (w, h) = e.size();
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if grid.tile(x, y).flatten().is_some()
                && e.here_at((x, y)).is_empty()
                && (x, y) != (def.start.0, def.start.1)
            {
                out.push((x, y));
            }
        }
    }
    out
}

#[test]
fn the_detail_row_opens_a_picker_that_searches_and_chooses_up_to_the_limit() {
    let mut app = in_editor("test:map:town");
    let data = data(&app);
    choose(&mut app, Entry::Stamp(Placing::Encounter));
    // Under the palette: the stamp's icon and Select... as one control in two shapes, the
    // sliders for the properties, and what the stamp drops.
    let tools = omnis_vector::editor::view::tools(editing(&app), &data);
    let icon = tools.control(edit(EditAct::Stamp)).expect("the icon");
    assert!(
        tools
            .control(edit(EditAct::Pick))
            .is_some_and(|c| c.label == "Select..."),
        "the button beside it"
    );
    assert!(
        matches!(
            icon.kind,
            Kind::Icon {
                name: "swords",
                chosen: true
            }
        ),
        "{:?}",
        icon.kind
    );
    let sliders = tools.control(edit(EditAct::Props)).expect("the sliders");
    assert!(
        matches!(
            sliders.kind,
            Kind::Icon {
                name: "sliders-horizontal",
                ..
            }
        ) && sliders.why_not.is_none(),
        "{sliders:?}"
    );
    let text = screen_text(app.world());
    let monsters = stamp::options(Placing::Encounter, &data);
    assert!(
        monsters.len() > stamp::limit(Placing::Encounter),
        "{monsters:?}"
    );
    assert!(text.contains("Select..."), "{text}");
    assert!(
        text.contains(&format!(
            "nothing chosen, the first: {}",
            stamp::short(&monsters[0])
        )),
        "{text}"
    );
    // Select... opens the picker over the editor: a search field, the count, every monster.
    press_edit(&mut app, EditAct::Pick);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorPick));
    let text = screen_text(app.world());
    assert!(text.contains("CHOOSE MONSTERS"), "{text}");
    assert!(text.contains("0 of 4 chosen"), "{text}");
    for m in &monsters {
        assert!(text.contains(&format!("[ ] {m}")), "{m} listed: {text}");
    }
    // The search keeps the ids that hold it, in any case.
    type_ui(&mut app, edit(EditAct::PickFilter), "RAT");
    assert_eq!(
        editing(&app).picking.as_ref().map(|p| p.filter.as_str()),
        Some("RAT")
    );
    let text = screen_text(app.world());
    let rats: Vec<&String> = monsters.iter().filter(|m| m.contains("rat")).collect();
    assert!(rats.len() >= 2, "{monsters:?}");
    for m in &monsters {
        assert_eq!(
            text.contains(&format!("[ ] {m}")),
            m.contains("rat"),
            "{m}: {text}"
        );
    }
    // Two chosen, marked; the count follows.
    for rat in rats.iter().take(2) {
        let row = row_of(&app, Placing::Encounter, rat);
        press_ui(&mut app, Pressable(edit(row)));
    }
    let text = screen_text(app.world());
    assert!(text.contains(&format!("[x] {}", rats[0])), "{text}");
    assert!(text.contains(&format!("[x] {}", rats[1])), "{text}");
    assert!(text.contains("2 of 4 chosen"), "{text}");
    // The filter cleared, the rest come back; at the limit the unchosen rows are dim.
    type_ui(&mut app, edit(EditAct::PickFilter), "");
    let others: Vec<&String> = monsters.iter().filter(|m| !m.contains("rat")).collect();
    for m in others.iter().take(2) {
        let row = row_of(&app, Placing::Encounter, m);
        press_ui(&mut app, Pressable(edit(row)));
    }
    assert!(screen_text(app.world()).contains("4 of 4 chosen"));
    let fifth = row_of(&app, Placing::Encounter, others[2]);
    assert!(
        !pressable(&mut app, edit(fifth)),
        "the fifth row is dim at the limit"
    );
    let chosen = row_of(&app, Placing::Encounter, rats[0]);
    assert!(pressable(&mut app, edit(chosen)), "a chosen row lets go");
    // The model refuses a fifth too (the socket's or a key's path): the choice stands, said why.
    let EditAct::PickToggle(i) = fifth else {
        unreachable!()
    };
    let packs = data.clone();
    app.world_mut()
        .resource_mut::<Editing>()
        .apply(&packs, EditAct::PickToggle(i), 0);
    assert_eq!(
        editing(&app).picking.as_ref().map(|p| p.chosen.len()),
        Some(4)
    );
    assert_eq!(message(&app), "at most 4 monsters");
    // Cancel keeps what the stamp held: nothing. The icon opens the picker as Select... does.
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_eq!(top_screen(&app), None);
    assert!(held(&app, Placing::Encounter).is_empty());
    press_edit(&mut app, EditAct::Stamp);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorPick));
    assert!(
        screen_text(app.world()).contains("0 of 4 chosen"),
        "the picker starts from the stamp again"
    );
    press_ui(&mut app, Pressable(ControlId::Resume));
}

#[test]
fn finish_opens_the_properties_and_a_click_drops_the_encounter_as_set() {
    let mut app = in_editor("test:map:town");
    pick(
        &mut app,
        Placing::Encounter,
        &["test:monster:giant_rat", "test:monster:goblin"],
    );
    press_ui(&mut app, Pressable(edit(EditAct::PickFinish)));
    assert_eq!(top_screen(&app), Some(ScreenId::EditorProps));
    let text = screen_text(app.world());
    assert!(text.contains("ENCOUNTER PROPERTIES"), "{text}");
    assert!(
        text.contains("test:monster:giant_rat") && text.contains("test:monster:goblin"),
        "{text}"
    );
    assert!(
        text.contains("disposition") && text.contains("stands apart"),
        "{text}"
    );
    // The click decides the id and the cell: the modal has no field for them.
    let labels: Vec<String> = props(&app, 0)
        .props_rows(0)
        .iter()
        .flat_map(|r| r.controls.iter().map(|c| c.label.clone()))
        .collect();
    for decided in ["id", "x", "y"] {
        assert!(!labels.iter().any(|l| l == decided), "{labels:?}");
    }
    // The first count stepped to 3, the disposition to Friendly.
    let ControlId::Edit(count) = count_of(&app, 0, 0) else {
        unreachable!()
    };
    arrow(&mut app, count, true);
    arrow(&mut app, count, true);
    let form = props(&app, 0);
    let rows = form.rows_of(0);
    assert_eq!(form.fields[rows[0][1]].number(), 3, "{form:?}");
    let ControlId::Edit(disposition) = prop_of(&app, 0, "disposition") else {
        unreachable!()
    };
    for _ in 0..3 {
        arrow(&mut app, disposition, true);
    }
    assert_eq!(props(&app, 0).choice_of("disposition"), "Friendly");
    // Finish holds it; the summary says what the stamp drops.
    press_ui(&mut app, Pressable(edit(EditAct::PropsFinish)));
    assert_eq!(top_screen(&app), None);
    assert_eq!(
        held(&app, Placing::Encounter),
        ["test:monster:giant_rat", "test:monster:goblin"]
    );
    let data = data(&app);
    assert_eq!(
        editing(&app).stamps.summary(Placing::Encounter, &data),
        "giant_rat x3, goblin"
    );
    // A click drops it as set, valid; another click mints another id.
    click(&mut app, (5, 2));
    let def = editing(&app).def().unwrap();
    let e = def
        .encounters
        .iter()
        .find(|e| (e.x, e.y) == (5, 2))
        .expect("the encounter");
    assert_eq!(e.id, "encounter_5_2");
    assert_eq!(
        e.stacks,
        vec![
            ("test:monster:giant_rat".to_owned(), 3),
            ("test:monster:goblin".to_owned(), 1)
        ]
    );
    assert_eq!(e.disposition, Disposition::Friendly);
    assert_eq!(message(&app), "Encounter encounter_5_2 placed at (5, 2)");
    valid(&app);
    click(&mut app, (6, 2));
    let def = editing(&app).def().unwrap();
    assert!(def.encounters.iter().any(|e| e.id == "encounter_6_2"));
    valid(&app);
    // The sliders reopen the properties from what is held.
    press_edit(&mut app, EditAct::Props);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorProps));
    assert_eq!(props(&app, 0).choice_of("disposition"), "Friendly");
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_eq!(top_screen(&app), None);
}

#[test]
fn two_object_kinds_drop_as_two_objects_with_their_own_ids_and_places() {
    let mut app = in_editor_on(
        &["crates/omnis-sim/tests/packs/objects"],
        "objects:map:yard",
    );
    let data = data(&app);
    let kinds = stamp::options(Placing::Object, &data);
    assert!(kinds.len() >= 2, "{kinds:?}");
    let bare = bare_cells(&app);
    let (cell, other) = (bare[0], bare[1]);
    let chosen = [kinds[0].as_str(), kinds[1].as_str()];
    pick(&mut app, Placing::Object, &chosen);
    press_ui(&mut app, Pressable(edit(EditAct::PickFinish)));
    assert_eq!(top_screen(&app), Some(ScreenId::EditorProps));
    let text = screen_text(app.world());
    assert!(text.contains("OBJECT PROPERTIES"), "{text}");
    assert!(
        text.contains(&kinds[0]) && text.contains(&kinds[1]),
        "a section a kind: {text}"
    );
    // The second kind stands two eighths east.
    let ControlId::Edit(pos_x) = prop_of(&app, 1, "pos x") else {
        unreachable!()
    };
    arrow(&mut app, pos_x, true);
    arrow(&mut app, pos_x, true);
    assert_eq!(props(&app, 1).number_of("pos x"), 2);
    press_ui(&mut app, Pressable(edit(EditAct::PropsFinish)));
    assert_eq!(top_screen(&app), None);
    assert_eq!(held(&app, Placing::Object), chosen);
    click(&mut app, cell);
    let def = editing(&app).def().unwrap();
    let on: Vec<_> = def.objects.iter().filter(|o| (o.x, o.y) == cell).collect();
    assert_eq!(on.len(), 2, "{on:?}");
    let (x, y) = cell;
    assert_eq!(on[0].id, format!("object_{x}_{y}"));
    assert_eq!(on[1].id, format!("object_{x}_{y}_2"), "minted apart");
    assert_eq!((on[0].kind.as_str(), on[0].pos), (chosen[0], (0, 0)));
    assert_eq!((on[1].kind.as_str(), on[1].pos), (chosen[1], (2, 0)));
    assert!(
        message(&app).starts_with(&format!(
            "Object object_{x}_{y} ({}), Object object_{x}_{y}_2 ({}) placed at ({x}, {y})",
            chosen[0], chosen[1]
        )),
        "{}",
        message(&app)
    );
    valid(&app);
    click(&mut app, other);
    let def = editing(&app).def().unwrap();
    let (ox, oy) = other;
    assert!(
        def.objects
            .iter()
            .any(|o| o.id == format!("object_{ox}_{oy}_2"))
    );
    valid(&app);
}

#[test]
fn a_pile_takes_counts_and_two_lore_texts_drop_as_two_entries() {
    let mut app = in_editor("test:map:town");
    let data = data(&app);
    pick(
        &mut app,
        Placing::Ground,
        &["base:item:arrows", "base:item:dagger"],
    );
    press_ui(&mut app, Pressable(edit(EditAct::PickFinish)));
    assert_eq!(top_screen(&app), Some(ScreenId::EditorProps));
    assert!(screen_text(app.world()).contains("GROUND PILE PROPERTIES"));
    let ControlId::Edit(arrows) = count_of(&app, 0, 0) else {
        unreachable!()
    };
    arrow(&mut app, arrows, true);
    arrow(&mut app, arrows, true);
    let ControlId::Edit(dagger) = count_of(&app, 0, 1) else {
        unreachable!()
    };
    arrow(&mut app, dagger, true);
    // A slider sets a count outright, through the model's own path.
    app.world_mut().resource_mut::<Editing>().set_in(arrows, 10);
    press_ui(&mut app, Pressable(edit(EditAct::PropsFinish)));
    assert_eq!(top_screen(&app), None);
    assert_eq!(
        editing(&app).stamps.summary(Placing::Ground, &data),
        "arrows x10, dagger x2"
    );
    click(&mut app, (6, 2));
    let def = editing(&app).def().unwrap();
    let g = def
        .ground
        .iter()
        .find(|g| (g.x, g.y) == (6, 2))
        .expect("the pile");
    assert_eq!(
        g.items,
        vec![
            ("base:item:arrows".to_owned(), 10),
            ("base:item:dagger".to_owned(), 2)
        ]
    );
    valid(&app);
    // Two texts: Finish closes the picker (a lore has no properties) and a click drops two.
    let keys = stamp::options(Placing::Lore, &data);
    assert!(keys.len() >= 2, "{keys:?}");
    pick(&mut app, Placing::Lore, &[&keys[0], &keys[1]]);
    press_ui(&mut app, Pressable(edit(EditAct::PickFinish)));
    assert_eq!(top_screen(&app), None);
    click(&mut app, (7, 2));
    let def = editing(&app).def().unwrap();
    let lore: Vec<_> = def.lore.iter().filter(|l| (l.x, l.y) == (7, 2)).collect();
    assert_eq!(lore.len(), 2, "{lore:?}");
    assert_eq!(
        (lore[0].id.as_str(), lore[0].text.as_str()),
        ("lore_7_2", keys[0].as_str())
    );
    assert_eq!(
        (lore[1].id.as_str(), lore[1].text.as_str()),
        ("lore_7_2_2", keys[1].as_str())
    );
    valid(&app);
}

/// W1 step 2: the portal picker offers the portals nothing links to (none among the shipped
/// maps, every one of which is paired); with nothing chosen a click places an unlinked portal,
/// valid as it lands. Step 2b lists the open map's unsaved portals too and links from a form.
#[test]
fn the_portal_picker_offers_unlinked_portals_and_a_click_with_none_places_an_unlinked_one() {
    let mut app = in_editor("test:map:town");
    let data = data(&app);
    assert!(
        stamp::options(Placing::Portal, &data).is_empty(),
        "every shipped portal is paired"
    );
    choose(&mut app, Entry::Stamp(Placing::Portal));
    press_edit(&mut app, EditAct::Pick);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorPick));
    let text = screen_text(app.world());
    assert!(
        text.contains("CHOOSE PORTALS")
            && text.contains("The packs have no portal.")
            && text.contains("0 of 1 chosen"),
        "{text}"
    );
    assert!(
        !pressable(&mut app, edit(EditAct::PickFinish)),
        "Finish is dim"
    );
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_eq!(top_screen(&app), None);
    click(&mut app, (8, 2));
    let def = editing(&app).def().unwrap();
    let p = def
        .portals
        .iter()
        .find(|p| (p.x, p.y) == (8, 2))
        .expect("the portal");
    assert_eq!((p.id.as_str(), p.link.as_deref()), ("portal_8_2", None));
    assert_eq!(message(&app), "Portal 'portal_8_2' placed at (8, 2)");
    valid(&app);
}

#[test]
fn a_right_click_copies_an_encounter_whole_and_the_picker_shows_it_chosen() {
    let mut app = in_editor("test:map:dungeon");
    let def = editing(&app).def().unwrap();
    let e = def
        .encounters
        .iter()
        .find(|e| e.level.0 == 0 && e.stacks[0].1 > 1)
        .cloned()
        .expect("an encounter of several on the ground level");
    right_click(&mut app, (e.x, e.y));
    assert_eq!(editing(&app).entry, Entry::Stamp(Placing::Encounter));
    assert_eq!(
        editing(&app).stamps.held(Placing::Encounter),
        [Place::Encounter(e.clone())],
        "the placement whole"
    );
    press_edit(&mut app, EditAct::Pick);
    let text = screen_text(app.world());
    for (monster, _) in &e.stacks {
        assert!(text.contains(&format!("[x] {monster}")), "{text}");
    }
    press_ui(&mut app, Pressable(ControlId::Resume));
}

#[test]
fn the_search_field_takes_the_keyboard_and_keeps_it_while_the_rows_change() {
    use common::app::{keyboard, keyboard_on};
    let (mut app, window) = keyboard_on(in_editor("test:map:town"));
    app.update();
    choose(&mut app, Entry::Stamp(Placing::Encounter));
    press_edit(&mut app, EditAct::Pick);
    let focused_field = |app: &mut App| {
        let focused = app.world().resource::<InputFocus>().get()?;
        app.world().get::<Pressable>(focused).map(|p| p.0)
    };
    assert_eq!(
        focused_field(&mut app),
        Some(edit(EditAct::PickFilter)),
        "the search takes the keyboard as the picker opens"
    );
    keyboard(&mut app, window, "ra");
    assert_eq!(
        editing(&app).picking.as_ref().map(|p| p.filter.as_str()),
        Some("ra"),
        "both keys, in order, through the rebuilt rows"
    );
    assert_eq!(focused_field(&mut app), Some(edit(EditAct::PickFilter)));
    let text = screen_text(app.world());
    assert!(text.contains("[ ] test:monster:giant_rat"), "{text}");
    assert!(!text.contains("goblin"), "{text}");
}
