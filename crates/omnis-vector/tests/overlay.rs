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

    // Closed again, the look is back with no click (the owner, 2026-11-24).
    tap(&mut app, KeyCode::KeyR);
    app.update();
    assert_eq!(top(&app), None);
    assert!(looking(&app), "the look comes back when the camp closes");
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);
}

fn looking(app: &App) -> bool {
    app.world().resource::<Intent>().looking
}

/// A view that looks, on the road.
fn looking_app() -> App {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    app.update();
    click(&mut app, (800.0, 450.0));
    assert!(looking(&app));
    app
}

#[test]
fn the_pause_and_the_actions_lend_the_pointer_and_give_the_look_back() {
    for (key, screen) in [
        (KeyCode::Escape, ScreenId::Pause),
        (KeyCode::Space, ScreenId::Actions),
    ] {
        let mut app = looking_app();
        tap(&mut app, key);
        app.update();
        assert_eq!(top(&app), Some(screen));
        assert!(!looking(&app), "{screen:?} has the pointer");
        assert_eq!(grab(&mut app), CursorGrabMode::None);
        tap(&mut app, KeyCode::Escape);
        app.update();
        assert_eq!(top(&app), None);
        assert!(looking(&app), "the look is back after {screen:?}");
        assert_eq!(grab(&mut app), CursorGrabMode::Locked);
    }
}

#[test]
fn a_look_turned_off_by_a_click_stays_off_after_a_screen() {
    let mut app = looking_app();
    click(&mut app, (800.0, 450.0));
    assert!(!looking(&app), "a second click turns it off");
    tap(&mut app, KeyCode::KeyR);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Camp));
    tap(&mut app, KeyCode::KeyR);
    app.update();
    app.update();
    assert_eq!(top(&app), None);
    assert!(!looking(&app), "nothing was lent, nothing comes back");
    assert_eq!(grab(&mut app), CursorGrabMode::None);
}

#[test]
fn a_new_start_forgets_a_lent_look() {
    let mut app = looking_app();
    tap(&mut app, KeyCode::Escape);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Pause));
    // A load, a new game and a return to the title all start the screens anew.
    let views = app
        .world()
        .resource::<omnis_vector::shell::session::Session>()
        .views
        .clone();
    app.world_mut()
        .resource_mut::<omnis_vector::shell::screen::Screens>()
        .reset(Some(&views));
    app.update();
    app.update();
    assert_eq!(top(&app), None);
    assert!(!looking(&app), "a new start does not take the pointer");
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
