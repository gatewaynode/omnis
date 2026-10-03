//! The Bevy shell: wiring only. `MovementPlugin` runs headless; `RenderPlugin` and
//! `HudPlugin` need a window.

pub mod actions;
pub mod capture;
pub mod cinema;
pub mod controls;
pub mod fight;
pub mod hud;
pub mod minimap;
pub mod movement;
pub mod notice;
pub mod panel;
pub mod render;
pub mod session;
pub mod text;

use bevy::prelude::*;

/// The frame's order: cursor grab, input, movement through the binder, drawing.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VectorSet {
    /// The cursor is grabbed or released.
    Grab,
    /// Keys, mouse and buttons become intent.
    Input,
    /// Intent moves the pose through the binder.
    Move,
    /// The view and the HUD follow.
    Draw,
}

/// Orders the sets; every other plugin adds to them.
pub struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                VectorSet::Grab,
                VectorSet::Input,
                VectorSet::Move,
                VectorSet::Draw,
            )
                .chain(),
        );
    }
}
