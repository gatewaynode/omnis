//! The boundary tools (the owner, 2026-10-24): wall runs drawn by corners or freehand, snapped
//! to the grid's corners and rasterised onto cell sides, previewed in the caution colour and
//! taken as one undo step. Headless on the town, the pointer written as the pane would report
//! it; the pointer's path through Bevy's own layout is `editor_pointer`'s.

use crate::common;
use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::tap;
use common::editor::{editing, in_editor, leave_plan, mouse, point_frac, press_edit};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::edit::{EdgeKind, Grid};
use omnis_vector::editor::EditAct;
use omnis_vector::editor::brush::Tool;
use omnis_vector::editor::plan::{self, Source, View};
use omnis_vector::editor::run::Preview;
use omnis_vector::editor::view;
use omnis_vector::minimap::PREVIEW;
use omnis_vector::screen::ControlId;
use omnis_vector::shell::editor::PlanHover;
use omnis_vector::shell::session::Packs;

/// Choose `tool` by its icon in the tool grid.
fn choose(app: &mut App, tool: Tool) {
    press_edit(app, EditAct::Tool(tool));
    assert_eq!(editing(app).tool, tool);
}

/// The pointer over the grid corner `corner`: the top left of the cell there.
fn point_corner(app: &mut App, corner: (u16, u16)) {
    point_frac(app, corner, 0.0, 0.0);
    app.update();
}

/// A left click on the corner.
fn click_corner(app: &mut App, corner: (u16, u16)) {
    point_corner(app, corner);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    mouse(app, MouseButton::Left, ButtonState::Released);
}

fn depth(app: &App) -> usize {
    editing(app)
        .session
        .document()
        .map_or(0, |d| d.undo_depth())
}

fn grid(app: &App) -> Grid {
    editing(app).grid().expect("the ground").clone()
}

fn corners(app: &App) -> Vec<(u16, u16)> {
    editing(app)
        .run
        .as_ref()
        .map(|r| r.corners.clone())
        .unwrap_or_default()
}

/// The hover node's display.
fn hover_shown(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut hovers = world.query_filtered::<&Node, With<PlanHover>>();
    hovers.single(world).expect("one hover node").display != Display::None
}

#[test]
fn a_run_by_corners_is_previewed_then_taken_as_one_step_of_walls() {
    let mut app = in_editor("test:map:town");
    choose(&mut app, Tool::Corners);
    let before = grid(&app);
    let depth_before = depth(&app);
    // The north sides of the street's upper shops are open; the west side of (6, 2) is open.
    assert_eq!(before.edge(3, 1, Facing::North), Some(EdgeKind::Open));
    assert_eq!(before.edge(6, 2, Facing::West), Some(EdgeKind::Open));
    // The pointer over a corner: the aim is the corner, no side hover.
    point_corner(&mut app, (2, 1));
    assert_eq!(editing(&app).corner, Some((2, 1)));
    assert_eq!(editing(&app).hover, None);
    assert!(!hover_shown(&mut app), "a run tool shows no side bar");
    let preview = editing(&app).preview().expect("the corner is previewed");
    assert!(preview.sides.is_empty());
    assert_eq!(preview.corner, Some((2, 1)));
    // Three corners: the run is pending, nothing painted, the whole run previewed.
    click_corner(&mut app, (2, 1));
    click_corner(&mut app, (6, 1));
    click_corner(&mut app, (6, 3));
    assert_eq!(corners(&app), vec![(2, 1), (6, 1), (6, 3)]);
    assert_eq!(
        depth(&app),
        depth_before,
        "nothing is painted until the run is taken"
    );
    assert_eq!(grid(&app), before);
    let preview = editing(&app).preview().expect("the run is previewed");
    assert_eq!(preview.sides.len(), 6, "{:?}", preview.sides);
    assert!(preview.sides.contains(&(true, 2, 1)));
    assert!(preview.sides.contains(&(false, 6, 2)));
    // The stretch to the pointer is previewed too, not yet part of the run.
    point_corner(&mut app, (9, 3));
    let preview = editing(&app).preview().expect("the run is previewed");
    assert_eq!(preview.sides.len(), 9);
    assert_eq!(corners(&app).len(), 3);
    // The tools column offers the run's buttons only while it is pending, in ASCII.
    let data = app.world().resource::<Packs>().data.clone();
    let tools = view::tools(editing(&app), &data);
    assert!(tools.control(ControlId::Edit(EditAct::TakeRun)).is_some());
    assert!(
        tools
            .control(ControlId::Edit(EditAct::DropCorner))
            .is_some()
    );
    assert!(tools.control(ControlId::Edit(EditAct::DropRun)).is_some());
    for control in tools.controls() {
        assert!(control.label.is_ascii(), "{}", control.label);
    }
    // Enter takes it: six sides walled, one step.
    tap(&mut app, KeyCode::Enter);
    assert_eq!(corners(&app), Vec::<(u16, u16)>::new());
    assert_eq!(depth(&app), depth_before + 1, "{:?}", editing(&app).message);
    let after = grid(&app);
    for x in 2..6 {
        assert_eq!(
            after.edge(x, 1, Facing::North),
            Some(EdgeKind::Wall),
            "x {x}"
        );
    }
    for y in 1..3 {
        assert_eq!(
            after.edge(6, y, Facing::West),
            Some(EdgeKind::Wall),
            "y {y}"
        );
    }
    assert_eq!(editing(&app).message.as_deref(), Some("6 sides walled"));
    let tools = view::tools(editing(&app), &data);
    assert!(tools.control(ControlId::Edit(EditAct::TakeRun)).is_none());
    // One Undo takes the whole run back.
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(grid(&app), before);
    assert_eq!(depth(&app), depth_before);
}

