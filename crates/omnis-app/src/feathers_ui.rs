//! `FeathersUiPlugin`: the Feathers experiment (PRD D26, ARCHITECTURE.md A11 as amended). Bevy's
//! own widgets and TrueType text, drawn in window space above the canvas sprite, for one screen.
//! Everything here sits behind the `feathers` cargo feature, so a release build carries none of it.
//!
//! `DefaultPlugins` already brings `bevy_ui`, its widgets, input focus and picking once the
//! feature is on; this plugin adds Feathers itself, its dark theme, and the two things the
//! canvas pipeline would otherwise get wrong: which camera the interface belongs to, and the
//! global nearest sampler on Feathers' icons.

use crate::pixel::OuterCamera;
use bevy::feathers::FeathersPlugins;
use bevy::feathers::constants::icons;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::prelude::*;

/// The Feathers interface plugin.
pub struct FeathersUiPlugin;

impl Plugin for FeathersUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<SmoothIcons>()
            .add_systems(Startup, smooth_icons)
            .add_systems(Update, claim_camera);
    }
}

/// Feathers' icons, held so the first load (whose sampler wins) is never dropped.
#[derive(Resource, Default)]
pub struct SmoothIcons(pub Vec<Handle<Image>>);

/// Feathers draws its 24-pixel icons at 14: under the app's global nearest sampler they would
/// be jagged, so they are loaded first with a linear one.
fn smooth_icons(server: Res<AssetServer>, mut held: ResMut<SmoothIcons>) {
    for path in [icons::CHEVRON_DOWN, icons::CHEVRON_RIGHT, icons::X] {
        held.0.push(
            server
                .load_builder()
                .with_settings(|s: &mut ImageLoaderSettings| s.sampler = ImageSampler::linear())
                .load(path),
        );
    }
}

/// The interface belongs to the window's camera, never to the canvas's. `bevy_ui` already
/// prefers a window camera; the marker makes it explicit. A headless app has no outer camera.
fn claim_camera(
    mut commands: Commands,
    cameras: Query<Entity, (With<OuterCamera>, Without<IsDefaultUiCamera>)>,
) {
    for camera in &cameras {
        commands.entity(camera).insert(IsDefaultUiCamera);
    }
}
