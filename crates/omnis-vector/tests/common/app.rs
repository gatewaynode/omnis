//! A headless shell: the movement, button and notice plugins with no window or GPU, and the
//! input a window would send.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonState, InputPlugin};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use omnis_vector::shell::ShellPlugin;
use omnis_vector::shell::controls::ControlsPlugin;
use omnis_vector::shell::fight::FightPlugin;
use omnis_vector::shell::movement::MovementPlugin;
use omnis_vector::shell::session::{Config, Session};
use std::path::PathBuf;
use std::time::Duration;

/// A key press or release as the window would send it.
pub fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Character("q".into()),
        state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

pub fn app() -> App {
    app_with(PathBuf::from(".omnis/vector-session.ron"), 1)
}

pub fn app_logging_to(log: PathBuf) -> App {
    app_with(log, 1)
}

pub fn app_with(log: PathBuf, seed: u64) -> App {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let config = Config {
        packs: vec![root.join("base"), root.join("test")],
        log,
        seed,
        ..Config::default()
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        InputPlugin,
        ShellPlugin,
        MovementPlugin,
        ControlsPlugin,
        FightPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )))
    .insert_resource(Session::start(&config).expect("the shipped packs start a session"));
    app
}

/// Set the interaction of every button carrying `marker`, as the UI's picking would.
pub fn set<C: Component + PartialEq + core::fmt::Debug>(
    app: &mut App,
    marker: &C,
    interaction: Interaction,
) {
    let world = app.world_mut();
    let mut query = world.query::<(&C, &mut Interaction)>();
    let mut found = false;
    for (c, mut i) in query.iter_mut(world) {
        if c == marker {
            *i = interaction;
            found = true;
        }
    }
    assert!(found, "a {marker:?} button exists");
}