#[test]
fn a_click_on_the_last_corner_takes_the_run_and_the_buttons_drop_it() {
    let mut app = in_editor("test:map:town");
    choose(&mut app, Tool::Corners);
    let depth_before = depth(&app);
    click_corner(&mut app, (1, 1));
    click_corner(&mut app, (4, 1));
    click_corner(&mut app, (4, 1));
    assert_eq!(
        depth(&app),
        depth_before + 1,
        "the second click on the last corner takes"
    );
    assert_eq!(editing(&app).run, None);
    assert_eq!(editing(&app).message.as_deref(), Some("3 sides walled"));
    // Backspace drops the last corner; Drop corner the button does too; Drop run the rest.
    click_corner(&mut app, (1, 3));
    click_corner(&mut app, (5, 3));
    click_corner(&mut app, (5, 4));
    assert_eq!(corners(&app).len(), 3);
    tap(&mut app, KeyCode::Backspace);
    assert_eq!(corners(&app), vec![(1, 3), (5, 3)]);
    press_edit(&mut app, EditAct::DropCorner);
    assert_eq!(corners(&app), vec![(1, 3)]);
    click_corner(&mut app, (5, 3));
    press_edit(&mut app, EditAct::DropRun);
    assert_eq!(editing(&app).run, None);
    assert_eq!(depth(&app), depth_before + 1, "dropping paints nothing");
    // Dropping the last corner drops the run; a run of one corner takes nothing.
    click_corner(&mut app, (2, 3));
    tap(&mut app, KeyCode::Backspace);
    assert_eq!(editing(&app).run, None);
    click_corner(&mut app, (2, 3));
    press_edit(&mut app, EditAct::TakeRun);
    assert_eq!(editing(&app).run, None);
    assert_eq!(depth(&app), depth_before + 1);
    assert_eq!(editing(&app).message.as_deref(), Some("nothing to wall"));
}

#[test]
fn a_pending_run_is_dropped_with_its_tool_level_or_history_and_a_pick_drops_it() {
    let mut app = in_editor("test:map:town");
    choose(&mut app, Tool::Corners);
    click_corner(&mut app, (1, 1));
    click_corner(&mut app, (3, 1));
    assert_eq!(corners(&app).len(), 2);
    // Zoom keeps it.
    press_edit(&mut app, EditAct::ZoomIn);
    assert_eq!(corners(&app).len(), 2);
    // Another tool drops it.
    choose(&mut app, Tool::Eraser);
    assert_eq!(editing(&app).run, None);
    choose(&mut app, Tool::Corners);
    click_corner(&mut app, (1, 1));
    click_corner(&mut app, (3, 1));
    click_corner(&mut app, (3, 1));
    assert_eq!(editing(&app).run, None, "taken");
    click_corner(&mut app, (1, 3));
    click_corner(&mut app, (3, 3));
    // Undo drops it (the history moved under it).
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(editing(&app).run, None);
    click_corner(&mut app, (1, 1));
    click_corner(&mut app, (3, 1));
    // A right click picks what is under the pointer: the brush changes and the run goes.
    point_frac(&mut app, (3, 2), 0.5, 0.5);
    mouse(&mut app, MouseButton::Right, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Right, ButtonState::Released);
    assert_eq!(editing(&app).run, None);
    assert!(!editing(&app).tool.is_run(), "{:?}", editing(&app).tool);
    assert_eq!(editing(&app).corner, None, "a brush aims at no corner");
}

