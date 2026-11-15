//! A headless shell: the movement, button and notice plugins with no window or GPU, and the
//! input a window would send.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
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
use omnis_vector::shell::editor::{Editing, EditorPlugin};
use omnis_vector::shell::fonts::FontsPlugin;
use omnis_vector::shell::movement::MovementPlugin;
use omnis_vector::shell::screen::ScreenPlugin;
use omnis_vector::shell::session::{Config, Packs, Session};
use omnis_vector::shell::settings::SettingsPlugin;
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

/// The command line the tests run with: the base pack and the frozen fixture.
pub fn config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    Config {
        packs: vec![root.join("packs/base"), root.join(super::FIXTURE)],
        // Never the player's quick save: a test that saves names its own file.
        save: ".omnis/test-saves/none.ron".to_owned(),
        // Never the player's settings: a test that applies names its own file.
        settings: ".omnis/test-saves/settings-none.ron".to_owned(),
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
        SettingsPlugin,
        FontsPlugin,
        EditorPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    let packs = Packs::load(&config).expect("the packs load");
    if let Some(map) = &config.editor {
        let mut editor =
            omnis_vector::editor::Editor::new(config.packs.clone()).expect("the editor opens");
        if let Some(id) = map {
            editor.open(id, &packs.data).expect("the map opens");
        }
        app.insert_resource(Editing(editor));
    }
    app.insert_resource(packs);
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

/// Where the pointer is on a control, as a test sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointing {
    /// Elsewhere.
    Rest,
    /// Over the control.
    Hover,
    /// Over it with the left button down.
    Press,
}

fn point<C: Component + PartialEq>(app: &mut App, marker: &C, pointing: Pointing) -> bool {
    use bevy::picking::hover::Hovered;
    use bevy::ui::Pressed;
    use omnis_vector::shell::controls::Control;
    let world = app.world_mut();
    let mut query = world.query_filtered::<(Entity, &C), With<Control>>();
    let found: Vec<Entity> = query
        .iter(world)
        .filter(|(_, c)| *c == marker)
        .map(|(e, _)| e)
        .collect();
    for &entity in &found {
        let mut control = world.entity_mut(entity);
        // As the window's picking does it: `Hovered` is written only when it changes.
        let over = pointing != Pointing::Rest;
        if control.get::<Hovered>().map(Hovered::get) != Some(over) {
            control.insert(Hovered(over));
        }
        if pointing == Pointing::Press {
            control.insert(Pressed);
        } else {
            control.remove::<Pressed>();
        }
    }
    !found.is_empty()
}

/// Put the pointer on every live control carrying `marker`, as the window's picking would. A
/// dim control is not live and fails this.
pub fn set<C: Component + PartialEq + core::fmt::Debug>(
    app: &mut App,
    marker: &C,
    pointing: Pointing,
) {
    assert!(point(app, marker, pointing), "a {marker:?} button exists");
}

/// Take the pointer off the controls carrying `marker`, if any still stand (a screen that
/// closed has none left).
pub fn let_go<C: Component + PartialEq>(app: &mut App, marker: &C) {
    point(app, marker, Pointing::Rest);
}

/// Take the pointer off every control.
pub fn rest(app: &mut App) {
    use bevy::picking::hover::Hovered;
    use bevy::ui::Pressed;
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<Pressed>>();
    let pressed: Vec<Entity> = query.iter(world).collect();
    for entity in pressed {
        world
            .entity_mut(entity)
            .remove::<Pressed>()
            .insert(Hovered(false));
    }
}

/// An app with the party three cells north of the dungeon's placed group, walked into it with
/// the Forward button, so the encounter is the simulation's own.
pub fn met_with(seed: u64) -> App {
    let app = app_with(PathBuf::from(".omnis/vector-session.ron"), seed);
    walked_into(app, "test:map:dungeon", (3, 6, Facing::South))
}

/// An app with the party beside the yard's two pack rats, walked into them: a fight on a map
/// whose biosphere (ember) writes every role's ink.
pub fn met_in_the_yard() -> App {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let app = shell(Config { packs, ..config() }, true);
    walked_into(app, "objects:map:yard", (3, 1, Facing::East))
}

/// A game on the road in the objects fixture's yard at `(x, y, facing)`: the two pack rats stand
/// at (5, 1).
pub fn in_the_yard((x, y, facing): (u16, u16, Facing)) -> App {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let mut app = shell(Config { packs, ..config() }, true);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let map = super::map(&session.data, "objects:map:yard");
        session.world.position = Position {
            map,
            level: omnis_sim::omnis_core::Level::GROUND,
            x,
            y,
            facing,
        };
        session.pose = Pose::at(session.world.position);
        session.refresh();
    }
    app.update();
    app
}

/// Put the party at `(x, y, facing)` on `map` and hold Forward until an encounter is met.
fn walked_into(mut app: App, map: &str, (x, y, facing): (u16, u16, Facing)) -> App {
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let map = super::map(&session.data, map);
        session.world.position = Position {
            map,
            level: omnis_sim::omnis_core::Level::GROUND,
            x,
            y,
            facing,
        };
        session.pose = Pose::at(session.world.position);
        session.refresh();
    }
    set(&mut app, &Action::Forward, Pointing::Press);
    for _ in 0..80 {
        app.update();
    }
    set(&mut app, &Action::Forward, Pointing::Rest);
    app.update();
    assert!(matches!(
        app.world().resource::<Session>().world.mode,
        Mode::Encounter(_)
    ));
    app
}

