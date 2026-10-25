//! The editor's entry and plan view (M5.4, `tasks/plans/m5-editor-v1.md` §Client side): the
//! title offers it, `--editor` opens a map in it, it runs with no game, the cursor and levels
//! move and the inspector follows, the plan paints the map, and the socket answers `editor.*`
//! while it is open.

use crate::common;

use bevy::prelude::*;
use common::app::{app_at_title, config, set, shell, tap};
use common::editor::{arrow, editing, in_editor, press_edit};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_vector::editor::plan::{self, Source, View};
use omnis_vector::editor::{EditAct, Editor};
use omnis_vector::minimap::WALL;
use omnis_vector::screen::nav::{Escape, Nav};
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::editor::{Column, Editing};
use omnis_vector::shell::screen::{Pressable, Screens, Scrolls};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::{Session, parse};
use std::path::PathBuf;

fn press(app: &mut App, id: ControlId) {
    set(app, &Pressable(id), Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut buttons = world.query::<(&Pressable, &mut Interaction)>();
    for (p, mut i) in buttons.iter_mut(world) {
        if p.0 == id {
            *i = Interaction::None;
        }
    }
    app.update();
}

fn top(app: &App) -> Option<ScreenId> {
    let game = app.world().get_resource::<Session>().map(|s| &s.views);
    app.world().resource::<Screens>().nav.top(game)
}

#[test]
fn the_title_offers_the_editor_and_it_opens_with_no_game() {
    let mut app = app_at_title(".omnis/test-saves/editor-none.ron");
    app.update();
    let text = screen_text(app.world());
    assert!(text.contains("[button] \"Editor\""), "{text}");
    assert!(app.world().get_resource::<Editing>().is_none());

    press(&mut app, ControlId::Editor);
    assert!(app.world().get_resource::<Session>().is_none(), "no game");
    let e = editing(&app);
    assert_eq!(e.session.pack(), "test", "the last pack loaded is edited");
    assert_eq!(e.maps.len(), 4, "the test pack's maps: {:?}", e.maps);
    assert!(e.map_id().is_none(), "nothing open yet");
    assert_eq!(top(&app), None, "the layer is the base: no screen over it");
    let text = screen_text(app.world());
    assert!(text.contains("EDITOR"), "{text}");
    assert!(text.contains("No map is open."), "{text}");
    assert!(
        !text.contains("[button] \"New game\""),
        "the title is gone: {text}"
    );
}

#[test]
fn esc_opens_the_editor_menu_and_back_to_title_closes_the_editor() {
    let mut app = app_at_title(".omnis/test-saves/editor-none.ron");
    app.update();
    press(&mut app, ControlId::Editor);
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::EditorMenu));
    let text = screen_text(app.world());
    assert!(text.contains("[button] \"Back to title\""), "{text}");
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None, "Esc closes it again");

    // The Menu button opens it too (every action has a mouse entry point).
    press_edit(&mut app, EditAct::Menu);
    assert_eq!(top(&app), Some(ScreenId::EditorMenu));
    press(&mut app, ControlId::QuitToTitle);
    app.update();
    assert!(app.world().get_resource::<Editing>().is_none(), "closed");
    assert_eq!(top(&app), Some(ScreenId::Title));
    let text = screen_text(app.world());
    assert!(!text.contains("EDITOR"), "{text}");
}

#[test]
fn the_navigation_treats_the_editor_as_the_base_with_no_game() {
    let mut nav = Nav::default();
    assert_eq!(nav.base(None), Some(ScreenId::Title));
    assert!(!nav.allowed(ScreenId::EditorMenu, None));
    assert!(!nav.allowed(ScreenId::EditorFiles, None));
    nav.edit(true);
    assert!(nav.editing());
    assert_eq!(nav.base(None), None);
    assert!(nav.allowed(ScreenId::EditorMenu, None));
    assert!(nav.allowed(ScreenId::EditorFiles, None), "the picker too");
    nav.open(ScreenId::EditorFiles, None);
    assert!(nav.paused(None), "the editor waits under the picker");
    assert_eq!(nav.escape(None, true), Escape::Closed);
    assert!(nav.allowed(ScreenId::Settings, None));
    assert!(
        !nav.allowed(ScreenId::NewGame, None),
        "no game from the editor"
    );
    assert_eq!(nav.escape(None, true), Escape::Paused);
    assert_eq!(nav.top(None), Some(ScreenId::EditorMenu));
    assert!(nav.paused(None));
    assert_eq!(nav.escape(None, true), Escape::Closed);
    nav.reset(None);
    assert!(nav.editing(), "a reset keeps the editor");
    nav.edit(false);
    assert_eq!(nav.base(None), Some(ScreenId::Title));
    assert_eq!(nav.escape(None, true), Escape::Nothing);
}

