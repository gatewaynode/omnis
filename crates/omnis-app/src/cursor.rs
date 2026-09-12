//! `CursorPlugin`: the window's logical size and the pointer as a canvas pixel. Both come from
//! window messages rather than the `Window` entity, so headless tests feed the same messages a
//! window would. The plugin also declares the UI system sets the other UI plugins slot into.

use crate::layout::window_to_canvas;
use crate::sim::SimSet;
use bevy::input::ButtonState;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::prelude::*;
use bevy::window::{CursorLeft, CursorMoved, WindowCreated, WindowResized};

/// The primary window's logical size.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WindowSize(pub f32, pub f32);

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize(1280.0, 720.0)
    }
}

/// The pointer as the UI sees it.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pointer {
    /// The canvas pixel under the pointer, if it is over the canvas.
    pub canvas: Option<(i32, i32)>,
    /// Whether the left button is down.
    pub held: bool,
    /// Whether the left button went down this frame.
    pub clicked: bool,
}

/// Where the UI systems run: pointer tracking, then click dispatch, inside
/// `SimSet::Collect`; the models, then the frame, inside `SimSet::Publish`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiSet {
    /// Window size and pointer.
    Cursor,
    /// Clicks and keys into commands and menu actions.
    Dispatch,
    /// The text models and message line.
    Model,
    /// The frame.
    Draw,
}

/// The cursor plugin.
pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WindowCreated>()
            .add_message::<WindowResized>()
            .add_message::<CursorMoved>()
            .add_message::<CursorLeft>()
            .add_message::<MouseButtonInput>()
            .init_resource::<WindowSize>()
            .init_resource::<Pointer>()
            .configure_sets(
                Update,
                (UiSet::Cursor, UiSet::Dispatch)
                    .chain()
                    .in_set(SimSet::Collect),
            )
            .configure_sets(
                Update,
                (UiSet::Model, UiSet::Draw).chain().in_set(SimSet::Publish),
            )
            .add_systems(
                Update,
                (track_window, track_pointer).chain().in_set(UiSet::Cursor),
            );
    }
}

fn track_window(
    mut created: MessageReader<WindowCreated>,
    mut resized: MessageReader<WindowResized>,
    windows: Query<&Window>,
    mut size: ResMut<WindowSize>,
) {
    let mut wanted = None;
    for event in created.read() {
        if let Ok(window) = windows.get(event.window) {
            wanted = Some(WindowSize(window.width(), window.height()));
        }
    }
    for event in resized.read() {
        wanted = Some(WindowSize(event.width, event.height));
    }
    if let Some(wanted) = wanted
        && *size != wanted
    {
        *size = wanted;
    }
}

fn track_pointer(
    mut moved: MessageReader<CursorMoved>,
    mut left: MessageReader<CursorLeft>,
    mut buttons: MessageReader<MouseButtonInput>,
    size: Res<WindowSize>,
    mut pointer: ResMut<Pointer>,
) {
    let mut next = *pointer;
    next.clicked = false;
    for event in moved.read() {
        next.canvas = window_to_canvas((event.position.x, event.position.y), size.0, size.1);
    }
    if left.read().next().is_some() {
        next.canvas = None;
    }
    for event in buttons.read() {
        if event.button != MouseButton::Left {
            continue;
        }
        match event.state {
            ButtonState::Pressed => {
                next.held = true;
                next.clicked = true;
            }
            ButtonState::Released => next.held = false,
        }
    }
    if *pointer != next {
        *pointer = next;
    }
}
