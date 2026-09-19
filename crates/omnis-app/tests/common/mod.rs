//! Shared setup for the app's headless tests: the plugin stack under `MinimalPlugins`, the
//! pointer and key messages a window would send, clicks on the composed frame's widgets, and
//! the menu flows by mouse. Real packs, no mocks.
#![allow(dead_code)]

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::window::CursorMoved;
use omnis_app::AppConfig;
use omnis_app::combat::CombatPlugin;
use omnis_app::cursor::{CursorPlugin, Pointer};
use omnis_app::input::InputPlugin;
use omnis_app::menu::{ROW_ADD, ROW_CLASS, ROW_RACE, ROW_SCORES};
use omnis_app::menus::{MenusPlugin, Screens};
use omnis_app::sim::{
    AppState, MenuState, PlayState, ShellCommand, SimEvent, SimPlugin, SimSet, SimWorld,
    WorldReplaced,
};
use omnis_app::ui::{UiFrame, UiPlugin};
use omnis_app::widget::{Part, Widget, WidgetId};
use omnis_sim::omnis_core::{Facing, MapId, Position};
use omnis_sim::{Event, SaveRule, World};
use std::path::PathBuf;

/// What presentation saw: every simulation event and how many `WorldReplaced` messages,
/// collected each frame after the simulation publishes (a reader that runs every frame
/// never misses a message, however many frames a round trip takes).
#[derive(Resource, Default)]
pub struct Seen {
    /// Every event, in order.
    pub events: Vec<Event>,
    /// How many times the world was replaced.
    pub replaced: usize,
    /// Every shell command, in order.
    pub shell: Vec<ShellCommand>,
}

fn collect(
    mut events: MessageReader<SimEvent>,
    mut replaced: MessageReader<WorldReplaced>,
    mut shell: MessageReader<ShellCommand>,
    mut seen: ResMut<Seen>,
) {
    seen.events.extend(events.read().map(|e| e.0.clone()));
    seen.replaced += replaced.read().count();
    seen.shell.extend(shell.read().copied());
}

/// The app under `MinimalPlugins` with every headless plugin, on the base and test packs.
pub fn ui_app(autostart: bool) -> App {
    ui_app_saving_to("pointer-save.ron", autostart)
}

/// The same, with the quick save under the given name in the target's temporary directory.
pub fn ui_app_saving_to(save: &str, autostart: bool) -> App {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(AppConfig {
            packs: vec![repo.join("packs/base"), repo.join("packs/test")],
            seed: 7,
            save_path: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(save),
            autostart,
        })
        .add_plugins((
            SimPlugin,
            InputPlugin,
            MenusPlugin,
            CombatPlugin,
            CursorPlugin,
            UiPlugin,
        ))
        .init_resource::<Seen>()
        .add_systems(Update, collect.after(SimSet::Publish));
    app
}

/// What presentation saw so far.
pub fn seen(app: &App) -> &Seen {
    app.world().resource::<Seen>()
}

pub fn move_to(app: &mut App, x: f32, y: f32) {
    app.world_mut()
        .resource_mut::<Messages<CursorMoved>>()
        .write(CursorMoved {
            window: Entity::PLACEHOLDER,
            position: Vec2::new(x, y),
            delta: None,
        });
}

pub fn button(app: &mut App, state: ButtonState) {
    app.world_mut()
        .resource_mut::<Messages<MouseButtonInput>>()
        .write(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window: Entity::PLACEHOLDER,
        });
}

pub fn pointer(app: &App) -> Pointer {
    *app.world().resource::<Pointer>()
}

pub fn frame(app: &App) -> &UiFrame {
    app.world().resource::<UiFrame>()
}

pub fn widget(app: &App, id: WidgetId) -> Widget {
    *frame(app)
        .frame
        .widget(id)
        .unwrap_or_else(|| panic!("{id:?} is not on the screen"))
}

/// Put the pointer on a canvas pixel directly, as a window's `CursorMoved` would.
pub fn point_at(app: &mut App, at: (i32, i32)) {
    app.world_mut().resource_mut::<Pointer>().canvas = Some(at);
}

/// A canvas pixel inside the widget's part.
pub fn spot(w: &Widget, part: Part) -> (i32, i32) {
    let r = match part {
        Part::Body => w.rect,
        Part::Left => w.left.expect("a left arrow"),
        Part::Right => w.right.expect("a right arrow"),
    };
    (r.x + 2, r.y + 3)
}

/// Click a widget of the current frame: press, a frame to dispatch, a frame to apply, release.
pub fn click(app: &mut App, id: WidgetId, part: Part) {
    let w = widget(app, id);
    click_at(app, spot(&w, part));
}

pub fn click_at(app: &mut App, at: (i32, i32)) {
    point_at(app, at);
    button(app, ButtonState::Pressed);
    app.update();
    app.update();
    button(app, ButtonState::Released);
    app.update();
}

/// A logical key press, as the menus and the fight screens read them.
pub fn key(app: &mut App, logical: Key) {
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

pub fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        key(app, Key::Character(c.to_string().into()));
    }
}

/// Escape through the key table. `reset_all`, not `clear`: `clear` keeps the key pressed,
/// so a second press would not count as one.
pub fn escape(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
}

pub fn play_state(app: &App) -> PlayState {
    *app.world().resource::<State<PlayState>>().get()
}

pub fn world(app: &App) -> &World {
    &app.world().resource::<SimWorld>().0
}

/// Put the party on a tile of a map directly, as a test of what follows a step needs.
pub fn place(app: &mut App, map: &str, x: u16, y: u16, facing: Facing) {
    let id: MapId = {
        let data = &app.world().resource::<omnis_app::sim::PackData>().0;
        data.registry
            .maps
            .get(map)
            .unwrap_or_else(|| panic!("{map} is not a map"))
    };
    app.world_mut().resource_mut::<SimWorld>().0.position = Position {
        map: id,
        x,
        y,
        facing,
    };
}

/// Title, new game by mouse: seed 42 typed, the save rule clicked to Relief, Start.
pub fn start_new_game_by_mouse(app: &mut App) {
    app.update();
    app.update();
    click(app, WidgetId::Row(0), Part::Body);
    assert_eq!(
        *app.world().resource::<State<MenuState>>().get(),
        MenuState::NewGame
    );
    click(app, WidgetId::Row(0), Part::Body);
    type_text(app, "42");
    click(app, WidgetId::Row(1), Part::Right);
    assert_eq!(
        app.world()
            .resource::<Screens>()
            .new_game
            .settings
            .save_rule,
        SaveRule::Relief
    );
    click(app, WidgetId::Row(3), Part::Body);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Playing
    );
    assert_eq!(play_state(app), PlayState::CreateParty);
}

/// The same human fighter the key test builds, by clicks: STR 15, DEX 14, CON 13, INT 12,
/// WIS 10, CHA 8, Athletics and Perception.
pub fn draft_fighter_by_mouse(app: &mut App) {
    click(app, WidgetId::Row(0), Part::Body);
    type_text(app, "Brenna");
    click(app, WidgetId::Row(ROW_RACE), Part::Left);
    click(app, WidgetId::Row(ROW_CLASS), Part::Right);
    for (k, raise) in [7, 6, 5, 4, 2, 0].into_iter().enumerate() {
        for _ in 0..raise {
            click(app, WidgetId::Row(ROW_SCORES + k), Part::Right);
        }
    }
    click(app, WidgetId::Skill(2), Part::Body);
    click(app, WidgetId::Skill(6), Part::Body);
    click(app, WidgetId::Row(ROW_ADD), Part::Body);
}
