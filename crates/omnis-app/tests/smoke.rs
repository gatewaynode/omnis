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
            packs: vec![repo.join("packs/base"), repo.join("packs/test")],
            seed: 7,
            save_path: save,
            autostart: true,
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

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use omnis_app::menus::MenusPlugin;
use omnis_app::sim::{MenuState, PlayState};

fn menu_app() -> App {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(AppConfig {
            packs: vec![repo.join("packs/base"), repo.join("packs/test")],
            seed: 7,
            save_path: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("menu-save.ron"),
            autostart: false,
        })
        .add_plugins((SimPlugin, InputPlugin, MenusPlugin));
    app
}

/// One logical key press, as a window would deliver it; one frame reads it, the next applies
/// the state it asked for.
fn key(app: &mut App, logical: Key) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code: KeyCode::F24,
            logical_key: logical,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    app.update();
}

fn keys(app: &mut App, sequence: &[Key]) {
    for k in sequence {
        key(app, k.clone());
    }
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        key(app, Key::Character(c.to_string().into()));
    }
}

fn play_state(app: &App) -> PlayState {
    *app.world().resource::<State<PlayState>>().get()
}

/// Title, new game: seed 42, save rule Relief, Start.
fn start_new_game_by_keys(app: &mut App) {
    key(app, Key::Enter);
    assert_eq!(
        *app.world().resource::<State<MenuState>>().get(),
        MenuState::NewGame
    );
    type_text(app, "42");
    keys(
        app,
        &[
            Key::ArrowDown,
            Key::ArrowRight,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
        ],
    );
}

/// A human fighter: STR 15, DEX 14, CON 13, INT 12, WIS 10, CHA 8, Athletics and Perception.
fn draft_fighter_by_keys(app: &mut App) {
    type_text(app, "Brenna");
    // Race: Left from dwarf wraps to human. Class: Right from cleric is fighter.
    keys(
        app,
        &[
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowDown,
            Key::ArrowRight,
        ],
    );
    // Past background and alignment to the scores.
    keys(app, &[Key::ArrowDown, Key::ArrowDown, Key::ArrowDown]);
    for raise in [7, 6, 5, 4, 2, 0] {
        for _ in 0..raise {
            key(app, Key::ArrowRight);
        }
        key(app, Key::ArrowDown);
    }
    // Skills: Athletics (third) and Perception (seventh) of the fighter's list.
    keys(app, &[Key::ArrowRight, Key::ArrowRight, Key::Enter]);
    keys(
        app,
        &[
            Key::ArrowRight,
            Key::ArrowRight,
            Key::ArrowRight,
            Key::ArrowRight,
            Key::Enter,
        ],
    );
    keys(app, &[Key::ArrowDown, Key::Enter]);
}

/// The menus, headless: the title, a typed seed and difficulty, a fighter built by keys,
/// begin, pause, resume, and a save the rule forbids.
#[test]
fn the_menus_build_a_party_without_a_window() {
    let mut app = menu_app();
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
    assert_eq!(
        *app.world().resource::<State<MenuState>>().get(),
        MenuState::Title
    );
    assert!(
        app.world().get_resource::<SimWorld>().is_none(),
        "no world before New Game"
    );

    start_new_game_by_keys(&mut app);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Playing
    );
    assert_eq!(play_state(&app), PlayState::CreateParty);
    let world = &app.world().resource::<SimWorld>().0;
    assert_eq!(world.seed, 42);
    assert_eq!(world.settings.save_rule, omnis_sim::SaveRule::Relief);
    assert!(world.party.members.is_empty());

    draft_fighter_by_keys(&mut app);
    let members = &app.world().resource::<SimWorld>().0.party.members;
    assert_eq!(members.len(), 1, "the draft became a member");
    assert_eq!(members[0].name, "Brenna");
    assert_eq!(members[0].hp_max, 12);
    assert_eq!(members[0].scores, [16, 15, 14, 13, 11, 9]);
    let screens = app.world().resource::<omnis_app::menus::Screens>();
    assert!(
        screens.creation.name.is_empty(),
        "the form reset for the next member"
    );

    keys(&mut app, &[Key::ArrowDown, Key::Enter]); // Begin
    assert_eq!(play_state(&app), PlayState::Explore);

    // Escape pauses through the key table; Enter on Resume returns.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    assert_eq!(play_state(&app), PlayState::Paused);
    key(&mut app, Key::Enter);
    assert_eq!(play_state(&app), PlayState::Explore);

    // F5 is refused under the Relief rule: no inns yet.
    app.world_mut()
        .resource_mut::<Messages<ShellCommand>>()
        .write(ShellCommand::Save);
    app.update();
    let notice = app.world().resource::<omnis_app::sim::Notice>();
    assert!(notice.0.contains("save rule"), "{}", notice.0);
}
