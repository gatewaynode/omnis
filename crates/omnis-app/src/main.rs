//! `omnis`: the Bevy application. M0 opens a window with nearest-neighbour sampling; the pixel
//! pipeline, viewport, and input arrive in M1.
#![forbid(unsafe_code)]

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Omnis".into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .run();
}
