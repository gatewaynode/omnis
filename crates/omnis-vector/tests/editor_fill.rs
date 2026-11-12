//! The fill tool (the editor's column, step 2, 2026-10-25): a click paints the palette's entry
//! over the region of one ground under it, bounded by every closed side, one undo step; a
//! terrain the region already is fills nothing; the fill takes a terrain or the height, so
//! choosing it with an edge chosen chooses the first terrain and choosing an edge leaves it for
//! the brush. Headless on the town, whose rooms are closed by walls and doors and whose green
//! lies open to the street.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::tap;
use common::editor::{editing, in_editor, mouse, point_at, press_edit};
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::{Entry, Tool};
use omnis_vector::editor::fill::region;
use omnis_vector::shell::editor::Editing;

/// The town's palette: nine terrains (street 0, green 1, inn 2 ... bank 8), then wall, door,
/// ramp and the height.
const WALL: u8 = 9;
const DOOR: u8 = 10;
const HEIGHT: u8 = 12;

fn depth(app: &App) -> usize {
    editing(app).status.undo
}

fn tile(app: &App, x: u16, y: u16) -> Option<u8> {
    editing(app).grid().expect("a grid").tile(x, y).flatten()
}

fn click(app: &mut App, cell: (u16, u16)) {
    point_at(app, cell, None);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

/// Every cell of the inn: x 0..3, y 0..2.
fn inn() -> Vec<(u16, u16)> {
    (0..3).flat_map(|x| (0..2).map(move |y| (x, y))).collect()
}

#[test]
fn a_fill_paints_the_region_of_one_ground_bounded_by_closed_sides_as_one_step() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    // The region of the inn's cell is the inn: its door onto the street bounds it.
    let mut inn_region = region(editing(&app).grid().unwrap(), (0, 0));
    inn_region.sort_unstable();
    assert_eq!(inn_region, inn());
    press_edit(&mut app, EditAct::Brush(1));
    press_edit(&mut app, EditAct::Tool(Tool::Fill));
    assert_eq!(editing(&app).tool, Tool::Fill);
    assert_eq!(editing(&app).entry, Entry::Terrain(1));
    click(&mut app, (0, 0));
    for (x, y) in inn() {
        assert_eq!(tile(&app, x, y), Some(1), "({x}, {y}) is green");
    }
    assert_eq!(
        tile(&app, 0, 2),
        Some(0),
        "the street beyond the door stays"
    );
    assert_eq!(
        tile(&app, 3, 0),
        Some(3),
        "the temple beyond the wall stays"
    );
    assert_eq!(depth(&app), steps + 1, "one step");
    assert_eq!(editing(&app).cursor, (0, 0));
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("6 cells filled with , green")
    );
    press_edit(&mut app, EditAct::Undo);
    for (x, y) in inn() {
        assert_eq!(tile(&app, x, y), Some(2), "({x}, {y}) is the inn again");
    }
    // An open side joins only the same ground: the green lies open to the street, and a fill of
    // the green with the inn's terrain leaves the street.
    press_edit(&mut app, EditAct::Brush(2));
    assert_eq!(editing(&app).tool, Tool::Fill, "a terrain keeps the fill");
    click(&mut app, (10, 3));
    for x in 9..12 {
        for y in 3..5 {
            assert_eq!(tile(&app, x, y), Some(2), "({x}, {y})");
        }
        assert_eq!(tile(&app, x, 2), Some(0), "the street at ({x}, 2) stays");
    }
    assert_eq!(depth(&app), steps + 1, "the undone fill left no step");
    // The ground the region already is fills nothing and says so.
    press_edit(&mut app, EditAct::Brush(0));
    click(&mut app, (5, 2));
    assert_eq!(depth(&app), steps + 1);
    assert_eq!(editing(&app).message.as_deref(), Some("already r street"));
    assert_eq!(
        editing(&app).cursor,
        (5, 2),
        "the cursor went there all the same"
    );
    // A drag fills once: the held button paints nothing more.
    press_edit(&mut app, EditAct::Brush(1));
    point_at(&mut app, (3, 0), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    for cell in [(4, 0), (4, 2), (7, 3)] {
        point_at(&mut app, cell, None);
        app.update();
    }
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(depth(&app), steps + 2, "one fill for the drag");
    assert_eq!(tile(&app, 4, 1), Some(1), "the temple is green");
    assert_eq!(tile(&app, 4, 2), Some(0), "the street is not");
    assert_eq!(tile(&app, 7, 3), Some(8), "the bank is not");
}

#[test]
fn a_fill_sets_the_height_over_the_region_and_takes_no_edge() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    // Choosing the fill with the wall chosen chooses the first terrain.
    press_edit(&mut app, EditAct::Brush(WALL));
    assert_eq!(
        editing(&app).entry,
        Entry::Edge(omnis_sim::omnis_data::edit::EdgeKind::Wall)
    );
    press_edit(&mut app, EditAct::Tool(Tool::Fill));
    assert_eq!(editing(&app).entry, Entry::Terrain(0));
    // The height fills the region's heights.
    press_edit(&mut app, EditAct::Brush(HEIGHT));
    assert_eq!(editing(&app).tool, Tool::Fill, "the height keeps the fill");
    app.world_mut().resource_mut::<Editing>().0.eighths = 3;
    click(&mut app, (1, 3));
    for x in 0..3 {
        for y in 3..5 {
            assert_eq!(
                editing(&app).grid().unwrap().height_at(x, y),
                Some(3),
                "({x}, {y})"
            );
        }
    }
    assert_eq!(
        editing(&app).grid().unwrap().height_at(1, 2),
        Some(0),
        "the street is flat"
    );
    assert_eq!(depth(&app), steps + 1);
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("6 cells set to height 3")
    );
    // Choosing an edge leaves the fill for the brush.
    press_edit(&mut app, EditAct::Brush(DOOR));
    assert_eq!(editing(&app).tool, Tool::Brush);
    // The key chooses the fill, as the icon does.
    tap(&mut app, KeyCode::KeyF);
    assert_eq!(editing(&app).tool, Tool::Fill);
    assert_eq!(
        editing(&app).entry,
        Entry::Terrain(0),
        "the door gave way to the first terrain"
    );
}
