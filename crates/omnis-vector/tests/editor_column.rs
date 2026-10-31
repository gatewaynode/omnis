//! The editor's left column as a drawing GUI, step 1 (the owner, 2026-10-25): a grid of
//! pointer-state icons (select, brush, run by corners, freehand run, eraser, then undo and redo),
//! a palette grid (a swatch a terrain, then wall, door, ramp and the height) with the chosen
//! entry's details under it, then the files and Save, Revert, Playtest and the menu; the view's
//! controls and the level choice on the bar, the level's edits in the inspector. Select moves the
//! cursor and paints nothing; the eraser opens an edge, removes a placement or clears the ground
//! under the pointer. Headless on the town.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::tap;
use common::editor::{editing, in_editor, mouse, point_at, point_frac, press_edit};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::edit::EdgeKind;
use omnis_vector::editor::brush::{self, Entry, Tool};
use omnis_vector::editor::{EditAct, view};
use omnis_vector::screen::{ControlId, Kind, Screen};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;

fn tools(app: &App) -> Screen {
    let data = app.world().resource::<Packs>().data.clone();
    view::tools(editing(app), &data)
}

/// Whether the control is the chosen one of its grid.
fn chosen(screen: &Screen, id: ControlId) -> bool {
    match &screen.control(id).expect("the control").kind {
        Kind::Icon { chosen, .. } | Kind::Swatch { chosen, .. } => *chosen,
        other => panic!("{other:?} is no grid control"),
    }
}

fn depth(app: &App) -> usize {
    editing(app).status.undo
}

