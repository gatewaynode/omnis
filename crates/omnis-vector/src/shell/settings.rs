//! The player's settings in the shell (presentation-ARCHITECTURE §10.5): the applied settings as a
//! resource, read from the settings file at start, and the quality preset carried to the
//! cameras' bloom and the lines' width whenever it changes. Reduced motion is read by the
//! movement, the floor grid by the 3D view. Headless-safe.

use super::VectorSet;
use crate::settings::Prefs;
use bevy::camera::Hdr;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

/// The settings as applied.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Current(pub Prefs);

/// Applies the settings: the quality preset to the cameras and the lines.
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Current>()
            .add_systems(Update, apply_quality.in_set(VectorSet::Screens));
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
