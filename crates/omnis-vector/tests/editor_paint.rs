//! Brushes and painting (M5.5, `tasks/plans/m5-editor-v1.md` §Client side "Input"): a click
//! paints the brush at the cell or the edge under the pointer and a drag is one undo step, a
//! right click picks what is under the pointer, the digits choose brushes and Enter paints at
//! the cursor, Undo and Redo walk the steps, Save is refused in the loader's words while the map
//! would not load and otherwise writes, reseals and reloads the packs, and Revert reopens the
//! file.

use crate::common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{config, key, set, shell, tap};
use common::editor::{
    arrow, editing, in_editor, leave_plan, mouse, point_at, point_frac, press_edit, scratch_pack,
};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::edit::EdgeKind;
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::{self, Target, Tool};
use omnis_vector::editor::plan::View;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::editor::{Editing, PlanHover};
use omnis_vector::shell::screen::{Pressable, Screens};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;

/// Choose `tool` by its button in the tools column.
fn choose(app: &mut App, tool: Tool) {
    let def = editing(app).def().expect("a map is open");
    let i = brush::palette(&def)
        .iter()
        .position(|t| *t == tool)
        .expect("the tool is in the palette");
    press_edit(app, EditAct::Brush(u8::try_from(i).unwrap()));
    assert_eq!(editing(app).tool, tool);
}

/// A click at `cell`'s middle (or a pixel in from its `side`).
fn click_at(app: &mut App, cell: (u16, u16), side: Option<Facing>) {
    point_at(app, cell, side);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

/// A cell with ground, not on the boundary, whose `side` edge is open.
fn open_edge(app: &App, side: Facing) -> (u16, u16) {
    let e = editing(app);
    let grid = e.grid().expect("the ground");
    let (w, h) = e.size();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if grid.edge(x, y, side) == Some(EdgeKind::Open) && grid.tile(x, y).flatten().is_some()
            {
                return (x, y);
            }
        }
    }
    panic!("an open {side:?} edge inside the map")
}

fn title_dirty(app: &App) -> bool {
    screen_text(app.world())
        .lines()
        .any(|l| l.contains("EDITOR  ") && l.trim_end_matches('"').ends_with("  *"))
}

