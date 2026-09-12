//! Headless mouse tests (ARCHITECTURE.md §11, tier 6): the pointer and window messages a
//! window would send, without a window, and the whole menu flow driven by clicks on the
//! composed frame's widgets.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::window::{CursorLeft, CursorMoved, WindowCreated, WindowResized, WindowResolution};
use omnis_app::AppConfig;
use omnis_app::cursor::{CursorPlugin, Pointer, WindowSize};
use omnis_app::input::InputPlugin;
use omnis_app::menu::{ROW_ADD, ROW_BEGIN, ROW_CLASS, ROW_RACE, ROW_SCORES};
use omnis_app::menus::{MenusPlugin, Screens};
use omnis_app::screen::{ALERT, PadButton, Part, Widget, WidgetId};
use omnis_app::sim::{
    AppState, MenuState, PlayState, ShellCommand, SimPlugin, SimWorld, WorldReplaced,
};
use omnis_app::ui::{MessageLine, Selected, UiFrame, UiPlugin};
use omnis_sim::SaveRule;
use omnis_sim::omnis_core::Facing;
use std::path::PathBuf;

/// How many `WorldReplaced` messages presentation saw.
#[derive(Resource, Default)]
struct Replaced(usize);

fn ui_app(autostart: bool) -> App {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(AppConfig {
            packs: vec![repo.join("packs/base"), repo.join("packs/test")],
            seed: 7,
            save_path: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("mouse-save.ron"),
            autostart,
        })
        .add_plugins((SimPlugin, InputPlugin, MenusPlugin, CursorPlugin, UiPlugin))
        .init_resource::<Replaced>()
        .add_systems(
            Update,
            |mut replaced: MessageReader<WorldReplaced>, mut count: ResMut<Replaced>| {
                count.0 += replaced.read().count();
            },
        );
    app
}

fn move_to(app: &mut App, x: f32, y: f32) {
    app.world_mut()
        .resource_mut::<Messages<CursorMoved>>()
        .write(CursorMoved {
            window: Entity::PLACEHOLDER,
            position: Vec2::new(x, y),
            delta: None,
        });
}

fn button(app: &mut App, state: ButtonState) {
    app.world_mut()
        .resource_mut::<Messages<MouseButtonInput>>()
        .write(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window: Entity::PLACEHOLDER,
        });
}

fn pointer(app: &App) -> Pointer {
    *app.world().resource::<Pointer>()
}

fn frame(app: &App) -> &UiFrame {
    app.world().resource::<UiFrame>()
}

fn widget(app: &App, id: WidgetId) -> Widget {
    *frame(app)
        .frame
        .widget(id)
        .unwrap_or_else(|| panic!("{id:?} is not on the screen"))
}

/// Put the pointer on a canvas pixel directly, as a window's `CursorMoved` would.
fn point_at(app: &mut App, at: (i32, i32)) {
    app.world_mut().resource_mut::<Pointer>().canvas = Some(at);
}

/// A canvas pixel inside the widget's part.
fn spot(w: &Widget, part: Part) -> (i32, i32) {
    let r = match part {
        Part::Body => w.rect,
        Part::Left => w.left.expect("a left arrow"),
        Part::Right => w.right.expect("a right arrow"),
    };
    (r.x + 2, r.y + 3)
}

/// Click a widget of the current frame: press, a frame to dispatch, a frame to apply, release.
fn click(app: &mut App, id: WidgetId, part: Part) {
    let w = widget(app, id);
    click_at(app, spot(&w, part));
}

fn click_at(app: &mut App, at: (i32, i32)) {
    point_at(app, at);
    button(app, ButtonState::Pressed);
    app.update();
    app.update();
    button(app, ButtonState::Released);
    app.update();
}

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

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        key(app, Key::Character(c.to_string().into()));
    }
}

/// Escape through the key table. `reset_all`, not `clear`: `clear` keeps the key pressed,
/// so a second press would not count as one.
fn escape(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
}

fn play_state(app: &App) -> PlayState {
    *app.world().resource::<State<PlayState>>().get()
}

fn world(app: &App) -> &omnis_sim::World {
    &app.world().resource::<SimWorld>().0
}

#[test]
fn cursor_messages_map_through_the_letterbox_and_track_the_button() {
    let mut app = ui_app(false);
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        WindowSize(1280.0, 720.0)
    );
    move_to(&mut app, 1000.0, 400.0);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((250, 100)));

    // A 1920x1200 window letterboxes 60 px above and below the canvas.
    app.insert_resource(WindowSize(1920.0, 1200.0));
    move_to(&mut app, 5.0, 5.0);
    app.update();
    assert_eq!(pointer(&app).canvas, None);
    move_to(&mut app, 600.0, 660.0);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((100, 100)));

    button(&mut app, ButtonState::Pressed);
    app.update();
    assert_eq!(
        pointer(&app),
        Pointer {
            canvas: Some((100, 100)),
            held: true,
            clicked: true
        }
    );
    app.update();
    assert!(
        pointer(&app).held && !pointer(&app).clicked,
        "a click lasts one frame"
    );
    button(&mut app, ButtonState::Released);
    app.update();
    assert!(!pointer(&app).held);

    app.world_mut()
        .resource_mut::<Messages<CursorLeft>>()
        .write(CursorLeft {
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert_eq!(
        pointer(&app).canvas,
        None,
        "leaving the window clears the pointer"
    );
}

#[test]
fn the_window_size_follows_creation_and_resizes() {
    let mut app = ui_app(false);
    app.update();
    let window = app
        .world_mut()
        .spawn(Window {
            resolution: WindowResolution::new(1000, 600),
            ..default()
        })
        .id();
    app.world_mut()
        .resource_mut::<Messages<WindowCreated>>()
        .write(WindowCreated { window });
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        WindowSize(1000.0, 600.0)
    );
    app.world_mut()
        .resource_mut::<Messages<WindowResized>>()
        .write(WindowResized {
            window,
            width: 640.0,
            height: 360.0,
        });
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        WindowSize(640.0, 360.0)
    );
    // The pointer maps through the new size: 640x360 shows the canvas at 2x.
    move_to(&mut app, 100.0, 50.0);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((50, 25)));
}

