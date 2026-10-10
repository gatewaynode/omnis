//! The screen kit (presentation-ARCHITECTURE §6): descriptions, `apply`'s refusals, the overlays
//! following the world, and the renderer's text as agents read it.

use crate::common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{app, key};
use common::fight::met;
use omnis_sim::omnis_core::Facing;
use omnis_vector::screen::nav::{Escape, Nav, base};
use omnis_vector::screen::{self, ControlId, Payload, Refused, ScreenId};
use omnis_vector::shell::screen::ctx;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Session;
use omnis_vector::sim::Views;

fn tap(app: &mut App, code: KeyCode) {
    key(app, code, ButtonState::Pressed);
    app.update();
    key(app, code, ButtonState::Released);
    app.update();
}

/// A session at the dungeon door: the square offers "Open the door".
fn at_the_door() -> Session {
    let mut app = app();
    app.update();
    let mut session = app
        .world_mut()
        .remove_resource::<Session>()
        .expect("a session");
    let dungeon = common::map(&session.data, "test:map:dungeon");
    session.pose = common::place(&mut session.world, dungeon, 3, 5, Facing::South);
    session.refresh();
    session
}

#[test]
fn apply_refuses_a_control_the_screen_does_not_have() {
    let session = at_the_door();
    let ctx = ctx(&session);
    assert_eq!(
        screen::apply(ScreenId::Actions, &ctx, ControlId::Leave, &Payload::Press),
        Err(Refused::NotHere)
    );
    assert_eq!(
        screen::apply(
            ScreenId::Actions,
            &ctx,
            ControlId::Action(7),
            &Payload::Press
        ),
        Err(Refused::NotHere),
        "only the actions the square offers"
    );
}

#[test]
fn the_shape_ignores_words_and_follows_controls() {
    let mut session = at_the_door();
    let closed = screen::content(ScreenId::Actions, &ctx(&session));
    session.order(omnis_sim::Command::Interact);
    let open = screen::content(ScreenId::Actions, &ctx(&session));
    assert_ne!(closed, open, "Open the door became Close the door");
    assert_eq!(closed.shape(), open.shape(), "the same controls");
    // Facing a wall, the square offers nothing: one control fewer.
    session.order(omnis_sim::Command::Turn(
        omnis_sim::omnis_core::Rotation::Left,
    ));
    let none = screen::content(ScreenId::Actions, &ctx(&session));
    assert_ne!(none.shape(), open.shape());
}

fn views(session: &Session) -> Views {
    session.views.clone()
}

#[test]
fn overlays_follow_the_world() {
    let mut session = at_the_door();
    let mut nav = Nav::default();
    nav.open(ScreenId::Actions, &views(&session));
    assert_eq!(nav.top(&views(&session)), Some(ScreenId::Actions));
    // Settling without a move keeps it; a move closes it.
    nav.settle(&views(&session), false);
    assert!(nav.is_open(ScreenId::Actions));
    nav.settle(&views(&session), true);
    assert!(!nav.is_open(ScreenId::Actions));
    // A fallen party: the base is the defeat screen and the actions cannot open over it.
    for member in &mut session.world.party.members {
        member.hp = 0;
    }
    session.refresh();
    assert_eq!(base(&views(&session)), Some(ScreenId::Defeat));
    nav.open(ScreenId::Actions, &views(&session));
    assert!(!nav.is_open(ScreenId::Actions));
}

#[test]
fn a_fight_closes_the_actions_and_esc_steps_its_menu_back() {
    let session = met();
    let mut nav = Nav::default();
    assert_eq!(base(&session.views), None, "the fight screen draws itself");
    nav.open(ScreenId::Actions, &session.views);
    assert!(!nav.is_open(ScreenId::Actions), "not over a fight");
    assert_eq!(nav.escape(&session.views), Escape::FightBack);
    let calm = at_the_door();
    nav.open(ScreenId::Actions, &calm.views);
    assert_eq!(nav.escape(&calm.views), Escape::Closed);
    assert_eq!(nav.escape(&calm.views), Escape::Nothing);
}

#[test]
fn the_screen_text_lists_the_open_screens_controls_with_their_keys() {
    let mut app = app();
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = common::map(&session.data, "test:map:dungeon");
        let pose = common::place(&mut session.world, dungeon, 3, 5, Facing::South);
        session.pose = pose;
        session.refresh();
    }
    app.update();
    tap(&mut app, KeyCode::Space);
    let text = screen_text(app.world());
    for line in [
        "\"Here: (3, 5) facing south\"",
        "[button] \"1  Open the door\"",
        "[button] \"2  Close\"",
    ] {
        assert!(text.contains(line), "{line} in:\n{text}");
    }
}

/// A change of words under an open screen rewrites them in place: the same button, new label.
#[test]
fn new_words_are_written_in_place_and_new_controls_rebuild() {
    use omnis_vector::shell::screen::Pressable;
    let mut app = app();
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = common::map(&session.data, "test:map:dungeon");
        session.pose = common::place(&mut session.world, dungeon, 3, 5, Facing::South);
        session.refresh();
    }
    app.update();
    tap(&mut app, KeyCode::Space);
    let button = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Pressable)>();
        q.iter(world)
            .find(|(_, p)| p.0 == ControlId::Action(0))
            .map(|(e, _)| e)
    };
    let before = button(&mut app).expect("the door's button");
    app.world_mut()
        .resource_mut::<Session>()
        .order(omnis_sim::Command::Interact);
    app.update();
    assert_eq!(button(&mut app), Some(before), "the same node");
    assert!(screen_text(app.world()).contains("\"1  Close the door\""));
}
