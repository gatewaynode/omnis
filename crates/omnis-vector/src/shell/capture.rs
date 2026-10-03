//! `--screenshot`: hold forward for `--walk` frames, settle, capture, exit. The capture renders
//! offscreen (no window: the camera targets an image), so it works where no window server
//! drives frames, such as an agent's shell or CI.

use super::VectorSet;
use super::movement::Intent;
use crate::pose::Motion;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use std::path::PathBuf;

/// Frames after the walk before the capture, so the pose settles and bloom converges.
const SETTLE: u32 = 30;
/// Frames after the capture before exiting, so the file is written.
const WRITE: u32 = 20;

/// The capture plan.
#[derive(Resource)]
pub struct Capture {
    /// Where the PNG goes.
    pub path: PathBuf,
    /// Frames to hold forward first.
    pub walk: u32,
    frame: u32,
}

impl Capture {
    /// A plan for `path` after `walk` frames of walking.
    #[must_use]
    pub fn new(path: PathBuf, walk: u32) -> Capture {
        Capture {
            path,
            walk,
            frame: 0,
        }
    }
}

/// The offscreen image the camera renders into while capturing.
#[derive(Resource, Clone)]
pub struct Offscreen(pub Handle<Image>);

/// Width and height of the offscreen capture, in pixels.
#[derive(Resource, Clone, Copy)]
pub struct CaptureSize(pub u32, pub u32);

/// Drives the capture when a `Capture` resource is present.
pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, offscreen.run_if(resource_exists::<CaptureSize>))
            .add_systems(
                Update,
                drive
                    .run_if(resource_exists::<Capture>)
                    .in_set(VectorSet::Input),
            );
    }
}

fn offscreen(mut commands: Commands, size: Res<CaptureSize>, mut images: ResMut<Assets<Image>>) {
    let image = Image::new_target_texture(size.0, size.1, TextureFormat::Bgra8UnormSrgb, None);
    commands.insert_resource(Offscreen(images.add(image)));
}

fn drive(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    mut intent: ResMut<Intent>,
    target: Option<Res<Offscreen>>,
    mut exit: MessageWriter<AppExit>,
) {
    capture.frame += 1;
    let frame = capture.frame;
    if frame <= capture.walk {
        intent.motion = Motion {
            forward: 1.0,
            strafe: 0.0,
        };
    } else if frame == capture.walk + SETTLE {
        let shot = target.map_or_else(Screenshot::primary_window, |t| {
            Screenshot::image(t.0.clone())
        });
        commands
            .spawn(shot)
            .observe(save_to_disk(capture.path.clone()));
    } else if frame >= capture.walk + SETTLE + WRITE {
        exit.write(AppExit::Success);
    }
}