/// Title, new game by mouse: seed 42 typed, the save rule clicked to Relief, Start.
fn start_new_game_by_mouse(app: &mut App) {
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
fn draft_fighter_by_mouse(app: &mut App) {
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

#[test]
fn the_mouse_starts_a_game_and_builds_a_party() {
    let mut app = ui_app(false);
    app.update();
    app.update();
    let w = widget(&app, WidgetId::Row(1));
    point_at(&mut app, spot(&w, Part::Body));
    app.update();
    assert_eq!(frame(&app).hover, Some(WidgetId::Row(1)));

    start_new_game_by_mouse(&mut app);
    assert_eq!(world(&app).seed, 42);
    assert_eq!(world(&app).settings.save_rule, SaveRule::Relief);
    assert_eq!(
        app.world().resource::<Replaced>().0,
        1,
        "a new game redraws the world before its first step"
    );

    draft_fighter_by_mouse(&mut app);
    let members = &world(&app).party.members;
    assert_eq!(members.len(), 1, "the draft became a member");
    assert_eq!(members[0].name, "Brenna");
    assert_eq!(members[0].hp_max, 12);
    assert_eq!(members[0].scores, [16, 15, 14, 13, 11, 9]);
    assert!(app.world().resource::<Screens>().creation.name.is_empty());
    assert!(
        frame(&app).frame.widget(WidgetId::Member(0)).is_some(),
        "the band shows the member"
    );

    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert!(frame(&app).frame.widget(WidgetId::Row(0)).is_none());
}

#[test]
fn pad_clicks_step_and_turn_the_party_while_exploring() {
    let mut app = ui_app(true);
    app.update();
    app.update();
    let start = world(&app).position;
    assert_eq!((start.x, start.y, start.facing), (16, 16, Facing::North));
    click(&mut app, WidgetId::Pad(PadButton::Forward), Part::Body);
    assert_eq!(world(&app).position.y, 15);
    click(&mut app, WidgetId::Pad(PadButton::TurnLeft), Part::Body);
    assert_eq!(world(&app).position.facing, Facing::West);

    // The gap between buttons hits nothing.
    let forward = widget(&app, WidgetId::Pad(PadButton::Forward));
    click_at(&mut app, (forward.rect.x - 1, forward.rect.y));
    assert_eq!(
        (world(&app).position.y, world(&app).position.facing),
        (15, Facing::West)
    );

    // Held shows pressed; released clears it.
    point_at(&mut app, spot(&forward, Part::Body));
    button(&mut app, ButtonState::Pressed);
    app.update();
    assert_eq!(frame(&app).pressed, Some(WidgetId::Pad(PadButton::Forward)));
    button(&mut app, ButtonState::Released);
    app.update();
    assert_eq!(frame(&app).pressed, None);
    let before = world(&app).position;

    // Paused: the pad is drawn dim and inert.
    escape(&mut app);
    assert_eq!(play_state(&app), PlayState::Paused);
    assert!(!widget(&app, WidgetId::Pad(PadButton::Forward)).enabled);
    click_at(&mut app, spot(&forward, Part::Body));
    assert_eq!(world(&app).position, before);
    assert_eq!(frame(&app).hover, None);
}

#[test]
fn party_rows_select_and_the_pause_menu_works_by_mouse() {
    let mut app = ui_app(false);
    start_new_game_by_mouse(&mut app);
    draft_fighter_by_mouse(&mut app);
    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);

    click(&mut app, WidgetId::Member(0), Part::Body);
    assert_eq!(*app.world().resource::<Selected>(), Selected(Some(0)));
    click(&mut app, WidgetId::Member(0), Part::Body);
    assert_eq!(*app.world().resource::<Selected>(), Selected(None));
    assert!(frame(&app).frame.widget(WidgetId::Member(1)).is_none());

    escape(&mut app);
    assert_eq!(play_state(&app), PlayState::Paused);
    click(&mut app, WidgetId::Row(0), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    escape(&mut app);
    click(&mut app, WidgetId::Row(1), Part::Body);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
    assert!(app.world().get_resource::<SimWorld>().is_none());
    assert!(
        frame(&app)
            .frame
            .widget(WidgetId::Pad(PadButton::Use))
            .is_none()
    );
}

#[test]
fn the_message_line_shows_rejections_and_notices() {
    let mut app = ui_app(false);
    start_new_game_by_mouse(&mut app);
    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);
    assert_eq!(play_state(&app), PlayState::CreateParty);
    assert_eq!(
        app.world().resource::<Screens>().creation.message,
        "Add at least one member"
    );
    let raster = &frame(&app).frame.raster;
    let alert = (0..320).any(|x| {
        raster
            .get(x, omnis_app::layout::BAND_MESSAGE.1 + 3)
            .is_some_and(|p| (p[0], p[1], p[2]) == ALERT)
    });
    assert!(alert, "the rejection is painted in the alert colour");

    draft_fighter_by_mouse(&mut app);
    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);
    app.world_mut()
        .resource_mut::<Messages<ShellCommand>>()
        .write(ShellCommand::Save);
    app.update();
    app.update();
    let line = app.world().resource::<MessageLine>();
    assert!(line.0.text.contains("save rule"), "{}", line.0.text);
    assert!(line.0.alert);
}
