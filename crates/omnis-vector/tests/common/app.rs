//! A headless shell: the movement, button and notice plugins with no window or GPU, and the
//! input a window would send.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseButtonInput;
use bevy::input::{ButtonState, InputPlugin};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use omnis_sim::Mode;
use omnis_sim::omnis_core::{Facing, Position};
use omnis_vector::pose::Pose;
use omnis_vector::shell::ShellPlugin;
use omnis_vector::shell::combat::CombatPlugin;
use omnis_vector::shell::controls::{Action, ControlsPlugin};
use omnis_vector::shell::movement::MovementPlugin;
use omnis_vector::shell::screen::ScreenPlugin;
use omnis_vector::shell::session::{Config, Packs, Session};
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

/// A left click at `point` (logical pixels from the top left) in the app's window, spawning a
/// 1600×900 window the first time: headless there is none, so the click needs one to land in.
pub fn click(app: &mut App, point: (f32, f32)) {
    let world = app.world_mut();
    let mut windows = world.query_filtered::<Entity, With<bevy::window::PrimaryWindow>>();
    let window = match windows.iter(world).next() {
        Some(window) => window,
        None => world
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1600, 900),
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id(),
    };
    world
        .get_mut::<Window>(window)
        .expect("the window")
        .set_cursor_position(Some(Vec2::new(point.0, point.1)));
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
        app.update();
    }
}

/// The shell with the party on the meadow start the walking tests were written on.
pub fn app() -> App {
    let mut app = app_with(PathBuf::from(".omnis/vector-session.ron"), 1);
    let mut session = app.world_mut().resource_mut::<Session>();
    let session = &mut *session;
    session.pose = super::to_meadow(&mut session.world, &session.data);
    session.refresh();
    app
}

/// The shell at the game's own start (the town), logging to `log`: its log replays.
pub fn app_logging_to(log: PathBuf) -> App {
    app_with(log, 1)
}

pub fn app_with(log: PathBuf, seed: u64) -> App {
    shell(
        Config {
            log,
            seed,
            ..config()
        },
        true,
    )
}

/// The command line the tests run with: the shipped packs.
pub fn config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    Config {
        packs: vec![root.join("base"), root.join("test")],
        // Never the player's quick save: a test that saves names its own file.
        save: ".omnis/test-saves/none.ron".to_owned(),
        ..Config::default()
    }
}

/// The headless shell with the packs from `config`, in a game started from it when `in_game`,
/// otherwise at the title.
pub fn shell(config: Config, in_game: bool) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        InputPlugin,
        StatesPlugin,
        ShellPlugin,
        MovementPlugin,
        ControlsPlugin,
        ScreenPlugin,
        CombatPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )))
    .insert_resource(Packs::load(&config).expect("the shipped packs load"));
    if in_game {
        app.insert_resource(Session::start(&config).expect("the shipped packs start a session"));
    }
    app
}

/// The shell at the title: the packs loaded, no game yet; saves go to `save`.
pub fn app_at_title(save: &str) -> App {
    shell(
        Config {
            save: save.to_owned(),
            ..config()
        },
        false,
    )
}

/// The shell in a game at the town's start, saving to `save`.
pub fn app_in_game(save: &str) -> App {
    shell(
        Config {
            save: save.to_owned(),
            ..config()
        },
        true,
    )
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

/// An app with the party three cells north of the dungeon's placed group, walked into it with
/// the Forward button, so the encounter is the simulation's own.
pub fn met_with(seed: u64) -> App {
    let mut app = app_with(PathBuf::from(".omnis/vector-session.ron"), seed);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = super::map(&session.data, "test:map:dungeon");
        session.world.position = Position {
            map: dungeon,
            x: 3,
            y: 6,
            facing: Facing::South,
        };
        session.pose = Pose::at(session.world.position);
        session.refresh();
    }
    set(&mut app, &Action::Forward, Interaction::Pressed);
    for _ in 0..80 {
        app.update();
    }
    set(&mut app, &Action::Forward, Interaction::None);
    app.update();
    assert!(matches!(
        app.world().resource::<Session>().world.mode,
        Mode::Encounter(_)
    ));
    app
}

/// Press a button carrying `marker`, then let it go, a frame each.
pub fn press_ui<C: Component + PartialEq + core::fmt::Debug + Copy>(app: &mut App, marker: C) {
    set(app, &marker, Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut query = world.query::<(&C, &mut Interaction)>();
    for (c, mut i) in query.iter_mut(world) {
        if *c == marker {
            *i = Interaction::None;
        }
    }
    app.update();
}

/// Tap a key: pressed for a frame, then released for one.
pub fn tap(app: &mut App, code: KeyCode) {
    key(app, code, ButtonState::Pressed);
    app.update();
    key(app, code, ButtonState::Released);
    app.update();
}

/// The screen on top of a running game.
pub fn top(app: &App) -> Option<omnis_vector::screen::ScreenId> {
    let session = app.world().resource::<Session>();
    app.world()
        .resource::<omnis_vector::shell::screen::Screens>()
        .nav
        .top(Some(&session.views))
}