/// Press a button carrying `marker`, then let it go, a frame each.
pub fn press_ui<C: Component + PartialEq + core::fmt::Debug + Copy>(app: &mut App, marker: C) {
    set(app, &marker, Pointing::Press);
    app.update();
    let_go(app, &marker);
    app.update();
}

/// Tap a key: pressed for a frame, then released for one.
pub fn tap(app: &mut App, code: KeyCode) {
    key(app, code, ButtonState::Pressed);
    app.update();
    key(app, code, ButtonState::Released);
    app.update();
}

/// The mouse wheel turned by `y` (notches with `MouseScrollUnit::Line`, pixels with `Pixel`;
/// up is positive), then a frame.
pub fn wheel(app: &mut App, y: f32, unit: MouseScrollUnit) {
    app.world_mut().write_message(MouseWheel {
        unit,
        x: 0.0,
        y,
        window: Entity::PLACEHOLDER,
        phase: TouchPhase::Moved,
    });
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

/// `app` with the window's keyboard path to a text field: focus, dispatch and Bevy's text
/// input, and a primary window to send the keys from.
pub fn keyboard_on(mut app: App) -> (App, Entity) {
    use bevy::input_focus::{InputDispatchPlugin, InputFocusPlugin};
    app.add_plugins((
        bevy::asset::AssetPlugin::default(),
        InputFocusPlugin,
        InputDispatchPlugin,
        bevy::text::TextPlugin,
        bevy::ui_widgets::TextInputPlugin,
    ))
    .add_message::<bevy::window::Ime>()
    .add_message::<bevy::picking::events::PointerRelease>()
    // A text field scrolls under a held pointer (0.20): the pointer's state, with no picking here.
    .init_resource::<bevy::picking::events::PointerState>()
    .init_resource::<bevy::ui::UiScale>();
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    (app, window)
}

/// Type `text` on the keyboard, one key at a time, to whatever has focus.
pub fn keyboard(app: &mut App, window: Entity, text: &str) {
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput};
    for c in text.chars() {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::KeyA,
                logical_key: Key::Character(c.to_string().into()),
                state,
                text: Some(c.to_string().into()),
                repeat: false,
                window,
            });
            app.update();
        }
    }
}

/// Run `app` under the window's condition for text fields: Bevy's layout takes every
/// `EditableText` mutably every frame there, so every field reads as changed every frame whether
/// or not anything was typed (lesson 2026-10-24). Headless nothing touches a field, so a route that
/// reads changed fields is tested with this churn on.
pub fn churn_fields(app: &mut App) {
    app.add_systems(
        PreUpdate,
        |mut fields: Query<&mut bevy::text::EditableText>| {
            for mut field in &mut fields {
                field.set_changed();
            }
        },
    );
}

/// The window's physical size and scale: a Retina-like window, so logical and physical pixels
/// differ.
pub const UI_PHYSICAL: (u32, u32) = (2400, 1400);
pub const UI_SCALE: f32 = 2.0;

/// Without a renderer no system fills the camera's target: this stands in for it, so the UI
/// lays out to the window.
fn target_info(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<&mut bevy::camera::Camera>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    for mut camera in &mut cameras {
        camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: window.physical_size(),
            scale_factor: window.scale_factor(),
        });
    }
}

/// The shell in the editor with the town open, under Bevy's UI layout and focus (no renderer),
/// with a window and a camera; the window's entity. Shared by the pointer and layout tests.
pub fn ui_editor_app() -> (App, Entity) {
    let config = Config {
        editor: Some(Some("test:map:town".to_owned())),
        ..config()
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        InputPlugin,
        StatesPlugin,
        bevy::window::WindowPlugin {
            primary_window: None,
            primary_cursor_options: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            close_when_requested: false,
        },
        bevy::asset::AssetPlugin::default(),
        bevy::text::TextPlugin,
        bevy::camera::CameraPlugin,
        bevy::picking::DefaultPickingPlugins,
        bevy::ui::UiPlugin,
    ));
    // The assets those plugins' systems read and no plugin here provides.
    app.init_asset::<Image>()
        .init_asset::<bevy::image::TextureAtlasLayout>()
        .init_asset::<bevy::mesh::Mesh>()
        .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
    app.add_plugins((
        ShellPlugin,
        MovementPlugin,
        ControlsPlugin,
        ScreenPlugin,
        CombatPlugin,
        SettingsPlugin,
        FontsPlugin,
        EditorPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )))
    .add_systems(PostUpdate, target_info.before(bevy::ui::UiSystems::Prepare));
    let packs = Packs::load(&config).expect("the packs load");
    let mut editor =
        omnis_vector::editor::Editor::new(config.packs.clone()).expect("the editor opens");
    editor
        .open("test:map:town", &packs.data)
        .expect("the town opens");
    app.insert_resource(Editing(editor));
    app.insert_resource(packs);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(UI_PHYSICAL.0, UI_PHYSICAL.1)
                    .with_scale_factor_override(UI_SCALE),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    app.world_mut()
        .spawn((bevy::camera::Camera2d, bevy::ui::IsDefaultUiCamera));
    for _ in 0..4 {
        app.update();
    }
    (app, window)
}
