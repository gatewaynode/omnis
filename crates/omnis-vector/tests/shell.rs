//! The shell's headless half (presentation-ARCHITECTURE.md §10.4): held keys and pressed buttons move
//! the world through the movement plugin with no window or GPU.

use crate::common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{app, app_logging_to, key, set};
use omnis_vector::shell::controls::{Action, Corner};
use omnis_vector::shell::session::Session;

#[test]
fn holding_w_walks_the_party_north_up_the_road() {
    let mut app = app();
    app.update();
    let start = app.world().resource::<Session>().world.position;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    for _ in 0..60 {
        app.update();
    }
    let session = app.world().resource::<Session>();
    assert_eq!(session.world.position.x, start.x);
    assert!(session.world.position.y < start.y, "the party moved north");
    assert_eq!(
        (session.binder.refusals, session.binder.disagreements),
        (0, 0)
    );
}

#[test]
fn q_turns_the_party_left_by_a_quarter() {
    let mut app = app();
    app.update();
    key(&mut app, KeyCode::KeyQ, ButtonState::Pressed);
    app.update();
    key(&mut app, KeyCode::KeyQ, ButtonState::Released);
    for _ in 0..60 {
        app.update();
    }
    let facing = app.world().resource::<Session>().world.position.facing;
    assert_eq!(facing, omnis_sim::omnis_core::Facing::West);
}

#[test]
fn holding_the_forward_button_walks_north() {
    let mut app = app();
    app.update();
    let start = app.world().resource::<Session>().world.position;
    set(&mut app, &Action::Forward, Interaction::Pressed);
    for _ in 0..60 {
        app.update();
    }
    let moved = app.world().resource::<Session>().world.position;
    assert!(moved.y < start.y, "the party moved north");
    // Released, the party stops.
    set(&mut app, &Action::Forward, Interaction::None);
    for _ in 0..60 {
        app.update();
    }
    let rest = app.world().resource::<Session>().world.position;
    assert!(start.y - rest.y <= start.y - moved.y + 1, "it stopped");
}

#[test]
fn a_turn_button_held_down_turns_once() {
    let mut app = app();
    app.update();
    set(&mut app, &Action::TurnLeft, Interaction::Pressed);
    for _ in 0..90 {
        app.update();
    }
    let facing = app.world().resource::<Session>().world.position.facing;
    assert_eq!(facing, omnis_sim::omnis_core::Facing::West);
}

