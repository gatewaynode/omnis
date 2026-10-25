//! The editor's pointer through Bevy's own UI (M5.5, after the owner's report that the mouse did
//! nothing in the window): layout, the UI stack and the focus system fill the plan's
//! `RelativeCursorPosition` from a real `Window` cursor, and a click lands on the cell under it.
//! The other editor tests write that component themselves; this one checks the convention they
//! write is the one Bevy fills (`normalized` runs from -0.5 to 0.5, not 0 to 1).

use super::common::app::ui_editor_app;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use omnis_vector::editor::EditAct;
use omnis_vector::shell::editor::{Column, Editing};
use omnis_vector::shell::session::Packs;

/// The window's logical cursor position over `cell`'s middle, from the plan's laid-out
/// rectangle and the model's view.
fn over(app: &mut App, cell: (u16, u16), dx: f32, dy: f32) -> Vec2 {
    let world = app.world_mut();
    let mut panes = world.query::<(&Column, &ComputedNode, &UiGlobalTransform)>();
    let (size, inverse, centre) = panes
        .iter(world)
        .find(|(c, _, _)| **c == Column::Plan)
        .map(|(_, node, tf)| (node.size(), node.inverse_scale_factor(), tf.translation))
        .expect("the plan is laid out");
    assert!(
        size.x > 0.0 && size.y > 0.0,
        "the plan has a size: {size:?}"
    );
    let top_left = centre - size / 2.0;
    let editing = world.resource::<Editing>();
    let (left, top, side) = editing.view.cell_rect(cell);
    #[allow(clippy::cast_precision_loss)]
    let in_pane = Vec2::new(
        left as f32 + side as f32 * dx,
        top as f32 + side as f32 * dy,
    );
    // Physical pixels, then the window's logical ones.
    (top_left + in_pane / inverse) * inverse
}

fn click(app: &mut App, window: Entity, button: MouseButton, at: Vec2) {
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("the window")
        .set_cursor_position(Some(at));
    app.update();
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button,
            state,
            window,
        });
        app.update();
    }
}

#[test]
fn a_click_in_the_window_lands_on_the_cell_under_the_pointer() {
    let (mut app, window) = ui_editor_app();
    let pane = app.world().resource::<Editing>().pane;
    assert!(
        pane.0 > 100 && pane.1 > 100,
        "the model took the laid-out pane: {pane:?}"
    );
    assert_eq!(
        app.world().resource::<Editing>().cursor,
        (10, 2),
        "the start"
    );
    let middle = over(&mut app, (5, 3), 0.5, 0.5);
    click(&mut app, window, MouseButton::Left, middle);
    let editing = app.world().resource::<Editing>();
    assert_eq!(editing.cursor, (5, 3), "{:?}", editing.message);
    // A right click picks: the brush becomes what is there and the cursor moves.
    let elsewhere = over(&mut app, (7, 1), 0.5, 0.5);
    click(&mut app, window, MouseButton::Right, elsewhere);
    assert_eq!(app.world().resource::<Editing>().cursor, (7, 1));
    // Just inside the eastern border of a cell, a Wall paints that edge.
    let data = app.world().resource::<Packs>().data.clone();
    let brushes = {
        let def = app.world().resource::<Editing>().def().expect("open");
        omnis_vector::editor::brush::palette(&def)
    };
    let wall = brushes
        .iter()
        .position(|t| {
            *t == omnis_vector::editor::brush::Entry::Edge(
                omnis_sim::omnis_data::edit::EdgeKind::Wall,
            )
        })
        .expect("a Wall brush");
    app.world_mut().resource_mut::<Editing>().apply(
        &data,
        EditAct::Brush(u8::try_from(wall).expect("small")),
        0,
    );
    let east = over(&mut app, (6, 3), 0.95, 0.5);
    click(&mut app, window, MouseButton::Left, east);
    let editing = app.world().resource::<Editing>();
    assert_eq!(editing.cursor, (6, 3), "{:?}", editing.message);
    let grid = editing.grid().expect("the grid");
    assert_eq!(
        grid.edge(6, 3, omnis_sim::omnis_core::Facing::East),
        Some(omnis_sim::omnis_data::edit::EdgeKind::Wall)
    );
}

