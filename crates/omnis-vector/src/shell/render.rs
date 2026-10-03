//! The 3D view: a `Camera3d` with HDR and bloom following the pose, and the map drawn as
//! glowing gizmo lines that fade with distance (alt-ARCHITECTURE.md §7).

use super::VectorSet;
use super::capture::Offscreen;
use super::movement::Intent;
use super::session::Session;
use crate::geom::EYE_HEIGHT;
use crate::geometry::{SegKind, Segment, extract};
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use omnis_sim::omnis_core::MapId;

/// Cells beyond the visibility depth before a line has faded out completely.
const FOG_EXTRA: f32 = 2.0;
/// Line width in logical pixels.
const LINE_WIDTH: f32 = 2.0;
/// Vertical field of view, radians: about 55°, which gives roughly 90° across 16:9 and 128°
/// across 32:9.
const FOV: f32 = 0.96;

/// The current map's segments, rebuilt when the map changes or a door moves.
#[derive(Resource, Default)]
pub struct Lines {
    map: Option<MapId>,
    segments: Vec<Segment>,
}

impl Lines {
    /// How many segments are loaded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Whether none are.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// Marks the 3D camera.
#[derive(Component)]
pub struct ViewCamera;

/// Camera, lines, cursor grab: everything that needs a window and a GPU.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lines>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Startup, (spawn_camera, line_style))
            .add_systems(Update, grab_cursor.in_set(VectorSet::Grab))
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

/// Click in the window to look with the mouse; Esc gives the cursor back.
fn grab_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut intent: ResMut<Intent>,
) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };
    let over_button = interactions.iter().any(|i| *i != Interaction::None);
    if mouse.just_pressed(MouseButton::Left) && !intent.looking && !over_button {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
        intent.looking = true;
    }
    if keys.just_pressed(KeyCode::Escape) && intent.looking {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        intent.looking = false;
    }
}

fn rebuild(mut session: ResMut<Session>, mut lines: ResMut<Lines>) {
    let map = session.world.position.map;
    if lines.map == Some(map) && !session.reshape {
        return;
    }
    session.reshape = false;
    if let Some(data) = session.data.maps.get(&map) {
        lines.segments = extract(data, map, &session.world);
        lines.map = Some(map);
    }
}

fn follow(session: Res<Session>, mut camera: Query<&mut Transform, With<ViewCamera>>) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    let pose = session.pose;
    *transform = Transform::from_xyz(pose.x, EYE_HEIGHT, pose.z)
        .with_rotation(Quat::from_rotation_y(pose.yaw));
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
    }
}

fn draw(session: Res<Session>, lines: Res<Lines>, mut gizmos: Gizmos) {
    let pose = session.pose;
    let depth = session
        .data
        .maps
        .get(&session.world.position.map)
        .and_then(|m| {
            m.cell(session.world.position.x, session.world.position.y)
                .map(|c| m.terrain(c).visibility_depth)
        })
        .unwrap_or(6);
    let end = f32::from(depth) + FOG_EXTRA;
    let start = end * 0.5;
    for segment in &lines.segments {
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
        gizmos.line(
            Vec3::from_array(segment.a),
            Vec3::from_array(segment.b),
            LinearRgba::new(r * fade, g * fade, b * fade, fade),
        );
    }
}