fn click(app: &mut App, cell: (u16, u16), side: Option<Facing>) {
    point_at(app, cell, side);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

#[test]
fn the_column_is_a_tool_grid_a_palette_grid_the_details_the_files_and_the_buttons_in_ascii() {
    let app = in_editor("test:map:town");
    let def = editing(&app).def().expect("open");
    let screen = tools(&app);
    // The tool grid: seven tools as icons, the brush chosen to begin, then undo and redo.
    for tool in Tool::ALL {
        let c = screen
            .control(ControlId::Edit(EditAct::Tool(tool)))
            .expect("a tool icon");
        assert_eq!(
            c.kind,
            Kind::Icon {
                name: tool.icon(),
                chosen: tool == Tool::Brush
            }
        );
    }
    for (act, name) in [(EditAct::Undo, "undo-2"), (EditAct::Redo, "redo-2")] {
        let c = screen.control(ControlId::Edit(act)).expect("history icon");
        assert!(matches!(c.kind, Kind::Icon { name: n, .. } if n == name));
        assert!(c.why_not.is_some(), "nothing to undo yet");
    }
    // The palette grid: a swatch a terrain in its colour with its glyph, then ten icons (the
    // wall, the door, the ramp, the height, a stamp a placement kind).
    let palette = brush::palette(&def);
    for (i, entry) in palette.iter().enumerate() {
        let c = screen
            .control(ControlId::Edit(EditAct::Brush(u8::try_from(i).unwrap())))
            .expect("a palette control");
        match entry {
            Entry::Terrain(t) => {
                let terrain = &def.terrains[usize::from(*t)];
                assert_eq!(
                    c.kind,
                    Kind::Swatch {
                        rgb: terrain.color,
                        glyph: terrain.glyph,
                        chosen: i == 0
                    }
                );
            }
            _ => assert_eq!(
                c.kind,
                Kind::Icon {
                    name: entry.icon().expect("an icon"),
                    chosen: false
                }
            ),
        }
    }
    // The details: the first terrain's words with its digit, Edit terrain and Add terrain.
    let text = screen_text(app.world());
    let first = &def.terrains[0];
    assert!(
        text.contains(&format!("\"1  {} {}\"", first.glyph, first.name)),
        "{text}"
    );
    assert!(
        screen
            .control(ControlId::Edit(EditAct::TerrainEdit))
            .is_some()
    );
    assert!(
        screen
            .control(ControlId::Edit(EditAct::TerrainAdd))
            .is_some()
    );
    assert!(text.contains("\"Brush  B\""), "the tool line: {text}");
    // The files and the buttons beneath; the view and the level are not the column's.
    for act in [
        EditAct::Map,
        EditAct::Open,
        EditAct::Plan,
        EditAct::Files,
        EditAct::OpenFile,
        EditAct::Save,
        EditAct::Revert,
        EditAct::Playtest,
        EditAct::Menu,
    ] {
        assert!(screen.control(ControlId::Edit(act)).is_some(), "{act:?}");
    }
    for act in [
        EditAct::ZoomIn,
        EditAct::Level,
        EditAct::LevelAdd,
        EditAct::LevelEdit,
    ] {
        assert!(screen.control(ControlId::Edit(act)).is_none(), "{act:?}");
    }
    // Every label of the three editor screens is ASCII.
    let data = app.world().resource::<Packs>().data.clone();
    for s in [
        screen,
        view::bar(editing(&app), &data),
        view::inspector(editing(&app), &data),
    ] {
        for c in s.controls() {
            assert!(c.label.is_ascii(), "{}", c.label);
        }
    }
    // The bar holds the level choice, the inspector the level's edits.
    let bar = view::bar(editing(&app), &data);
    assert!(bar.control(ControlId::Edit(EditAct::Level)).is_some());
    let inspector = view::inspector(editing(&app), &data);
    assert!(
        inspector
            .control(ControlId::Edit(EditAct::LevelAdd))
            .is_some()
    );
    assert!(
        inspector
            .control(ControlId::Edit(EditAct::LevelEdit))
            .is_some_and(|c| c.why_not.is_some()),
        "the ground level is the map"
    );
}

#[test]
fn choosing_a_tool_or_an_entry_frames_it_and_the_keys_choose_tools() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Tool(Tool::Eraser));
    let screen = tools(&app);
    assert!(chosen(
        &screen,
        ControlId::Edit(EditAct::Tool(Tool::Eraser))
    ));
    assert!(!chosen(
        &screen,
        ControlId::Edit(EditAct::Tool(Tool::Brush))
    ));
    assert!(screen_text(app.world()).contains("\"Eraser  X\""));
    // Choosing an entry chooses to paint.
    press_edit(&mut app, EditAct::Brush(1));
    assert_eq!(editing(&app).tool, Tool::Brush);
    assert_eq!(editing(&app).entry, Entry::Terrain(1));
    let screen = tools(&app);
    assert!(chosen(&screen, ControlId::Edit(EditAct::Brush(1))));
    assert!(!chosen(&screen, ControlId::Edit(EditAct::Brush(0))));
    // A run tool takes an edge, a terrain or the height (a cell run, step 4): with a terrain
    // chosen it stays; with a stamp chosen the wall is chosen for it.
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    assert_eq!(editing(&app).entry, Entry::Terrain(1));
    // Choosing the door or a terrain keeps the run tool; choosing a stamp brings the brush back.
    let def = editing(&app).def().unwrap();
    let palette = brush::palette(&def);
    let at =
        |entry: Entry| u8::try_from(palette.iter().position(|e| *e == entry).unwrap()).unwrap();
    press_edit(&mut app, EditAct::Brush(at(Entry::Edge(EdgeKind::Door))));
    assert_eq!(editing(&app).tool, Tool::Corners);
    assert_eq!(editing(&app).entry, Entry::Edge(EdgeKind::Door));
    press_edit(&mut app, EditAct::Brush(0));
    assert_eq!(
        editing(&app).tool,
        Tool::Corners,
        "a terrain is a cell run's"
    );
    let lore = Entry::Stamp(omnis_vector::editor::forms::Placing::Lore);
    press_edit(&mut app, EditAct::Brush(at(lore)));
    assert_eq!(editing(&app).tool, Tool::Brush);
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    assert_eq!(
        editing(&app).entry,
        Entry::Edge(EdgeKind::Wall),
        "a stamp has no run"
    );
    press_edit(&mut app, EditAct::Brush(1));
    // The keys.
    for (key, tool) in [
        (KeyCode::KeyV, Tool::Select),
        (KeyCode::KeyX, Tool::Eraser),
        (KeyCode::KeyP, Tool::Corners),
        (KeyCode::KeyN, Tool::Freehand),
        (KeyCode::KeyB, Tool::Brush),
    ] {
        tap(&mut app, key);
        assert_eq!(editing(&app).tool, tool, "{key:?}");
    }
    // The height entry shows its eighths choice in the details.
    let height = brush::palette(&def)
        .iter()
        .position(|e| *e == Entry::Height)
        .unwrap();
    press_edit(&mut app, EditAct::Brush(u8::try_from(height).unwrap()));
    assert!(
        tools(&app)
            .control(ControlId::Edit(EditAct::Eighths))
            .is_some()
    );
    assert!(
        tools(&app)
            .control(ControlId::Edit(EditAct::TerrainEdit))
            .is_none(),
        "Edit terrain is a terrain's"
    );
}

#[test]
fn select_moves_the_cursor_and_paints_nothing() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Tool(Tool::Select));
    let before = editing(&app).grid().unwrap().clone();
    let steps = depth(&app);
    click(&mut app, (3, 2), None);
    assert_eq!(editing(&app).cursor, (3, 2));
    click(&mut app, (5, 1), Some(Facing::North));
    assert_eq!(editing(&app).cursor, (5, 1), "a border is the cell too");
    assert_eq!(
        editing(&app).hover.map(|t| t.edge),
        Some(None),
        "select aims at cells, never sides"
    );
    assert_eq!(*editing(&app).grid().unwrap(), before);
    assert_eq!(depth(&app), steps);
    assert!(!editing(&app).status.dirty);
}

