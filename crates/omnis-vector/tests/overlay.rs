//! A screen stands over the rest of the interface and takes the pointer back from mouse look.

use crate::common;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use common::app::{app_with, click, tap, top};
use omnis_vector::screen::ScreenId;
use omnis_vector::shell::movement::Intent;
use omnis_vector::shell::screen::ScreenLayer;

fn grab(app: &mut App) -> CursorGrabMode {
    let world = app.world_mut();
    let mut cursor = world.query_filtered::<&CursorOptions, With<PrimaryWindow>>();
    cursor.single(world).expect("the window's cursor").grab_mode
}

#[test]
fn opening_the_camp_while_the_mouse_looks_gives_the_pointer_back() {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    app.update();
    click(&mut app, (800.0, 450.0));
    assert!(
        app.world().resource::<Intent>().looking,
        "a click in the view looks"
    );
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);

    tap(&mut app, KeyCode::KeyR);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Camp));
    assert!(
        !app.world().resource::<Intent>().looking,
        "the camp needs the pointer"
    );
    assert_eq!(grab(&mut app), CursorGrabMode::None);
}

#[test]
fn the_screen_layer_draws_over_every_other_root() {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    let world = app.world_mut();
    let mut layer = world.query_filtered::<Option<&GlobalZIndex>, With<ScreenLayer>>();
    let above = layer
        .single(world)
        .expect("the screen layer")
        .map_or(0, |z| z.0);
    let mut roots = world.query_filtered::<(Entity, Option<&GlobalZIndex>), (
        With<Node>,
        Without<ChildOf>,
        Without<ScreenLayer>,
    )>();
    let others: Vec<(Entity, i32)> = roots
        .iter(world)
        .map(|(e, z)| (e, z.map_or(0, |z| z.0)))
        .collect();
    assert!(
        others.len() > 2,
        "the column, the buttons and the message stand beside it"
    );
    for (entity, z) in others {
        assert!(z < above, "{entity} draws at {z}, the screens at {above}");
    }
}
