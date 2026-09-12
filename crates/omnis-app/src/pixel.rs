//! `PixelPlugin`: the fixed internal resolution pipeline (ARCHITECTURE.md §8.2, Bevy's
//! `pixel_grid_snap` example). An inner camera renders the canvas image; an outer camera shows
//! it as a sprite at the largest integer scale that fits the window (`cursor::WindowSize`).

use crate::cursor::WindowSize;
use crate::layout::{CANVAS_HEIGHT, CANVAS_WIDTH, PANEL_COLOR};
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

/// Everything drawn at internal resolution.
pub const PIXEL_LAYER: RenderLayers = RenderLayers::layer(0);
/// The canvas sprite and native-resolution UI.
pub const WINDOW_LAYER: RenderLayers = RenderLayers::layer(1);

/// The camera that renders the canvas.
#[derive(Component)]
pub struct InnerCamera;

/// The camera that shows the canvas in the window.
#[derive(Component)]
pub struct OuterCamera;

/// The canvas sprite.
#[derive(Component)]
pub struct Canvas;

/// The canvas image, for screenshots of the internal resolution.
#[derive(Resource, Debug, Clone)]
pub struct CanvasImage(pub Handle<Image>);

/// The pixel pipeline plugin.
pub struct PixelPlugin;

impl Plugin for PixelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindowSize>()
            .add_systems(Startup, setup)
            .add_systems(Update, fit_canvas.run_if(resource_changed::<WindowSize>));
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let canvas = Image::new_target_texture(
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        TextureFormat::Bgra8UnormSrgb,
        None,
    );
    let handle = images.add(canvas);
    commands.insert_resource(CanvasImage(handle.clone()));
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::srgb_u8(
                PANEL_COLOR.0,
                PANEL_COLOR.1,
                PANEL_COLOR.2,
            )),
            ..default()
        },
        RenderTarget::Image(handle.clone().into()),
        Msaa::Off,
        InnerCamera,
        PIXEL_LAYER,
    ));
    commands.spawn((Sprite::from_image(handle), Canvas, WINDOW_LAYER));
    commands.spawn((
        Camera2d,
        Msaa::Off,
        OuterCamera,
        IsDefaultUiCamera,
        WINDOW_LAYER,
    ));
}

/// Integer scale: the reciprocal of the rounded smaller window-to-canvas ratio.
#[must_use]
pub fn integer_scale(window_width: f32, window_height: f32) -> f32 {
    1.0 / crate::layout::window_scale(window_width, window_height)
}

/// Fit the canvas whenever the tracked window size changes, including the first frame, so a
/// window that opens at the requested size (winit sends no resize for it) is scaled too.
fn fit_canvas(size: Res<WindowSize>, mut projection: Single<&mut Projection, With<OuterCamera>>) {
    let Projection::Orthographic(projection) = &mut **projection else {
        return;
    };
    projection.scale = integer_scale(size.0, size.1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_an_integer_fit() {
        assert_eq!(integer_scale(1280.0, 720.0), 0.25);
        assert_eq!(integer_scale(1920.0, 1080.0), 1.0 / 6.0);
        assert_eq!(integer_scale(2560.0, 1440.0), 0.125);
        assert_eq!(integer_scale(3840.0, 2160.0), 1.0 / 12.0);
        assert_eq!(integer_scale(200.0, 100.0), 1.0, "never below one");
    }
}