#[test]
fn the_eraser_opens_an_edge_removes_a_placement_or_clears_the_ground_under_the_pointer() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Tool(Tool::Eraser));
    let steps = depth(&app);
    // A wall's side: open. The street's north side at (3, 2) is a wall with a door beside.
    assert_eq!(
        editing(&app).grid().unwrap().edge(3, 2, Facing::North),
        Some(EdgeKind::Wall)
    );
    point_frac(&mut app, (3, 2), 0.5, 0.05);
    app.update();
    assert_eq!(
        editing(&app).hover.and_then(|t| t.edge),
        Some(Facing::North),
        "the aim shows the side before the click"
    );
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(
        editing(&app).grid().unwrap().edge(3, 2, Facing::North),
        Some(EdgeKind::Open)
    );
    assert_eq!(depth(&app), steps + 1);
    // A cell with a placement: the placement goes, the ground stays.
    let (x, y) = (11, 2);
    assert!(
        editing(&app)
            .def()
            .unwrap()
            .portals
            .iter()
            .any(|p| (p.x, p.y) == (x, y)),
        "the town's road out carries a portal"
    );
    let ground = editing(&app).grid().unwrap().tile(x, y);
    click(&mut app, (x, y), None);
    let e = editing(&app);
    assert!(
        e.def()
            .unwrap()
            .portals
            .iter()
            .all(|p| (p.x, p.y) != (x, y))
    );
    assert_eq!(e.grid().unwrap().tile(x, y), ground, "the ground stays");
    assert_eq!(e.cursor, (x, y));
    assert_eq!(depth(&app), steps + 2);
    // A bare cell: no ground; a drag over three is one step.
    click(&mut app, (x, y), None);
    assert_eq!(editing(&app).grid().unwrap().tile(x, y), Some(None));
    assert_eq!(depth(&app), steps + 3);
    point_at(&mut app, (1, 2), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    for x in 2..4 {
        point_at(&mut app, (x, 2), None);
        app.update();
    }
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    for x in 1..4 {
        assert_eq!(editing(&app).grid().unwrap().tile(x, 2), Some(None), "{x}");
    }
    assert_eq!(depth(&app), steps + 4, "one stroke");
    // A boundary side is refused and said; nothing changes.
    click(&mut app, (0, 2), Some(Facing::West));
    assert_eq!(depth(&app), steps + 4);
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("boundary")),
        "{:?}",
        editing(&app).message
    );
    // Undo takes each step back; the portal returns.
    for _ in 0..4 {
        press_edit(&mut app, EditAct::Undo);
    }
    assert!(
        editing(&app)
            .def()
            .unwrap()
            .portals
            .iter()
            .any(|p| (p.x, p.y) == (11, 2))
    );
    // A right click on an open edge chooses the eraser; on a terrain cell the brush and its
    // swatch.
    press_edit(&mut app, EditAct::Tool(Tool::Brush));
    point_at(&mut app, (3, 1), Some(Facing::North));
    mouse(&mut app, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Right, ButtonState::Released);
    assert_eq!(
        editing(&app).tool,
        Tool::Eraser,
        "{:?}",
        editing(&app).hover
    );
    point_at(&mut app, (5, 2), None);
    mouse(&mut app, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Right, ButtonState::Released);
    let e = editing(&app);
    let terrain = e.grid().unwrap().tile(5, 2).unwrap().unwrap();
    assert_eq!((e.tool, e.entry), (Tool::Brush, Entry::Terrain(terrain)));
}

#[test]
fn a_run_takes_the_entry_s_edge_kind() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    let def = editing(&app).def().unwrap();
    let door = brush::palette(&def)
        .iter()
        .position(|e| *e == Entry::Edge(EdgeKind::Door))
        .unwrap();
    press_edit(&mut app, EditAct::Brush(u8::try_from(door).unwrap()));
    let steps = depth(&app);
    for corner in [(2, 1), (5, 1), (5, 1)] {
        point_frac(&mut app, corner, 0.0, 0.0);
        app.update();
        mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
        mouse(&mut app, MouseButton::Left, ButtonState::Released);
    }
    assert_eq!(depth(&app), steps + 1, "{:?}", editing(&app).message);
    for x in 2..5 {
        assert_eq!(
            editing(&app).grid().unwrap().edge(x, 1, Facing::North),
            Some(EdgeKind::Door),
            "x {x}"
        );
    }
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("3 sides set to door")
    );
}
