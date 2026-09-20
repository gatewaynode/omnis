//! `CapturePlugin` (feature `feathers`, which implies `devtools`): a capture of everything the
//! window shows, the canvas and any window-space interface above it, without reading the window. A window capture comes
//! back black for a process that no desktop session is showing (agents, CI), and the canvas
//! capture cannot see `bevy_ui`; so for a few frames a second camera draws the canvas sprite
//! into an image of the window's physical size, the interface is pointed at that camera, and
//! the image is what gets saved. The interface blinks out of the window for those frames.
//!
//! Known limit: an image target has a scale factor of one, so on a HiDPI window the interface
//! lays out at half the window's size in the capture. The owner's displays report one.

use crate::cursor::WindowSize;
use crate::pixel::WINDOW_LAYER;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::path::PathBuf;

/// Frames between aiming the interface at the capture camera and reading the image: one for
/// the layout to move, one for the render world to see it, one to spare.
const SETTLE_FRAMES: u8 = 3;

/// Ask for a composed capture at this path (a `.png`).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ComposeCapture(pub PathBuf);

/// A composed capture finished.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ComposedSaved {
    /// Where it was to be written.
    pub path: PathBuf,
    /// Why it was not, if it was not.
    pub error: Option<String>,
}

/// The capture under way, if any.
#[derive(Resource, Debug, Default)]
struct Shot(Option<Pending>);

#[derive(Debug)]
struct Pending {
    path: PathBuf,
    camera: Entity,
    image: Handle<Image>,
    wait: u8,
}

/// The composed capture plugin.
pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ComposeCapture>()
            .add_message::<ComposedSaved>()
            .init_resource::<Shot>()
            .add_systems(Update, (begin, advance).chain());
    }
}

/// Write a captured image as an RGB PNG.
pub fn save_png(image: &Image, path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    image
        .clone()
        .try_into_dynamic()
        .map_err(|e| e.to_string())?
        .to_rgb8()
        .save(path)
        .map_err(|e| e.to_string())
}

type Roots<'w, 's> = Query<'w, 's, Entity, (With<Node>, Without<ChildOf>)>;

/// Point every interface root at the capture camera, or back at the window's.
fn aim(commands: &mut Commands, roots: &Roots, camera: Option<Entity>) {
    for root in roots {
        match camera {
            Some(camera) => commands.entity(root).insert(UiTargetCamera(camera)),
            None => commands.entity(root).remove::<UiTargetCamera>(),
        };
    }
}

fn begin(
    mut asks: MessageReader<ComposeCapture>,
    mut commands: Commands,
    mut shot: ResMut<Shot>,
    mut images: ResMut<Assets<Image>>,
    size: Res<WindowSize>,
    roots: Roots,
) {
    let Some(ComposeCapture(path)) = asks.read().last().cloned() else {
        return;
    };
    if shot.0.is_some() {
        return;
    }
    let (width, height) = size.physical();
    let image = images.add(Image::new_target_texture(
        width.max(1),
        height.max(1),
        TextureFormat::Bgra8UnormSrgb,
        None,
    ));
    // An image target's scale factor is one, so a canvas pixel is `fit.scale` target pixels
    // at a projection scale of one over it (`pixel::outer_scale` with a factor of one).
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                ..default()
            },
            Projection::Orthographic(OrthographicProjection {
                scale: 1.0 / size.fit().scale as f32,
                ..OrthographicProjection::default_2d()
            }),
            RenderTarget::Image(image.clone().into()),
            Msaa::Off,
            WINDOW_LAYER,
        ))
        .id();
    aim(&mut commands, &roots, Some(camera));
    shot.0 = Some(Pending {
        path,
        camera,
        image,
        wait: SETTLE_FRAMES,
    });
}

fn advance(mut commands: Commands, mut shot: ResMut<Shot>) {
    let Some(pending) = shot.0.as_mut() else {
        return;
    };
    if pending.wait == 0 {
        return;
    }
    pending.wait -= 1;
    if pending.wait > 0 {
        return;
    }
    commands
        .spawn(Screenshot::image(pending.image.clone()))
        .observe(finish);
}

fn finish(
    captured: On<ScreenshotCaptured>,
    mut commands: Commands,
    mut shot: ResMut<Shot>,
    mut images: ResMut<Assets<Image>>,
    mut saved: MessageWriter<ComposedSaved>,
    roots: Roots,
) {
    let Some(pending) = shot.0.take() else {
        return;
    };
    let error = save_png(&captured.image, &pending.path).err();
    aim(&mut commands, &roots, None);
    commands.entity(pending.camera).despawn();
    images.remove(&pending.image);
    saved.write(ComposedSaved {
        path: pending.path,
        error,
    });
}
