//! The Bevy shell: wiring only. `MovementPlugin` runs headless; `RenderPlugin` and
//! `HudPlugin` need a window.

pub mod capture;
pub mod cinema;
pub mod combat;
pub mod controls;
pub mod fonts;
pub mod host;
pub mod hud;
pub mod minimap;
pub mod movement;
pub mod notice;
pub mod render;
pub mod roster;
pub mod screen;
mod screen_draw;
pub mod screen_text;
pub mod session;
pub mod settings;
#[cfg(feature = "devtools")]
pub mod socket;
pub mod text;

use bevy::prelude::*;

/// The frame's order: cursor grab, the screens (their presses and keys, then drawing them),
/// input, movement through the binder, drawing.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VectorSet {
    /// The cursor is grabbed or released.
    Grab,
    /// The top screen routes its input and is drawn (`shell/screen.rs`).
    Screens,
    /// Keys, mouse and buttons become intent.
    Input,
    /// Intent moves the pose through the binder.
    Move,
    /// The view and the HUD follow.
    Draw,
}

/// Which screen is up: the 3D view while exploring, the fight screen while monsters are met or
/// fought (`shell/combat.rs` sets it from the simulation's mode).
#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ViewState {
    /// The 3D view.
    #[default]
    Explore,
    /// The fight screen.
    Fight,
}

/// Orders the sets; every other plugin adds to them.
pub struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                VectorSet::Grab,
                VectorSet::Screens,
                VectorSet::Input,
                VectorSet::Move,
                VectorSet::Draw,
            )
                .chain(),
        )
        // The game moves only while one runs and no screen holds it; it is drawn while one runs.
        // The cursor is grabbed or released whatever holds the game, so a screen opened by a key
        // while the mouse looks still gets the pointer back.
        .configure_sets(
            Update,
            (VectorSet::Input, VectorSet::Move).run_if(screen::playing),
        )
        .configure_sets(
            Update,
            VectorSet::Draw.run_if(resource_exists::<session::Session>),
        );
    }
}
