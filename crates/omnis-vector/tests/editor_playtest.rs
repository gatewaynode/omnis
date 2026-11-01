//! Playtest (M5.8): a game on the edited map as the editor holds it, the fixed party on the
//! cursor's cell facing its way, the editor parked and brought back as it was left by Back to
//! editor; what stops a playtest is said on its button.

use crate::common;

use bevy::prelude::*;
use common::app::{set, tap};
use common::editor::{editing, in_editor, press_edit};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::EdgeKind;
use omnis_vector::editor::brush::{Entry, Target, Tool};
use omnis_vector::editor::{EditAct, view};
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::editor::{Editing, Playtesting};
use omnis_vector::shell::screen::{Pressable, Screens};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::{Packs, Session};

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

/// An open east edge on row `y` of the open map.
fn open_edge(app: &App, y: u16) -> (u16, u16) {
    let e = editing(app);
    let grid = e.grid().unwrap();
    (0..e.size().0 - 1)
        .find(|&x| grid.edge(x, y, Facing::East) == Some(EdgeKind::Open))
        .map(|x| (x, y))
        .expect("an open edge")
}

#[test]
fn playtest_plays_the_edited_map_from_the_cursor_and_back_to_editor_returns_it_as_left() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let town = data.registry.maps.get("test:map:town").unwrap();
    // A wall painted across the street: the edited map, not the file's.
    let wall = open_edge(&app, 3);
    {
        let mut e = app.world_mut().resource_mut::<Editing>();
        e.tool = Tool::Brush;
        e.entry = Entry::Edge(EdgeKind::Wall);
        assert!(e.paint(
            &data,
            Target {
                cell: wall,
                edge: Some(Facing::East)
            },
            false
        ));
        e.point((4, 3));
        e.facing = Facing::East;
    }
    app.update();
    let (cursor, facing, undo) = {
        let e = editing(&app);
        assert!(e.status.dirty);
        (e.cursor, e.facing, e.status.undo)
    };
    assert_eq!(
        view::tools(editing(&app), &data)
            .control(ControlId::Edit(EditAct::Playtest))
            .unwrap()
            .why_not,
        None,
        "the street is playable"
    );
    press_edit(&mut app, EditAct::Playtest);
    app.update();
    assert!(
        app.world().get_resource::<Editing>().is_none(),
        "the editor is parked"
    );
    assert!(app.world().get_resource::<Playtesting>().is_some());
    {
        let session = app.world().resource::<Session>();
        let p = session.views.position;
        assert_eq!(
            (p.map, p.level, p.x, p.y, p.facing),
            (town, Level::GROUND, cursor.0, cursor.1, facing)
        );
        assert_eq!(session.views.party.members.len(), 4, "the fixed party");
        // The session plays the edited map; the packs are untouched.
        let played = session.data.maps[&town]
            .cell(Level::GROUND, wall.0, wall.1)
            .unwrap();
        assert!(
            played.walls.has(Facing::East),
            "the painted wall stands in the playtest"
        );
        let kept = app.world().resource::<Packs>().data.maps[&town]
            .cell(Level::GROUND, wall.0, wall.1)
            .unwrap();
        assert!(!kept.walls.has(Facing::East), "the packs are as loaded");
    }
    // Esc: the pause offers Back to editor, and keeps nothing of a playtest.
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause));
    let text = screen_text(app.world());
    assert!(text.contains("Back to editor"), "{text}");
    assert!(text.contains("Save: a playtest is not saved"), "{text}");
    assert!(!text.contains("Quit to title"), "{text}");
    press(&mut app, ControlId::BackToEditor);
    app.update();
    assert!(app.world().get_resource::<Session>().is_none());
    assert!(app.world().get_resource::<Playtesting>().is_none());
    let e = editing(&app);
    assert_eq!((e.cursor, e.facing), (cursor, facing), "as left");
    assert!(
        e.status.dirty && e.status.undo == undo,
        "the document untouched"
    );
    assert_eq!(top(&app), None, "the editor is the base again");
    assert!(screen_text(app.world()).contains("EDITOR"));
}

#[test]
fn playtest_is_dim_with_its_reason_until_the_cursor_stands_on_playable_ground() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let reason = |app: &App| {
        view::tools(editing(app), &data)
            .control(ControlId::Edit(EditAct::Playtest))
            .unwrap()
            .why_not
            .clone()
    };
    assert_eq!(reason(&app), None);
    // No ground under the cursor: a street cell, not the start (erasing the start's ground is
    // the loader's error since the entrances step 1, and the reason would be its count).
    {
        let mut e = app.world_mut().resource_mut::<Editing>();
        e.tool = Tool::Eraser;
        e.cursor = (8, 2);
        let cell = e.cursor;
        assert!(e.paint(&data, Target { cell, edge: None }, false));
    }
    app.update();
    assert!(
        editing(&app).status.errors.is_empty(),
        "the map still loads"
    );
    assert_eq!(
        reason(&app).as_deref(),
        Some("the cursor stands on no ground")
    );
    press_edit(&mut app, EditAct::Undo);
    assert_eq!(reason(&app), None);
    // A map the loader refuses: a ramp joining heights more than 16 apart.
    let ramp = open_edge(&app, 3);
    {
        let mut e = app.world_mut().resource_mut::<Editing>();
        e.tool = Tool::Brush;
        e.entry = Entry::Edge(EdgeKind::Ramp);
        assert!(e.paint(
            &data,
            Target {
                cell: ramp,
                edge: Some(Facing::East)
            },
            false
        ));
        e.entry = Entry::Height;
        e.eighths = 24;
        assert!(e.paint(
            &data,
            Target {
                cell: ramp,
                edge: None
            },
            false
        ));
    }
    app.update();
    assert_eq!(
        reason(&app).as_deref(),
        Some("1 error"),
        "the count alone; the inspector lists the words"
    );
}

#[test]
fn quit_to_title_from_a_playtest_leaves_nothing_parked() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Playtest);
    app.update();
    assert!(app.world().get_resource::<Session>().is_some());
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause));
    // Quit to title is Back to editor's neighbour on the Debug route only; here the pause has none,
    // so the editor's own way out is the title through Back to editor then the editor's menu.
    press(&mut app, ControlId::BackToEditor);
    app.update();
    press_edit(&mut app, EditAct::Menu);
    press(&mut app, ControlId::QuitToTitle);
    assert!(app.world().get_resource::<Session>().is_none());
    assert!(app.world().get_resource::<Editing>().is_none());
    assert!(app.world().get_resource::<Playtesting>().is_none());
    assert_eq!(top(&app), Some(ScreenId::Title));
}
