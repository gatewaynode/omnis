//! Headless smoke test (ARCHITECTURE.md §11, tier 6): `MinimalPlugins`, no window, no GPU.
//! Boot loads the real test pack, a key press becomes a command, the world moves, events are
//! published, a save round-trips through the shell.

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use omnis_app::sim::{AppState, PlayerCommand, ShellCommand, SimEvent, SimPlugin, SimWorld};
use omnis_app::{AppConfig, input::InputPlugin};
use omnis_sim::omnis_core::{Direction, Facing};
use omnis_sim::{Command, Event};
use std::path::PathBuf;

fn app() -> App {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let save = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("smoke-save.ron");
    let _ = std::fs::remove_file(&save);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(AppConfig {
            packs: vec![repo.join("packs/test")],
            seed: 7,
            save_path: save,
        })
        .add_plugins((SimPlugin, InputPlugin));
    app
}

#[test]
fn boots_steps_and_saves_without_a_window() {
    let mut app = app();
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Playing
    );
    let start = app.world().resource::<SimWorld>().0.position;
    assert_eq!((start.x, start.y, start.facing), (16, 16, Facing::North));

    // A command message moves the party and publishes events.
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(Command::Step(Direction::Forward)));
    app.update();
    let after = app.world().resource::<SimWorld>().0.position;
    assert_eq!((after.x, after.y), (16, 15));
    let events = app.world().resource::<Messages<SimEvent>>();
    let mut cursor = events.get_cursor();
    let published: Vec<&Event> = cursor.read(events).map(|e| &e.0).collect();
    assert!(published.iter().any(|e| matches!(e, Event::Moved { .. })));
    assert!(published.iter().any(|e| matches!(e, Event::Visible { .. })));

    // A key press is a command too.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyA);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    assert_eq!(
        app.world().resource::<SimWorld>().0.position.facing,
        Facing::West
    );

    // Save, move on, load: back where the save was taken.
    app.world_mut()
        .resource_mut::<Messages<ShellCommand>>()
        .write(ShellCommand::Save);
    app.update();
    let saved_fingerprint = app.world().resource::<SimWorld>().0.fingerprint().unwrap();
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(Command::Step(Direction::Forward)));
    app.update();
    assert_ne!(
        app.world().resource::<SimWorld>().0.fingerprint().unwrap(),
        saved_fingerprint
    );
    app.world_mut()
        .resource_mut::<Messages<ShellCommand>>()
        .write(ShellCommand::Load);
    app.update();
    assert_eq!(
        app.world().resource::<SimWorld>().0.fingerprint().unwrap(),
        saved_fingerprint
    );
    let notice = app.world().resource::<omnis_app::sim::Notice>();
    assert!(notice.0.starts_with("Loaded"), "{}", notice.0);
}

#[test]
fn a_missing_pack_exits_with_an_error() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(AppConfig {
            packs: vec![PathBuf::from("/nonexistent/pack")],
            ..AppConfig::default()
        })
        .add_plugins(SimPlugin);
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Boot
    );
    let exits = app.world().resource::<Messages<AppExit>>();
    let mut cursor = exits.get_cursor();
    assert!(cursor.read(exits).any(|e| e.is_error()));
}
