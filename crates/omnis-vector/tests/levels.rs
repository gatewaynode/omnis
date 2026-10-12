//! Levels in the client (M8.5 step 1b), on the engine's tower fixture: the views and the pose
//! follow a stair, the HUD names the level, the Actions menu climbs a ladder, and the debug
//! panel's teleport picks a level.

use crate::common;

use common::app::config;
use omnis_sim::omnis_core::{Direction, Facing, Level, Position, Vertical};
use omnis_sim::{Command, DevCommand};
use omnis_vector::pose::Pose;
use omnis_vector::screen::debug::{DebugAct, Page};
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, ScreenId};
use omnis_vector::shell::hud::place;
use omnis_vector::shell::screen::ctx;
use omnis_vector::shell::session::{Config, Session};
use std::path::PathBuf;

/// A session with the tower fixture loaded, the party at the town start.
fn tower() -> Session {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../omnis-sim/tests/packs/levels");
    let mut config: Config = config();
    config.packs.push(fixture);
    Session::start(&config).expect("a session with the tower")
}

/// The party put on the tower at `(x, y)` of `level`, facing `facing`.
fn put(session: &mut Session, level: Level, x: u16, y: u16, facing: Facing) {
    let map = common::map(&session.data, "levels:map:tower");
    session.world.position = Position {
        map,
        level,
        x,
        y,
        facing,
    };
    session.pose = Pose::at(session.world.position);
    session.refresh();
}

fn command(effect: Effect) -> Command {
    match effect {
        Effect::Command { command, .. } => command,
        other => panic!("a command, got {other:?}"),
    }
}

#[test]
fn a_stair_taken_puts_the_views_on_the_level_above_and_the_pose_stays_on_its_cell() {
    let mut session = tower();
    put(&mut session, Level::GROUND, 2, 0, Facing::West);
    assert!(session.order(Command::Step(Direction::Forward)));
    let p = session.views.position;
    assert_eq!((p.level, p.x, p.y), (Level(1), 1, 0));
    assert_eq!(session.pose, Pose::at(p), "on the stair's cell, facing on");
    assert!(
        session
            .lines
            .iter()
            .any(|l| l == "Moved to (1, 0) on Upper floor (1) facing West"),
        "{:?}",
        session.lines
    );
}

#[test]
fn the_hud_names_the_level_on_a_map_of_several_and_nothing_on_a_map_of_one() {
    let mut session = tower();
    let town = place(&session.data, &session.views.here);
    assert!(!town.contains("(0)"), "{town}");
    put(&mut session, Level(2), 1, 1, Facing::North);
    assert_eq!(
        place(&session.data, &session.views.here),
        "levels:map:tower  Roof (2)  cell (1, 1) facing North"
    );
}

#[test]
fn the_actions_menu_climbs_a_ladder_and_offers_no_climb_off_one() {
    let mut session = tower();
    put(&mut session, Level::GROUND, 1, 1, Facing::North);
    let labels: Vec<String> = screen::actions::available(&session.world, &session.data)
        .into_iter()
        .map(|o| o.label)
        .collect();
    assert_eq!(labels, ["Climb up", "Climb down"]);
    let mut forms = Forms::default();
    let up = screen::apply(
        ScreenId::Actions,
        &ctx(&session),
        &mut forms,
        ControlId::Action(0),
        &Payload::Press,
        0,
    )
    .expect("the action");
    assert_eq!(command(up.clone()), Command::Climb(Vertical::Up));
    assert!(session.order(command(up)));
    assert_eq!(session.views.position.level, Level(1));
    put(&mut session, Level::GROUND, 2, 1, Facing::North);
    assert!(screen::actions::available(&session.world, &session.data).is_empty());
}

#[test]
fn the_debug_teleport_goes_to_the_level_picked() {
    let mut session = tower();
    put(&mut session, Level::GROUND, 2, 0, Facing::South);
    let mut forms = Forms::default();
    forms.debug.page = Page::World;
    let use_ = |session: &Session, forms: &mut Forms, act, payload| {
        screen::apply(
            ScreenId::Debug,
            &ctx(session),
            forms,
            ControlId::Debug(act),
            &payload,
            0,
        )
        .unwrap_or_else(|e| panic!("{act:?}: {e:?}"))
    };
    use_(&session, &mut forms, DebugAct::Level, Payload::Right);
    let go = command(use_(&session, &mut forms, DebugAct::Go, Payload::Press));
    let Command::Dev(DevCommand::Teleport { level, x, y, .. }) = go.clone() else {
        panic!("{go:?}");
    };
    assert_eq!(
        (level, x, y),
        (Level(1), 2, 0),
        "the level above the party's"
    );
    session.ordered(go.clone()).expect("accepted");
    forms.accepted(&go);
    assert_eq!(session.views.position.level, Level(1));
    // Down past the ground to the cellar, round the four levels.
    use_(&session, &mut forms, DebugAct::Level, Payload::Left);
    use_(&session, &mut forms, DebugAct::Level, Payload::Left);
    let go = command(use_(&session, &mut forms, DebugAct::Go, Payload::Press));
    assert!(
        matches!(
            go,
            Command::Dev(DevCommand::Teleport {
                level: Level(-1),
                ..
            })
        ),
        "{go:?}"
    );
}