#[test]
fn the_save_button_writes_a_log_that_replays() {
    let dir = std::env::temp_dir().join(format!("omnis-vector-save-{}", std::process::id()));
    let path = dir.join("session.ron");
    let mut app = app_logging_to(path.clone());
    app.update();
    set(&mut app, &Action::Forward, Interaction::Pressed);
    for _ in 0..30 {
        app.update();
    }
    set(&mut app, &Action::Forward, Interaction::None);
    // The log is saved from the pause, which the Menu button opens.
    set(&mut app, &Action::Menu, Interaction::Pressed);
    app.update();
    app.update();
    set(
        &mut app,
        &omnis_vector::shell::screen::Pressable(omnis_vector::screen::ControlId::SaveLog),
        Interaction::Pressed,
    );
    app.update();
    let message = app
        .world()
        .resource::<omnis_vector::shell::screen::Screens>()
        .forms
        .message
        .clone()
        .expect("an outcome");
    assert!(message.text.starts_with("Log saved"), "{message:?}");
    let session = app.world().resource::<Session>();
    let saved: omnis_sim::Replay =
        omnis_sim::omnis_data::ron_io::read_ron(&path, &path).expect("the log was written");
    assert_eq!(saved.commands, session.binder.log);
    assert_eq!(
        saved.fingerprint,
        session.world.fingerprint().expect("a fingerprint")
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The corner group a button sits in: button → row → group.
fn corner_of(app: &mut App, action: Action) -> Corner {
    let world = app.world_mut();
    let mut buttons = world.query::<(Entity, &Action)>();
    let button = buttons
        .iter(world)
        .find(|(_, a)| **a == action)
        .map(|(e, _)| e)
        .expect("the button");
    let parent = |e: Entity| world.get::<ChildOf>(e).expect("a parent").parent();
    let group = parent(parent(button));
    *world.get::<Corner>(group).expect("a corner group")
}

#[test]
fn the_menu_is_kept_away_from_the_movement_pad_and_buttons_are_opaque() {
    let mut app = app();
    app.update();
    for action in [Action::Forward, Action::Back, Action::TurnLeft] {
        assert_eq!(corner_of(&mut app, action), Corner::Left, "{action:?}");
    }
    for action in [Action::Menu, Action::Actions, Action::Log] {
        assert_eq!(corner_of(&mut app, action), Corner::Right, "{action:?}");
    }
    // Rows of three at most: both groups fit side by side at 1280 pixels (owner, 2026-10-17).
    assert!(omnis_vector::shell::controls::widest_row(Corner::Left) <= 3);
    assert!(omnis_vector::shell::controls::widest_row(Corner::Right) <= 3);
    let world = app.world_mut();
    let mut backgrounds = world.query_filtered::<&BackgroundColor, With<Action>>();
    for colour in backgrounds.iter(world) {
        assert!((colour.0.alpha() - 1.0).abs() < f32::EPSILON, "opaque");
    }
}

#[test]
fn the_hud_reports_a_smoothed_frame_rate_and_frame_time() {
    use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
    use omnis_vector::shell::hud::{HudPlugin, StatusText};
    let mut app = app();
    app.add_plugins((FrameTimeDiagnosticsPlugin::default(), HudPlugin));
    for _ in 0..60 {
        app.update();
    }
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Text, With<StatusText>>();
    let status = query.single(world).expect("the status text").0.clone();
    // Frames are 16 ms apart: about 62 frames a second.
    assert!(status.contains("62 fps  16.0 ms"), "{status}");
}

/// The views the shell reads are the world's at the end of every frame: walking, a door, monsters
/// met and a fight (A17: the shell reads views, so a stale one would show the wrong world).
#[test]
fn the_views_are_the_world_s_after_every_frame() {
    use omnis_sim::omnis_core::{Facing, Position};
    use omnis_sim::{CombatCommand, Command, EncounterChoice, ModeKind};
    use omnis_vector::pose::Pose;
    use omnis_vector::sim::Views;
    let fresh = |app: &App, when: &str| {
        let session = app.world().resource::<Session>();
        assert_eq!(
            session.views,
            Views::of(&session.world, &session.data),
            "stale views {when}"
        );
    };
    let mut app = app_logging_to(".omnis/vector-views.ron".into());
    app.update();
    fresh(&app, "at the start");
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = common::map(&session.data, "test:map:dungeon");
        session.world.position = Position {
            map: dungeon,
            level: omnis_sim::omnis_core::Level::GROUND,
            x: 3,
            y: 5,
            facing: Facing::South,
        };
        session.pose = Pose::at(session.world.position);
        session.refresh();
        let before = session.binder.log.len();
        session.order(Command::Interact);
        assert!(session.binder.log.len() > before, "the door opened");
    }
    fresh(&app, "after the door");
    set(&mut app, &Action::Forward, Interaction::Pressed);
    let mut met = false;
    for frame in 0..120 {
        app.update();
        fresh(&app, &format!("walking, frame {frame}"));
        if app.world().resource::<Session>().views.combat.is_some() {
            met = true;
            break;
        }
    }
    set(&mut app, &Action::Forward, Interaction::None);
    assert!(met, "the rats are met");
    let mut session = app.world_mut().resource_mut::<Session>();
    session.order(Command::Encounter(EncounterChoice::Attack));
    let mut fought = false;
    for _ in 0..200 {
        let events_before = session.fight_log.len();
        session.order(Command::Combat(CombatCommand::Dodge));
        session.order(Command::Combat(CombatCommand::EndTurn));
        assert_eq!(
            session.views,
            Views::of(&session.world, &session.data),
            "in the fight"
        );
        fought |= session.fight_log.len() != events_before;
        if session.views.here.mode != ModeKind::Combat {
            break;
        }
    }
    assert!(fought, "the fight ran");
}
