//! The lasso, the selection and the cell runs (the editor's column, step 4, owner 2026-10-31): a
//! freehand loop dragged with the lasso selects the cells inside it, outlined on the plan and
//! counted under the tool's name; Move pressed inside it carries everything on the cells by the
//! drag's offset in one undo step, the source left bare and the destination replaced, a
//! tile-keyed kind already there refusing; a run tool with a terrain or the height paints the
//! cells of the line between its points. Headless on the town: the inn's block at the top left
//! (cells (0..2, 0..1), its site on (1, 1)), the shops' sites on their door cells, the portal on
//! (11, 2), the street row 2.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::tap;
use common::editor::{arrow, editing, in_editor, mouse, point_at, press_edit};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::edit::{EdgeKind, Grid};
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::{self, Entry, Tool};
use omnis_vector::editor::forms::Placing;
use omnis_vector::editor::run;
use omnis_vector::screen::{ControlId, Kind};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;
use std::collections::BTreeSet;

fn depth(app: &App) -> usize {
    editing(app).status.undo
}

fn message(app: &App) -> String {
    editing(app).message.clone().unwrap_or_default()
}

fn grid(app: &App) -> Grid {
    editing(app).grid().expect("the ground").clone()
}

fn site_at(app: &App, cell: (u16, u16)) -> Option<String> {
    editing(app)
        .def()
        .expect("open")
        .sites
        .iter()
        .find(|s| (s.x, s.y) == cell)
        .map(|s| s.service.clone())
}

fn selected(app: &App) -> BTreeSet<(u16, u16)> {
    editing(app)
        .selection
        .as_ref()
        .map(|s| s.cells.clone())
        .unwrap_or_default()
}

fn set(cells: &[(u16, u16)]) -> BTreeSet<(u16, u16)> {
    cells.iter().copied().collect()
}

/// A drag with `button` from `from` over `through` to the last of them, let go there.
fn drag(app: &mut App, button: MouseButton, from: (u16, u16), through: &[(u16, u16)]) {
    point_at(app, from, None);
    mouse(app, button, ButtonState::Pressed);
    for &cell in through {
        point_at(app, cell, None);
        app.update();
    }
    mouse(app, button, ButtonState::Released);
}

fn click(app: &mut App, cell: (u16, u16)) {
    point_at(app, cell, None);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

/// The inn's block, the six cells (0..2, 0..1), selected by a loop round them.
const INN: [(u16, u16); 6] = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)];

fn lasso_the_inn(app: &mut App) {
    press_edit(app, EditAct::Tool(Tool::Lasso));
    drag(
        app,
        MouseButton::Left,
        (0, 0),
        &[(1, 0), (2, 0), (2, 1), (1, 1), (0, 1)],
    );
    assert_eq!(selected(app), set(&INN));
}

#[test]
fn a_lasso_loop_selects_the_cells_inside_it_and_a_click_clears_the_selection() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    // The tool is in the grid with its icon and key.
    let tools = omnis_vector::editor::view::tools(editing(&app), &data);
    let icon = tools
        .control(ControlId::Edit(EditAct::Tool(Tool::Lasso)))
        .expect("the lasso's icon");
    assert!(
        matches!(
            icon.kind,
            Kind::Icon {
                name: "lasso-select",
                ..
            }
        ),
        "{:?}",
        icon.kind
    );
    tap(&mut app, KeyCode::KeyL);
    assert_eq!(editing(&app).tool, Tool::Lasso);
    assert!(screen_text(app.world()).contains("Lasso  L"));
    // A loop round the inn: the six cells, said, counted and outlined.
    let steps = depth(&app);
    lasso_the_inn(&mut app);
    assert_eq!(message(&app), "6 cells selected");
    assert_eq!(depth(&app), steps, "a selection changes nothing");
    assert!(editing(&app).lasso.is_none(), "the loop is done");
    let outline = editing(&app).selection_outline().expect("an outline");
    assert_eq!(
        outline.len(),
        10,
        "3 + 3 + 2 + 2 sides face out: {outline:?}"
    );
    assert!(outline.contains(&(true, 0, 0)) && outline.contains(&(true, 2, 2)));
    assert!(outline.contains(&(false, 0, 1)) && outline.contains(&(false, 3, 0)));
    assert!(!outline.contains(&(false, 1, 0)), "an inner side");
    assert!(screen_text(app.world()).contains("Selection: 6 cells"));
    // The selection stands through a tool change; a loop that encloses selects the middle too.
    press_edit(&mut app, EditAct::Tool(Tool::Brush));
    assert_eq!(selected(&app).len(), 6);
    press_edit(&mut app, EditAct::Tool(Tool::Lasso));
    drag(
        &mut app,
        MouseButton::Left,
        (3, 2),
        &[(5, 2), (5, 4), (3, 4)],
    );
    let got = selected(&app);
    assert_eq!(got.len(), 9, "{got:?}");
    assert!(got.contains(&(4, 3)), "the middle: {got:?}");
    assert_eq!(message(&app), "9 cells selected");
    // A click clears it; an act mid-loop drops the loop and selects nothing.
    click(&mut app, (7, 2));
    assert!(editing(&app).selection.is_none());
    assert_eq!(message(&app), "selection cleared");
    point_at(&mut app, (6, 3), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    point_at(&mut app, (7, 3), None);
    app.update();
    assert_eq!(editing(&app).lasso.as_ref().map(Vec::len), Some(2));
    // The loop in progress is previewed as the path of cells it has passed, joined cell to cell
    // when the pointer jumps (the owner's hand test of step 4: the drawn line must be seen
    // before it snaps to a selection).
    let drawn = editing(&app)
        .preview()
        .expect("the loop in progress is previewed");
    assert_eq!(drawn.cells, run::cells((6, 3), (7, 3)));
    assert!(drawn.sides.is_empty() && drawn.corner.is_none());
    point_at(&mut app, (9, 3), None);
    app.update();
    let drawn = editing(&app).preview().expect("still previewed");
    assert_eq!(drawn.cells, vec![(6, 3), (7, 3), (8, 3), (9, 3)]);
    press_edit(&mut app, EditAct::Tool(Tool::Select));
    assert!(editing(&app).preview().is_none(), "no loop, no preview");
    assert!(editing(&app).lasso.is_none(), "an act ends the loop");
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert!(editing(&app).selection.is_none());
    // Opening the map again drops a selection.
    press_edit(&mut app, EditAct::Tool(Tool::Lasso));
    lasso_the_inn(&mut app);
    press_edit(&mut app, EditAct::Open);
    assert!(editing(&app).selection.is_none());
}

