//! The shell's headless half (alt-ARCHITECTURE.md §10.4): held keys and pressed buttons move
//! the world through the movement plugin with no window or GPU.

mod common;

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
    set(&mut app, &Action::SaveLog, Interaction::Pressed);
    app.update();
    let session = app.world().resource::<Session>();
    let saved: omnis_sim::Replay =
        omnis_sim::omnis_data::ron_io::read_ron(&path, &path).expect("the log was written");
    assert_eq!(saved.commands, session.binder.log);
    assert_eq!(
        saved.fingerprint,
        session.world.fingerprint().expect("a fingerprint")
    );
    assert!(session.lines.iter().any(|l| l.starts_with("Log saved")));
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
fn quit_is_kept_away_from_the_movement_pad_and_buttons_are_opaque() {
    let mut app = app();
    app.update();
    for action in [Action::Forward, Action::Back, Action::TurnLeft] {
        assert_eq!(corner_of(&mut app, action), Corner::Left, "{action:?}");
    }
    for action in [Action::Quit, Action::SaveLog, Action::Interact] {
        assert_eq!(corner_of(&mut app, action), Corner::Right, "{action:?}");
    }
    let world = app.world_mut();
    let mut backgrounds = world.query_filtered::<&BackgroundColor, With<Action>>();
    for colour in backgrounds.iter(world) {
        assert!((colour.0.alpha() - 1.0).abs() < f32::EPSILON, "opaque");
    }
}
