//! The editor's pointer through Bevy's own UI (M5.5, after the owner's report that the mouse did
//! nothing in the window): layout, the UI stack and the focus system fill the plan's
//! `RelativeCursorPosition` from a real `Window` cursor, and a click lands on the cell under it.
//! The other editor tests write that component themselves; this one checks the convention they
//! write is the one Bevy fills (`normalized` runs from -0.5 to 0.5, not 0 to 1).

use super::common::app::config;
use bevy::camera::{Camera, Camera2d, RenderTargetInfo};
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, IsDefaultUiCamera, UiGlobalTransform, UiSystems};
use bevy::window::{ExitCondition, PrimaryWindow, WindowPlugin, WindowResolution};
use omnis_vector::editor::EditAct;
use omnis_vector::shell::ShellPlugin;
use omnis_vector::shell::combat::CombatPlugin;
use omnis_vector::shell::controls::ControlsPlugin;
use omnis_vector::shell::editor::{Column, Editing, EditorPlugin};
use omnis_vector::shell::fonts::FontsPlugin;
use omnis_vector::shell::movement::MovementPlugin;
use omnis_vector::shell::screen::ScreenPlugin;
use omnis_vector::shell::session::{Config, Packs};
use omnis_vector::shell::settings::SettingsPlugin;
use std::time::Duration;

/// The window's physical size and scale: a Retina-like window, so logical and physical pixels
/// differ.
const PHYSICAL: (u32, u32) = (2400, 1400);
const SCALE: f32 = 2.0;

/// Without a renderer no system fills the camera's target: this stands in for it, so the UI
/// lays out to the window.
fn target_info(windows: Query<&Window, With<PrimaryWindow>>, mut cameras: Query<&mut Camera>) {
    let Ok(window) = windows.single() else {
        return;
    };
    for mut camera in &mut cameras {
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: window.physical_size(),
            scale_factor: window.scale_factor(),
        });
    }
}

/// The shell in the editor with the town open, under Bevy's UI layout and focus (no renderer),
/// with a window and a camera; the window's entity.
fn ui_app() -> (App, Entity) {
    let config = Config {
        editor: Some(Some("test:map:town".to_owned())),
        ..config()
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        InputPlugin,
        StatesPlugin,
        WindowPlugin {
            primary_window: None,
            primary_cursor_options: None,
            exit_condition: ExitCondition::DontExit,
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
    .add_systems(PostUpdate, target_info.before(UiSystems::Prepare));
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
                resolution: WindowResolution::new(PHYSICAL.0, PHYSICAL.1)
                    .with_scale_factor_override(SCALE),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    app.world_mut().spawn((Camera2d, IsDefaultUiCamera));
    for _ in 0..4 {
        app.update();
    }
    (app, window)
}

/// The window's logical cursor position over `cell`'s middle, from the plan's laid-out
/// rectangle and the model's view.
fn over(app: &mut App, cell: (u16, u16), dx: f32, dy: f32) -> Vec2 {
    let world = app.world_mut();
    let mut panes = world.query::<(&Column, &ComputedNode, &UiGlobalTransform)>();
    let (size, inverse, centre) = panes
        .iter(world)
        .find(|(c, _, _)| **c == Column::Plan)
        .map(|(_, node, tf)| (node.size(), node.inverse_scale_factor(), tf.translation))
        .expect("the plan is laid out");
    assert!(
        size.x > 0.0 && size.y > 0.0,
        "the plan has a size: {size:?}"
    );
    let top_left = centre - size / 2.0;
    let editing = world.resource::<Editing>();
    let (left, top, side) = editing.view.cell_rect(cell);
    #[allow(clippy::cast_precision_loss)]
    let in_pane = Vec2::new(
        left as f32 + side as f32 * dx,
        top as f32 + side as f32 * dy,
    );
    // Physical pixels, then the window's logical ones.
    (top_left + in_pane / inverse) * inverse
}

fn click(app: &mut App, window: Entity, button: MouseButton, at: Vec2) {
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("the window")
        .set_cursor_position(Some(at));
    app.update();
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button,
            state,
            window,
        });
        app.update();
    }
}

#[test]
fn a_click_in_the_window_lands_on_the_cell_under_the_pointer() {
    let (mut app, window) = ui_app();
    let pane = app.world().resource::<Editing>().pane;
    assert!(
        pane.0 > 100 && pane.1 > 100,
        "the model took the laid-out pane: {pane:?}"
    );
    assert_eq!(
        app.world().resource::<Editing>().cursor,
        (10, 2),
        "the start"
    );
    let middle = over(&mut app, (5, 3), 0.5, 0.5);
    click(&mut app, window, MouseButton::Left, middle);
    let editing = app.world().resource::<Editing>();
    assert_eq!(editing.cursor, (5, 3), "{:?}", editing.message);
    // A right click picks: the brush becomes what is there and the cursor moves.
    let elsewhere = over(&mut app, (7, 1), 0.5, 0.5);
    click(&mut app, window, MouseButton::Right, elsewhere);
    assert_eq!(app.world().resource::<Editing>().cursor, (7, 1));
    // Just inside the eastern border of a cell, a Wall paints that edge.
    let data = app.world().resource::<Packs>().data.clone();
    let brushes = {
        let def = app.world().resource::<Editing>().def().expect("open");
        omnis_vector::editor::brush::palette(&def)
    };
    let wall = brushes
        .iter()
        .position(|t| {
            *t == omnis_vector::editor::brush::Tool::Edge(
                omnis_sim::omnis_data::edit::EdgeKind::Wall,
            )
        })
        .expect("a Wall brush");
    app.world_mut().resource_mut::<Editing>().apply(
        &data,
        EditAct::Brush(u8::try_from(wall).expect("small")),
        0,
    );
    let east = over(&mut app, (6, 3), 0.95, 0.5);
    click(&mut app, window, MouseButton::Left, east);
    let editing = app.world().resource::<Editing>();
    assert_eq!(editing.cursor, (6, 3), "{:?}", editing.message);
    let grid = editing.grid().expect("the grid");
    assert_eq!(
        grid.edge(6, 3, omnis_sim::omnis_core::Facing::East),
        Some(omnis_sim::omnis_data::edit::EdgeKind::Wall)
    );
}
