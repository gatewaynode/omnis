//! The editor's attribute bar and forms (M5.6, `tasks/plans/m5-editor-v1.md` row 6): the bar
//! over the plan edits the cursor's cell where it stands (terrain, height, a stair's rise), a
//! column whose words change keeps its nodes, and no column takes the keyboard without a field.

use crate::common;

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use common::editor::{arrow, editing, in_editor, in_editor_on, panel, press_edit};
use omnis_sim::omnis_core::Facing;
use omnis_vector::editor::EditAct;
use omnis_vector::shell::editor::{Column, Editing};
use omnis_vector::shell::screen_text::screen_text;

#[test]
fn the_bar_steps_the_cursor_cell_s_terrain_and_height_one_undo_step_each() {
    let mut app = in_editor("test:map:town");
    let start = editing(&app).cursor;
    let tile = editing(&app).cell_terrain().expect("the start has ground");
    let terrains = editing(&app).def().expect("a map").terrains.len();
    let text = screen_text(app.world());
    assert!(
        text.contains(&format!("Cell ({}, {})  level 0", start.0, start.1)),
        "{text}"
    );
    assert!(
        text.contains("Terrain") && text.contains("Height"),
        "{text}"
    );
    // The terrain steps through the map's list and round through no ground.
    arrow(&mut app, EditAct::CellTerrain, true);
    let next = usize::from(tile) + 1;
    let expected = (next < terrains).then(|| u8::try_from(next).unwrap());
    assert_eq!(editing(&app).cell_terrain(), expected);
    assert_eq!(editing(&app).cursor, start, "the cursor stays");
    for _ in next..terrains {
        arrow(&mut app, EditAct::CellTerrain, true);
    }
    assert_eq!(
        editing(&app).cell_terrain(),
        None,
        "past the last terrain lies no ground"
    );
    let text = screen_text(app.world());
    assert!(
        text.contains("No ground") && text.contains("no ground"),
        "{text}"
    );
    arrow(&mut app, EditAct::CellTerrain, true);
    assert_eq!(editing(&app).cell_terrain(), Some(0), "and round");
    // The height steps by an eighth either way.
    arrow(&mut app, EditAct::CellHeight, true);
    assert_eq!(editing(&app).cell_height(), 1);
    arrow(&mut app, EditAct::CellHeight, false);
    arrow(&mut app, EditAct::CellHeight, false);
    assert_eq!(editing(&app).cell_height(), -1);
    let text = screen_text(app.world());
    assert!(text.contains("Height -1 eighths"), "{text}");
    // One undo step each: the three heights, then the terrains one by one.
    for _ in 0..3 {
        press_edit(&mut app, EditAct::Undo);
    }
    assert_eq!(editing(&app).cell_height(), 0);
    assert_eq!(editing(&app).cell_terrain(), Some(0));
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(editing(&app).cell_terrain(), None);
}

#[test]
fn rise_is_offered_on_a_stair_cell_only_and_turns_the_way_it_climbs() {
    let mut app = in_editor_on(&["crates/omnis-sim/tests/packs/levels"], "levels:map:tower");
    // The ground level's stair up at (2, 0) rises west.
    app.world_mut().resource_mut::<Editing>().point((2, 0));
    app.update();
    assert_eq!(editing(&app).cell_rise(), Some(Facing::West));
    let text = screen_text(app.world());
    assert!(text.contains("Rise") && text.contains("west"), "{text}");
    arrow(&mut app, EditAct::CellRise, true);
    assert_eq!(editing(&app).cell_rise(), Some(Facing::North));
    let def = editing(&app).def().expect("a map");
    assert!(
        def.stairs
            .iter()
            .any(|s| s.level == 0 && s.x == 2 && s.y == 0 && s.rise == Facing::North),
        "{:?}",
        def.stairs
    );
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(
        editing(&app).cell_rise(),
        Some(Facing::West),
        "one undo step"
    );
    // Off a stair, the bar offers no rise.
    app.world_mut().resource_mut::<Editing>().point((0, 0));
    app.update();
    assert_eq!(editing(&app).cell_rise(), None);
    let text = screen_text(app.world());
    assert!(!text.contains("Rise"), "{text}");
}

#[test]
fn a_column_whose_words_change_keeps_its_nodes_and_no_column_takes_the_keyboard() {
    let mut app = in_editor("test:map:town");
    // The first step makes the map dirty, which wakes Undo: the tools column's shape changes.
    arrow(&mut app, EditAct::CellHeight, true);
    let (bar, _) = panel(&mut app, Column::Bar);
    let (tools, _) = panel(&mut app, Column::Tools);
    let (inspector, _) = panel(&mut app, Column::Inspector);
    // The second changes words only: the bar's height, the inspector's height line.
    arrow(&mut app, EditAct::CellHeight, true);
    assert_eq!(editing(&app).cell_height(), 2);
    assert_eq!(
        panel(&mut app, Column::Bar).0,
        bar,
        "the bar's words changed, not its shape"
    );
    assert_eq!(panel(&mut app, Column::Tools).0, tools);
    assert_eq!(panel(&mut app, Column::Inspector).0, inspector);
    let text = screen_text(app.world());
    assert!(text.contains("Height 2 eighths"), "{text}");
    // Each column's words are its own: the bar's title overwrote no other column's.
    assert!(
        text.contains("EDITOR  ") && text.contains("INSPECTOR") && text.contains("Cell ("),
        "{text}"
    );
    assert!(
        app.world()
            .get_resource::<InputFocus>()
            .is_none_or(|f| f.get().is_none()),
        "no field, so nothing takes the keyboard"
    );
}
