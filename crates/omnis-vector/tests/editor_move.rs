//! The move tool (the editor's column, step 2, 2026-10-25): a press on a cell carries what the
//! inspector lists there first, the start or a placement, to the cell let go on (a placement as a
//! removal and a placement in one undo step, refused where one of its kind already is); a press elsewhere pans the plan by whole cells, as
//! a drag with the middle button does under any tool. Headless on the town: the portal on
//! (11, 2), the sites on the shops' door cells.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::tap;
use common::editor::{editing, in_editor, leave_plan, mouse, point_at, press_edit};
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::Tool;

fn depth(app: &App) -> usize {
    editing(app).status.undo
}

fn portal_at(app: &App, cell: (u16, u16)) -> bool {
    editing(app)
        .def()
        .expect("open")
        .portals
        .iter()
        .any(|p| (p.x, p.y) == cell)
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

/// The cell of the town's site of `service`: the owner moves the town's sites.
fn cell_of(app: &App, service: &str) -> (u16, u16) {
    editing(app)
        .def()
        .expect("open")
        .sites
        .iter()
        .find(|s| s.service == service)
        .map(|s| (s.x, s.y))
        .unwrap_or_else(|| panic!("the {service} site"))
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

#[test]
fn a_move_carries_the_placement_under_the_press_to_the_cell_let_go_on_as_one_step() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    press_edit(&mut app, EditAct::Tool(Tool::Move));
    assert!(portal_at(&app, (11, 2)));
    // The press takes the portal and puts the cursor on its cell; the drag's cursor follows.
    point_at(&mut app, (11, 2), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    let grab = editing(&app).grab.clone().expect("a grab");
    assert_eq!(grab.from, Some((11, 2)));
    assert!(
        grab.held
            .as_ref()
            .is_some_and(|h| h.label().starts_with("Portal"))
    );
    assert_eq!(editing(&app).cursor, (11, 2));
    point_at(&mut app, (9, 2), None);
    app.update();
    assert_eq!(editing(&app).cursor, (9, 2), "the cursor follows the drag");
    assert_eq!(depth(&app), steps, "nothing moves until let go");
    point_at(&mut app, (8, 2), None);
    app.update();
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert!(editing(&app).grab.is_none());
    assert!(portal_at(&app, (8, 2)) && !portal_at(&app, (11, 2)));
    assert_eq!(
        depth(&app),
        steps + 1,
        "a removal and a placement, one step"
    );
    assert_eq!(editing(&app).cursor, (8, 2));
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("Portal 'town-gate' to 'meadow-gate' moved to (8, 2)")
    );
    // The inspector lists it on the new cell; the note stays the old cell's.
    assert!(
        editing(&app)
            .here()
            .iter()
            .any(|h| h.label.starts_with("Portal"))
    );
    assert!(
        editing(&app)
            .session
            .notes()
            .get("test:map:town/0/11,2")
            .is_some(),
        "a note is the cell's, not the portal's"
    );
    press_edit(&mut app, EditAct::Undo);
    assert!(portal_at(&app, (11, 2)) && !portal_at(&app, (8, 2)));
    assert_eq!(depth(&app), steps);
    // Where one of its kind already is, the move is refused and said.
    let inn_at = cell_of(&app, "base:service:inn");
    let temple_at = cell_of(&app, "base:service:temple");
    let inn = site_at(&app, inn_at).expect("the inn's site");
    let temple = site_at(&app, temple_at).expect("the temple's site");
    drag(&mut app, MouseButton::Left, inn_at, &[temple_at]);
    assert_eq!(
        editing(&app).message.as_deref(),
        Some(format!("a site is already at ({}, {})", temple_at.0, temple_at.1).as_str())
    );
    assert_eq!(site_at(&app, inn_at), Some(inn.clone()));
    assert_eq!(site_at(&app, temple_at), Some(temple));
    assert_eq!(depth(&app), steps);
    // Let go on its own cell, or off the plan: nothing moves.
    drag(
        &mut app,
        MouseButton::Left,
        inn_at,
        &[(inn_at.0 + 1, inn_at.1), inn_at],
    );
    assert_eq!(site_at(&app, inn_at), Some(inn.clone()));
    assert_eq!(depth(&app), steps);
    point_at(&mut app, inn_at, None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    leave_plan(&mut app);
    app.update();
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert!(editing(&app).grab.is_none());
    assert_eq!(site_at(&app, inn_at), Some(inn));
    assert_eq!(depth(&app), steps);
    // A site moved onto a bare cell (the street) lands there.
    let trainer_at = cell_of(&app, "base:service:trainer");
    drag(&mut app, MouseButton::Left, trainer_at, &[(5, 2)]);
    assert!(site_at(&app, trainer_at).is_none() && site_at(&app, (5, 2)).is_some());
    assert_eq!(depth(&app), steps + 1);
}

#[test]
fn a_press_on_a_bare_cell_pans_the_plan_by_whole_cells_as_the_middle_button_does_anywhere() {
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    let origin = editing(&app).view.origin;
    tap(&mut app, KeyCode::KeyM);
    assert_eq!(editing(&app).tool, Tool::Move);
    // A drag two cells right with the Move tool looks two cells further left.
    point_at(&mut app, (5, 2), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    assert!(
        editing(&app)
            .grab
            .as_ref()
            .is_some_and(|g| g.held.is_none())
    );
    point_at(&mut app, (7, 2), None);
    app.update();
    assert_eq!(editing(&app).view.origin, (origin.0 - 2, origin.1));
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert!(editing(&app).grab.is_none());
    assert_eq!(editing(&app).view.origin, (origin.0 - 2, origin.1));
    assert_eq!(depth(&app), steps, "a pan is no step");
    press_edit(&mut app, EditAct::Centre);
    let centred = editing(&app).view.origin;
    // The middle button pans under the brush, and paints nothing.
    press_edit(&mut app, EditAct::Tool(Tool::Brush));
    press_edit(&mut app, EditAct::Brush(1));
    point_at(&mut app, (5, 2), None);
    mouse(&mut app, MouseButton::Middle, ButtonState::Pressed);
    point_at(&mut app, (4, 3), None);
    app.update();
    assert_eq!(editing(&app).view.origin, (centred.0 + 1, centred.1 - 1));
    mouse(&mut app, MouseButton::Middle, ButtonState::Released);
    assert!(editing(&app).grab.is_none());
    assert_eq!(depth(&app), steps);
    assert_eq!(editing(&app).tool, Tool::Brush);
    // An act while dragging ends the drag where it is.
    point_at(&mut app, (5, 2), None);
    mouse(&mut app, MouseButton::Middle, ButtonState::Pressed);
    press_edit(&mut app, EditAct::Centre);
    assert!(editing(&app).grab.is_none());
    mouse(&mut app, MouseButton::Middle, ButtonState::Released);
}

#[test]
fn a_move_carries_the_start_marker_as_it_carries_a_placement() {
    // Owner, 2026-10-25, hand-testing step 2: "I couldn't move the start marker". The town starts
    // on (10, 2) facing west, a bare street cell: a press there panned the plan instead.
    let mut app = in_editor("test:map:town");
    let steps = depth(&app);
    let start = |app: &App| editing(app).def().expect("open").start;
    assert_eq!(start(&app).0, 10);
    assert_eq!(start(&app).1, 2);
    let facing = start(&app).2;
    press_edit(&mut app, EditAct::Tool(Tool::Move));
    let origin = editing(&app).view.origin;
    point_at(&mut app, (10, 2), None);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    assert!(
        editing(&app)
            .grab
            .as_ref()
            .is_some_and(|g| g.held.as_ref().is_some_and(|h| h.label() == "Start")),
        "the press takes the start, not the plan"
    );
    point_at(&mut app, (8, 2), None);
    app.update();
    assert_eq!(editing(&app).view.origin, origin, "the plan does not pan");
    assert_eq!(editing(&app).cursor, (8, 2), "the cursor follows");
    point_at(&mut app, (5, 2), None);
    app.update();
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(start(&app), (5, 2, facing), "moved, facing as before");
    assert_eq!(depth(&app), steps + 1, "one step");
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("Start moved to (5, 2)")
    );
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(start(&app), (10, 2, facing));
    assert_eq!(depth(&app), steps);
    // Let go on its own cell: nothing.
    drag(&mut app, MouseButton::Left, (10, 2), &[(9, 2), (10, 2)]);
    assert_eq!(start(&app), (10, 2, facing));
    assert_eq!(depth(&app), steps);
}
