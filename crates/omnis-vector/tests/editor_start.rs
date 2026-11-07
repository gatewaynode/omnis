//! The start as a real entrance (W1 step 3, 2026-11-07): the map's start is on a level. Set
//! start here puts it on the shown level, the plan marks it on that level alone with a diamond
//! notched toward its facing, the inspector names the level, and a map opens on its start's
//! level. Played on the `levels` fixture's tower; what saves works on a scratch copy of it.

use crate::common;
use bevy::prelude::*;
use common::app::{config, shell};
use common::editor::{arrow, editing, in_editor_on, press_edit, scratch_of};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::Data;
use omnis_vector::editor::plan::{self, Source};
use omnis_vector::editor::{EditAct, view};
use omnis_vector::minimap::Raster;
use omnis_vector::screen::ControlId;
use omnis_vector::shell::editor::Editing;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;
use std::path::PathBuf;

const LEVELS: &str = "crates/omnis-sim/tests/packs/levels";
const TOWER: &str = "levels:map:tower";
const UPPER: Level = Level(1);

fn data(app: &App) -> std::sync::Arc<Data> {
    app.world().resource::<Packs>().data.clone()
}

/// The shell in the editor on a scratch copy of the levels fixture, the tower open.
fn tower_scratch() -> App {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scratch = scratch_of(&root.join(LEVELS), "start-tower");
    let mut config = config();
    config.packs.push(scratch);
    config.editor = Some(Some(TOWER.to_owned()));
    let mut app = shell(config, false);
    app.update();
    app
}

/// The plan painted as the layer paints it: the start when it is on the shown level.
fn painted(app: &App) -> Raster {
    let e = editing(app);
    let def = e.def().expect("a map");
    let data = data(app);
    let marks = plan::marks(&def, &data, e.level);
    let start = (e.level == def.start_level).then_some(def.start);
    plan::paint(
        Source {
            grid: e.grid().expect("a grid"),
            terrains: &def.terrains,
            size: (def.width, def.height),
            level: e.level,
            resolved: e.resolved.as_ref(),
            start,
            marks: &marks,
            preview: None,
            selection: None,
        },
        e.view,
        e.pane,
    )
}

/// How many pixels of the start's colour lie in the plan's `cell`.
fn start_pixels_in(app: &App, raster: &Raster, cell: (u16, u16)) -> usize {
    let (left, top, side) = editing(app).view.cell_rect(cell);
    let mut n = 0;
    for y in top..top + i64::from(side) {
        for x in left..left + i64::from(side) {
            if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y))
                && raster.pixel(x, y) == Some(plan::START)
            {
                n += 1;
            }
        }
    }
    n
}

/// Whether the pixel at the cell's side toward `facing`, where the notch ends, is the start's.
fn notched(app: &App, raster: &Raster, cell: (u16, u16), facing: Facing) -> bool {
    let (left, top, side) = editing(app).view.cell_rect(cell);
    let side = i64::from(side);
    assert!(
        side >= 8,
        "the fitted tower is zoomed in enough for a notch"
    );
    let (dx, dy) = facing.delta();
    let reach = side / 2 - 1;
    let x = left + side / 2 + i64::from(dx) * reach;
    let y = top + side / 2 + i64::from(dy) * reach;
    let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
        return false;
    };
    raster.pixel(x, y) == Some(plan::START)
}

#[test]
fn the_start_is_set_on_the_shown_level_and_marked_there_alone_with_its_facing() {
    let mut app = in_editor_on(&[LEVELS], TOWER);
    assert_eq!(
        editing(&app).level,
        Level::GROUND,
        "the tower starts on the road"
    );
    let raster = painted(&app);
    assert!(
        start_pixels_in(&app, &raster, (0, 2)) > 0,
        "the diamond on the ground"
    );
    assert!(
        notched(&app, &raster, (0, 2), Facing::East),
        "the notch runs east"
    );
    assert!(
        !notched(&app, &raster, (0, 2), Facing::West),
        "and not west"
    );
    // Up to the upper floor: the ground's start is not drawn there, and the bridge takes it.
    arrow(&mut app, EditAct::Level, true);
    assert_eq!(editing(&app).level, UPPER);
    assert_eq!(
        start_pixels_in(&app, &painted(&app), (0, 2)),
        0,
        "the ground's start is not drawn upstairs"
    );
    {
        let mut e = app.world_mut().resource_mut::<Editing>();
        e.point((4, 2));
        e.facing = Facing::North;
    }
    app.update();
    let inspector = view::inspector(editing(&app), &data(&app));
    let button = inspector
        .control(ControlId::Edit(EditAct::SetStart))
        .expect("the button");
    assert!(
        button.why_not.is_none(),
        "offered off the ground: {:?}",
        button.why_not
    );
    press_edit(&mut app, EditAct::SetStart);
    let def = editing(&app).def().expect("a map");
    assert_eq!(def.start, (4, 2, Facing::North));
    assert_eq!(def.start_level, UPPER);
    assert!(
        editing(&app).status.errors.is_empty(),
        "the bridge stands: {:?}",
        editing(&app).status.errors
    );
    let text = screen_text(app.world());
    assert!(
        text.contains("Start (4, 2) on level Upper floor (1) facing north"),
        "{text}"
    );
    let raster = painted(&app);
    assert!(
        start_pixels_in(&app, &raster, (4, 2)) > 0,
        "the diamond upstairs"
    );
    assert!(notched(&app, &raster, (4, 2), Facing::North));
    assert!(!notched(&app, &raster, (4, 2), Facing::South));
    // Back on the ground no start is drawn: it lives upstairs now.
    arrow(&mut app, EditAct::Level, false);
    assert_eq!(editing(&app).level, Level::GROUND);
    let raster = painted(&app);
    assert_eq!(
        start_pixels_in(&app, &raster, (0, 2)) + start_pixels_in(&app, &raster, (4, 2)),
        0
    );
    // The button on the ground where the start was: offered, since the start is elsewhere.
    app.world_mut().resource_mut::<Editing>().point((0, 2));
    app.update();
    let inspector = view::inspector(editing(&app), &data(&app));
    assert!(
        inspector
            .control(ControlId::Edit(EditAct::SetStart))
            .is_some_and(|c| c.why_not.is_none())
    );
}

#[test]
fn a_saved_start_on_an_upper_floor_opens_the_map_on_that_floor() {
    let mut app = tower_scratch();
    arrow(&mut app, EditAct::Level, true);
    app.world_mut().resource_mut::<Editing>().point((4, 2));
    app.update();
    press_edit(&mut app, EditAct::SetStart);
    press_edit(&mut app, EditAct::Save);
    assert!(
        !editing(&app).status.dirty,
        "saved: {:?}",
        editing(&app).message
    );
    // Down to the ground, then the tower opened anew: on its start's level, the cursor on it.
    arrow(&mut app, EditAct::Level, false);
    assert_eq!(editing(&app).level, Level::GROUND);
    let data = data(&app);
    app.world_mut()
        .resource_mut::<Editing>()
        .0
        .open(TOWER, &data)
        .unwrap_or_else(|e| panic!("{e}"));
    app.update();
    assert_eq!(editing(&app).level, UPPER, "opens on the start's level");
    assert_eq!(editing(&app).cursor, (4, 2));
    let def = editing(&app).def().expect("a map");
    assert_eq!((def.start.0, def.start.1, def.start_level), (4, 2, UPPER));
}
