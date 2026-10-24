//! The editor's preview picture (M5.7, `tasks/plans/m5-editor-v1.md` §Client side "Preview"):
//! a second 3D camera on its own render layer draws the edited map's scene
//! (`editor::preview`) into an image the editor's right column shows, from the cursor's cell
//! the way the cursor faces. The scene is rebuilt when the map's resolve changes or the level
//! does; the camera moves with the cursor; the lines are the view's own, coloured and faded the
//! same way. Needs a renderer, like the view: the headless shell does without it, and the
//! scene it would draw is tested Bevy-free.

use super::VectorSet;
use super::editor::Editing;
use super::render::{self, CREST, LINE_DEPTH_BIAS, LINE_WIDTH, mesh};
use super::session::Packs;
use super::settings::Current;
use crate::editor::preview;
use crate::scene::Scene;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use omnis_sim::omnis_core::Level;

/// The preview's render layer: the view's is 0, the fight's 1.
const LAYER: usize = 2;
/// The picture's pixels, 16:9, twice the node for high-density screens.
const SIZE: (u32, u32) = (800, 450);

/// The image the preview draws into; the editor's right column shows it.
#[derive(Resource, Clone)]
pub struct PreviewTarget(pub Handle<Image>);

/// Marks the preview's camera.
#[derive(Component)]
pub struct PreviewCamera;

/// The preview's fills, on its layer.
#[derive(Component)]
struct PreviewFills;

/// The preview's lines.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct PreviewGizmos;

/// What the preview last drew: the scene for `(resolve, level)`, kept while the map does not
/// resolve.
#[derive(Resource, Default)]
struct Preview {
    scene_key: Option<(u64, Level)>,
    scene: Scene,
}

/// The camera, the fills and the lines of the preview; on while the editor is.
pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Preview>()
            .init_gizmo_group::<PreviewGizmos>()
            .add_systems(Startup, spawn)
            .add_systems(
                Update,
                (
                    switch::<true>.run_if(resource_added::<Editing>),
                    switch::<false>.run_if(resource_removed::<Editing>),
                    (rebuild, place, draw)
                        .chain()
                        .run_if(resource_exists::<Editing>),
                )
                    .in_set(VectorSet::Draw),
            );
    }
}

/// The target image, the camera drawing into it (off until the editor opens), the fills'
/// entity and the lines' style, all on the preview's layer.
fn spawn(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut store: ResMut<GizmoConfigStore>,
) {
    let target = images.add(Image::new_target_texture(
        SIZE.0,
        SIZE.1,
        TextureFormat::Bgra8UnormSrgb,
        None,
    ));
    commands.insert_resource(PreviewTarget(target.clone()));
    commands.spawn((
        PreviewCamera,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: -1,
            ..default()
        },
        RenderTarget::Image(target.into()),
        Hdr,
        Tonemapping::None,
        Bloom {
            intensity: 0.25,
            ..Bloom::NATURAL
        },
        Projection::Perspective(PerspectiveProjection {
            fov: render::FOV,
            near: 0.02,
            ..default()
        }),
        Transform::default(),
        RenderLayers::layer(LAYER),
    ));
    let material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    commands.spawn((
        PreviewFills,
        MeshMaterial3d(material),
        Transform::default(),
        RenderLayers::layer(LAYER),
    ));
    let (config, _) = store.config_mut::<PreviewGizmos>();
    config.line.width = LINE_WIDTH;
    config.depth_bias = LINE_DEPTH_BIAS;
    config.render_layers = RenderLayers::layer(LAYER);
}

/// The camera on while the editor is open, off otherwise.
fn switch<const ON: bool>(mut cameras: Query<&mut Camera, With<PreviewCamera>>) {
    for mut camera in &mut cameras {
        camera.is_active = ON;
    }
}

/// The scene built anew when the map's resolve or the shown level changed; while the map does
/// not resolve the last scene stays (the inspector says why).
fn rebuild(
    mut commands: Commands,
    packs: Res<Packs>,
    editing: Res<Editing>,
    mut preview: ResMut<Preview>,
    mut meshes: ResMut<Assets<Mesh>>,
    fills: Query<Entity, With<PreviewFills>>,
) {
    let key = (editing.resolved_at, editing.level);
    if preview.scene_key == Some(key) {
        return;
    }
    let Some(map) = editing.resolved.as_ref() else {
        return;
    };
    preview.scene_key = Some(key);
    preview.scene = preview::scene(&packs.data, map, editing.level);
    for entity in &fills {
        let handle = meshes.add(mesh(&preview.scene.fills));
        commands.entity(entity).insert(Mesh3d(handle));
    }
}

/// The camera at the cursor's cell, its eye over the ground, looking the way the cursor faces.
fn place(
    editing: Res<Editing>,
    preview: Res<Preview>,
    mut cameras: Query<&mut Transform, With<PreviewCamera>>,
) {
    let pose = preview::pose(editing.cursor, editing.facing);
    let (eye, yaw) = preview::look(&preview.scene, pose);
    for mut transform in &mut cameras {
        *transform = Transform::from_translation(Vec3::from_array(eye))
            .with_rotation(render::rotation(yaw, 0.0));
    }
}

/// The scene's lines as the view draws them, faded over the preview's sight.
fn draw(
    editing: Res<Editing>,
    preview: Res<Preview>,
    current: Res<Current>,
    cameras: Query<&Transform, With<PreviewCamera>>,
    mut gizmos: Gizmos<PreviewGizmos>,
) {
    let pose = preview::pose(editing.cursor, editing.facing);
    let fade = |a: [f32; 3], b: [f32; 3]| {
        let mid = ((a[0] + b[0]) * 0.5, (a[2] + b[2]) * 0.5);
        render::fade((mid.0 - pose.x).hypot(mid.1 - pose.z), preview::SIGHT)
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
    for (segment, dim) in preview.scene.strokes(current.0.grid) {
        line(segment.a, segment.b, render::colour(segment), dim);
    }
    if let Ok(eye) = cameras.single() {
        let reach = render::fog_end(preview::SIGHT);
        for (a, b) in preview
            .scene
            .ground
            .crests(eye.translation.to_array(), reach)
        {
            line(a, b, CREST, 1.0);
        }
    }
}

/// The height of the picture's node for a column `width` pixels wide: 16:9 inside the padding.
#[must_use]
pub fn picture_height(width: f32) -> f32 {
    (width * 9.0 / 16.0).round()
}