#[test]
fn a_click_paints_the_brush_at_the_cell_and_a_drag_is_one_undo_step() {
    let mut app = in_editor("test:map:town");
    assert!(!title_dirty(&app));
    choose(&mut app, Tool::Clear);
    point_at(&mut app, (1, 1), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    let e = editing(&app);
    assert_eq!(
        e.grid().unwrap().tile(1, 1),
        Some(None),
        "no ground painted"
    );
    assert_eq!(e.cursor, (1, 1), "the cursor follows the paint");
    assert_eq!((e.status.undo, e.status.dirty), (1, true));
    assert!(title_dirty(&app), "{}", screen_text(app.world()));
    // Still held, the pointer moves on: the same stroke, one step.
    point_at(&mut app, (2, 1), None);
    app.update();
    point_at(&mut app, (3, 1), None);
    app.update();
    let e = editing(&app);
    assert_eq!(e.grid().unwrap().tile(3, 1), Some(None));
    assert_eq!(e.status.undo, 1, "a drag is one step");
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    point_at(&mut app, (4, 1), None);
    app.update();
    assert_ne!(
        editing(&app).grid().unwrap().tile(4, 1),
        Some(None),
        "nothing painted after the release"
    );
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    let e = editing(&app);
    assert_eq!(e.grid().unwrap().tile(4, 1), Some(None));
    assert_eq!(e.status.undo, 2, "a new click is a new step");

    press_edit(&mut app, EditAct::Undo);
    let e = editing(&app);
    assert_ne!(e.grid().unwrap().tile(4, 1), Some(None), "the click undone");
    assert_eq!(e.grid().unwrap().tile(3, 1), Some(None), "the drag stands");
    press_edit(&mut app, EditAct::Undo);
    let e = editing(&app);
    assert_ne!(
        e.grid().unwrap().tile(1, 1),
        Some(None),
        "the drag undone whole"
    );
    assert_eq!(
        (e.status.undo, e.status.redo, e.status.dirty),
        (0, 2, false)
    );
    assert!(!title_dirty(&app));
    press_edit(&mut app, EditAct::Redo);
    let e = editing(&app);
    assert_eq!(e.grid().unwrap().tile(3, 1), Some(None), "redone");
    assert_eq!((e.status.undo, e.status.redo), (1, 1));
    let text = screen_text(app.world());
    assert!(text.contains("[button] \"Undo"), "{text}");
    assert!(text.contains("[button] \"Redo"), "{text}");
}

#[test]
fn the_band_along_a_cell_s_border_targets_that_edge_and_the_edge_brushes_paint_it() {
    let view = View {
        origin: (0, 0),
        zoom: 3,
    };
    let size = (10, 10);
    let at = |px: f32, py: f32| view.target(size, px, py);
    assert_eq!(
        at(8.0, 8.0),
        Some(Target {
            cell: (0, 0),
            edge: None
        }),
        "the middle"
    );
    assert_eq!(at(8.0, 1.0).unwrap().edge, Some(Facing::North));
    assert_eq!(at(15.0, 8.0).unwrap().edge, Some(Facing::East));
    assert_eq!(at(8.0, 14.0).unwrap().edge, Some(Facing::South));
    assert_eq!(at(2.0, 8.0).unwrap().edge, Some(Facing::West));
    assert_eq!(
        at(8.0, 5.0).unwrap().edge,
        None,
        "past the band (a quarter)"
    );
    assert_eq!(
        at(1.0, 2.0).unwrap().edge,
        Some(Facing::West),
        "a corner: the nearer"
    );
    assert_eq!(at(20.0, 8.0).unwrap().cell, (1, 0));
    assert_eq!(at(-1.0, 8.0), None);

    let mut app = in_editor("test:map:town");
    let cell = open_edge(&app, Facing::East);
    choose(&mut app, Tool::Edge(EdgeKind::Wall));
    click_at(&mut app, cell, Some(Facing::East));
    let e = editing(&app);
    assert_eq!(
        e.grid().unwrap().edge(cell.0, cell.1, Facing::East),
        Some(EdgeKind::Wall)
    );
    assert_eq!(e.cursor, cell);
    let text = screen_text(app.world());
    assert!(text.contains("E wall"), "the inspector: {text}");

    // From the keys, an edge brush paints the side the cursor faces.
    let cell = open_edge(&app, Facing::West);
    choose(&mut app, Tool::Edge(EdgeKind::Door));
    app.world_mut().resource_mut::<Editing>().point(cell);
    while editing(&app).facing != Facing::West {
        press_edit(&mut app, EditAct::FaceLeft);
    }
    tap(&mut app, KeyCode::Enter);
    let e = editing(&app);
    assert_eq!(
        e.grid().unwrap().edge(cell.0, cell.1, Facing::West),
        Some(EdgeKind::Door)
    );
    choose(&mut app, Tool::Edge(EdgeKind::Open));
    click_at(&mut app, cell, Some(Facing::West));
    assert_eq!(
        editing(&app)
            .grid()
            .unwrap()
            .edge(cell.0, cell.1, Facing::West),
        Some(EdgeKind::Open),
        "opened again"
    );

    // A boundary edge is always a wall: refused and said, nothing changed.
    choose(&mut app, Tool::Edge(EdgeKind::Door));
    let steps = editing(&app).status.undo;
    click_at(&mut app, (0, 2), Some(Facing::West));
    let e = editing(&app);
    assert_eq!(
        e.grid().unwrap().edge(0, 2, Facing::West),
        Some(EdgeKind::Wall)
    );
    assert_eq!(e.status.undo, steps);
    assert!(
        e.message.as_deref().is_some_and(|m| m.contains("boundary")),
        "{:?}",
        e.message
    );
}

#[test]
fn the_digits_choose_the_first_nine_brushes_and_the_height_steps_by_key_and_arrow() {
    let mut app = in_editor("test:map:town");
    let def = editing(&app).def().unwrap();
    let palette = brush::palette(&def);
    assert!(palette.len() >= 9, "{palette:?}");
    assert_eq!(editing(&app).tool, palette[0], "the first terrain to begin");
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(editing(&app).tool, palette[1]);
    let text = screen_text(app.world());
    assert!(text.contains("\"> 2  "), "the current brush marked: {text}");
    assert!(text.contains("\"1  "), "the others by their digit: {text}");
    tap(&mut app, KeyCode::Digit9);
    assert_eq!(editing(&app).tool, palette[8]);
    press_edit(&mut app, EditAct::Brush(0));
    assert_eq!(editing(&app).tool, palette[0]);

    choose(&mut app, Tool::Height);
    assert_eq!(editing(&app).eighths, 0);
    for _ in 0..3 {
        tap(&mut app, KeyCode::Equal);
    }
    assert_eq!(editing(&app).eighths, 3, "= raises the height");
    arrow(&mut app, EditAct::Eighths, false);
    assert_eq!(editing(&app).eighths, 2, "the choice's arrow");
    tap(&mut app, KeyCode::Minus);
    assert_eq!(editing(&app).eighths, 1);
    let text = screen_text(app.world());
    assert!(text.contains("\"1\""), "the height shown: {text}");
    tap(&mut app, KeyCode::Enter);
    let e = editing(&app);
    let (x, y) = e.cursor;
    assert_eq!(
        e.grid().unwrap().height_at(x, y),
        Some(1),
        "Enter paints at the cursor"
    );
    let text = screen_text(app.world());
    assert!(text.contains("Height 1 eighths"), "{text}");
    for _ in 0..200 {
        tap(&mut app, KeyCode::Minus);
    }
    assert_eq!(
        editing(&app).eighths,
        *brush::EIGHTHS.start(),
        "held to the brush's range"
    );
}

#[test]
fn a_right_click_picks_what_is_under_the_pointer_and_paints_nothing() {
    let mut app = in_editor("test:map:town");
    choose(&mut app, Tool::Height);
    for _ in 0..3 {
        tap(&mut app, KeyCode::Equal);
    }
    app.world_mut().resource_mut::<Editing>().point((2, 2));
    tap(&mut app, KeyCode::Enter);
    assert_eq!(editing(&app).grid().unwrap().height_at(2, 2), Some(3));
    let before = editing(&app).grid().unwrap().clone();
    let steps = editing(&app).status.undo;
    choose(&mut app, Tool::Clear);
    tap(&mut app, KeyCode::Minus);

    point_at(&mut app, (2, 2), None);
    mouse(&mut app, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Right, ButtonState::Released);
    let e = editing(&app);
    let terrain = before.tile(2, 2).unwrap().unwrap();
    assert_eq!(
        e.tool,
        Tool::Terrain(terrain),
        "the terrain under the pointer"
    );
    assert_eq!(e.eighths, 3, "and its height");
    assert_eq!(e.cursor, (2, 2), "the cursor goes there");
    assert_eq!(*e.grid().unwrap(), before, "nothing painted");
    assert_eq!(e.status.undo, steps);

    point_at(&mut app, (0, 0), Some(Facing::West));
    mouse(&mut app, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Right, ButtonState::Released);
    assert_eq!(
        editing(&app).tool,
        Tool::Edge(EdgeKind::Wall),
        "the boundary's wall"
    );
    let cell = open_edge(&app, Facing::North);
    point_at(&mut app, cell, Some(Facing::North));
    mouse(&mut app, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Right, ButtonState::Released);
    assert_eq!(editing(&app).tool, Tool::Edge(EdgeKind::Open));
    assert_eq!(*editing(&app).grid().unwrap(), before);
}

/// A copy of the test pack to save into.
fn test_version(app: &App) -> String {
    app.world()
        .resource::<Packs>()
        .data
        .packs
        .iter()
        .find(|p| p.id == "test")
        .map(|p| p.version.clone())
        .expect("the test pack")
}

#[test]
fn save_is_refused_in_the_loader_s_words_while_invalid_and_writes_reseals_and_reloads_when_valid() {
    let scratch = scratch_pack("editor-paint-save");
    let mut config = config();
    config.packs.pop();
    config.packs.push(scratch.clone());
    config.editor = Some(Some("test:map:town".to_owned()));
    let mut app = shell(config, false);
    app.update();
    let file = scratch.join("data/maps/town.ron");
    let text_before = std::fs::read_to_string(&file).unwrap();
    let version_before = test_version(&app);

    // A ramp joining heights three walls apart: the loader refuses, Save is dim with the
    // count and the first refusal.
    let ramp = open_edge(&app, Facing::East);
    choose(&mut app, Tool::Edge(EdgeKind::Ramp));
    click_at(&mut app, ramp, Some(Facing::East));
    assert!(editing(&app).status.errors.is_empty(), "a flat ramp walks");
    choose(&mut app, Tool::Height);
    for _ in 0..24 {
        tap(&mut app, KeyCode::Equal);
    }
    tap(&mut app, KeyCode::Enter);
    let e = editing(&app);
    assert_eq!(e.cursor, ramp, "the cursor stayed on the ramp's cell");
    assert!(
        e.status
            .errors
            .iter()
            .any(|m| m.contains("more than 16 apart")),
        "the loader refuses: {:?}",
        e.status.errors
    );
    let text = screen_text(app.world());
    assert!(!text.contains("[button] \"Save"), "dim: {text}");
    assert!(
        text.contains("\"Save  Cmd S: 1 error: "),
        "with the reason: {text}"
    );
    key(&mut app, KeyCode::SuperLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::KeyS);
    key(&mut app, KeyCode::SuperLeft, ButtonState::Released);
    app.update();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        text_before,
        "nothing written"
    );
    assert_eq!(test_version(&app), version_before);
    let e = editing(&app);
    assert!(
        e.message
            .as_deref()
            .is_some_and(|m| m.starts_with("Not saved")),
        "{:?}",
        e.message
    );

    press_edit(&mut app, EditAct::Undo);
    let cell = open_edge(&app, Facing::East);
    choose(&mut app, Tool::Edge(EdgeKind::Wall));
    click_at(&mut app, cell, Some(Facing::East));
    assert!(editing(&app).status.dirty);
    let text = screen_text(app.world());
    assert!(text.contains("[button] \"Save"), "live again: {text}");
    press_edit(&mut app, EditAct::Save);
    let e = editing(&app);
    assert!(!e.status.dirty, "clean after the save");
    assert!(
        e.message.as_deref().is_some_and(|m| m.starts_with("Saved")),
        "{:?}",
        e.message
    );
    let written = std::fs::read_to_string(&file).unwrap();
    assert_ne!(written, text_before, "the file written");
    let version = test_version(&app);
    assert_ne!(
        version, version_before,
        "the packs reloaded with the new seal"
    );
    assert!(!title_dirty(&app));
    let grid = editing(&app).grid().unwrap();
    assert_eq!(
        grid.edge(cell.0, cell.1, Facing::East),
        Some(EdgeKind::Wall)
    );

    // Cmd+S saves too, and the menu's Save: each change saved reseals again (the same content
    // keeps its version: the seal is of the content).
    key(&mut app, KeyCode::SuperLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::KeyS);
    key(&mut app, KeyCode::SuperLeft, ButtonState::Released);
    app.update();
    assert_eq!(
        test_version(&app),
        version,
        "nothing changed: the same seal"
    );
    let next = open_edge(&app, Facing::South);
    click_at(&mut app, next, Some(Facing::South));
    key(&mut app, KeyCode::SuperLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::KeyS);
    key(&mut app, KeyCode::SuperLeft, ButtonState::Released);
    app.update();
    let again = test_version(&app);
    assert_ne!(again, version, "Cmd+S saved the change");
    assert!(!editing(&app).status.dirty);
    let next = open_edge(&app, Facing::North);
    click_at(&mut app, next, Some(Facing::North));
    tap(&mut app, KeyCode::Escape);
    let text = screen_text(app.world());
    assert!(
        text.contains("[button] \"Save\""),
        "the menu's Save: {text}"
    );
    set(
        &mut app,
        &Pressable(ControlId::Edit(EditAct::Save)),
        Interaction::Pressed,
    );
    app.update();
    app.update();
    assert_ne!(test_version(&app), again, "the menu saved");
    let text = screen_text(app.world());
    assert!(text.contains("Saved "), "said on the menu: {text}");
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn revert_reopens_the_file_and_the_menu_warns_before_leaving_with_changes() {
    let mut app = in_editor("test:map:town");
    tap(&mut app, KeyCode::Escape);
    let text = screen_text(app.world());
    assert!(text.contains("[button] \"Back to title\""), "{text}");
    assert!(text.contains("[button] \"Save\""), "{text}");
    tap(&mut app, KeyCode::Escape);
    let text = screen_text(app.world());
    assert!(
        !text.contains("[button] \"Revert"),
        "nothing to revert: {text}"
    );

    let before = editing(&app).grid().unwrap().clone();
    choose(&mut app, Tool::Clear);
    click_at(&mut app, (1, 1), None);
    assert!(editing(&app).status.dirty);
    tap(&mut app, KeyCode::Escape);
    let top = app.world().resource::<Screens>().nav.top(None);
    assert_eq!(top, Some(ScreenId::EditorMenu));
    let text = screen_text(app.world());
    assert!(
        text.contains("[button] \"Discard changes and leave\""),
        "{text}"
    );
    assert!(!text.contains("\"Back to title\""), "{text}");
    tap(&mut app, KeyCode::Escape);

    press_edit(&mut app, EditAct::Revert);
    let e = editing(&app);
    assert_eq!(*e.grid().unwrap(), before, "the file's layout again");
    assert_eq!((e.status.dirty, e.status.undo), (false, 0));
    assert_eq!(e.cursor, (1, 1), "the cursor stays");
    assert!(
        e.message
            .as_deref()
            .is_some_and(|m| m.starts_with("Reverted")),
        "{:?}",
        e.message
    );

    // Cmd+Z undoes, Cmd+Shift+Z redoes.
    click_at(&mut app, (1, 1), None);
    assert_eq!(editing(&app).grid().unwrap().tile(1, 1), Some(None));
    key(&mut app, KeyCode::SuperLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::KeyZ);
    assert_eq!(*editing(&app).grid().unwrap(), before, "Cmd+Z");
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::KeyZ);
    assert_eq!(
        editing(&app).grid().unwrap().tile(1, 1),
        Some(None),
        "Cmd+Shift+Z"
    );
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Released);
    key(&mut app, KeyCode::SuperLeft, ButtonState::Released);
    app.update();
    // With a modifier held, C is not Centre and Z is not a brush: nothing else moved.
    assert_eq!(editing(&app).cursor, (1, 1));
}

