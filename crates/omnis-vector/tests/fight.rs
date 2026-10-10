//! The fallen party's notice, headless (presentation-ARCHITECTURE.md §9): a party that can no longer
//! fight does not walk on, and can start again. The fight itself is `tests/combat.rs`.

use crate::common;

use bevy::prelude::*;
use common::app::{app, set};
use omnis_vector::shell::controls::Action;
use omnis_vector::shell::fight::fallen;
use omnis_vector::shell::notice::Order;
use omnis_vector::shell::session::Session;

/// The orders of the notice's buttons that can be pressed, in screen order.
fn enabled(app: &mut App) -> Vec<Order> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Order, With<Button>>();
    query.iter(world).cloned().collect()
}

fn press(app: &mut App, order: &Order) {
    set(app, order, Interaction::Pressed);
    app.update();
}

#[test]
fn a_fallen_party_is_offered_a_fresh_start() {
    let mut app = app();
    app.update();
    let start = app.world().resource::<Session>().world.position;
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        for member in &mut session.world.party.members {
            member.hp = 0;
        }
        assert!(fallen(&session.world, &session.data));
    }
    app.update();
    assert_eq!(enabled(&mut app), vec![Order::Restart]);
    // A fallen party does not walk.
    set(&mut app, &Action::Forward, Interaction::Pressed);
    for _ in 0..30 {
        app.update();
    }
    set(&mut app, &Action::Forward, Interaction::None);
    assert_eq!(app.world().resource::<Session>().world.position, start);
    press(&mut app, &Order::Restart);
    app.update();
    let session = app.world().resource::<Session>();
    assert!(!fallen(&session.world, &session.data));
    let town = common::map(&session.data, "test:map:town");
    let (p, facing) = (session.world.position, omnis_sim::omnis_core::Facing::West);
    assert_eq!(
        (p.map, p.x, p.y, p.facing),
        (town, 10, 2, facing),
        "the game's start"
    );
    assert_eq!(session.binder.log.len(), 4, "only the party's creation");
    assert!(enabled(&mut app).is_empty(), "the notice closed");
}
