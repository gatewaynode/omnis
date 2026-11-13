//! The 3D view: a `Camera3d` with HDR and bloom following the pose over the ground, and the map
//! drawn as glowing lines that fade with distance (presentation-ARCHITECTURE.md §7, §8.4) over
//! dark solid fills that hide what lies behind them (§8.2, M8.5 step 3). Lines and fills are
//! meshes built when the scene changes; the lines are drawn by `lines::LineMaterial`
//! (rendering size 1).

use super::VectorSet;
use super::aim::Aimed;
use super::capture::Offscreen;
use super::lines::{self, LineMaterial};
use super::movement::View;
use super::session::Session;
use super::settings::Current;
use crate::fill::Tri;
use crate::ground::{ease_eye, eye};
use crate::scene::Scene;
use crate::strokes::Strokes;
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
pub(super) const LINE_WIDTH: f32 = 2.0;
/// How far the lines are drawn toward the camera, so a wall's own edges and the floor grid are
/// not hidden by the face they lie on: a power of the depth, as Bevy's gizmo lines have it
/// (presentation-ARCHITECTURE.md §8.2). Tuned on the captures.
pub(super) const LINE_DEPTH_BIAS: f32 = -0.002;
/// Vertical field of view, radians: about 55°, which gives roughly 90° across 16:9 and 128°
/// across 32:9.
pub(super) const FOV: f32 = 0.96;

/// The party's level of the current map with its neighbours, rebuilt when the map or the level
/// changes or a door moves.
#[derive(Resource, Default)]
pub struct Lines {
    map: Option<(MapId, Level)>,
    /// Whether the floor and ceiling grids were shown when the line mesh was built.
    grid: bool,
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

/// Marks the entity that holds the view's lines as a mesh: one mesh in the scene's order, since
/// of two lines in the same place the first shows.
#[derive(Component)]
pub struct ViewLines;

/// Marks the entity that holds the view's crests, found again as the eye moves.
#[derive(Component)]
pub struct ViewCrests;

/// Marks the entity that holds the outline of the floor the pointer is on a pile of.
#[derive(Component)]
pub struct ViewAim;

/// The view's line material.
#[derive(Resource)]
pub struct ViewMaterial(pub Handle<LineMaterial>);

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
        if !app.is_plugin_added::<lines::LinesPlugin>() {
            app.add_plugins(lines::LinesPlugin);
        }
        app.init_resource::<Lines>()
            .init_resource::<Current>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Startup, (spawn_camera, spawn_fills, spawn_lines))
            .add_systems(
                Update,
                (rebuild, follow, (tune, crests, aim_mark))
                    .chain()
                    .in_set(VectorSet::Draw),
            );
    }
}

/// What every camera of the client carries so its lines glow alike: HDR, bloom, and linear
/// tonemapping. Linear, not `None`: since Bevy 0.20 `None` passes the image through with no
/// dither and no clamp of negative channels, and faded lines band.
#[must_use]
pub fn glow() -> impl Bundle {
    (
        Hdr,
        Tonemapping::Linear,
        Bloom {
            intensity: 0.25,
            ..Bloom::NATURAL
        },
    )
}

fn spawn_camera(mut commands: Commands, offscreen: Option<Res<Offscreen>>) {
    let mut camera = commands.spawn((
        ViewCamera,
        Camera3d::default(),
        IsDefaultUiCamera,
        glow(),
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

/// The view's line material and the entities its meshes hang on; the meshes come with the
/// first scene.
fn spawn_lines(mut commands: Commands, mut materials: ResMut<Assets<LineMaterial>>) {
    let material = materials.add(LineMaterial::new(fog_end(0.0), LINE_WIDTH, LINE_DEPTH_BIAS));
    commands.spawn((ViewLines, lines::bundle(material.clone())));
    commands.spawn((ViewCrests, lines::bundle(material.clone())));
    commands.spawn((ViewAim, lines::bundle(material.clone())));
    commands.insert_resource(ViewMaterial(material));
}

/// The fog's end follows the party's sight (the width follows the quality in
/// `settings::apply_quality`).
fn tune(
    session: Res<Session>,
    material: Res<ViewMaterial>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    let end = fog_end(f32::from(session.views.sight));
    let stale = materials
        .get(&material.0)
        .is_some_and(|m| m.fog_end() != end);
    if stale && let Some(mut m) = materials.get_mut(&material.0) {
        m.params.x = end;
    }
}

/// The ground's crests from where the camera stands, found again when it has moved or the
/// scene changed.
fn crests(
    mut commands: Commands,
    session: Res<Session>,
    lines: Res<Lines>,
    mut meshes: ResMut<Assets<Mesh>>,
    camera: Query<&Transform, With<ViewCamera>>,
    part: Query<(Entity, Option<&Mesh3d>), With<ViewCrests>>,
    mut at: Local<lines::CrestsAt>,
) {
    let (Ok(eye), Ok(part)) = (camera.single(), part.single()) else {
        return;
    };
    let reach = fog_end(f32::from(session.views.sight));
    let from = (eye.translation.to_array(), reach, lines.is_changed());
    lines::hang_crests(
        &mut commands,
        &mut meshes,
        part,
        &mut at,
        &lines.scene.ground,
        from,
    );
}

/// The outline of the floor the pointer is on a pile of, drawn again when the aim or the scene
/// changes and taken away when the pointer is on none.
fn aim_mark(
    mut commands: Commands,
    aimed: Option<Res<Aimed>>,
    lines: Res<Lines>,
    mut meshes: ResMut<Assets<Mesh>>,
    part: Query<(Entity, Option<&Mesh3d>), With<ViewAim>>,
) {
    let (Some(aimed), Ok(part)) = (aimed, part.single()) else {
        return;
    };
    if !aimed.is_changed() && !lines.is_changed() {
        return;
    }
    let strokes = aimed
        .0
        .map(|cell| lines::aim_mark(&lines.scene.ground, cell))
        .unwrap_or_default();
    lines::hang(&mut commands, &mut meshes, part, &strokes);
}

fn rebuild(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut lines: ResMut<Lines>,
    mut meshes: ResMut<Assets<Mesh>>,
    fills: Query<Entity, With<Fills>>,
    parts: Query<Entity, With<ViewLines>>,
    current: Res<Current>,
) {
    let (map, level) = (session.views.position.map, session.views.position.level);
    let grid = current.0.grid;
    if lines.map == Some((map, level)) && !session.reshape && lines.grid == grid {
        return;
    }
    lines.grid = grid;
    session.reshape = false;
    if session.data.maps.contains_key(&map) {
        lines.scene = Scene::build(&session.data, map, level, &session.world);
        lines.map = Some((map, level));
        for entity in &fills {
            let handle = meshes.add(mesh(&lines.scene.fills));
            commands.entity(entity).insert(Mesh3d(handle));
        }
        for entity in &parts {
            let strokes = Strokes::of(lines.scene.strokes(grid));
            let handle = meshes.add(lines::mesh(&strokes));
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

/// Where the fog of a sight of `sight` cells ends: `FOG_EXTRA` beyond it.
pub(super) fn fog_end(sight: f32) -> f32 {
    sight + FOG_EXTRA
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_s_glow_is_hdr_with_linear_tonemapping() {
        let mut world = World::new();
        let camera = world.spawn(glow()).id();
        assert!(world.get::<Hdr>(camera).is_some());
        assert!(world.get::<Bloom>(camera).is_some());
        // `None` would compile and draw, without dither: the choice is pinned here.
        assert!(matches!(
            world.get::<Tonemapping>(camera),
            Some(Tonemapping::Linear)
        ));
    }
}
