//! Placements as palette stamps (the editor's column, step 3, 2026-10-25; step 3b, 2026-10-31):
//! the palette ends in a stamp a placement kind, each holding what it drops (chosen in the
//! picker modal, `editor_pick.rs`; here held directly); a click with the brush places it at
//! once, valid as it lands, refused where its kind already is; a drag places once; a right
//! click on a placement copies it into its stamp; the inspector's Add row is gone.
//! Headless on the town: the sites on the shops' door cells, the portal on (11, 2), the street
//! (row 2) bare with ground.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::editor::{editing, held, hold, in_editor, in_editor_on, mouse, point_at, press_edit};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::Place;
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::{self, Entry, Tool};
use omnis_vector::editor::forms::Placing;
use omnis_vector::editor::stamp;
use omnis_vector::screen::{ControlId, Kind};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;
use std::sync::Arc;

fn depth(app: &App) -> usize {
    editing(app).status.undo
}

fn data(app: &App) -> Arc<Data> {
    app.world().resource::<Packs>().data.clone()
}

/// Choose `entry` in the palette by its position, as its icon's press would.
fn choose(app: &mut App, entry: Entry) {
    let def = editing(app).def().expect("a map");
    let i = brush::palette(&def)
        .iter()
        .position(|e| *e == entry)
        .expect("in the palette");
    press_edit(
        app,
        EditAct::Brush(u8::try_from(i).expect("a short palette")),
    );
    assert_eq!(editing(app).entry, entry);
}

/// Hold `id` in the chosen stamp, as the picker's Finish would.
fn detail_to(app: &mut App, kind: Placing, id: &str) {
    let options = stamp::options(kind, &data(app));
    assert!(options.iter().any(|o| o == id), "{id} among {options:?}");
    hold(app, kind, &[id]);
}

/// The packs' first of `kind`: what a stamp drops while nothing is chosen.
fn first(app: &App, kind: Placing) -> String {
    stamp::options(kind, &data(app))
        .first()
        .cloned()
        .expect("the packs have some")
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

#[test]
fn an_activity_stamp_places_the_chosen_service_with_a_click_and_refuses_an_occupied_tile() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    let sites = editing(&app).def().unwrap().sites.len();
    choose(&mut app, Entry::Stamp(Placing::Activity));
    assert_eq!(
        editing(&app).tool,
        Tool::Brush,
        "choosing a stamp chooses the brush"
    );
    detail_to(&mut app, Placing::Activity, "base:service:tavern");
    let text = screen_text(app.world());
    assert!(
        text.contains("Activity: a service's site, placed with a click"),
        "{text}"
    );
    // Under the palette: the stamp's icon, Select... and what it holds (step 3b).
    let data = data(&app);
    let tools = omnis_vector::editor::view::tools(editing(&app), &data);
    let select = tools
        .control(ControlId::Edit(EditAct::Stamp))
        .expect("the stamp's icon")
        .clone();
    assert!(
        matches!(select.kind, Kind::Icon { name, chosen: true } if Some(name) == Entry::Stamp(Placing::Activity).icon()),
        "{:?}",
        select.kind
    );
    assert!(text.contains("Select..."), "{text}");
    assert!(
        text.contains("tavern"),
        "the summary names the service: {text}"
    );
    // A click places it at once, one undo step, the cursor on the cell.
    click(&mut app, (5, 2));
    let def = editing(&app).def().unwrap();
    let site = def
        .sites
        .iter()
        .find(|s| (s.x, s.y) == (5, 2))
        .expect("the site");
    assert_eq!(site.service, "base:service:tavern");
    assert_eq!(def.sites.len(), sites + 1);
    assert_eq!(depth(&app), steps + 1);
    assert_eq!(editing(&app).cursor, (5, 2));
    assert_eq!(
        message(&app),
        "Service base:service:tavern placed at (5, 2)"
    );
    valid(&app);
    let text = screen_text(app.world());
    assert!(text.contains("Service base:service:tavern"), "{text}");
    // On the inn's tile the site is refused and nothing changes.
    click(&mut app, (1, 1));
    assert_eq!(message(&app), "a site is already at (1, 1)");
    assert_eq!(depth(&app), steps + 1);
    assert_eq!(editing(&app).def().unwrap().sites.len(), sites + 1);
    assert_eq!(
        editing(&app)
            .def()
            .unwrap()
            .sites
            .iter()
            .find(|s| (s.x, s.y) == (1, 1))
            .map(|s| s.service.clone()),
        Some("base:service:inn".to_owned()),
        "the inn stands"
    );
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(editing(&app).def().unwrap().sites.len(), sites);
    assert_eq!(depth(&app), steps);
}

