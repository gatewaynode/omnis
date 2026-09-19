//! `PixelPlugin`: the fixed internal resolution pipeline (ARCHITECTURE.md §8.2, Bevy's
//! `pixel_grid_snap` example). An inner camera renders the canvas image; an outer camera shows
//! it as a sprite at the largest whole multiple that fits the window's physical pixels
//! (`cursor::WindowSize`, `layout::fit`), on a whole-pixel letterbox.

use crate::cursor::{UiSet, WindowSize};
use crate::layout::{CANVAS_HEIGHT, CANVAS_WIDTH, PANEL_COLOR, SizeClass, canvas_translation};
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat};
use bevy::window::WindowCreated;

/// Everything drawn at internal resolution.
pub const PIXEL_LAYER: RenderLayers = RenderLayers::layer(0);
/// The canvas sprite.
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

/// The window size class asked for on the command line; `None` for fullscreen.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestedWindow(pub Option<SizeClass>);

/// The pixel pipeline plugin.
pub struct PixelPlugin;

impl Plugin for PixelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindowSize>()
            .init_resource::<RequestedWindow>()
            .add_systems(Startup, setup)
            .add_systems(Update, size_window)
            .add_systems(
                Update,
                fit_canvas
                    .run_if(resource_changed::<WindowSize>)
                    .after(UiSet::Cursor),
            );
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
    commands.spawn((Camera2d, Msaa::Off, OuterCamera, WINDOW_LAYER));
}

/// The outer camera's projection scale: logical pixels per world unit is the physical
/// multiple over the scale factor, so a canvas pixel is exactly `fit.scale` physical pixels.
#[must_use]
pub fn outer_scale(size: &WindowSize) -> f32 {
    size.scale_factor / size.fit().scale as f32
}

/// A window opened for a size class is given that physical size once it exists: the request
/// was in logical pixels, which a HiDPI backend scales up. A window with a scale factor
/// override is left as requested.
fn size_window(
    mut created: MessageReader<WindowCreated>,
    mut windows: Query<&mut Window>,
    requested: Res<RequestedWindow>,
) {
    let Some(class) = requested.0 else {
        created.clear();
        return;
    };
    for event in created.read() {
        if let Ok(mut window) = windows.get_mut(event.window)
            && window.resolution.scale_factor_override().is_none()
            && window.resolution.scale_factor() != 1.0
        {
            let (width, height) = class.physical();
            window.resolution.set_physical_resolution(width, height);
        }
    }
}

/// Fit the canvas whenever the tracked window size changes, including the first frame, so a
/// window that opens at the requested size (winit sends no resize for it) is scaled too. The
/// canvas image takes the fit's width; the sprite follows its image.
fn fit_canvas(
    size: Res<WindowSize>,
    target: Option<Res<CanvasImage>>,
    mut images: ResMut<Assets<Image>>,
    mut projection: Single<&mut Projection, With<OuterCamera>>,
    mut canvas: Single<&mut Transform, With<Canvas>>,
) {
    let Projection::Orthographic(projection) = &mut **projection else {
        return;
    };
    let fit = size.fit();
    if let Some(target) = target
        && images
            .get(&target.0)
            .is_some_and(|i| i.width() != fit.width)
        && let Some(mut image) = images.get_mut(&target.0)
    {
        image.resize(Extent3d {
            width: fit.width,
            height: CANVAS_HEIGHT,
            depth_or_array_layers: 1,
        });
    }
    projection.scale = outer_scale(&size);
    let (x, y) = canvas_translation(fit, size.physical());
    canvas.translation = Vec3::new(x, y, 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outer_camera_scales_to_whole_physical_pixels() {
        let size = |width, height, scale_factor| WindowSize {
            width,
            height,
            scale_factor,
        };
        let (w, h) = (CANVAS_WIDTH as f32, CANVAS_HEIGHT as f32);
        assert_eq!(outer_scale(&size(w, h, 1.0)), 1.0);
        assert_eq!(outer_scale(&size(3.0 * w, 3.0 * h, 1.0)), 1.0 / 3.0);
        assert_eq!(outer_scale(&size(2.0 * w + 100.0, 2.0 * h, 1.0)), 0.5);
        // A 4K panel at 2x logical: three physical pixels a canvas pixel, 1.5 logical.
        assert_eq!(outer_scale(&size(1.5 * w, 1.5 * h, 2.0)), 2.0 / 3.0);
        assert_eq!(
            outer_scale(&size(w / 2.0, h / 2.0, 1.0)),
            1.0,
            "never below one"
        );
        assert_eq!(
            RequestedWindow::default(),
            RequestedWindow(None),
            "fullscreen"
        );
    }
}
