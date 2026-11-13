//! The editor's preview picture (M5.7, `tasks/plans/m5-editor-v1.md` §Client side "Preview"):
//! a second 3D camera on its own render layer draws the edited map's scene
//! (`editor::preview`) into an image the editor's right column shows, from the cursor's cell
//! the way the cursor faces. The scene is rebuilt when the map's resolve changes or the level
//! does; the camera moves with the cursor; the lines are the view's own, coloured and faded the
//! same way. The picture needs a renderer, like the view; headless the plugin still keeps the
//! scene (tested), and spawns no camera. The shell's `VectorSet::Draw` runs only while a game
//! session exists, so the preview's systems run after `VectorSet::Screens` on their own
//! condition: the editor being open.

use super::VectorSet;
use super::editor::Editing;
use super::lines::{self, LineMaterial};
use super::render::{self, LINE_DEPTH_BIAS, LINE_WIDTH, mesh};
use super::session::Packs;
use super::settings::Current;
use crate::editor::preview;
use crate::scene::Scene;
use crate::strokes::Strokes;
use crate::style::Palette;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
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

/// Marks the entity that holds the preview's lines as a mesh, on its layer.
#[derive(Component)]
pub struct PreviewLines;

/// Marks the entity that holds the preview's crests.
#[derive(Component)]
pub struct PreviewCrests;

/// What the preview last drew: the scene for `(resolve, level)`, kept while the map does not
/// resolve.
#[derive(Resource, Default)]
pub struct Preview {
    /// The `Editor.resolved_at` and level the scene was built for.
    pub scene_key: Option<(u64, Level)>,
    /// The scene drawn.
    pub scene: Scene,
    /// Whether the floor and ceiling grids were shown when the line mesh was built.
    grid: Option<bool>,
}

/// The camera, the fills and the lines of the preview; on while the editor is.
pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        // Not in `VectorSet::Draw`: that set runs only while a game session exists.
        app.init_resource::<Preview>()
            .add_systems(Startup, spawn)
            .add_systems(
                Update,
                (
                    switch::<true>.run_if(resource_added::<Editing>),
                    switch::<false>.run_if(resource_removed::<Editing>),
                    (rebuild, paint, place, crests)
                        .chain()
                        .run_if(resource_exists::<Editing>),
                )
                    .after(VectorSet::Screens),
            );
    }
}

/// The target image, the camera drawing into it (off until the editor opens), the fills'
/// entity and the lines' style, all on the preview's layer. Headless, with no images, materials
/// or gizmos, nothing is spawned and the editor shows no picture.
fn spawn(
    mut commands: Commands,
    images: Option<ResMut<Assets<Image>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    lines: Option<ResMut<Assets<LineMaterial>>>,
) {
    // The line entities need only their material's store, which a headless test can have.
    if let Some(mut lines) = lines {
        let material = lines.add(LineMaterial::new(
            render::fog_end(preview::SIGHT),
            LINE_WIDTH,
            LINE_DEPTH_BIAS,
        ));
        let layer = RenderLayers::layer(LAYER);
        commands.spawn((PreviewLines, lines::bundle(material.clone()), layer.clone()));
        commands.spawn((PreviewCrests, lines::bundle(material), layer));
    }
    let (Some(mut images), Some(mut materials)) = (images, materials) else {
        return;
    };
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
        render::glow(),
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
    (packs, editing): (Res<Packs>, Res<Editing>),
    mut preview: ResMut<Preview>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    fills: Query<Entity, With<PreviewFills>>,
    parts: Query<Entity, With<PreviewLines>>,
    current: Res<Current>,
) {
    let key = (editing.resolved_at, editing.level);
    let grid = current.0.grid;
    let built = preview.scene_key == Some(key);
    if built && preview.grid == Some(grid) {
        return;
    }
    if !built {
        let Some(map) = editing.resolved.as_ref() else {
            return;
        };
        preview.scene_key = Some(key);
        preview.scene = preview::scene(&packs.data, map, editing.level);
    }
    preview.grid = Some(grid);
    let Some(mut meshes) = meshes else { return };
    if !built {
        for entity in &fills {
            let handle = meshes.add(mesh(&preview.scene.fills));
            commands.entity(entity).insert(Mesh3d(handle));
        }
    }
    for entity in &parts {
        let strokes = Strokes::of(preview.scene.strokes(grid));
        let handle = meshes.add(lines::mesh(&strokes));
        commands.entity(entity).insert(Mesh3d(handle));
    }
}

/// The preview's lines in the edited map's biosphere, as the game would draw them.
fn paint(
    (packs, editing): (Res<Packs>, Res<Editing>),
    materials: Option<ResMut<Assets<LineMaterial>>>,
    parts: Query<&MeshMaterial3d<LineMaterial>, With<PreviewLines>>,
) {
    let (Some(mut materials), Some(map)) = (materials, editing.resolved.as_ref()) else {
        return;
    };
    let palette = Palette::of(packs.data.biosphere_for(&map.def));
    for part in &parts {
        LineMaterial::paint(&mut materials, &part.0, &palette);
    }
}

/// The ground's crests from where the preview's camera stands, found again when it has moved
/// or the scene changed.
fn crests(
    mut commands: Commands,
    preview: Res<Preview>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    cameras: Query<&Transform, With<PreviewCamera>>,
    part: Query<(Entity, Option<&Mesh3d>), With<PreviewCrests>>,
    mut at: Local<lines::CrestsAt>,
) {
    let (Some(mut meshes), Ok(eye), Ok(part)) = (meshes, cameras.single(), part.single()) else {
        return;
    };
    let reach = render::fog_end(preview::SIGHT);
    let from = (eye.translation.to_array(), reach, preview.is_changed());
    lines::hang_crests(
        &mut commands,
        &mut meshes,
        part,
        &mut at,
        &preview.scene.ground,
        from,
    );
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

/// The height of the picture's node for a column `width` pixels wide: 16:9 inside the padding.
#[must_use]
pub fn picture_height(width: f32) -> f32 {
    (width * 9.0 / 16.0).round()
}
