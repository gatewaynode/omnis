//! A screen stands over the rest of the interface, takes the pointer back from mouse look, and
//! shows a scrollbar beside a panel whose rows run past its foot.

use crate::common;

use bevy::prelude::*;
use bevy::ui_widgets::{Scrollbar, ScrollbarThumb};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use common::app::{app_with, click, tap, top};
use omnis_vector::screen::ScreenId;
use omnis_vector::shell::movement::Intent;
use omnis_vector::shell::screen::{ScreenLayer, Scrolls};

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

/// Each panel's scrollbar and whether it shows, the panel's size set as a layout would.
fn bars(app: &mut App, visible: f32, content: f32) -> Vec<Visibility> {
    let world = app.world_mut();
    let mut panels = world.query_filtered::<(Entity, &mut ComputedNode), With<Scrolls>>();
    let mut targets = Vec::new();
    for (entity, mut node) in panels.iter_mut(world) {
        node.size = Vec2::new(300.0, visible);
        node.content_size = Vec2::new(300.0, content);
        targets.push(entity);
    }
    assert!(!targets.is_empty(), "the screen has a panel");
    app.update();
    let world = app.world_mut();
    let mut bars = world.query::<(&Scrollbar, &Visibility, &Children)>();
    let mut thumbs = world.query_filtered::<(), With<ScrollbarThumb>>();
    targets
        .iter()
        .map(|panel| {
            let found: Vec<_> = bars
                .iter(world)
                .filter(|(bar, _, _)| bar.target == *panel)
                .map(|(_, shown, children)| (*shown, children.to_vec()))
                .collect();
            assert_eq!(found.len(), 1, "one scrollbar for each panel");
            let (shown, children) = found.into_iter().next().unwrap();
            assert!(
                children.iter().any(|c| thumbs.get(world, *c).is_ok()),
                "the bar has its thumb"
            );
            shown
        })
        .collect()
}

#[test]
fn a_panel_shows_a_scrollbar_only_while_its_rows_run_past_its_foot() {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    app.update();
    tap(&mut app, KeyCode::KeyR);
    assert_eq!(top(&app), Some(ScreenId::Camp));
    assert!(
        bars(&mut app, 400.0, 380.0)
            .iter()
            .all(|v| *v == Visibility::Hidden),
        "it fits"
    );
    assert!(
        bars(&mut app, 400.0, 900.0)
            .iter()
            .all(|v| *v == Visibility::Visible),
        "it runs over"
    );
    assert!(
        bars(&mut app, 400.0, 400.0)
            .iter()
            .all(|v| *v == Visibility::Hidden),
        "it fits again"
    );
}
