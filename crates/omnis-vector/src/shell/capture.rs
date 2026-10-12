//! `--screenshot`: hold forward for `--walk` frames, settle, capture, exit. The capture renders
//! offscreen (no window: the camera targets an image), so it works where no window server
//! drives frames, such as an agent's shell or CI. With `--timing` the loop runs uncapped and
//! the frame times from the walk to the capture are reported (presentation-ARCHITECTURE §8.6).
//!
//! [`ShotRequest`]: what the running window shows, for the dev socket's `screenshot`. Both
//! cameras render into an image the window's size for a few frames (a plain window capture is
//! black when the window is not on a screen), the image is saved as an RGB PNG, and the cameras
//! go back to the window. The window does not update during those frames.

use super::VectorSet;
use super::combat::FightCamera;
use super::movement::Intent;
use super::render::ViewCamera;
use crate::pose::Motion;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::window::{PrimaryWindow, WindowRef};
use std::collections::VecDeque;
use std::path::PathBuf;

/// Frames a live capture renders into its image before it is taken, so bloom converges.
const LIVE_SETTLE: u32 = 4;

/// Capture what the window shows into this PNG (the dev socket's `screenshot`).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ShotRequest(pub PathBuf);

/// A live capture finished: written, or why not.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ShotSaved {
    /// The file asked for.
    pub path: PathBuf,
    /// Why it was not written, if it was not.
    pub error: Option<String>,
}

/// The live captures: the one rendering now and those waiting.
#[derive(Resource, Default)]
struct Live {
    current: Option<(PathBuf, Handle<Image>, u32)>,
    waiting: VecDeque<PathBuf>,
}

/// Frames after the walk before the capture, so the pose settles and bloom converges.
const SETTLE: u32 = 30;
/// Frames after the capture before exiting, so the file is written.
const WRITE: u32 = 20;

/// Frames left out of the timing at the start, while pipelines compile and assets load.
const WARM: u32 = 10;

/// The capture plan.
#[derive(Resource)]
pub struct Capture {
    /// Where the PNG goes.
    pub path: PathBuf,
    /// Frames to hold forward first.
    pub walk: u32,
    /// The frame times, when they are reported.
    pub timing: Option<FrameTimes>,
    frame: u32,
}

impl Capture {
    /// A plan for `path` after `walk` frames of walking.
    #[must_use]
    pub fn new(path: PathBuf, walk: u32) -> Capture {
        Capture {
            path,
            walk,
            timing: None,
            frame: 0,
        }
    }

    /// The same plan, reporting its frame times.
    #[must_use]
    pub fn timed(self) -> Capture {
        Capture {
            timing: Some(FrameTimes::default()),
            ..self
        }
    }
}

/// Frame times, in seconds, in the order they came.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameTimes(pub Vec<f32>);

impl FrameTimes {
    /// The frame count, and the mean and worst frame time in milliseconds; `None` with no frames.
    #[must_use]
    pub fn summary(&self) -> Option<(usize, f32, f32)> {
        let n = self.0.len();
        if n == 0 {
            return None;
        }
        // A capture's frame count is far below f32's exact integers.
        #[allow(clippy::cast_precision_loss)]
        let mean = self.0.iter().sum::<f32>() / n as f32;
        let worst = self.0.iter().copied().fold(0.0, f32::max);
        Some((n, mean * 1000.0, worst * 1000.0))
    }

