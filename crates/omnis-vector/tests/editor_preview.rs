//! The editor's preview (M5.7): the edited map's scene built with no world is the game's scene
//! with every door shut, follows a paint, shows the yard's objects (hidden ones too) and its
//! standing group, and looks from the cursor the way it faces.

use crate::common;

use common::editor::{editing, in_editor, in_editor_on};
use omnis_sim::Settings;
use omnis_sim::omnis_core::{Facing, Level, Position};
use omnis_sim::omnis_data::edit::EdgeKind;
use omnis_vector::editor::brush::{Target, Tool};
use omnis_vector::editor::preview;
use omnis_vector::geom::yaw_of;
use omnis_vector::geometry::SegKind;
use omnis_vector::pose::Pose;
use omnis_vector::scene::Scene;
use omnis_vector::shell::editor::Editing;
use omnis_vector::shell::session::Packs;
use omnis_vector::sim::SimWorld;

#[test]
fn the_preview_s_scene_is_the_game_s_with_every_door_shut_and_looks_from_the_cursor() {
    let app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let map = editing(&app).resolved.clone().expect("the town resolves");
    let scene = preview::scene(&data, &map, Level::GROUND);
    assert!(!scene.own.is_empty() && !scene.fills.is_empty());
    // A fresh game has every door shut: the game's scene of the same map is the same scene.
    let world = SimWorld::new(omnis_sim::World::new(&data, 7, Settings::default()).unwrap());
    let id = data.registry.maps.get("test:map:town").unwrap();
    let game = Scene::build(&data, id, Level::GROUND, &world);
    assert_eq!(scene.own, game.own);
    assert_eq!(scene.neighbours, game.neighbours);
    assert_eq!(scene.fills.len(), game.fills.len());
    // The pose is the game's for the same cell and facing; the eye stands over the ground.
    let pose = preview::pose((10, 2), Facing::West);
    assert_eq!(
        pose,
        Pose::at(Position {
            map: id,
            level: Level::GROUND,
            x: 10,
            y: 2,
            facing: Facing::West,
        })
    );
    let (eye, yaw) = preview::look(&scene, pose);
    assert!(
        eye[1] > 0.0 && eye[1] < 2.0,
        "the eye stands over the ground: {eye:?}"
    );
    assert_eq!((eye[0], eye[2]), (10.5, 2.5));
    assert_eq!(yaw, yaw_of(Facing::West));
}

#[test]
fn a_painted_door_shows_in_the_preview_shut() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let before = {
        let map = editing(&app).resolved.clone().unwrap();
        preview::scene(&data, &map, Level::GROUND)
    };
    let doors = |scene: &Scene| scene.own.iter().filter(|s| s.kind == SegKind::Door).count();
    // An open interior edge gets a door.
    let cell = {
        let e = editing(&app);
        let grid = e.grid().unwrap();
        let (w, h) = e.size();
        (0..h)
            .flat_map(|y| (0..w.saturating_sub(1)).map(move |x| (x, y)))
            .find(|&(x, y)| grid.edge(x, y, Facing::East) == Some(EdgeKind::Open))
            .expect("an open interior edge")
    };
    {
        let mut editing = app.world_mut().resource_mut::<Editing>();
        editing.tool = Tool::Edge(EdgeKind::Door);
        assert!(editing.paint(
            &data,
            Target {
                cell,
                edge: Some(Facing::East)
            },
            false
        ));
    }
    app.update();
    let map = editing(&app).resolved.clone().expect("still resolves");
    let after = preview::scene(&data, &map, Level::GROUND);
    assert!(
        doors(&after) > doors(&before),
        "{} > {}",
        doors(&after),
        doors(&before)
    );
    // Shut: a closed door's frame carries its cross, two lines more than an open one.
    let world = SimWorld::new(omnis_sim::World::new(&data, 7, Settings::default()).unwrap());
    assert!(!world.door_open(
        data.registry.maps.get("test:map:town").unwrap(),
        Level::GROUND,
        cell.0,
        cell.1,
        Facing::East
    ));
}

#[test]
fn the_yard_s_objects_hidden_ones_too_and_its_standing_group_are_in_the_preview() {
    let app = in_editor_on(
        &["crates/omnis-sim/tests/packs/objects"],
        "objects:map:yard",
    );
    let data = app.world().resource::<Packs>().data.clone();
    let map = editing(&app).resolved.clone().expect("the yard resolves");
    let scene = preview::scene(&data, &map, Level::GROUND);
    assert!(
        scene.own.iter().any(|s| s.kind == SegKind::Object),
        "objects drawn"
    );
    assert!(
        scene.own.iter().any(|s| s.kind == SegKind::Being),
        "the rat drawn"
    );
    assert!(
        scene.own.iter().any(|s| s.kind == SegKind::Pile),
        "the pile drawn"
    );
    // Against the game's view from a fresh world: the hidden lever and rune are drawn here only.
    let world = SimWorld::new(omnis_sim::World::new(&data, 7, Settings::default()).unwrap());
    let id = data.registry.maps.get("objects:map:yard").unwrap();
    let game = Scene::build(&data, id, Level::GROUND, &world);
    let objects = |scene: &Scene| {
        scene
            .own
            .iter()
            .filter(|s| s.kind == SegKind::Object)
            .count()
    };
    assert!(
        objects(&scene) > objects(&game),
        "{} > {}",
        objects(&scene),
        objects(&game)
    );
    // The loft is the level above: its crate is in that level's preview.
    let loft = preview::scene(&data, &map, Level(1));
    assert!(loft.own.iter().any(|s| s.kind == SegKind::Object));
}
