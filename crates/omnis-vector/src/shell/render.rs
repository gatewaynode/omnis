//! The 3D view: a `Camera3d` with HDR and bloom following the pose over the ground, and the map
//! drawn as glowing gizmo lines that fade with distance (presentation-ARCHITECTURE.md §7, §8.4)
//! over dark solid fills that hide what lies behind them (§8.2, M8.5 step 3).

use super::VectorSet;
use super::capture::Offscreen;
use super::movement::View;
use super::session::Session;
use super::settings::Current;
use crate::fill::Tri;
use crate::geometry::{SegKind, Segment};
use crate::ground::{ease_eye, eye};
use crate::scene::Scene;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use omnis_sim::omnis_core::{Level, MapId};

/// Cells beyond the visibility depth before a line has faded out completely.
const FOG_EXTRA: f32 = 2.0;
/// Line width in logical pixels.
const LINE_WIDTH: f32 = 2.0;
/// How far the lines are drawn toward the camera, so a wall's own edges and the floor grid are
/// not hidden by the face they lie on: the gizmos' bias, a power of the depth (presentation-
/// ARCHITECTURE.md §8.2). Tuned on the captures.
const LINE_DEPTH_BIAS: f32 = -0.002;
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

/// Marks the entity that holds the scene's fills.
#[derive(Component)]
pub struct Fills;

/// The fills as a mesh: a triangle list, each triangle its own three vertices with its face's
/// normal (the material is unlit, so the normals only keep the vertex layout whole).
#[must_use]
pub fn mesh(fills: &[Tri]) -> Mesh {
    let positions: Vec<[f32; 3]> = fills.iter().flatten().copied().collect();
    let normals: Vec<[f32; 3]> = fills
        .iter()
        .flat_map(|[a, b, c]| {
            let (a, b, c) = (Vec3::from(*a), Vec3::from(*b), Vec3::from(*c));
            let n = (b - a).cross(c - a).normalize_or(Vec3::Y).to_array();
            [n; 3]
        })
        .collect();
    // A scene's vertex count is far below u32's range.
    #[allow(clippy::cast_possible_truncation)]
    let indices = (0..positions.len() as u32).collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// Camera and lines: everything that needs a window and a GPU.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lines>()
            .init_resource::<Current>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Startup, (spawn_camera, spawn_fills, line_style))
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

/// The fills' entity, unlit in the clear colour and drawn from both sides; its mesh comes with
/// the first scene.
fn spawn_fills(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    commands.spawn((Fills, MeshMaterial3d(material), Transform::default()));
}

fn line_style(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = LINE_WIDTH;
    config.depth_bias = LINE_DEPTH_BIAS;
}

fn rebuild(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut lines: ResMut<Lines>,
    mut meshes: ResMut<Assets<Mesh>>,
    fills: Query<Entity, With<Fills>>,
) {
    let (map, level) = (session.views.position.map, session.views.position.level);
    if lines.map == Some((map, level)) && !session.reshape {
        return;
    }
    session.reshape = false;
    if let Some(data) = session.data.maps.get(&map) {
        lines.scene = Scene::build(data, map, level, &session.world);
        lines.map = Some((map, level));
        for entity in &fills {
            let handle = meshes.add(mesh(&lines.scene.fills));
            commands.entity(entity).insert(Mesh3d(handle));
        }
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

/// A crest's line, where the ground turns away from the eye. Provisional until the colour roles
/// (M8.5 step 5): brighter than the floor grid, so a rise's top shows against the sky.
const CREST: (f32, f32, f32) = (1.1, 1.1, 1.1);

fn draw(
    session: Res<Session>,
    lines: Res<Lines>,
    current: Res<Current>,
    camera: Query<&Transform, With<ViewCamera>>,
    mut gizmos: Gizmos,
) {
    let pose = session.pose;
    let depth = session.views.sight;
    let end = f32::from(depth) + FOG_EXTRA;
    let start = end * 0.5;
    let grid = current.0.grid;
    let fade = |a: [f32; 3], b: [f32; 3]| {
        let mid = ((a[0] + b[0]) * 0.5, (a[2] + b[2]) * 0.5);
        let distance = (mid.0 - pose.x).hypot(mid.1 - pose.z);
        if distance >= end {
            None
        } else if distance <= start {
            Some(1.0)
        } else {
            Some(1.0 - (distance - start) / (end - start))
        }
    };
    let mut line = |a: [f32; 3], b: [f32; 3], (r, g, b_): (f32, f32, f32), dim: f32| {
        let Some(fade) = fade(a, b) else { return };
        let k = fade * dim;
        gizmos.line(
            Vec3::from_array(a),
            Vec3::from_array(b),
            LinearRgba::new(r * k, g * k, b_ * k, fade),
        );
    };
    for (segment, dim) in lines.scene.strokes(grid) {
        line(segment.a, segment.b, colour(segment), dim);
    }
    if let Ok(eye) = camera.single() {
        for (a, b) in lines.scene.ground.crests(eye.translation.to_array(), end) {
            line(a, b, CREST, 1.0);
        }
    }
}
