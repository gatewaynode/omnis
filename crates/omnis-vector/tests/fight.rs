//! The fight notice, headless (alt-ARCHITECTURE.md §9): walking into the placed group opens it
//! with the encounter's choices, every choice it offers is one the simulation accepts, a fight
//! runs to its end from its buttons alone, and a fallen party can start again.

mod common;

use bevy::prelude::*;
use common::app::{app, app_with, set};
use common::map;
use omnis_sim::omnis_core::{Facing, Position};
use omnis_sim::{Command, EncounterChoice, Mode};
use omnis_vector::grid::cell_of;
use omnis_vector::pose::Pose;
use omnis_vector::shell::controls::Action;
use omnis_vector::shell::fight::{Order, fallen, notice};
use omnis_vector::shell::session::Session;
use std::path::PathBuf;

/// An app with the party three cells north of the dungeon's placed group, walked into it.
fn met() -> App {
    met_with(1)
}

fn met_with(seed: u64) -> App {
    let mut app = app_with(PathBuf::from(".omnis/vector-session.ron"), seed);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = map(&session.data, "test:map:dungeon");
        session.world.position = Position {
            map: dungeon,
            x: 3,
            y: 6,
            facing: Facing::South,
        };
        session.pose = Pose::at(session.world.position);
    }
    set(&mut app, &Action::Forward, Interaction::Pressed);
    for _ in 0..80 {
        app.update();
    }
    set(&mut app, &Action::Forward, Interaction::None);
    app.update();
    assert!(matches!(
        app.world().resource::<Session>().world.mode,
        Mode::Encounter(_)
    ));
    app
}

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
fn meeting_the_group_offers_the_four_choices() {
    let mut app = met();
    let session = app.world().resource::<Session>();
    let shown = notice(&session.world, &session.data).expect("a notice");
    let labels: Vec<&str> = shown.choices.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels[0], "Fight");
    assert!(labels[1].starts_with("Bribe"));
    assert_eq!(&labels[2..], ["Hide", "Run"]);
    assert!(!shown.lines.is_empty(), "the monsters are listed");
    // The panel's buttons are the notice's choices that can be taken.
    let orders = enabled(&mut app);
    assert!(orders.contains(&Order::Command(Command::Encounter(EncounterChoice::Attack))));
}

#[test]
fn running_puts_the_party_and_the_pose_back_where_it_came_from() {
    // Seed 2: the run check succeeds (seed 1 fails into a fight).
    let mut app = met_with(2);
    let (retreat, pose_before) = {
        let session = app.world().resource::<Session>();
        match &session.world.mode {
            Mode::Encounter(state) => (state.retreat, session.pose),
            _ => unreachable!(),
        }
    };
    press(
        &mut app,
        &Order::Command(Command::Encounter(EncounterChoice::Run)),
    );
    let session = app.world().resource::<Session>();
    assert!(
        matches!(session.world.mode, Mode::Explore),
        "the party got away"
    );
    assert_eq!(session.world.position, retreat);
    assert_ne!(session.pose, pose_before, "the pose followed");
    let here = cell_of(session.pose.x, session.pose.z);
    assert_eq!(here, (i32::from(retreat.x), i32::from(retreat.y)));
    assert_eq!(session.binder.refusals, 0);
    assert!(enabled(&mut app).is_empty(), "the notice closed");
}

#[test]
fn a_fight_runs_to_its_end_from_the_notice_alone() {
    let mut app = met();
    press(
        &mut app,
        &Order::Command(Command::Encounter(EncounterChoice::Attack)),
    );
    assert!(matches!(
        app.world().resource::<Session>().world.mode,
        Mode::Combat(_)
    ));
    for _ in 0..300 {
        if !matches!(
            app.world().resource::<Session>().world.mode,
            Mode::Combat(_)
        ) {
            break;
        }
        let first = enabled(&mut app).into_iter().next().expect("a choice");
        press(&mut app, &first);
    }
    let session = app.world().resource::<Session>();
    assert!(
        matches!(session.world.mode, Mode::Explore),
        "the fight ended"
    );
    assert_eq!(
        session.binder.refusals, 0,
        "every offered choice was accepted"
    );
    let ending = ["Victory", "The party got away", "The party has fallen"];
    assert!(
        session
            .lines
            .iter()
            .any(|l| ending.iter().any(|e| l.starts_with(e))),
        "the HUD says how it ended: {:?}",
        session.lines
    );
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
    assert_eq!(session.world.position, start);
    assert_eq!(session.binder.log.len(), 4, "only the party's creation");
    assert!(enabled(&mut app).is_empty(), "the notice closed");
}