#[test]
fn the_command_line_opens_a_map_in_the_editor() {
    let args = |list: &[&str]| parse(list.iter().map(|s| (*s).to_owned()));
    assert_eq!(
        args(&["--editor", "test:map:dungeon"]).unwrap().editor,
        Some(Some("test:map:dungeon".to_owned()))
    );
    let bare = args(&["--editor", "--windowed"]).unwrap();
    assert_eq!(bare.editor, Some(None), "no map named");
    assert!(bare.windowed, "the next flag is read as a flag");
    assert_eq!(args(&["--editor"]).unwrap().editor, Some(None));
    assert!(
        args(&["--editor", "x", "--seed", "3"]).is_err(),
        "not with a game"
    );
    assert!(args(&["--windowed"]).unwrap().editor.is_none());

    let app = in_editor("test:map:town");
    let e = editing(&app);
    assert_eq!(e.map_id().as_deref(), Some("test:map:town"));
    assert_eq!(e.cursor, (10, 2), "the cursor starts at the map's start");
    assert_eq!(e.facing, Facing::West);
    assert_eq!(e.level, Level::GROUND);
    assert!(e.resolved.is_some(), "the loader takes the town");
    let text = screen_text(app.world());
    assert!(text.contains("test:map:town  12 x 5"), "{text}");
    assert!(text.contains("Cell (10, 2) facing west"), "{text}");
    assert!(text.contains("Start (10, 2) facing west"), "{text}");
    assert!(text.contains("The loader takes this map."), "{text}");
}

#[test]
fn the_arrow_keys_move_the_cursor_and_the_inspector_follows() {
    let mut app = in_editor("test:map:town");
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(editing(&app).cursor, (9, 2));
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(editing(&app).cursor, (9, 3));
    let text = screen_text(app.world());
    assert!(text.contains("Cell (9, 3) facing west"), "{text}");
    assert!(text.contains("Terrain "), "{text}");
    assert!(text.contains("Height 0 eighths"), "{text}");
    // Never off the map.
    for _ in 0..4 {
        tap(&mut app, KeyCode::ArrowDown);
    }
    assert_eq!(editing(&app).cursor, (9, 4), "the last row");
    tap(&mut app, KeyCode::KeyQ);
    assert_eq!(
        editing(&app).facing,
        Facing::South,
        "Q turns the facing left"
    );
    tap(&mut app, KeyCode::KeyE);
    assert_eq!(editing(&app).facing, Facing::West);
    // The keys are the editor's only while no screen stands over it.
    tap(&mut app, KeyCode::Escape);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(editing(&app).cursor, (9, 4), "the menu holds the keys");
}

/// The scrolling panel of the tools column, and how far it is scrolled.
fn tools_panel(app: &mut App) -> (Entity, f32) {
    let world = app.world_mut();
    let mut panels = world.query_filtered::<(Entity, &ScrollPosition), With<Scrolls>>();
    let found: Vec<(Entity, f32)> = panels.iter(world).map(|(e, at)| (e, at.0.y)).collect();
    for (panel, y) in found {
        let mut up = panel;
        while let Some(parent) = world.get::<ChildOf>(up) {
            up = parent.0;
            if world.get::<Column>(up) == Some(&Column::Tools) {
                return (panel, y);
            }
        }
    }
    panic!("the tools column has a scrolling panel")
}

#[test]
fn a_rebuilt_tools_column_keeps_its_scroll() {
    let mut app = in_editor("test:map:town");
    // Scrolled down to the brushes, as the wheel would.
    let (panel, _) = tools_panel(&mut app);
    app.world_mut()
        .get_mut::<ScrollPosition>(panel)
        .expect("the panel")
        .0
        .y = 120.0;
    app.update();
    // Stepping the height changes the column's words only (M5.6): its nodes stand, scrolled.
    arrow(&mut app, EditAct::Eighths, true);
    assert_eq!(editing(&app).eighths, 1);
    assert_eq!(tools_panel(&mut app), (panel, 120.0), "the nodes stand");
    // A paint wakes Undo, which changes the column's shape: it is rebuilt where it stood.
    arrow(&mut app, EditAct::CellHeight, true);
    let (rebuilt, y) = tools_panel(&mut app);
    assert_ne!(rebuilt, panel, "the column was rebuilt");
    assert_eq!(y, 120.0, "where it stood");
    // By key too, on the rebuilt column.
    tap(&mut app, KeyCode::Equal);
    app.update();
    assert_eq!(editing(&app).eighths, 2);
    assert_eq!(tools_panel(&mut app).1, 120.0);
}