#[test]
fn a_freehand_drag_snaps_to_the_corners_passed_and_is_taken_on_release_as_one_step() {
    let mut app = in_editor("test:map:town");
    choose(&mut app, Tool::Freehand);
    let before = grid(&app);
    let depth_before = depth(&app);
    point_corner(&mut app, (1, 1));
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    assert_eq!(corners(&app), vec![(1, 1)]);
    // Through the corners, with a wobble inside a cell that snaps to the same corner.
    for (fx, fy) in [(0.4, 0.1), (1.0, 0.0), (1.1, 0.1), (2.0, 0.0), (2.0, 1.0)] {
        point_frac(&mut app, (1, 1), fx, fy);
        app.update();
    }
    assert_eq!(corners(&app), vec![(1, 1), (2, 1), (3, 1), (3, 2)]);
    assert_eq!(
        depth(&app),
        depth_before,
        "a drag paints nothing until let go"
    );
    let preview = editing(&app).preview().expect("the drag is previewed");
    assert_eq!(preview.sides.len(), 3);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(editing(&app).run, None);
    assert_eq!(depth(&app), depth_before + 1, "{:?}", editing(&app).message);
    let after = grid(&app);
    assert_eq!(
        after.edge(1, 1, Facing::North),
        Some(EdgeKind::Wall),
        "{:?}",
        editing(&app).message
    );
    assert_eq!(after.edge(2, 1, Facing::North), Some(EdgeKind::Wall));
    assert_eq!(after.edge(3, 1, Facing::West), Some(EdgeKind::Wall));
    assert_eq!(editing(&app).message.as_deref(), Some("3 sides walled"));
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(grid(&app), before);
    // A drag let go off the plan is taken as drawn.
    point_corner(&mut app, (1, 3));
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    point_corner(&mut app, (2, 3));
    app.update();
    leave_plan(&mut app);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(editing(&app).run, None);
    assert_eq!(depth(&app), depth_before + 1);
    assert_eq!(grid(&app).edge(1, 3, Facing::North), Some(EdgeKind::Wall));
}

#[test]
fn a_run_along_the_boundary_walls_nothing_and_says_so() {
    let mut app = in_editor("test:map:town");
    choose(&mut app, Tool::Freehand);
    let before = grid(&app);
    let depth_before = depth(&app);
    // Just above the map's top: the pointer snaps to the boundary corners.
    point_frac(&mut app, (0, 0), 0.0, -0.3);
    app.update();
    assert_eq!(editing(&app).corner, Some((0, 0)));
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    for x in 1..=5 {
        point_frac(&mut app, (x, 0), 0.0, -0.3);
        app.update();
    }
    assert_eq!(corners(&app).len(), 6);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert_eq!(depth(&app), depth_before, "no step for no change");
    assert_eq!(grid(&app), before);
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("nothing to wall, 5 on the boundary stay walls")
    );
    // Part inside, part on the boundary: the inside walls, the boundary is counted.
    choose(&mut app, Tool::Corners);
    click_corner(&mut app, (0, 3));
    click_corner(&mut app, (0, 1));
    click_corner(&mut app, (2, 1));
    tap(&mut app, KeyCode::Enter);
    assert_eq!(depth(&app), depth_before + 1);
    assert_eq!(
        editing(&app).message.as_deref(),
        Some("2 sides walled, 2 on the boundary stay walls")
    );
    // Further out, the pointer aims at nothing.
    point_frac(&mut app, (0, 0), 0.0, -0.8);
    app.update();
    assert_eq!(editing(&app).corner, None);
}

#[test]
fn the_plan_paints_the_preview_in_the_caution_colour_over_the_walls() {
    let app = in_editor("test:map:town");
    let e = editing(&app);
    let def = e.def().expect("open");
    let grid = e.grid().expect("the ground");
    let view = View {
        origin: (0, 0),
        zoom: 3,
    };
    assert_eq!(view.scale(), 16);
    let preview = Preview {
        sides: vec![(true, 2, 1), (false, 6, 2)],
        corner: Some((2, 1)),
    };
    let source = Source {
        grid,
        terrains: &def.terrains,
        size: (def.width, def.height),
        level: e.level,
        resolved: e.resolved.as_ref(),
        start: None,
        marks: &[],
        preview: Some(&preview),
    };
    let raster = plan::paint(source, view, (400, 200));
    // Along the north side of (2, 1): the grid line at y 16, x 32 to 48.
    assert_eq!(raster.pixel(40, 16), Some(PREVIEW));
    assert_eq!(
        raster.pixel(40, 15),
        Some(PREVIEW),
        "two pixels thick at 16"
    );
    assert_ne!(raster.pixel(40, 20), Some(PREVIEW), "the bar is thin");
    // Along the west side of (6, 2): x 96, y 32 to 48.
    assert_eq!(raster.pixel(96, 40), Some(PREVIEW));
    assert_ne!(raster.pixel(96, 60), Some(PREVIEW));
    // The corner at (32, 16) as a square.
    assert_eq!(raster.pixel(30, 14), Some(PREVIEW));
    assert_eq!(raster.pixel(34, 18), Some(PREVIEW));
    // Without a preview nothing is caution-coloured there.
    let plain = plan::paint(
        Source {
            preview: None,
            ..source
        },
        view,
        (400, 200),
    );
    assert_ne!(plain.pixel(40, 16), Some(PREVIEW));
    assert_ne!(plain.pixel(30, 14), Some(PREVIEW));
}