/// The hover node's place and whether it shows.
fn hover_node(app: &mut App) -> Node {
    let world = app.world_mut();
    let mut hovers = world.query_filtered::<&Node, With<PlanHover>>();
    hovers.single(world).expect("one hover node").clone()
}

fn px(v: Val) -> f32 {
    match v {
        Val::Px(p) => p,
        other => panic!("{other:?} is not in pixels"),
    }
}

#[test]
fn an_edge_brush_aims_at_the_nearest_side_wherever_the_click_lands_and_the_hover_shows_it() {
    let mut app = in_editor("test:map:town");
    let cell = open_edge(&app, Facing::West);
    #[allow(clippy::cast_precision_loss)]
    let (left, top, side) = {
        let (l, t, s) = editing(&app).view.cell_rect(cell);
        (l as f32, t as f32, s as f32)
    };
    // Past the quarter band, nearer the west side than any other: the aim is that side, and
    // the hover is a bar along it before anything is clicked.
    choose(&mut app, Tool::Edge(EdgeKind::Wall));
    point_frac(&mut app, cell, 0.35, 0.5);
    app.update();
    assert_eq!(
        editing(&app).hover,
        Some(Target {
            cell,
            edge: Some(Facing::West)
        })
    );
    let bar = hover_node(&mut app);
    assert_eq!(bar.display, Display::Flex);
    assert_eq!(px(bar.height), side, "as tall as the cell");
    assert!(
        px(bar.width) < side / 2.0,
        "a bar, not a cell: {:?}",
        bar.width
    );
    assert!(
        (px(bar.left) - left).abs() <= side / 8.0 + 1.0,
        "{:?}",
        bar.left
    );
    assert_eq!(px(bar.top), top);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    let e = editing(&app);
    assert_eq!(
        e.grid().unwrap().edge(cell.0, cell.1, Facing::West),
        Some(EdgeKind::Wall),
        "{:?}",
        e.message
    );
    assert_eq!(
        e.facing,
        Facing::West,
        "the start's facing, never consulted"
    );

    // The eraser works the same way, and is named for it.
    choose(&mut app, Tool::Edge(EdgeKind::Open));
    assert!(
        screen_text(app.world()).contains("Open edge (erase)"),
        "the eraser says what it does"
    );
    point_frac(&mut app, cell, 0.3, 0.5);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(
        editing(&app)
            .grid()
            .unwrap()
            .edge(cell.0, cell.1, Facing::West),
        Some(EdgeKind::Open)
    );

    // Nearer the top: the north side.
    point_frac(&mut app, cell, 0.5, 0.2);
    app.update();
    assert_eq!(editing(&app).hover.unwrap().edge, Some(Facing::North));

    // A terrain brush at the same point aims at the cell: the hover is its outline.
    choose(&mut app, Tool::Terrain(0));
    point_frac(&mut app, cell, 0.35, 0.5);
    app.update();
    assert_eq!(editing(&app).hover, Some(Target { cell, edge: None }));
    let outline = hover_node(&mut app);
    assert_eq!((px(outline.left), px(outline.top)), (left, top));
    assert_eq!((px(outline.width), px(outline.height)), (side, side));

    // A right click picks what the hover shows: with an edge brush, the nearest side's kind.
    choose(&mut app, Tool::Edge(EdgeKind::Door));
    point_frac(&mut app, cell, 0.35, 0.5);
    mouse(&mut app, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Right, ButtonState::Released);
    assert_eq!(
        editing(&app).tool,
        Tool::Edge(EdgeKind::Open),
        "the west side is open"
    );

    // Off the plan, no aim and no hover.
    leave_plan(&mut app);
    app.update();
    assert_eq!(editing(&app).hover, None);
    assert_eq!(hover_node(&mut app).display, Display::None);
}