    /// The report: a line to read and a JSON line to parse.
    #[must_use]
    pub fn report(&self) -> String {
        match self.summary() {
            Some((n, mean, worst)) => format!(
                "timing: {n} frames, mean {mean:.2} ms, worst {worst:.2} ms\n\
                 {{\"frames\":{n},\"mean_ms\":{mean:.3},\"worst_ms\":{worst:.3}}}"
            ),
            None => "timing: no frames (the walk was shorter than the warm-up)".to_owned(),
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
        app.add_message::<ShotRequest>()
            .add_message::<ShotSaved>()
            .init_resource::<Live>()
            .add_systems(Update, live.in_set(VectorSet::Screens))
            .add_systems(PreStartup, offscreen.run_if(resource_exists::<CaptureSize>))
            .add_systems(
                Update,
                drive
                    .run_if(resource_exists::<Capture>)
                    .in_set(VectorSet::Screens),
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
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    capture.frame += 1;
    let frame = capture.frame;
    let shot = capture.walk + SETTLE;
    if (WARM < frame && frame <= shot)
        && let Some(times) = capture.timing.as_mut()
    {
        times.0.push(time.delta_secs());
    }
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
        if let Some(times) = &capture.timing {
            println!("{}", times.report());
        }
        exit.write(AppExit::Success);
    }
}

/// The cameras of the 3D view and the fight.
type Cameras<'w, 's> = Query<'w, 's, Entity, Or<(With<ViewCamera>, With<FightCamera>)>>;

/// Run the live captures one at a time: point the cameras at an image, let it settle, take it.
/// Without a window (a headless test, the offscreen capture) a request fails at once.
fn live(
    mut commands: Commands,
    mut state: ResMut<Live>,
    mut requests: MessageReader<ShotRequest>,
    mut saved: MessageWriter<ShotSaved>,
    window: Query<&Window, With<PrimaryWindow>>,
    images: Option<ResMut<Assets<Image>>>,
    cameras: Cameras,
) {
    let window = window.single().ok();
    for ShotRequest(path) in requests.read() {
        if window.is_none() || images.is_none() {
            saved.write(ShotSaved {
                path: path.clone(),
                error: Some("no window to capture; is the renderer running?".to_owned()),
            });
        } else {
            state.waiting.push_back(path.clone());
        }
    }
    let (Some(window), Some(mut images)) = (window, images) else {
        return;
    };
    match &mut state.current {
        None => {
            let Some(path) = state.waiting.pop_front() else {
                return;
            };
            let (w, h) = (
                window.physical_width().max(1),
                window.physical_height().max(1),
            );
            let image = images.add(Image::new_target_texture(
                w,
                h,
                TextureFormat::Bgra8UnormSrgb,
                None,
            ));
            for camera in &cameras {
                commands
                    .entity(camera)
                    .insert(RenderTarget::Image(image.clone().into()));
            }
            state.current = Some((path, image, 0));
        }
        Some((path, image, frame)) => {
            *frame += 1;
            if *frame != LIVE_SETTLE {
                return;
            }
            let target = path.clone();
            commands.spawn(Screenshot::image(image.clone())).observe(
                move |captured: On<ScreenshotCaptured>,
                      mut saved: MessageWriter<ShotSaved>,
                      mut state: ResMut<Live>,
                      cameras: Cameras,
                      mut commands: Commands| {
                    let error = captured
                        .image
                        .clone()
                        .try_into_dynamic()
                        .map_err(|e| e.to_string())
                        .and_then(|image| image.to_rgb8().save(&target).map_err(|e| e.to_string()))
                        .err();
                    saved.write(ShotSaved {
                        path: target.clone(),
                        error,
                    });
                    for camera in &cameras {
                        commands
                            .entity(camera)
                            .insert(RenderTarget::Window(WindowRef::Primary));
                    }
                    state.current = None;
                    commands.entity(captured.entity).despawn();
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FrameTimes;

    #[test]
    fn the_frame_times_report_their_mean_and_worst_in_milliseconds() {
        let times = FrameTimes(vec![0.010, 0.020, 0.030]);
        let (n, mean, worst) = times.summary().expect("frames");
        assert_eq!(n, 3);
        assert!((mean - 20.0).abs() < 1e-3, "{mean}");
        assert!((worst - 30.0).abs() < 1e-3, "{worst}");
        assert!(
            times
                .report()
                .contains(r#"{"frames":3,"mean_ms":20.000,"worst_ms":30.000}"#),
            "{}",
            times.report()
        );
        assert_eq!(FrameTimes::default().summary(), None);
    }
}
