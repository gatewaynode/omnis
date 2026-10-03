//! The extended WASD keys and the action menu, headless (alt-ARCHITECTURE.md §8): E turns right,
//! Space opens the square's actions, Shift+Space runs the default action or opens the menu, and
//! the menu lists only what the simulation would accept.

mod common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{app, key, set};
use common::map;
use omnis_sim::omnis_core::{Facing, Position};
use omnis_vector::pose::Pose;
use omnis_vector::shell::actions::{Menu, available, menu};
use omnis_vector::shell::controls::Action;
use omnis_vector::shell::notice::Order;
use omnis_vector::shell::panel::{Showing, current};
use omnis_vector::shell::session::Session;

fn tap(app: &mut App, code: KeyCode) {
    key(app, code, ButtonState::Pressed);
    app.update();
    key(app, code, ButtonState::Released);
    app.update();
}

fn shift_space(app: &mut App) {
    key(app, KeyCode::ShiftLeft, ButtonState::Pressed);
    tap(app, KeyCode::Space);
    key(app, KeyCode::ShiftLeft, ButtonState::Released);
    app.update();
}

fn open(app: &App) -> bool {
    app.world().resource::<Menu>().open
}

/// An app with the party at the dungeon's (3, 5), facing the closed door on its south side.
fn at_the_door() -> App {
    let mut app = app();
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = map(&session.data, "test:map:dungeon");
        session.world.position = Position {
            map: dungeon,
            x: 3,
            y: 5,
            facing: Facing::South,
        };
        session.pose = Pose::at(session.world.position);
    }
    app.update();
    app
}

fn door_open(app: &App) -> bool {
    let session = app.world().resource::<Session>();
    let p = session.world.position;
    session.world.maps[&p.map].door_open(p.x, p.y, p.facing)
}

fn labels(app: &App) -> Vec<String> {
    let session = app.world().resource::<Session>();
    current(session, open(app))
        .map(|n| n.choices.into_iter().map(|c| c.label).collect())
        .unwrap_or_default()
}

#[test]
fn e_turns_the_party_right_by_a_quarter() {
    let mut app = app();
    app.update();
    tap(&mut app, KeyCode::KeyE);
    for _ in 0..60 {
        app.update();
    }
    let facing = app.world().resource::<Session>().world.position.facing;
    assert_eq!(facing, Facing::East);
}

#[test]
fn space_opens_the_squares_actions_and_a_choice_runs_and_closes_it() {
    let mut app = at_the_door();
    tap(&mut app, KeyCode::Space);
    assert!(open(&app));
    assert!(app.world().resource::<Showing>().0, "the panel is up");
    assert_eq!(labels(&app), ["Open the door", "Close"]);
    tap(&mut app, KeyCode::Digit1);
    assert!(door_open(&app), "the door opened");
    assert!(!open(&app), "choosing closes the menu");
    // Opened again, the same door offers to close.
    tap(&mut app, KeyCode::Space);
    assert_eq!(labels(&app), ["Close the door", "Close"]);
    // Space again closes the menu without acting.
    tap(&mut app, KeyCode::Space);
    assert!(!open(&app));
    assert!(door_open(&app));
    assert_eq!(app.world().resource::<Session>().binder.refusals, 0);
}

#[test]
fn shift_space_runs_the_default_action_when_there_is_one() {
    let mut app = at_the_door();
    shift_space(&mut app);
    assert!(door_open(&app), "the door opened at once");
    assert!(!open(&app), "no menu");
}

#[test]
fn shift_space_with_nothing_to_do_opens_the_menu_saying_so() {
    let mut app = app();
    app.update();
    {
        let session = app.world().resource::<Session>();
        assert!(available(&session.world, &session.data).is_empty());
        let shown = menu(&session.world, &session.data);
        assert_eq!(shown.lines, ["Nothing to do here"]);
    }
    shift_space(&mut app);
    assert!(open(&app));
    assert_eq!(labels(&app), ["Close"]);
    // The Close button closes it.
    set(&mut app, &Order::Close, Interaction::Pressed);
    app.update();
    assert!(!open(&app));
    assert_eq!(app.world().resource::<Session>().binder.refusals, 0);
}

#[test]
fn the_actions_button_toggles_the_menu_and_moving_or_esc_closes_it() {
    let mut app = at_the_door();
    set(&mut app, &Action::Actions, Interaction::Pressed);
    app.update();
    assert!(open(&app));
    set(&mut app, &Action::Actions, Interaction::None);
    app.update();
    tap(&mut app, KeyCode::Escape);
    assert!(!open(&app), "Esc closes it");
    tap(&mut app, KeyCode::Space);
    assert!(open(&app));
    // Walking north into the next cell closes the menu.
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.pose.yaw = 0.0;
    }
    set(&mut app, &Action::Forward, Interaction::Pressed);
    for _ in 0..40 {
        app.update();
    }
    set(&mut app, &Action::Forward, Interaction::None);
    assert_ne!(app.world().resource::<Session>().world.position.y, 5);
    assert!(!open(&app), "moving closes it");
}
