//! The shell's headless half (alt-ARCHITECTURE.md §10.4): held keys move the world through the
//! movement plugin with no window or GPU.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonState, InputPlugin};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use omnis_vector::shell::ShellPlugin;
use omnis_vector::shell::movement::MovementPlugin;
use omnis_vector::shell::session::{Config, Session};
use std::path::PathBuf;
use std::time::Duration;

/// A key press or release as the window would send it.
fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Character("q".into()),
        state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

fn app() -> App {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let config = Config {
        packs: vec![root.join("base"), root.join("test")],
        ..Config::default()
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, InputPlugin, ShellPlugin, MovementPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )))
        .insert_resource(Session::start(&config).expect("the shipped packs start a session"));
    app
}

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
