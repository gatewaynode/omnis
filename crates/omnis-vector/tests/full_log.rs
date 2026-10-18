//! The full log (owner, 2026-10-17): every line the session said, exploration and fight alike,
//! each once, stamped with the party's clock, the newest `HISTORY` kept; a screen that shows
//! them scrolled to the newest, filtered by every word typed, opened by L, its button or a click
//! on the running log.

use crate::common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Skill;
use omnis_sim::{ActorRef, CheckKind, Event};
use omnis_vector::history::{HISTORY, History};
use omnis_vector::labels;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, ScreenId};
use omnis_vector::shell::controls::Action;
use omnis_vector::shell::hud::{HudPlugin, LogColumn, LogLine};
use omnis_vector::shell::screen::{Screens, ctx};
use omnis_vector::shell::screen_draw::{Scrolls, wheeled};
use omnis_vector::shell::session::Session;
use std::path::PathBuf;

#[test]
fn the_history_keeps_the_newest_and_a_filter_needs_every_word_in_any_case() {
    let mut history = History::default();
    // Midnight of day one, but the last five on day two.
    for n in 0..HISTORY {
        history.push(0, format!("line {n}"));
    }
    for n in 0..5 {
        history.push(1440, format!("late line {n}"));
    }
    assert_eq!(history.len(), HISTORY);
    let first = history.iter().next().expect("kept");
    assert_eq!(first.text, "line 5", "the oldest go first");
    assert_eq!(first.stamped(), format!("{}  line 5", labels::clock(0)));
    // Every word, in any case, in the stamp or the words.
    // A word matches inside a word, as a filter does: "59" in "line 159".
    let found = history.matching("LINE 59");
    let want = history.iter().filter(|e| e.text.contains("59")).count();
    assert_eq!(found.len(), want);
    assert!(found.iter().all(|e| e.text.contains("59")));
    // The late lines were said on day two: the stamp's words count.
    assert_eq!(history.matching("Day 2 late line 4").len(), 1);
    assert!(
        history.matching("day 1 late line 4").is_empty(),
        "the stamp counts"
    );
    assert!(history.matching("line nothing").is_empty(), "every word");
    assert_eq!(
        history.matching("  ").len(),
        HISTORY,
        "no words, every line"
    );
}

/// A running game in town with the fixed party, a frame drawn.
fn game() -> App {
    let mut app = common::app::shell(common::app::config(), true);
    app.add_plugins(HudPlugin);
    app.update();
    app
}

fn first_member(app: &App) -> CharacterId {
    app.world().resource::<Session>().views.party.members[0].member
}

#[test]
fn every_line_is_kept_once_with_the_fight_s_own_and_the_party_s_clock() {
    let mut app = game();
    let member = first_member(&app);
    let check = Event::Check {
        actor: ActorRef::Member(member),
        kind: CheckKind::Skill(Skill::Athletics),
        roll: None,
        dc: 10,
        success: true,
    };
    let mut session = app.world_mut().resource_mut::<Session>();
    let before = session.history.len();
    session.note(&[check, Event::RoundStarted { round: 1 }]);
    let age = session.views.here.age;
    let kept: Vec<(i64, String)> = session
        .history
        .iter()
        .skip(before)
        .map(|e| (e.minute, e.text.clone()))
        .collect();
    assert_eq!(
        kept.len(),
        2,
        "a check both describe is kept once: {kept:?}"
    );
    assert!(kept[0].1.contains("Athletics"), "{kept:?}");
    assert_eq!(kept[1].1, "Round 1", "the fight's own line joins");
    assert!(kept.iter().all(|(m, _)| *m == age), "the party's clock");
    assert!(
        !session.lines.iter().any(|l| l == "Round 1"),
        "the running log keeps to its own lines"
    );
    // A refusal and a host's line are kept too.
    let _ = session.ordered(omnis_sim::Command::Search);
    session.say("Loaded quick.ron".to_owned());
    assert_eq!(
        session.history.iter().last().map(|e| e.text.as_str()),
        Some("Loaded quick.ron")
    );
}

#[test]
fn the_screen_shows_the_newest_matches_and_how_many() {
    let mut app = game();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.history = History::default();
        for n in 0..600 {
            session.history.push(0, format!("line {n}"));
        }
    }
    let session = app.world().resource::<Session>();
    let mut forms = Forms::default();
    let shown = screen::content(ScreenId::Log, &ctx(session), &forms);
    let words = |s: &screen::Screen| -> Vec<String> {
        s.panels
            .iter()
            .flat_map(|p| &p.rows)
            .flat_map(|r| r.spans.iter().chain(&r.after))
            .map(|s| s.text.clone())
            .collect()
    };
    let all = words(&shown);
    assert!(
        all.iter().any(|w| w == "The newest 500 of 600 lines"),
        "{all:?}"
    );
    let lines = all.last().expect("the lines");
    assert!(lines.ends_with("line 599"), "newest at the foot");
    assert!(!lines.contains("line 99\n"), "older than the newest 500");
    assert!(lines.contains("line 100"));
    assert_eq!(lines.lines().count(), 500);
    // Filtered.
    let effect = screen::apply(
        ScreenId::Log,
        &ctx(session),
        &mut forms,
        ControlId::LogFilter,
        &Payload::Text("Line 59".to_owned()),
        0,
    );
    assert_eq!(effect, Ok(Effect::None));
    let shown = screen::content(ScreenId::Log, &ctx(session), &forms);
    let all = words(&shown);
    let want = (0..600).filter(|n| n.to_string().contains("59")).count();
    assert!(
        all.iter().any(|w| *w == format!("{want} of 600 lines")),
        "{all:?}"
    );
    assert_eq!(all.last().expect("the lines").lines().count(), want);
    // Nothing matches.
    forms.log_filter = "zebra".to_owned();
    let all = words(&screen::content(ScreenId::Log, &ctx(session), &forms));
    assert!(all.iter().any(|w| w == "0 of 600 lines"), "{all:?}");
    // Close.
    assert_eq!(
        screen::apply(
            ScreenId::Log,
            &ctx(session),
            &mut forms,
            ControlId::Close,
            &Payload::Press,
            0
        ),
        Ok(Effect::Back)
    );
}

