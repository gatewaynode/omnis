//! Headless mouse tests (ARCHITECTURE.md §11, tier 6): the pointer and window messages a
//! window would send, without a window.

use bevy::input::ButtonState;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::window::{CursorLeft, CursorMoved, WindowCreated, WindowResized, WindowResolution};
use omnis_app::AppConfig;
use omnis_app::cursor::{CursorPlugin, Pointer, WindowSize};
use omnis_app::input::InputPlugin;
use omnis_app::menus::MenusPlugin;
use omnis_app::sim::SimPlugin;
use std::path::PathBuf;

fn ui_app() -> App {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(AppConfig {
            packs: vec![repo.join("packs/base"), repo.join("packs/test")],
            seed: 7,
            save_path: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("mouse-save.ron"),
            autostart: false,
        })
        .add_plugins((SimPlugin, InputPlugin, MenusPlugin, CursorPlugin));
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

#[test]
fn cursor_messages_map_through_the_letterbox_and_track_the_button() {
    let mut app = ui_app();
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
    let mut app = ui_app();
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
