//! The outcome notice (owner, 2026-10-16; presentation-ARCHITECTURE §6.6): what a command the
//! player started said, centred where the screens stand, repeated in the HUD's lines, fading with
//! the distance from where it was done and dropped once out of range. A move never sets it; a
//! refusal shows in the alert role; it hides under a screen.

use crate::common;

use bevy::prelude::*;
use omnis_sim::omnis_core::{Facing, Level, MapId, Position, Rotation, Vertical};
use omnis_sim::{Command, ItemCommand};
use omnis_vector::notice::{GONE, Notice, alpha};
use omnis_vector::pose::Pose;
use omnis_vector::screen::ScreenId;
use omnis_vector::shell::notice::NoticeText;
use omnis_vector::shell::screen::{Screens, colour};
use omnis_vector::shell::session::{Config, Session};
use omnis_vector::style::Role;
use std::path::PathBuf;

fn at(x: u16, y: u16) -> Position {
    Position {
        map: MapId(1),
        level: Level::GROUND,
        x,
        y,
        facing: Facing::South,
    }
}

#[test]
fn a_notice_is_full_near_where_it_was_done_and_gone_a_cell_and_a_half_away() {
    let here = at(4, 1);
    let notice = Notice::new(
        vec!["Here the yard.".to_owned()],
        Role::Interface,
        here,
        Pose::at(here),
    );
    let pose = Pose::at(here);
    let off = |dx: f32, dz: f32| Pose {
        x: pose.x + dx,
        z: pose.z + dz,
        yaw: pose.yaw,
    };
    assert!((alpha(&notice, &here, &pose) - 1.0).abs() < 1e-6);
    // Turning on the spot keeps it.
    let turned = Pose { yaw: 1.0, ..pose };
    assert!((alpha(&notice, &here, &turned) - 1.0).abs() < 1e-6);
    assert!((alpha(&notice, &here, &off(0.5, 0.0)) - 1.0).abs() < 1e-6);
    assert!((alpha(&notice, &here, &off(0.0, -1.0)) - 0.5).abs() < 1e-6);
    assert!(
        (alpha(&notice, &here, &off(0.6, 0.8)) - 0.5).abs() < 1e-6,
        "by distance, not axis"
    );
    assert_eq!(alpha(&notice, &here, &off(GONE, 0.0)), 0.0);
    assert_eq!(alpha(&notice, &here, &off(0.0, 3.0)), 0.0);
    // Another level or map: gone.
    let up = Position {
        level: Level(1),
        ..here
    };
    assert_eq!(alpha(&notice, &up, &pose), 0.0);
    let elsewhere = Position {
        map: MapId(2),
        ..here
    };
    assert_eq!(alpha(&notice, &elsewhere, &pose), 0.0);
}

/// The notice as drawn: its words, colour and alpha, when shown.
fn drawn(app: &mut App) -> Option<(String, Color)> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Text, &TextColor, &Visibility), With<NoticeText>>();
    let (text, tint, visibility) = q.single(world).expect("the notice's text");
    (*visibility != Visibility::Hidden).then(|| (text.0.clone(), tint.0))
}

fn yard_app(x: u16, y: u16, facing: Facing) -> App {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = common::app::config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let mut app = common::app::shell(
        Config {
            packs,
            ..common::app::config()
        },
        true,
    );
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = common::place(&mut session.world, map, x, y, facing);
        session.refresh();
    }
    app.update();
    app
}

fn order(app: &mut App, command: Command) {
    let _ = app.world_mut().resource_mut::<Session>().ordered(command);
    app.update();
}

const PLAQUE: &str = "Here the yard. Mind the loft.";

