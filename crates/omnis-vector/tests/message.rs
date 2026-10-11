//! The explore message line (parity P5, presentation-ARCHITECTURE §6.6): the last event worth a
//! line, a refusal in the alert role, cleared by a bare move, shown only while exploring with no
//! screen up.

use crate::common;

use bevy::prelude::*;
use omnis_sim::omnis_core::{Direction, Rotation};
use omnis_sim::{Command, ItemCommand};
use omnis_vector::screen::ScreenId;
use omnis_vector::shell::message::MessageLine;
use omnis_vector::shell::screen::{Screens, colour};
use omnis_vector::shell::session::Session;
use omnis_vector::style::Role;

/// The line as drawn: its words and colour when shown.
fn drawn(app: &mut App) -> Option<(String, Color)> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Text, &TextColor, &Visibility), With<MessageLine>>();
    let (text, tint, visibility) = q.single(world).expect("the message line");
    (*visibility != Visibility::Hidden).then(|| (text.0.clone(), tint.0))
}

fn order(app: &mut App, command: Command) {
    app.world_mut().resource_mut::<Session>().order(command);
    app.update();
}

#[test]
fn the_line_shows_the_last_event_a_refusal_in_alert_and_clears_on_a_move() {
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    assert_eq!(drawn(&mut app), None, "nothing yet");
    // The start faces West down the street; north of it, nothing to use.
    order(&mut app, Command::Turn(Rotation::Right));
    assert_eq!(drawn(&mut app), None, "a turn says nothing");
    order(&mut app, Command::Interact);
    let (text, tint) = drawn(&mut app).expect("the event's line");
    assert_eq!(
        (text.as_str(), tint),
        ("There is nothing to do here", colour(Role::Interface))
    );

    let brenna = app.world().resource::<Session>().views.party.members[0].member;
    order(
        &mut app,
        Command::Item(ItemCommand::Equip {
            member: brenna,
            item: "base:item:no_such_thing".into(),
        }),
    );
    let (text, tint) = drawn(&mut app).expect("the refusal");
    assert!(text.starts_with("Refused: "), "{text}");
    assert_eq!(tint, colour(Role::Alert));

    order(&mut app, Command::Turn(Rotation::Left));
    order(&mut app, Command::Step(Direction::Forward));
    assert_eq!(drawn(&mut app), None, "a bare move clears it");

    order(&mut app, Command::Turn(Rotation::Right));
    order(&mut app, Command::Interact);
    assert!(drawn(&mut app).is_some(), "the next event or refusal");
    let views = app.world().resource::<Session>().views.clone();
    app.world_mut()
        .resource_mut::<Screens>()
        .nav
        .open(ScreenId::Pause, Some(&views));
    app.update();
    assert_eq!(drawn(&mut app), None, "hidden under a screen");
}
