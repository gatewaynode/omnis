//! The player's settings in the shell (presentation-ARCHITECTURE §10.5): the applied settings as a
//! resource, read from the settings file at start, and the quality preset carried to the
//! cameras' bloom and the lines' width whenever it changes. Reduced motion is read by the
//! movement, the floor grid by the 3D view. Headless-safe.

use super::VectorSet;
use super::capture::CaptureSize;
use super::combat::size;
use crate::settings::{Prefs, fits};
use bevy::camera::Hdr;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The settings as applied.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Current(pub Prefs);

/// The largest interface scale the window holds, in percent (`settings::fits`).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fitted(pub u16);

impl Default for Fitted {
    fn default() -> Fitted {
        Fitted(crate::settings::SCALE_MAX)
    }
}

/// Applies the settings: the quality preset to the cameras and the lines.
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Current>()
            .init_resource::<Fitted>()
            .add_systems(
                Update,
                (apply_quality, apply_scale).in_set(VectorSet::Screens),
            );
    }
}

/// The interface scale as Bevy's `UiScale`, as far as the window holds it (`settings::fits`), so
/// the HUD's buttons and the party column never run into each other: every interface length and
/// text size follows it, and the fight's screen, whose lines are drawn in the window's pixels,
/// zooms to match (`arena::zoom`). Checked every frame, as the window may be resized.
pub fn apply_scale(
    mut commands: Commands,
    current: Res<Current>,
    window: Query<&Window, With<PrimaryWindow>>,
    capture: Option<Res<CaptureSize>>,
    ui: Option<Res<UiScale>>,
    mut fitted: ResMut<Fitted>,
) {
    let (w, h) = size(window.single().ok(), capture.as_deref());
    fitted.set_if_neq(Fitted(fits(w, h)));
    let scale = f32::from(current.0.scale.min(fitted.0)) / 100.0;
    if ui.is_none_or(|u| (u.0 - scale).abs() > f32::EPSILON) {
        commands.insert_resource(UiScale(scale));
    }
}

/// The quality preset on every HDR camera (the 3D view's and the fight's) and every gizmo group,
/// when the settings change or a camera appears.
pub fn apply_quality(
    mut commands: Commands,
    current: Res<Current>,
    cameras: Query<(Entity, Option<&mut Bloom>), With<Hdr>>,
    added: Query<(), Added<Hdr>>,
    store: Option<ResMut<GizmoConfigStore>>,
) {
    if !current.is_changed() && added.is_empty() {
        return;
    }
    let quality = current.0.quality;
    let intensity = quality.bloom();
    for (entity, bloom) in cameras {
        match bloom {
            Some(_) if intensity <= 0.0 => {
                commands.entity(entity).remove::<Bloom>();
            }
            Some(mut b) => b.intensity = intensity,
            None if intensity > 0.0 => {
                commands.entity(entity).insert(Bloom {
                    intensity,
                    ..Bloom::NATURAL
                });
            }
            None => {}
        }
    }
    if let Some(mut store) = store {
        for (_, config, _) in store.iter_mut() {
            config.line.width = quality.line_width();
        }
    }
}