#[test]
fn wasd_moves_the_cursor_like_the_arrows() {
    let mut app = in_editor("test:map:town");
    tap(&mut app, KeyCode::KeyA);
    assert_eq!(editing(&app).cursor, (9, 2), "A is west");
    tap(&mut app, KeyCode::KeyS);
    assert_eq!(editing(&app).cursor, (9, 3), "S is south");
    tap(&mut app, KeyCode::KeyD);
    assert_eq!(editing(&app).cursor, (10, 3), "D is east");
    tap(&mut app, KeyCode::KeyW);
    assert_eq!(editing(&app).cursor, (10, 2), "W is north");
    let text = screen_text(app.world());
    assert!(text.contains("Cell (10, 2) facing west"), "{text}");
}

#[test]
fn the_tools_choose_and_open_a_map_and_the_level_choice_steps_through_levels() {
    let mut app = app_at_title(".omnis/test-saves/editor-none.ron");
    app.update();
    press(&mut app, ControlId::Editor);
    let first = editing(&app).chosen;
    arrow(&mut app, EditAct::Map, true);
    assert_eq!(editing(&app).chosen, first + 1);
    let chosen = editing(&app).maps[first + 1].id.clone();
    press_edit(&mut app, EditAct::Open);
    assert_eq!(editing(&app).map_id(), Some(chosen));

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../omnis-sim/tests/packs/levels");
    let mut config = config();
    config.packs.push(fixture);
    config.editor = Some(Some("levels:map:tower".to_owned()));
    let mut app = shell(config, false);
    app.update();
    assert_eq!(
        editing(&app).levels(),
        vec![Level(-1), Level(0), Level(1), Level(2)]
    );
    arrow(&mut app, EditAct::Level, true);
    assert_eq!(editing(&app).level, Level(1));
    let text = screen_text(app.world());
    assert!(text.contains("Level Upper floor (1)"), "{text}");
    arrow(&mut app, EditAct::Level, false);
    arrow(&mut app, EditAct::Level, false);
    assert_eq!(editing(&app).level, Level(-1), "round the other way");
    arrow(&mut app, EditAct::Level, false);
    assert_eq!(editing(&app).level, Level(2), "and round again");
}

#[test]
fn the_plan_paints_the_town_with_its_walls_and_its_start() {
    let roots = config().packs;
    let data =
        omnis_sim::omnis_data::load_packs(&roots.iter().map(PathBuf::as_path).collect::<Vec<_>>())
            .expect("the packs");
    let mut editor = Editor::new(roots).expect("the editor");
    editor.open("test:map:town", &data).expect("the town");
    let def = editor.def().expect("open");
    // 900 wide: the town's twelve cells at 64 fill 768 and a cell's margin is left either side.
    let pane = (900, 400);
    let view = View::fitted((def.width, def.height), pane);
    assert_eq!(
        view.scale(),
        64,
        "the largest of the zooms the town fits in 900 x 400"
    );
    assert_eq!(view.origin, (-1, 0));
    let raster = plan::paint(
        Source {
            grid: editor.grid().expect("the ground"),
            terrains: &def.terrains,
            size: (def.width, def.height),
            level: Level::GROUND,
            resolved: editor.resolved.as_ref(),
            start: Some((def.start.0, def.start.1)),
            marks: &[],
        },
        view,
        pane,
    );
    assert_eq!((raster.width, raster.height), pane, "sized to the pane");
    // The map's north boundary is a wall: the top pixel row of cell (0, 0).
    let (left, top, side) = view.cell_rect((0, 0));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let at = |dx: i64, dy: i64| raster.pixel((left + dx) as u32, (top + dy) as u32).unwrap();
    assert_eq!(at(side as i64 / 2, 0), WALL, "the boundary wall");
    let grid = editor.grid().unwrap();
    let terrain = &def.terrains[usize::from(grid.tile(0, 0).unwrap().unwrap())];
    assert_eq!(
        at(side as i64 / 2, side as i64 / 2),
        plan::fill(terrain.color, grid.height_at(0, 0).unwrap()),
        "the cell's fill"
    );
    let (sl, st, ss) = view.cell_rect((def.start.0, def.start.1));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let start_mark = raster
        .pixel(
            (sl + ss as i64 / 2) as u32,
            (st + ss as i64 / 2 - ss as i64 / 4) as u32,
        )
        .unwrap();
    assert_eq!(start_mark, plan::START, "the start's diamond");
    assert_eq!(
        view.hit((def.width, def.height), 0.0, 0.0),
        None,
        "the margin"
    );
    assert_eq!(
        view.hit((def.width, def.height), (left + 1) as f32, (top + 1) as f32),
        Some((0, 0))
    );
}