#[test]
fn move_carries_the_selection_by_the_drag_s_offset_as_one_step_and_undo_puts_it_back() {
    let mut app = in_editor("test:map:town");
    let before = (editing(&app).def().unwrap(), grid(&app));
    let inn = site_at(&app, (1, 1)).expect("the inn's site");
    lasso_the_inn(&mut app);
    let steps = depth(&app);
    press_edit(&mut app, EditAct::Tool(Tool::Move));
    // A press inside the selection carries it; the outline follows the cursor as a ghost.
    point_at(&mut app, (1, 0), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    point_at(&mut app, (5, 1), None);
    app.update();
    let ghost = editing(&app).selection_outline().expect("the ghost");
    assert!(
        ghost.contains(&(true, 4, 1)) && ghost.contains(&(false, 7, 1)),
        "the outline four cells right and one down: {ghost:?}"
    );
    point_at(&mut app, (9, 2), None);
    app.update();
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    // Landed eight across and two down, in one step: the floor, the site, the walls.
    assert_eq!(depth(&app), steps + 1, "{}", message(&app));
    assert!(
        message(&app).starts_with("6 cells moved to (8, 2)"),
        "{}",
        message(&app)
    );
    let after = grid(&app);
    let inn_floor = before.1.tile(1, 1).flatten().expect("the inn's floor");
    for (x, y) in [(8, 2), (9, 2), (10, 2), (8, 3), (9, 3), (10, 3)] {
        assert_eq!(after.tile(x, y).flatten(), Some(inn_floor), "({x}, {y})");
    }
    for (x, y) in INN {
        assert_eq!(after.tile(x, y), Some(None), "({x}, {y}) is bare");
        assert_eq!(after.height_at(x, y), Some(0));
    }
    assert_eq!(site_at(&app, (9, 3)).as_deref(), Some(inn.as_str()));
    assert!(site_at(&app, (1, 1)).is_none());
    // The inn's east wall (east of (2, 0) and (2, 1)) went with it; its old side is open; the
    // shop wall the block landed over is replaced by the block's open inside.
    assert_eq!(after.edge(10, 2, Facing::East), Some(EdgeKind::Wall));
    assert_eq!(after.edge(10, 3, Facing::East), Some(EdgeKind::Wall));
    assert_eq!(after.edge(2, 0, Facing::East), Some(EdgeKind::Open));
    assert_eq!(after.edge(8, 3, Facing::East), Some(EdgeKind::Open));
    assert_eq!(
        selected(&app),
        set(&[(8, 2), (9, 2), (10, 2), (8, 3), (9, 3), (10, 3)]),
        "the selection moved with it"
    );
    // One undo restores every cell and placement.
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(depth(&app), steps);
    assert_eq!(editing(&app).def().unwrap(), before.0);
    assert_eq!(grid(&app), before.1);
    // A move onto a shop's door cell refuses: its site stands there.
    lasso_the_inn(&mut app);
    press_edit(&mut app, EditAct::Tool(Tool::Move));
    drag(&mut app, MouseButton::Left, (1, 0), &[(4, 1), (7, 2)]);
    assert_eq!(message(&app), "a site is already at (7, 3)");
    assert_eq!(depth(&app), steps);
    assert_eq!(grid(&app), before.1);
    // A move off the map refuses.
    drag(&mut app, MouseButton::Left, (1, 0), &[(0, 0)]);
    assert_eq!(message(&app), "the selection would leave the map");
    assert_eq!(depth(&app), steps);
    // An overlapping move writes the source bare before the destination.
    drag(&mut app, MouseButton::Left, (0, 0), &[(1, 0)]);
    assert_eq!(depth(&app), steps + 1, "{}", message(&app));
    let after = grid(&app);
    for x in 1..=3 {
        assert_eq!(after.tile(x, 0).flatten(), Some(inn_floor), "({x}, 0)");
    }
    assert_eq!(after.tile(0, 0), Some(None));
    assert_eq!(
        after.edge(3, 0, Facing::East),
        Some(EdgeKind::Wall),
        "the wall moved on"
    );
    assert_eq!(
        after.edge(2, 0, Facing::East),
        Some(EdgeKind::Open),
        "its old side opened"
    );
    assert_eq!(site_at(&app, (2, 1)).as_deref(), Some(inn.as_str()));
    assert_eq!(
        site_at(&app, (4, 1)).as_deref(),
        Some("base:service:temple"),
        "untouched"
    );
    // A press outside the selection carries what is under it, as before.
    press_edit(&mut app, EditAct::Undo);
    lasso_the_inn(&mut app);
    press_edit(&mut app, EditAct::Tool(Tool::Move));
    drag(&mut app, MouseButton::Left, (4, 1), &[(5, 2)]);
    assert_eq!(
        site_at(&app, (5, 2)).as_deref(),
        Some("base:service:temple")
    );
    assert!(
        message(&app).contains("moved to (5, 2)"),
        "{}",
        message(&app)
    );
}

#[test]
fn a_run_with_a_terrain_or_the_height_paints_the_cells_of_its_line() {
    let mut app = in_editor("test:map:town");
    let def = editing(&app).def().unwrap();
    let palette = brush::palette(&def);
    let at =
        |entry: Entry| u8::try_from(palette.iter().position(|e| *e == entry).unwrap()).unwrap();
    let steps = depth(&app);
    // By corners with the street: the points are cells, previewed as the line's cells.
    press_edit(&mut app, EditAct::Brush(0));
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    assert_eq!(
        editing(&app).entry,
        Entry::Terrain(0),
        "a terrain stays for a cell run"
    );
    assert!(editing(&app).cell_run());
    click(&mut app, (1, 3));
    assert_eq!(
        editing(&app).run.as_ref().map(|r| r.corners.clone()),
        Some(vec![(1, 3)])
    );
    assert_eq!(
        editing(&app).hover.map(|t| t.cell),
        Some((1, 3)),
        "the hover shows the cell"
    );
    point_at(&mut app, (5, 4), None);
    app.update();
    let preview = editing(&app).preview().expect("a preview");
    assert_eq!(preview.cells, run::cells((1, 3), (5, 4)));
    assert_eq!(preview.cells.len(), 5);
    assert!(preview.sides.is_empty() && preview.corner.is_none());
    click(&mut app, (5, 4));
    assert_eq!(editing(&app).run.as_ref().map(|r| r.corners.len()), Some(2));
    tap(&mut app, KeyCode::Enter);
    assert!(editing(&app).run.is_none());
    assert_eq!(depth(&app), steps + 1);
    assert_eq!(
        message(&app),
        format!("5 cells painted {}", Entry::Terrain(0).label(&def))
    );
    let after = grid(&app);
    for cell in run::cells((1, 3), (5, 4)) {
        assert_eq!(after.tile(cell.0, cell.1).flatten(), Some(0), "{cell:?}");
    }
    // Freehand with the height: the cells passed are set to it, one step.
    press_edit(&mut app, EditAct::Brush(at(Entry::Height)));
    assert_eq!(
        editing(&app).tool,
        Tool::Corners,
        "the height is a cell run's too"
    );
    press_edit(&mut app, EditAct::Tool(Tool::Freehand));
    arrow(&mut app, EditAct::Eighths, true);
    arrow(&mut app, EditAct::Eighths, true);
    assert_eq!(editing(&app).eighths, 2);
    drag(&mut app, MouseButton::Left, (6, 3), &[(7, 3), (8, 3)]);
    assert_eq!(depth(&app), steps + 2, "{}", message(&app));
    assert_eq!(message(&app), "3 cells set to height 2");
    let after = grid(&app);
    for x in 6..=8 {
        assert_eq!(after.height_at(x, 3), Some(2));
    }
    assert_eq!(after.height_at(9, 3), Some(0));
    // A stamp has no run: a run tool with one walls, as before.
    press_edit(&mut app, EditAct::Brush(at(Entry::Stamp(Placing::Lore))));
    press_edit(&mut app, EditAct::Tool(Tool::Corners));
    assert_eq!(editing(&app).entry, Entry::Edge(EdgeKind::Wall));
    assert!(!editing(&app).cell_run());
}
