//! The view's lines as a mesh entity (rendering size 1), headless: the scene's lines hang on
//! one entity as a mesh of quads under the line
//! material; the material's fog follows the party's sight and its width the quality; the grid
//! setting and a reshape rebuild the mesh; the crests are a second mesh, found again as the eye
//! moves. What the shader draws is compared by capture, not here.

use crate::common;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use common::app::{config, shell};
use omnis_vector::scene::Scene;
use omnis_vector::settings::Quality;
use omnis_vector::shell::lines::{
    ATTRIBUTE_COLOUR, ATTRIBUTE_CORNER, ATTRIBUTE_END, ATTRIBUTE_STYLE, LineMaterial,
};
use omnis_vector::shell::render::{RenderPlugin, ViewCamera, ViewCrests, ViewLines, ViewMaterial};
use omnis_vector::shell::session::Session;
use omnis_vector::shell::settings::Current;
use omnis_vector::strokes::Strokes;

/// The headless shell in a game, with the view's renderer and the assets it keeps.
fn view() -> App {
    let mut app = shell(config(), true);
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(RenderPlugin);
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
    let mut app = view();
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
    // The mesh carries each line's weight and pattern as the strokes hold them: the town has
    // walls at the plain style and a floor grid at the hairline.
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(styles)) =
        mesh.attribute(ATTRIBUTE_STYLE)
    else {
        panic!("the style is four floats a vertex");
    };
    let held = {
        let s = world.resource::<Session>();
        let pos = s.views.position;
        Strokes::of(Scene::build(&s.data, pos.map, pos.level, &s.world).strokes(true)).style
    };
    assert_eq!(styles, &held);
    assert!(styles.contains(&[1.0, 0.0, 0.0, 0.0]) && styles.contains(&[0.5, 0.0, 0.0, 0.0]));
    assert_eq!(
        mesh.indices().map(bevy::mesh::Indices::len),
        Some(lines * 6)
    );
}

#[test]
fn the_fog_follows_the_sight_and_the_width_the_quality() {
    let mut app = view();
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
    let mut app = view();
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

/// The view on the hills fixture (relief, so crests), the eye snapping to its height.
fn hills(x: u16, y: u16) -> App {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/heights"));
    let mut app = shell(
        omnis_vector::shell::session::Config { packs, ..config() },
        true,
    );
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(RenderPlugin);
    app.update();
    app.world_mut().resource_mut::<Current>().0.reduced_motion = true;
    stand(&mut app, x, y);
    app
}

/// Put the party on `(x, y)` of the hills, facing east, and let the view follow.
fn stand(app: &mut App, x: u16, y: u16) {
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "heights:map:hills");
        session.pose = common::place(
            &mut session.world,
            map,
            x,
            y,
            omnis_sim::omnis_core::Facing::East,
        );
        session.refresh();
    }
    app.update();
    app.update();
}

/// The crest mesh's handle and vertex count; `None` while the entity has no mesh.
fn crest_mesh(app: &mut App) -> Option<(Handle<Mesh>, usize)> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Option<&Mesh3d>, With<ViewCrests>>();
    let handle = q.single(world).expect("the crest entity")?.0.clone();
    let count = world
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .expect("the mesh")
        .count_vertices();
    Some((handle, count))
}

/// How many of the crest mesh's vertices carry the plain style: all of them, or the test fails.
fn crest_styles(app: &mut App) -> usize {
    let Some((handle, count)) = crest_mesh(app) else {
        return 0;
    };
    let meshes = app.world().resource::<Assets<Mesh>>();
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(styles)) =
        meshes.get(&handle).unwrap().attribute(ATTRIBUTE_STYLE)
    else {
        panic!("the style is four floats a vertex");
    };
    assert!(styles.iter().all(|s| *s == omnis_vector::strokes::PLAIN));
    assert_eq!(styles.len(), count);
    count
}

/// How many crests the ground shows from where the camera stands.
fn crests_seen(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Transform, With<ViewCamera>>();
    let eye = q.single(world).expect("the camera").translation.to_array();
    let s = world.resource::<Session>();
    let pos = s.views.position;
    let scene = Scene::build(&s.data, pos.map, pos.level, &s.world);
    let reach = f32::from(s.views.sight) + 2.0;
    // The ground's own answer, not the renderer's wrapping of it.
    scene.ground.crests(eye, reach).len()
}

#[test]
fn the_crests_are_found_from_the_eye_and_again_only_when_it_moves() {
    // Somewhere on the hills that shows a crest, and somewhere that shows a different count.
    let cells: Vec<(u16, u16)> = (0..6).flat_map(|y| (0..8).map(move |x| (x, y))).collect();
    let mut app = hills(0, 4);
    let mut seen = Vec::new();
    for &(x, y) in &cells {
        let on_map = {
            let s = app.world().resource::<Session>();
            let map = common::map(&s.data, "heights:map:hills");
            s.data.maps[&map]
                .cell(omnis_sim::omnis_core::Level::GROUND, x, y)
                .is_some()
        };
        if !on_map {
            continue;
        }
        stand(&mut app, x, y);
        let n = crests_seen(&mut app);
        assert_eq!(crest_styles(&mut app), n * 4, "crests are plain lines");
        // No crest, no mesh: an empty one is never hung.
        let drawn = crest_mesh(&mut app).map(|(_, count)| count);
        assert_eq!(drawn, (n > 0).then_some(n * 4), "at ({x}, {y})");
        seen.push(((x, y), n));
    }
    let (with, n) = *seen
        .iter()
        .find(|(_, n)| *n > 0)
        .expect("a crest somewhere");
    let (other, _) = *seen
        .iter()
        .find(|(_, m)| *m != n && *m > 0)
        .expect("a different view with crests");
    assert!(seen.iter().any(|(_, m)| *m == 0), "and a view with none");

    // Standing still, the crests are not found again: a mesh put in their place stays.
    stand(&mut app, with.0, with.1);
    let (handle, count) = crest_mesh(&mut app).expect("crests here");
    assert_eq!(count, n * 4);
    let one = Strokes::of([(
        &omnis_vector::geometry::Segment {
            a: [0.0; 3],
            b: [1.0; 3],
            kind: omnis_vector::geometry::SegKind::Wall,
            rgb: (0, 0, 0),
        },
        1.0,
    )]);
    app.world_mut()
        .resource_mut::<Assets<Mesh>>()
        .insert(handle.id(), omnis_vector::shell::lines::mesh(&one))
        .expect("the handle lives");
    app.update();
    app.update();
    assert_eq!(
        crest_mesh(&mut app).map(|(_, count)| count),
        Some(4),
        "nothing moved: left alone"
    );
    // The eye moves: found again, in the same mesh.
    stand(&mut app, other.0, other.1);
    stand(&mut app, with.0, with.1);
    let (again, count) = crest_mesh(&mut app).expect("crests again");
    assert_eq!(count, n * 4);
    assert_eq!(again, handle, "replaced in place");
}