#[test]
fn clicking_the_plan_and_zooming_are_the_model_s_point_and_view() {
    let mut app = in_editor("test:map:dungeon");
    let before = editing(&app).revision;
    app.world_mut().resource_mut::<Editing>().point((3, 3));
    app.update();
    let e = editing(&app);
    assert_eq!(e.cursor, (3, 3));
    assert!(e.revision > before, "a redraw follows");
    let scale = e.view.scale();
    tap(&mut app, KeyCode::BracketRight);
    assert!(
        editing(&app).view.scale() >= scale,
        "] zooms in or stays at the largest"
    );
    tap(&mut app, KeyCode::BracketLeft);
    tap(&mut app, KeyCode::BracketLeft);
    assert!(editing(&app).view.scale() < scale, "[ zooms out");
    press_edit(&mut app, EditAct::Centre);
    let e = editing(&app);
    assert_eq!(
        e.view.centre_cell(e.pane),
        (i64::from(e.cursor.0), i64::from(e.cursor.1))
    );
    let text = screen_text(app.world());
    assert!(text.contains("Cell (3, 3)"), "{text}");
}

#[cfg(feature = "devtools")]
mod socket {
    use super::*;
    use omnis_vector::shell::capture::CapturePlugin;
    use omnis_vector::shell::socket::{DevSocketPlugin, NO_EDITOR};
    use serde_json::{Value, json};
    use std::io::{BufRead, BufReader, ErrorKind, Write};
    use std::net::{SocketAddr, TcpStream};

    fn send(
        app: &mut App,
        stream: &mut TcpStream,
        reader: &mut BufReader<TcpStream>,
        line: &str,
    ) -> Value {
        stream.write_all(line.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        for _ in 0..200 {
            app.update();
            let mut reply = String::new();
            match reader.read_line(&mut reply) {
                Ok(0) => panic!("closed after {line}"),
                Ok(_) => return serde_json::from_str(&reply).unwrap(),
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) => panic!("{e}"),
            }
        }
        panic!("no reply to {line}")
    }