#[test]
fn a_click_in_the_window_near_a_corner_starts_a_wall_run_there() {
    let (mut app, window) = ui_editor_app();
    let data = app.world().resource::<Packs>().data.clone();
    app.world_mut().resource_mut::<Editing>().apply(
        &data,
        EditAct::Tool(omnis_vector::editor::brush::Tool::Corners),
        0,
    );
    // Near the bottom right of cell (2, 1): the corner (3, 2) is the nearest.
    let near = over(&mut app, (2, 1), 0.95, 0.95);
    click(&mut app, window, MouseButton::Left, near);
    let editing = app.world().resource::<Editing>();
    assert_eq!(
        editing.run.as_ref().map(|r| r.corners.clone()),
        Some(vec![(3, 2)]),
        "{:?}",
        editing.message
    );
    assert_eq!(editing.corner, Some((3, 2)));
    assert_eq!(editing.hover, None, "a run tool shows no side hover");
    // A second corner, then a click on it takes the run: the sides between become walls.
    let next = over(&mut app, (5, 1), 0.95, 0.95);
    click(&mut app, window, MouseButton::Left, next);
    click(&mut app, window, MouseButton::Left, next);
    let editing = app.world().resource::<Editing>();
    assert_eq!(editing.run, None);
    let grid = editing.grid().expect("the grid");
    for x in 3..6 {
        assert_eq!(
            grid.edge(x, 2, omnis_sim::omnis_core::Facing::North),
            Some(omnis_sim::omnis_data::edit::EdgeKind::Wall),
            "x {x}: {:?}",
            editing.message
        );
    }
}

/// A drag in the window with `button`: the cursor set over `from`, pressed, moved over each of
/// `through` a frame apart, let go on the last.
fn drag_in_window(
    app: &mut App,
    window: Entity,
    button: MouseButton,
    from: Vec2,
    through: &[Vec2],
) {
    let set = |app: &mut App, at: Vec2| {
        app.world_mut()
            .get_mut::<Window>(window)
            .expect("the window")
            .set_cursor_position(Some(at));
        app.update();
    };
    set(app, from);
    app.world_mut().write_message(MouseButtonInput {
        button,
        state: ButtonState::Pressed,
        window,
    });
    app.update();
    for &at in through {
        set(app, at);
    }
    app.world_mut().write_message(MouseButtonInput {
        button,
        state: ButtonState::Released,
        window,
    });
    app.update();
}

#[test]
fn a_drag_in_the_window_moves_the_start_marker_from_under_the_cursor() {
    // The owner, 2026-10-25, twice: "I couldn't move the start marker" in the window, while the
    // headless move test passed. After Open the cursor stands on the start.
    let (mut app, window) = ui_editor_app();
    let data = app.world().resource::<Packs>().data.clone();
    app.world_mut().resource_mut::<Editing>().apply(
        &data,
        EditAct::Tool(omnis_vector::editor::brush::Tool::Move),
        0,
    );
    assert_eq!(
        app.world().resource::<Editing>().cursor,
        (10, 2),
        "the start"
    );
    let from = over(&mut app, (10, 2), 0.5, 0.5);
    let mid = over(&mut app, (8, 2), 0.5, 0.5);
    let to = over(&mut app, (5, 2), 0.5, 0.5);
    let origin = app.world().resource::<Editing>().view.origin;
    drag_in_window(&mut app, window, MouseButton::Left, from, &[mid, to]);
    let editing = app.world().resource::<Editing>();
    let start = editing.def().expect("open").start;
    assert_eq!(
        (start.0, start.1),
        (5, 2),
        "message {:?}, origin {:?} was {origin:?}, grab {:?}",
        editing.message,
        editing.view.origin,
        editing.grab
    );
}