#[test]
fn an_encounter_stamp_mints_a_free_id_and_refuses_a_second_on_the_cell() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    choose(&mut app, Entry::Stamp(Placing::Encounter));
    detail_to(&mut app, Placing::Encounter, "test:monster:goblin");
    click(&mut app, (5, 2));
    let def = editing(&app).def().unwrap();
    let e = def
        .encounters
        .iter()
        .find(|e| (e.x, e.y) == (5, 2))
        .expect("the encounter");
    assert_eq!(e.id, "encounter_5_2");
    assert_eq!(e.stacks, vec![("test:monster:goblin".to_owned(), 1)]);
    assert_eq!(depth(&app), steps + 1);
    assert_eq!(message(&app), "Encounter encounter_5_2 placed at (5, 2)");
    valid(&app);
    // A second on the cell is refused before the document would take it.
    click(&mut app, (5, 2));
    assert!(
        message(&app).contains("already has an encounter"),
        "{}",
        message(&app)
    );
    assert_eq!(depth(&app), steps + 1);
    // Another cell mints another id; the inspector's Edit opens the form filled.
    click(&mut app, (6, 2));
    let def = editing(&app).def().unwrap();
    assert!(def.encounters.iter().any(|e| e.id == "encounter_6_2"));
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().expect("the form");
    assert_eq!(form.text_of("id"), "encounter_6_2");
    assert!(form.remove.is_some());
}

/// The cells of the open map with ground and nothing placed, other than the start: where a
/// stamp may land and the loader takes it.
fn bare_cells(app: &App) -> Vec<(u16, u16)> {
    let e = editing(app);
    let def = e.def().expect("a map");
    let grid = e.grid().expect("a grid");
    let mut out = Vec::new();
    for y in 0..def.height {
        for x in 0..def.width {
            if grid.tile(x, y).flatten().is_some()
                && (def.start.0, def.start.1) != (x, y)
                && e.here_at((x, y)).is_empty()
            {
                out.push((x, y));
            }
        }
    }
    out
}

#[test]
fn an_object_stamp_mints_a_free_id_of_the_chosen_kind_where_the_packs_have_kinds() {
    // The shipped packs place no objects: the stamp says so and places nothing.
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    choose(&mut app, Entry::Stamp(Placing::Object));
    click(&mut app, (5, 2));
    assert_eq!(
        message(&app),
        "nothing to stamp: the packs have no object kind"
    );
    assert_eq!(depth(&app), steps);
    assert!(editing(&app).def().unwrap().objects.is_empty());
    // The objects fixture pack has kinds: a free id, the chosen kind, valid as it lands.
    let mut app = in_editor_on(
        &["crates/omnis-sim/tests/packs/objects"],
        "objects:map:yard",
    );
    let data = data(&app);
    let steps = depth(&app);
    let bare = bare_cells(&app);
    let (cell, other, third) = (bare[0], bare[1], bare[2]);
    choose(&mut app, Entry::Stamp(Placing::Object));
    let kind = first(&app, Placing::Object);
    assert!(data.objects.values().any(|k| k.id == kind), "{kind}");
    click(&mut app, cell);
    let def = editing(&app).def().unwrap();
    let o = def
        .objects
        .iter()
        .find(|o| (o.x, o.y) == cell)
        .expect("the object");
    let id = format!("object_{}_{}", cell.0, cell.1);
    assert_eq!(
        (o.id.as_str(), o.kind.as_str()),
        (id.as_str(), kind.as_str())
    );
    assert_eq!(
        message(&app),
        format!("Object {id} ({kind}) placed at ({}, {})", cell.0, cell.1)
    );
    assert_eq!(depth(&app), steps + 1);
    valid(&app);
    // Objects and encounters share one id space (the loader's rule): an encounter named as
    // the object on `other` would be, and the object's id steps past it.
    choose(&mut app, Entry::Stamp(Placing::Encounter));
    click(&mut app, third);
    press_edit(&mut app, EditAct::EditAt(0));
    let form = editing(&app).form.clone().expect("the encounter's form");
    let taken = format!("object_{}_{}", other.0, other.1);
    common::editor::type_into(
        &mut app,
        EditAct::Field(common::editor::field_of(&form, "id")),
        &taken,
    );
    press_edit(&mut app, EditAct::Apply);
    assert!(editing(&app).form.is_none(), "{:?}", editing(&app).message);
    let def = editing(&app).def().unwrap();
    assert!(def.encounters.iter().any(|e| e.id == taken));
    assert_eq!(
        def.encounters
            .iter()
            .filter(|e| (e.x, e.y) == third)
            .count(),
        1,
        "renamed, not doubled: the old id's entry went in the same step"
    );
    valid(&app);
    choose(&mut app, Entry::Stamp(Placing::Object));
    click(&mut app, other);
    let def = editing(&app).def().unwrap();
    let o = def
        .objects
        .iter()
        .find(|o| (o.x, o.y) == other)
        .expect("the second object");
    assert_eq!(o.id, format!("{taken}_2"), "past the encounter's id");
    valid(&app);
}