    #[test]
    fn the_socket_answers_editor_ops_only_while_the_editor_is_open_and_the_layer_follows() {
        let dir = std::env::temp_dir().join(format!("omnis-editor-socket-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let addr_file = dir.join("dev.addr");
        let mut app = app_at_title(".omnis/test-saves/editor-none.ron");
        app.add_plugins((
            CapturePlugin,
            DevSocketPlugin {
                addr: "127.0.0.1:0".into(),
                addr_file: addr_file.clone(),
                root: dir.clone(),
            },
        ));
        app.update();
        let addr: SocketAddr = std::fs::read_to_string(&addr_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_millis(20)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut ask = |app: &mut App, line: &str| send(app, &mut stream, &mut reader, line);

        let reply = ask(&mut app, r#"{"id": 1, "op": "editor.status"}"#);
        assert_eq!(reply["ok"], json!(false), "{reply}");
        assert!(
            reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains(NO_EDITOR),
            "{reply}"
        );

        press(&mut app, ControlId::Editor);
        let reply = ask(&mut app, r#"{"id": 2, "op": "editor.status"}"#);
        assert_eq!(reply["ok"], json!(true), "{reply}");
        assert_eq!(reply["result"]["reply"], json!("editor"));
        assert_eq!(reply["result"]["editor"]["map"], Value::Null);

        let before = editing(&app).revision;
        let reply = ask(
            &mut app,
            r#"{"id": 3, "op": "editor.open", "args": {"map": "test:map:meadow"}}"#,
        );
        assert_eq!(reply["ok"], json!(true), "{reply}");
        assert_eq!(reply["result"]["editor"]["map"], json!("test:map:meadow"));
        let e = editing(&app);
        assert_eq!(
            e.map_id().as_deref(),
            Some("test:map:meadow"),
            "the layer's editor"
        );
        assert!(e.revision > before, "and it redraws");
        assert!(e.resolved.is_some());
        let text = screen_text(app.world());
        assert!(text.contains("test:map:meadow"), "{text}");

        let reply = ask(&mut app, r#"{"id": 4, "op": "game.status"}"#);
        assert_eq!(
            reply["ok"],
            json!(false),
            "no game for the game's ops: {reply}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The wheel over the plan zooms about the pointer (M5.9d, the owner's ask): the cell under
/// the mouse stays under it, a notch a step, down out and up in. The buttons and keys still zoom
/// about the centre.
#[test]
fn the_wheel_over_the_plan_zooms_about_the_pointer() {
    use bevy::input::mouse::MouseScrollUnit::Line;
    use common::app::wheel;
    use common::editor::point_at;
    let mut app = in_editor("test:map:dungeon");
    app.update();
    let (before, pane) = {
        let e = editing(&app);
        (e.view, e.pane)
    };
    // A cell off the pane's middle.
    let cell = (1, 1);
    point_at(&mut app, cell, None);
    app.update();
    assert_eq!(editing(&app).hover.map(|t| t.cell), Some(cell));
    wheel(&mut app, -1.0, Line);
    let e = editing(&app);
    assert!(
        e.view.scale() < before.scale(),
        "a notch down zooms out: {} < {}",
        e.view.scale(),
        before.scale()
    );
    assert_eq!(
        e.hover.map(|t| t.cell),
        Some(cell),
        "the cell under the pointer stays under it"
    );
    assert_ne!(e.view, before.zoomed(-1, pane), "not about the centre");
    wheel(&mut app, 1.0, Line);
    assert_eq!(editing(&app).view, before, "a notch up brings it back");
    assert_eq!(editing(&app).hover.map(|t| t.cell), Some(cell));
    for _ in 0..plan::ZOOMS.len() {
        wheel(&mut app, 1.0, Line);
    }
    assert_eq!(editing(&app).view.scale(), *plan::ZOOMS.last().unwrap());
    wheel(&mut app, 1.0, Line);
    assert_eq!(
        editing(&app).view.scale(),
        *plan::ZOOMS.last().unwrap(),
        "the largest holds"
    );
}

/// A trackpad's pixels add up to one step a line; the fraction left is dropped when the pointer
/// leaves the plan. Off the plan, or under a screen, the wheel zooms nothing.
#[test]
fn a_trackpad_s_pixels_add_up_to_a_step_and_the_wheel_off_the_plan_zooms_nothing() {
    use bevy::input::mouse::MouseScrollUnit::{Line, Pixel};
    use common::app::wheel;
    use common::editor::{leave_plan, point_at};
    let mut app = in_editor("test:map:dungeon");
    app.update();
    let scale = editing(&app).view.scale();
    point_at(&mut app, (1, 1), None);
    wheel(&mut app, -15.0, Pixel);
    wheel(&mut app, -15.0, Pixel);
    assert_eq!(
        editing(&app).view.scale(),
        scale,
        "30 px is short of a line"
    );
    wheel(&mut app, -15.0, Pixel);
    let stepped = editing(&app).view.scale();
    assert!(stepped < scale, "45 px is a step: {stepped} < {scale}");
    wheel(&mut app, -15.0, Pixel);
    assert_eq!(
        editing(&app).view.scale(),
        stepped,
        "20 px carried, no step"
    );
    leave_plan(&mut app);
    wheel(&mut app, -1.0, Line);
    assert_eq!(editing(&app).view.scale(), stepped, "off the plan, nothing");
    point_at(&mut app, (1, 1), None);
    wheel(&mut app, -30.0, Pixel);
    assert_eq!(
        editing(&app).view.scale(),
        stepped,
        "the 20 px were dropped off the plan: 30 px is short of a line"
    );
    press_edit(&mut app, EditAct::OpenFile);
    assert_eq!(
        app.world().resource::<Screens>().nav.top(None),
        Some(ScreenId::EditorFiles)
    );
    wheel(&mut app, -1.0, Line);
    assert_eq!(
        editing(&app).view.scale(),
        stepped,
        "under the picker, nothing"
    );
}