#[test]
fn a_chosen_act_s_words_stand_centred_and_in_the_log_and_fade_as_the_party_walks_off() {
    let mut app = yard_app(4, 1, Facing::South);
    assert_eq!(drawn(&mut app), None, "nothing yet");
    order(&mut app, Command::Interact);
    let (text, tint) = drawn(&mut app).expect("the plaque's words");
    assert_eq!((text.as_str(), tint), (PLAQUE, colour(Role::Interface)));
    assert!(
        app.world()
            .resource::<Session>()
            .lines
            .iter()
            .any(|l| l == PLAQUE),
        "repeated in the log"
    );
    // A turn on the spot keeps it whole.
    order(&mut app, Command::Turn(Rotation::Left));
    let (text, tint) = drawn(&mut app).expect("still shown");
    assert_eq!(text, PLAQUE, "a command that only moved leaves the notice");
    assert!((tint.alpha() - 1.0).abs() < 1e-3);
    // A cell north, half faded.
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.pose.z -= 1.0;
    }
    app.update();
    let (_, tint) = drawn(&mut app).expect("fading");
    assert!(
        tint.alpha() < 0.75 && tint.alpha() > 0.25,
        "{}",
        tint.alpha()
    );
    // Out of range it is dropped, and coming back does not bring it back.
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.pose.z -= 1.0;
    }
    app.update();
    assert_eq!(drawn(&mut app), None);
    assert!(
        app.world().resource::<Session>().notice.is_none(),
        "dropped"
    );
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.pose.z += 2.0;
    }
    app.update();
    assert_eq!(drawn(&mut app), None, "gone for good");
}

#[test]
fn a_refusal_shows_in_alert_a_move_sets_nothing_and_a_screen_hides_it() {
    let mut app = yard_app(4, 1, Facing::South);
    let first = app.world().resource::<Session>().views.party.members[0].member;
    order(
        &mut app,
        Command::Item(ItemCommand::Equip {
            member: first,
            item: "base:item:no_such_thing".into(),
        }),
    );
    let (text, tint) = drawn(&mut app).expect("the refusal");
    assert!(text.starts_with("Refused: "), "{text}");
    assert_eq!(tint, colour(Role::Alert));
    // A move the binder makes says its line in the log only.
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.notice = None;
        let s = &mut *session;
        let events = s
            .binder
            .apply(&mut s.world, &s.data, Command::Turn(Rotation::Left))
            .expect("a turn");
        s.note(&events);
    }
    app.update();
    assert_eq!(drawn(&mut app), None, "a move sets no notice");
    // Nor does a chosen command that only moves the party: a climb up the yard's ladder.
    let mut app = yard_app(6, 1, Facing::North);
    order(&mut app, Command::Climb(Vertical::Up));
    assert_eq!(
        app.world().resource::<Session>().views.position.level,
        Level(1)
    );
    assert_eq!(drawn(&mut app), None, "a climb's move line is the log's");
    let mut app = yard_app(4, 1, Facing::South);
    order(&mut app, Command::Turn(Rotation::Right));
    order(&mut app, Command::Interact);
    assert!(drawn(&mut app).is_some());
    let views = app.world().resource::<Session>().views.clone();
    app.world_mut()
        .resource_mut::<Screens>()
        .nav
        .open(ScreenId::Pause, Some(&views));
    app.update();
    assert_eq!(drawn(&mut app), None, "hidden under a screen");
}

/// The window draws a cliff flat until M8.5 step 4, and the mirror holds the pose before the
/// simulation hears of the step: the log says why, once, where a step into it would.
#[test]
fn walking_into_a_cliff_says_it_is_too_steep_once() {
    use bevy::input::ButtonState;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut config = common::app::config();
    config
        .packs
        .push(root.join("crates/omnis-sim/tests/packs/heights"));
    let mut app = common::app::shell(config, true);
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let hills = common::map(&session.data, "heights:map:hills");
        session.pose = common::place(
            &mut session.world,
            hills,
            0,
            0,
            omnis_sim::omnis_core::Facing::East,
        );
        session.refresh();
    }
    app.update();
    common::app::key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
    for _ in 0..120 {
        app.update();
    }
    common::app::key(&mut app, KeyCode::KeyW, ButtonState::Released);
    app.update();
    let session = app.world().resource::<Session>();
    assert_eq!(
        (session.views.position.x, session.views.position.y),
        (2, 0),
        "the slope walks, the cliff holds"
    );
    let said: Vec<_> = session
        .lines
        .iter()
        .filter(|l| l.as_str() == "Blocked: too steep")
        .collect();
    assert_eq!(said.len(), 1, "{:?}", session.lines);
    assert_eq!(drawn(&mut app), None, "walking sets no notice");
}
