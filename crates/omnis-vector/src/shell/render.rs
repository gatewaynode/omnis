//! The 3D view: a `Camera3d` with HDR and bloom following the pose over the ground, and the map
//! drawn as glowing gizmo lines that fade with distance (presentation-ARCHITECTURE.md §7, §8.4).

use super::VectorSet;
use super::capture::Offscreen;
use super::movement::View;
use super::session::Session;
use super::settings::Current;
use crate::geometry::{SegKind, Segment};
use crate::ground::{ease_eye, eye};
use crate::scene::Scene;
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use omnis_sim::omnis_core::{Level, MapId};

/// Cells beyond the visibility depth before a line has faded out completely.
const FOG_EXTRA: f32 = 2.0;
/// Line width in logical pixels.
const LINE_WIDTH: f32 = 2.0;
/// Vertical field of view, radians: about 55°, which gives roughly 90° across 16:9 and 128°
/// across 32:9.
const FOV: f32 = 0.96;

/// The party's level of the current map with its neighbours, rebuilt when the map or the level
/// changes or a door moves.
#[derive(Resource, Default)]
pub struct Lines {
    map: Option<(MapId, Level)>,
    scene: Scene,
}

impl Lines {
    /// How many segments of the party's level are loaded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.scene.own.len()
    }

    /// Whether none are.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scene.own.is_empty()
    }

    /// How many segments of the levels above and below are loaded.
    #[must_use]
    pub fn neighbours(&self) -> usize {
        self.scene.neighbours.len()
    }
}

/// Marks the 3D camera.
#[derive(Component)]
pub struct ViewCamera;

/// Camera and lines: everything that needs a window and a GPU.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lines>()
            .init_resource::<Current>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Startup, (spawn_camera, line_style))
            .add_systems(
                Update,
                (rebuild, follow, draw).chain().in_set(VectorSet::Draw),
            );
    }
}

fn spawn_camera(mut commands: Commands, offscreen: Option<Res<Offscreen>>) {
    let mut camera = commands.spawn((
        ViewCamera,
        Camera3d::default(),
        IsDefaultUiCamera,
        Hdr,
        Tonemapping::None,
        Bloom {
            intensity: 0.25,
            ..Bloom::NATURAL
        },
        Projection::Perspective(PerspectiveProjection {
            fov: FOV,
            near: 0.02,
            ..default()
        }),
        Transform::default(),
    ));
    if let Some(target) = offscreen {
        camera.insert(RenderTarget::Image(target.0.clone().into()));
    }
}

fn line_style(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = LINE_WIDTH;
}

fn rebuild(mut session: ResMut<Session>, mut lines: ResMut<Lines>) {
    let (map, level) = (session.views.position.map, session.views.position.level);
    if lines.map == Some((map, level)) && !session.reshape {
        return;
    }
    session.reshape = false;
    if let Some(data) = session.data.maps.get(&map) {
        lines.scene = Scene::build(data, map, level, &session.world);
        lines.map = Some((map, level));
    }
}

/// The camera at the pose, its height eased toward the eye over the ground.
fn follow(
    time: Res<Time>,
    current: Res<Current>,
    session: Res<Session>,
    lines: Res<Lines>,
    view: Res<View>,
    mut camera: Query<&mut Transform, With<ViewCamera>>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    let pose = session.pose;
    let y = ease_eye(
        transform.translation.y,
        eye(&lines.scene.ground, pose),
        time.delta_secs(),
        current.0.reduced_motion,
    );
    *transform =
        Transform::from_xyz(pose.x, y, pose.z).with_rotation(rotation(pose.yaw, view.pitch));
}

/// The camera's turn: the body's yaw, then the head's pitch (positive looks up).
#[must_use]
pub fn rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
}

/// Line colour by kind: walls bright green, doors amber, solid and impassable terrain in the
/// terrain's colour, the floor and ceiling grids dim. Values above 1.0 bloom.
fn colour(segment: &Segment) -> (f32, f32, f32) {
    let (r, g, b) = segment.rgb;
    let t = |k: f32| {
        (
            f32::from(r) / 255.0 * k,
            f32::from(g) / 255.0 * k,
            f32::from(b) / 255.0 * k,
        )
    };
    match segment.kind {
        SegKind::Wall => (0.15, 3.0, 0.5),
        SegKind::Door => (3.2, 1.6, 0.2),
        SegKind::Block => t(3.0),
        SegKind::Barrier => t(2.5),
        SegKind::Floor => t(0.6),
        SegKind::Ceiling => t(0.3),
        // Provisional until the colour roles (M8.5 step 5): rock warm, ramps pale.
        SegKind::Cliff => (2.2, 1.4, 0.6),
        SegKind::Ramp => (0.9, 1.8, 2.2),
        // Joins (M8.5 step 1b), provisional as cliffs and ramps: stairs and ladders pale gold,
        // a hole a warning red.
        SegKind::Stair | SegKind::Ladder => (2.4, 2.1, 0.8),
        SegKind::Hole => (2.6, 0.5, 0.4),
    }
}

fn draw(session: Res<Session>, lines: Res<Lines>, current: Res<Current>, mut gizmos: Gizmos) {
    let pose = session.pose;
    let depth = session.views.sight;
    let end = f32::from(depth) + FOG_EXTRA;
    let start = end * 0.5;
    let grid = current.0.grid;
    for (segment, dim) in lines.scene.strokes(grid) {
        let mid = (
            (segment.a[0] + segment.b[0]) * 0.5,
            (segment.a[2] + segment.b[2]) * 0.5,
        );
        let distance = (mid.0 - pose.x).hypot(mid.1 - pose.z);
        if distance >= end {
            continue;
        }
        let fade = if distance <= start {
            1.0
        } else {
            1.0 - (distance - start) / (end - start)
        };
        let (r, g, b) = colour(segment);
        let k = fade * dim;
        gizmos.line(
            Vec3::from_array(segment.a),
            Vec3::from_array(segment.b),
            LinearRgba::new(r * k, g * k, b * k, fade),
        );
    }
}
