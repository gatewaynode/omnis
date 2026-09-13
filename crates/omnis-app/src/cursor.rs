//! `CursorPlugin`: the window's logical size and the pointer as a canvas pixel. Both come from
//! window messages rather than the `Window` entity, so headless tests feed the same messages a
//! window would. The plugin also declares the UI system sets the other UI plugins slot into.

use crate::canvas::Layout;
use crate::layout::{CANVAS_HEIGHT, CANVAS_WIDTH, Fit, fit, physical_size, window_to_canvas};
use crate::sim::SimSet;
use bevy::input::ButtonState;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::prelude::*;
use bevy::window::{
    CursorLeft, CursorMoved, WindowCreated, WindowResized, WindowScaleFactorChanged,
};

/// The primary window's logical size and scale factor; the canvas fits its physical pixels.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WindowSize {
    /// Logical width.
    pub width: f32,
    /// Logical height.
    pub height: f32,
    /// Physical pixels per logical pixel.
    pub scale_factor: f32,
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize {
            width: CANVAS_WIDTH as f32,
            height: CANVAS_HEIGHT as f32,
            scale_factor: 1.0,
        }
    }
}

impl WindowSize {
    /// The window in physical pixels.
    #[must_use]
    pub fn physical(&self) -> (u32, u32) {
        physical_size((self.width, self.height), self.scale_factor)
    }

    /// How the canvas fits the window.
    #[must_use]
    pub fn fit(&self) -> Fit {
        fit(self.physical())
    }

    /// The canvas pixel under a logical window position.
    #[must_use]
    pub fn to_canvas(&self, logical: (f32, f32)) -> Option<(i32, i32)> {
        window_to_canvas(logical, (self.width, self.height), self.scale_factor)
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
            .add_message::<WindowScaleFactorChanged>()
            .add_message::<CursorMoved>()
            .add_message::<CursorLeft>()
            .add_message::<MouseButtonInput>()
            .init_resource::<WindowSize>()
            .init_resource::<Layout>()
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

/// The window's size from its creation, resizes, and scale factor changes; the backend
/// reports the scale factor on creation without a change message, so it is read from the
/// window then. The layout follows the size's fit, from the resource so a size set directly
/// counts too.
fn track_window(
    mut created: MessageReader<WindowCreated>,
    mut resized: MessageReader<WindowResized>,
    mut rescaled: MessageReader<WindowScaleFactorChanged>,
    windows: Query<&Window>,
    mut size: ResMut<WindowSize>,
    mut layout: ResMut<Layout>,
) {
    let mut wanted = *size;
    for event in created.read() {
        if let Ok(window) = windows.get(event.window) {
            wanted = WindowSize {
                width: window.width(),
                height: window.height(),
                scale_factor: window.scale_factor(),
            };
        }
    }
    for event in resized.read() {
        wanted.width = event.width;
        wanted.height = event.height;
    }
    for event in rescaled.read() {
        wanted.scale_factor = event.scale_factor as f32;
    }
    if *size != wanted {
        *size = wanted;
    }
    let fitted = Layout::for_width(size.fit().width);
    if *layout != fitted {
        *layout = fitted;
    }
}

/// The pointer from this frame's cursor and button messages; `ui::hit` runs after it.
pub fn track_pointer(
    mut moved: MessageReader<CursorMoved>,
    mut left: MessageReader<CursorLeft>,
    mut buttons: MessageReader<MouseButtonInput>,
    size: Res<WindowSize>,
    mut pointer: ResMut<Pointer>,
) {
    let mut next = *pointer;
    next.clicked = false;
    for event in moved.read() {
        next.canvas = size.to_canvas((event.position.x, event.position.y));
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