fn top(app: &App) -> Option<ScreenId> {
    common::app::top(app)
}

fn hud_lines_shown(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<(&LogLine, &Visibility)>();
    q.iter(world)
        .filter(|(_, v)| **v != Visibility::Hidden)
        .count()
}

#[test]
fn l_its_button_and_the_running_log_open_it_and_typing_l_does_not_close_it() {
    let mut app = game();
    app.world_mut()
        .resource_mut::<Session>()
        .say("first".to_owned());
    app.update();
    assert!(hud_lines_shown(&mut app) > 0);
    common::app::tap(&mut app, KeyCode::KeyL);
    assert_eq!(top(&app), Some(ScreenId::Log), "L opens it");
    assert_eq!(
        hud_lines_shown(&mut app),
        0,
        "the running log hides under it"
    );
    // Opened at the foot.
    {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&ScrollPosition, With<Scrolls>>();
        assert!(
            q.iter(world).any(|p| p.0.y == f32::MAX),
            "scrolled to the newest"
        );
    }
    // L again types into the filter: it stays.
    common::app::key(&mut app, KeyCode::KeyL, ButtonState::Pressed);
    app.update();
    common::app::key(&mut app, KeyCode::KeyL, ButtonState::Released);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Log), "L does not close it");
    common::app::tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None, "Esc closes it");
    // The button opens it; the buttons rest under a screen that pauses, as Camp's does.
    common::app::press_ui(&mut app, Action::Log);
    assert_eq!(top(&app), Some(ScreenId::Log));
    common::app::tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None);
    // A click on the running log.
    common::app::press_ui(&mut app, LogColumn);
    assert_eq!(top(&app), Some(ScreenId::Log));
    let screens = app.world().resource::<Screens>();
    let session = app.world().resource::<Session>();
    assert!(
        screens.nav.paused(Some(&session.views)),
        "nothing moves under it"
    );
}

#[test]
fn the_wheel_moves_from_the_foot_and_stays_inside_the_rows() {
    // At the foot (any value past the end), a wheel up moves up from the end.
    assert_eq!(wheeled(f32::MAX, 40.0, 300.0), 260.0);
    assert_eq!(wheeled(100.0, -40.0, 300.0), 140.0);
    assert_eq!(wheeled(10.0, 40.0, 300.0), 0.0);
    assert_eq!(wheeled(290.0, -40.0, 300.0), 300.0);
}

/// A session met by the dungeon's group, then a fight of dodges, until it ends or 30 turns pass.
fn fought() -> Session {
    use omnis_sim::{CombatCommand, Command, EncounterChoice};
    let mut session = common::fight::met();
    session.order(Command::Encounter(EncounterChoice::Attack));
    for _ in 0..30 {
        if !common::fight::fighting(&session) {
            break;
        }
        session.order(Command::Combat(CombatCommand::Dodge));
    }
    session
}

fn said(history: &History) -> Vec<(i64, String)> {
    history.iter().map(|e| (e.minute, e.text.clone())).collect()
}

#[test]
fn the_full_log_is_retold_from_the_party_s_log_as_it_was_said() {
    let session = fought();
    let live = said(&session.history);
    assert!(
        live.iter().any(|(_, l)| l.starts_with("Moved to"))
            && live.iter().any(|(_, l)| l == "Round 1"),
        "a walk and a fight: {live:?}"
    );
    let (retold, _) = omnis_vector::shell::retold::retold(
        session.world.history(),
        &session.views.party,
        &session.data,
    );
    assert_eq!(said(&retold), live);
}

#[test]
fn a_load_retells_the_full_log_from_the_save() {
    let mut session = fought();
    let live = said(&session.history);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vector-retold");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    omnis_vector::game_io::save(&session.world, &session.data, &root, "one.ron").unwrap();
    let loaded = omnis_vector::game_io::load(&session.data, &root, "one.ron", false).unwrap();
    // Another session's history is not this save's.
    session.history = History::default();
    session.history.push(0, "said by another game".to_owned());
    session.replace(loaded.world, "loaded");
    assert_eq!(said(&session.history), live, "every line, at its clock");
}

#[test]
fn a_retold_object_is_named_from_the_map_it_was_found_on() {
    use omnis_sim::{Logged, Notice};
    let data = common::objects_data();
    let (world, _) = common::session(&data);
    let party = common::views(&world, &data).party;
    let found = Logged {
        minute: 5,
        map: common::map(&data, "objects:map:yard"),
        event: Event::Noticed {
            object: "lever_1".into(),
            how: Notice::Search,
        },
    };
    let (retold, _) = omnis_vector::shell::retold::retold(&[found], &party, &data);
    let lines = said(&retold);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].0, 5);
    assert!(
        lines[0].1.starts_with("Found: ") && !lines[0].1.contains("Something"),
        "{lines:?}"
    );
}