#[test]
fn the_pile_lore_and_portal_stamps_start_valid() {
    let mut app = in_editor("test:map:town");
    let data = data(&app);
    let steps = depth(&app);
    // A pile: one of the chosen item.
    choose(&mut app, Entry::Stamp(Placing::Ground));
    let item = first(&app, Placing::Ground);
    assert!(data.items.values().any(|i| i.id == item), "{item}");
    click(&mut app, (6, 2));
    let def = editing(&app).def().unwrap();
    let g = def
        .ground
        .iter()
        .find(|g| (g.x, g.y) == (6, 2))
        .expect("the pile");
    assert_eq!(g.items, vec![(item.clone(), 1)]);
    assert_eq!(message(&app), "Ground pile placed at (6, 2)");
    valid(&app);
    // A lore: a free id, the chosen text key, one the packs define.
    choose(&mut app, Entry::Stamp(Placing::Lore));
    let key = first(&app, Placing::Lore);
    assert!(data.registry.text.get(&key).is_some(), "{key}");
    click(&mut app, (7, 2));
    let def = editing(&app).def().unwrap();
    let l = def
        .lore
        .iter()
        .find(|l| (l.x, l.y) == (7, 2))
        .expect("the lore");
    assert_eq!((l.id.as_str(), l.text.as_str()), ("lore_7_2", key.as_str()));
    valid(&app);
    // A portal (W1 step 2): unlinked with nothing chosen (every shipped portal is paired, so
    // there is nothing to choose), a fresh id, facing the author, a marker of the tileset.
    choose(&mut app, Entry::Stamp(Placing::Portal));
    assert!(
        stamp::options(Placing::Portal, &data).is_empty(),
        "no unlinked portal to link to"
    );
    click(&mut app, (8, 2));
    let def = editing(&app).def().unwrap();
    let p = def
        .portals
        .iter()
        .find(|p| (p.x, p.y) == (8, 2))
        .expect("the portal");
    assert_eq!(
        (p.id.as_str(), p.link.as_deref(), p.facing),
        ("portal_8_2", None, editing(&app).facing.opposite())
    );
    let tileset = data
        .registry
        .tilesets
        .get(&def.tileset)
        .and_then(|id| data.tilesets.get(&id))
        .expect("the town's tileset");
    let marker = p.marker.as_deref().expect("a marker");
    assert!(
        matches!(
            tileset.surfaces.get(marker).map(|s| s.kind),
            Some(omnis_sim::omnis_data::SlotKind::Object)
        ),
        "{marker}"
    );
    assert_eq!(message(&app), "Portal 'portal_8_2' placed at (8, 2)");
    valid(&app);
    assert_eq!(depth(&app), steps + 3, "one step each");
    // A second portal or pile on its tile is refused.
    click(&mut app, (8, 2));
    assert_eq!(message(&app), "a portal is already at (8, 2)");
    choose(&mut app, Entry::Stamp(Placing::Ground));
    click(&mut app, (6, 2));
    assert_eq!(message(&app), "a ground pile is already at (6, 2)");
    assert_eq!(depth(&app), steps + 3);
}

#[test]
fn a_drag_with_a_stamp_places_once() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    choose(&mut app, Entry::Stamp(Placing::Ground));
    point_at(&mut app, (5, 2), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    for cell in [(6, 2), (7, 2)] {
        point_at(&mut app, cell, None);
        app.update();
    }
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    let def = editing(&app).def().unwrap();
    let piles: Vec<(u16, u16)> = def
        .ground
        .iter()
        .filter(|g| g.y == 2)
        .map(|g| (g.x, g.y))
        .collect();
    assert_eq!(piles, vec![(5, 2)], "the press places, the drag does not");
    assert_eq!(depth(&app), steps + 1);
}

