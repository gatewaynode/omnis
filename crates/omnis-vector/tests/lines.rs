//! The view's lines as a mesh entity (rendering size 1, step 2), headless: with the mesh
//! renderer chosen the scene's lines hang on one entity as a mesh of quads under the line
//! material; the material's fog follows the party's sight and its width the quality; the grid
//! setting and a reshape rebuild the mesh. (The gizmo renderer, kept beside it until the swap,
//! needs a GPU-side plugin and is compared by capture.)

use crate::common;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use common::app::{config, shell};
use omnis_vector::scene::Scene;
use omnis_vector::settings::Quality;
use omnis_vector::shell::lines::{
    ATTRIBUTE_COLOUR, ATTRIBUTE_CORNER, ATTRIBUTE_END, ATTRIBUTE_STYLE, LineMaterial, LinesPlugin,
};
use omnis_vector::shell::render::{LineRenderer, RenderPlugin, ViewLines, ViewMaterial};
use omnis_vector::shell::session::Session;
use omnis_vector::shell::settings::Current;
use omnis_vector::strokes::Strokes;

/// The headless shell in a game, with the view's renderer and the assets it keeps.
fn view(renderer: LineRenderer) -> App {
    let mut app = shell(config(), true);
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(renderer)
        .add_plugins((LinesPlugin, RenderPlugin));
    app.update();
    app.update();
    app
}

/// The line mesh's vertex count, when the entity has a mesh.
fn vertices(app: &mut App) -> Option<usize> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Option<&Mesh3d>, With<ViewLines>>();
    let handle = q.single(world).expect("one line entity")?.0.clone();
    let meshes = world.resource::<Assets<Mesh>>();
    Some(meshes.get(&handle).expect("the mesh").count_vertices())
}

/// How many lines the session's scene has with the grid shown or hidden.
fn expected(app: &App, grid: bool) -> usize {
    let s = app.world().resource::<Session>();
    let pos = s.views.position;
    let scene = Scene::build(&s.data, pos.map, pos.level, &s.world);
    Strokes::of(scene.strokes(grid)).len()
}

fn material(app: &App) -> LineMaterial {
    let handle = app.world().resource::<ViewMaterial>().0.clone();
    app.world()
        .resource::<Assets<LineMaterial>>()
        .get(&handle)
        .expect("the view's material")
        .clone()
}

#[test]
fn the_scene_s_lines_hang_on_one_entity_as_quads_under_the_line_material() {
    let mut app = view(LineRenderer::Mesh);
    let lines = expected(&app, true);
    assert!(lines > 0);
    assert_eq!(vertices(&mut app), Some(lines * 4));
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Mesh3d, &MeshMaterial3d<LineMaterial>), With<ViewLines>>();
    let (mesh, held) = q.single(world).expect("the line entity");
    let (mesh, held) = (mesh.0.clone(), held.0.clone());
    assert_eq!(held, world.resource::<ViewMaterial>().0);
    let mesh = world.resource::<Assets<Mesh>>().get(&mesh).unwrap();
    for attribute in [
        Mesh::ATTRIBUTE_POSITION,
        ATTRIBUTE_END,
        ATTRIBUTE_CORNER,
        ATTRIBUTE_COLOUR,
        ATTRIBUTE_STYLE,
    ] {
        assert!(mesh.attribute(attribute).is_some(), "{attribute:?}");
    }
    assert_eq!(
        mesh.indices().map(bevy::mesh::Indices::len),
        Some(lines * 6)
    );
}

#[test]
fn the_fog_follows_the_sight_and_the_width_the_quality() {
    let mut app = view(LineRenderer::Mesh);
    let sight = f32::from(app.world().resource::<Session>().views.sight);
    let m = material(&app);
    assert_eq!(m.fog_end(), sight + 2.0, "two cells beyond the sight");
    assert_eq!(m.width(), Quality::default().line_width());
    assert!(m.params.z < 0.0, "lifted toward the eye");

    app.world_mut().resource_mut::<Current>().0.quality = Quality::Low;
    app.update();
    let m = material(&app);
    assert_eq!(m.fog_end(), sight + 2.0);
    assert_eq!(m.width(), Quality::Low.line_width());
    assert_ne!(Quality::Low.line_width(), Quality::default().line_width());
}

#[test]
fn the_grid_setting_and_a_reshape_rebuild_the_mesh() {
    let mut app = view(LineRenderer::Mesh);
    let (shown, hidden) = (expected(&app, true), expected(&app, false));
    assert!(hidden < shown);
    assert_eq!(vertices(&mut app), Some(shown * 4));
    app.world_mut().resource_mut::<Current>().0.grid = false;
    app.update();
    app.update();
    assert_eq!(vertices(&mut app), Some(hidden * 4), "without the grids");

    // A reshape builds a new mesh: the handle changes though the scene is the same.
    let handle = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&Mesh3d, With<ViewLines>>();
        q.single(world).expect("the line entity").0.clone()
    };
    let before = handle(&mut app);
    app.update();
    assert_eq!(handle(&mut app), before, "nothing changed: nothing rebuilt");
    app.world_mut().resource_mut::<Session>().reshape = true;
    app.update();
    app.update();
    assert_ne!(handle(&mut app), before);
    assert_eq!(vertices(&mut app), Some(hidden * 4));
}
