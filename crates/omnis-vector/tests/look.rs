//! Looking up and down (M8.5 step 4b, presentation-ARCHITECTURE §8.6, VA7): the mouse and the
//! keys pitch the view, stopping just short of straight up and down (B8, 2026-10-22), the body
//! walks and faces as before, and a step returns the view to level as the setting says.

use crate::common;

use bevy::input::ButtonState;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use common::app::{app, key, set};
use omnis_vector::look::{LevelOnStep, PITCH_LIMIT};
use omnis_vector::shell::controls::Action;
use omnis_vector::shell::movement::{Intent, View};
use omnis_vector::shell::render::rotation;
use omnis_vector::shell::session::Session;
use omnis_vector::shell::settings::Current;

/// The shell on the meadow, settled, with the setting `policy` and reduced motion `snap`.
fn shell(policy: LevelOnStep, snap: bool) -> App {
    let mut app = app();
    {
        let mut current = app.world_mut().resource_mut::<Current>();
        current.0.level_on_step = policy;
        current.0.reduced_motion = snap;
    }
    frames(&mut app, 30);
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn pitch(app: &App) -> f32 {
    app.world().resource::<View>().pitch
}

fn set_pitch(app: &mut App, p: f32) {
    app.world_mut().resource_mut::<View>().pitch = p;
}

/// The mouse moved by `(dx, dy)` pixels this frame, as the window would send it.
fn mouse(app: &mut App, dx: f32, dy: f32) {
    app.world_mut().write_message(MouseMotion {
        delta: Vec2::new(dx, dy),
    });
}

fn looking(app: &mut App, on: bool) {
    app.world_mut().resource_mut::<Intent>().looking = on;
}

/// Hold W until the party enters the next cell, then let go: one step.
fn step(app: &mut App) {
    let start = app.world().resource::<Session>().world.position;
    key(app, KeyCode::KeyW, ButtonState::Pressed);
    for _ in 0..120 {
        app.update();
        if app.world().resource::<Session>().world.position != start {
            break;
        }
    }
    key(app, KeyCode::KeyW, ButtonState::Released);
    app.update();
    assert_ne!(
        app.world().resource::<Session>().world.position,
        start,
        "the party stepped"
    );
}

#[test]
fn the_camera_looks_up_with_a_positive_pitch_and_stays_upright_at_the_limit() {
    let ahead = |yaw: f32, p: f32| rotation(yaw, p) * Vec3::NEG_Z;
    let level = ahead(0.0, 0.0);
    assert!(
        level.y.abs() < 1e-6 && level.z < -0.99,
        "level, north: {level}"
    );
    assert!(ahead(0.0, 0.4).y > 0.3, "up");
    assert!(ahead(0.0, -0.4).y < -0.3, "down");
    let at_limit = ahead(0.0, PITCH_LIMIT);
    assert!(
        at_limit.y > 0.999 && at_limit.z < 0.0,
        "at the limit: nearly straight up, still ahead: {at_limit}"
    );
    let up = rotation(0.0, PITCH_LIMIT) * Vec3::Y;
    assert!(up.y > 0.0, "and upright: {up}");
    // The pitch tips about the body's own right, whichever way it faces.
    let east = ahead(-core::f32::consts::FRAC_PI_2, 0.4);
    assert!(east.x > 0.8 && east.y > 0.3, "east and up: {east}");
}

#[test]
fn the_mouse_and_page_keys_pitch_the_view_and_home_and_the_button_level_it() {
    let mut app = shell(LevelOnStep::Unlocked, false);
    looking(&mut app, true);
    // 100 pixels up at 0.003 radians a pixel.
    mouse(&mut app, 0.0, -100.0);
    app.update();
    assert!(
        (pitch(&app) - 0.3).abs() < 1e-5,
        "the mouse: {}",
        pitch(&app)
    );

    key(&mut app, KeyCode::Home, ButtonState::Pressed);
    app.update();
    key(&mut app, KeyCode::Home, ButtonState::Released);
    assert_eq!(pitch(&app), 0.0, "Home levels");

    key(&mut app, KeyCode::PageUp, ButtonState::Pressed);
    frames(&mut app, 10);
    key(&mut app, KeyCode::PageUp, ButtonState::Released);
    app.update();
    assert!(pitch(&app) > 0.2, "PageUp looks up: {}", pitch(&app));

    set(&mut app, &Action::Level, Interaction::Pressed);
    app.update();
    set(&mut app, &Action::Level, Interaction::None);
    assert_eq!(pitch(&app), 0.0, "the Level button levels");

    key(&mut app, KeyCode::PageDown, ButtonState::Pressed);
    frames(&mut app, 10);
    assert!(pitch(&app) < -0.2, "PageDown looks down: {}", pitch(&app));
}

#[test]
fn the_pitch_stops_just_short_of_straight_up_and_down() {
    let mut app = shell(LevelOnStep::Never, false);
    looking(&mut app, true);
    // 14 frames of 160 pixels up: far past straight overhead if nothing stopped it.
    for _ in 0..14 {
        mouse(&mut app, 0.0, -160.0);
        app.update();
    }
    let up = pitch(&app);
    assert_eq!(up, PITCH_LIMIT, "held just short of straight up");
    let ahead = rotation(0.0, up) * Vec3::NEG_Z;
    assert!(ahead.z < 0.0, "still looking forward: {ahead}");
    assert!((rotation(0.0, up) * Vec3::Y).y > 0.0, "still upright");
    for _ in 0..28 {
        mouse(&mut app, 0.0, 160.0);
        app.update();
    }
    let down = pitch(&app);
    assert_eq!(down, -PITCH_LIMIT, "held just short of straight down");
    // The keys stop at the same place.
    key(&mut app, KeyCode::PageUp, ButtonState::Pressed);
    frames(&mut app, 240);
    key(&mut app, KeyCode::PageUp, ButtonState::Released);
    assert_eq!(pitch(&app), up, "PageUp held stops where the mouse did");
}

#[test]
fn the_mouse_turns_the_body_the_same_way_at_any_pitch() {
    let turned = |p: f32| {
        let mut app = shell(LevelOnStep::Never, false);
        looking(&mut app, true);
        set_pitch(&mut app, p);
        let before = app.world().resource::<Session>().pose.yaw;
        // 33⅓ pixels left: a tenth of a radian.
        mouse(&mut app, -100.0 / 3.0, 0.0);
        app.update();
        app.world().resource::<Session>().pose.yaw - before
    };
    assert!((turned(0.3) - 0.1).abs() < 1e-4, "level-ish");
    assert!((turned(PITCH_LIMIT) - 0.1).abs() < 1e-4, "at the top");
    assert!((turned(-PITCH_LIMIT) - 0.1).abs() < 1e-4, "at the bottom");
}

#[test]
fn a_step_levels_the_view_unless_the_mouse_is_looking() {
    let mut app = shell(LevelOnStep::Unlocked, false);
    set_pitch(&mut app, 0.6);
    step(&mut app);
    assert!(
        pitch(&app) < 0.6 && pitch(&app) > 0.0,
        "easing: {}",
        pitch(&app)
    );
    frames(&mut app, 90);
    assert_eq!(pitch(&app), 0.0, "level after the ease");

    let mut app = shell(LevelOnStep::Unlocked, true);
    set_pitch(&mut app, -0.6);
    step(&mut app);
    assert_eq!(pitch(&app), 0.0, "reduced motion: at once");

    let mut app = shell(LevelOnStep::Unlocked, false);
    looking(&mut app, true);
    set_pitch(&mut app, 0.6);
    step(&mut app);
    frames(&mut app, 60);
    assert_eq!(pitch(&app), 0.6, "mouse look keeps it");
}

#[test]
fn always_levels_while_looking_and_never_keeps_the_view() {
    let mut app = shell(LevelOnStep::Always, true);
    looking(&mut app, true);
    set_pitch(&mut app, 0.6);
    step(&mut app);
    assert_eq!(pitch(&app), 0.0, "always");

    let mut app = shell(LevelOnStep::Never, true);
    set_pitch(&mut app, 0.6);
    step(&mut app);
    frames(&mut app, 60);
    assert_eq!(pitch(&app), 0.6, "never");
}

#[test]
fn a_turn_without_a_step_keeps_the_view() {
    let mut app = shell(LevelOnStep::Unlocked, true);
    set_pitch(&mut app, 0.6);
    key(&mut app, KeyCode::KeyQ, ButtonState::Pressed);
    app.update();
    key(&mut app, KeyCode::KeyQ, ButtonState::Released);
    frames(&mut app, 60);
    assert_eq!(
        app.world().resource::<Session>().world.position.facing,
        omnis_sim::omnis_core::Facing::West
    );
    assert_eq!(pitch(&app), 0.6);
}

#[test]
fn the_pitch_never_reaches_the_body_or_the_rules() {
    // The view pushed to its limit, then a walk: the same cells and pose as with no pitch at all.
    let walk = |loop_over: bool| {
        let mut app = shell(LevelOnStep::Never, false);
        if loop_over {
            looking(&mut app, true);
            for _ in 0..14 {
                mouse(&mut app, 0.0, -160.0);
                app.update();
            }
            looking(&mut app, false);
            assert!(
                pitch(&app).abs() > 0.1,
                "somewhere off level: {}",
                pitch(&app)
            );
        }
        key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
        frames(&mut app, 60);
        key(&mut app, KeyCode::KeyW, ButtonState::Released);
        frames(&mut app, 60);
        let session = app.world().resource::<Session>();
        (
            session.world.position,
            session.pose,
            session.binder.log.len(),
        )
    };
    assert_eq!(walk(true), walk(false));
}

#[test]
fn a_new_game_starts_level() {
    let mut app = shell(LevelOnStep::Never, false);
    set_pitch(&mut app, 0.6);
    app.update();
    assert_eq!(pitch(&app), 0.6);
    let fresh = Session::start(&common::app::config()).expect("a session");
    app.insert_resource(fresh);
    app.update();
    assert_eq!(pitch(&app), 0.0);
}