#[test]
fn a_right_click_on_a_placement_picks_its_stamp_and_detail() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    assert!(matches!(editing(&app).entry, Entry::Terrain(_)));
    // The inn's site: the activity stamp with the inn's service held.
    right_click(&mut app, (1, 1));
    assert_eq!(editing(&app).entry, Entry::Stamp(Placing::Activity));
    assert_eq!(editing(&app).tool, Tool::Brush);
    assert_eq!(held(&app, Placing::Activity), ["base:service:inn"]);
    assert_eq!(editing(&app).cursor, (1, 1));
    // The portal on (11, 2): its stamp with the portal's facing and marker, the link left
    // behind (a link is one portal's; a copy would claim the partner twice).
    right_click(&mut app, (11, 2));
    assert_eq!(editing(&app).entry, Entry::Stamp(Placing::Portal));
    let gate = editing(&app)
        .def()
        .unwrap()
        .portals
        .iter()
        .find(|p| (p.x, p.y) == (11, 2))
        .cloned()
        .expect("the portal");
    assert!(gate.link.is_some(), "the gate states its link");
    match editing(&app).stamps.held(Placing::Portal) {
        [Place::Portal(p)] => assert_eq!(
            (p.link.as_deref(), p.facing, &p.marker),
            (None, gate.facing, &gate.marker)
        ),
        other => panic!("{other:?}"),
    }
    assert!(held(&app, Placing::Portal).is_empty(), "no link held");
    // A bare street cell picks its terrain, as before.
    right_click(&mut app, (5, 2));
    let street = editing(&app)
        .grid()
        .unwrap()
        .tile(5, 2)
        .flatten()
        .expect("ground");
    assert_eq!(editing(&app).entry, Entry::Terrain(street));
    assert_eq!(depth(&app), steps, "a pick changes nothing");
}

#[test]
fn the_tools_keep_their_entries_and_the_inspector_has_no_add_row() {
    let mut app = in_editor("test:map:town");
    let data = data(&app);
    let def = editing(&app).def().unwrap();
    let text = screen_text(app.world());
    assert!(!text.contains("Add here"), "{text}");
    // Fill takes a terrain or the height: a stamp chosen gives way to the first terrain.
    choose(&mut app, Entry::Stamp(Placing::Object));
    press_edit(&mut app, EditAct::Tool(Tool::Fill));
    assert_eq!(editing(&app).entry, Entry::first(&def));
    // Choosing a stamp with Fill current chooses the brush.
    choose(&mut app, Entry::Stamp(Placing::Object));
    assert_eq!(editing(&app).tool, Tool::Brush);
    // A run tool with a stamp chosen takes the wall.
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    assert_eq!(
        editing(&app).entry,
        Entry::Edge(omnis_sim::omnis_data::edit::EdgeKind::Wall)
    );
    // The summary under the palette: the packs' first while nothing is chosen, then what is
    // held, by the ids' last segments.
    choose(&mut app, Entry::Stamp(Placing::Lore));
    let options = stamp::options(Placing::Lore, &data);
    assert!(options.len() > 1, "{options:?}");
    let summary = |app: &App| editing(app).stamps.summary(Placing::Lore, &data);
    assert_eq!(
        summary(&app),
        format!("nothing chosen, the first: {}", stamp::short(&options[0]))
    );
    hold(&mut app, Placing::Lore, &[&options[1], &options[0]]);
    assert_eq!(
        summary(&app),
        format!(
            "{}, {}",
            stamp::short(&options[1]),
            stamp::short(&options[0])
        )
    );
    assert!(
        screen_text(app.world()).contains(&summary(&app)),
        "under the palette"
    );
    // Every stamp has its icon in the palette grid.
    let tools = omnis_vector::editor::view::tools(editing(&app), &data);
    for (i, entry) in brush::palette(&def).iter().enumerate() {
        if let Entry::Stamp(kind) = entry {
            let c = tools
                .control(ControlId::Edit(EditAct::Brush(u8::try_from(i).unwrap())))
                .expect("a palette icon");
            assert_eq!(c.label, kind.label());
            assert!(
                matches!(c.kind, Kind::Icon { name, .. } if Some(name) == entry.icon()),
                "{:?}",
                c.kind
            );
        }
    }
}
