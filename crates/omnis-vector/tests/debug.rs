//! The debug panel (parity P8, presentation-ARCHITECTURE §10.6): its four pages, numbers typed
//! and sent saturated, an accepted edit clearing what was typed, items to a member or the
//! stores, the teleport, the fight's stacks, every button lit by the engine's trial, the Pause
//! button dim outside a devtools world, backtick and Esc, and the edit's line in the HUD.

use crate::common;

use bevy::prelude::*;
use common::app::{config, press_ui, tap};
use common::fight::turn_of;
use omnis_sim::omnis_core::Rotation;
use omnis_sim::{Command, DevCommand, Settings};
use omnis_vector::screen::debug::{DebugAct, Num, Page};
use omnis_vector::screen::{self, ControlId, Effect, Forms, HostOp, Kind, Payload, ScreenId};
use omnis_vector::shell::movement::Intent;
use omnis_vector::shell::screen::{Pressable, Screens, ctx};
use omnis_vector::shell::session::Session;
use omnis_vector::sim::SimWorld;

fn town() -> Session {
    Session::start(&config()).expect("a session at the town")
}

fn shown(session: &Session, forms: &Forms) -> screen::Screen {
    screen::content(ScreenId::Debug, &ctx(session), forms)
}

fn use_(session: &Session, forms: &mut Forms, act: DebugAct, payload: Payload) -> Effect {
    screen::apply(
        ScreenId::Debug,
        &ctx(session),
        forms,
        ControlId::Debug(act),
        &payload,
        0,
    )
    .unwrap_or_else(|e| panic!("{act:?}: {e:?}"))
}

/// Send a command the panel asked for, as the shell does: through the session, then the form.
fn send(session: &mut Session, forms: &mut Forms, effect: Effect) {
    let Effect::Command { command, .. } = effect else {
        panic!("a command, got {effect:?}");
    };
    session.ordered(command.clone()).expect("accepted");
    forms.accepted(&command);
}

fn value(screen: &screen::Screen, num: Num) -> String {
    match screen
        .control(ControlId::Debug(DebugAct::Field(num)))
        .map(|c| &c.kind)
    {
        Some(Kind::Text { value, .. }) => value.clone(),
        other => panic!("{num:?}: {other:?}"),
    }
}

fn why(screen: &screen::Screen, act: DebugAct) -> Option<String> {
    screen
        .control(ControlId::Debug(act))
        .unwrap_or_else(|| panic!("{act:?} on the page"))
        .why_not
        .clone()
}

fn page(forms: &mut Forms, page: Page) {
    forms.debug.page = page;
}

#[test]
fn the_member_page_reads_the_member_and_a_typed_number_is_sent_saturated() {
    let mut session = town();
    let mut forms = Forms::default();
    let s = shown(&session, &forms);
    assert_eq!(value(&s, Num::Hp), "12", "Brenna's hit points");
    assert_eq!(why(&s, DebugAct::Set(Num::Hp)), None);
    let hp_row = s.panels[0]
        .rows
        .iter()
        .find(|r| {
            r.controls
                .iter()
                .any(|c| c.id == ControlId::Debug(DebugAct::Set(Num::Hp)))
        })
        .expect("the hit points' row");
    assert_eq!(hp_row.after[0].text, "of 12");
    // Past the command's type: held at its largest, with no game limit.
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Hp),
        Payload::Text("4000000000".to_owned()),
    );
    let set = use_(&session, &mut forms, DebugAct::Set(Num::Hp), Payload::Press);
    let brenna = session.views.party.members[0].member;
    assert_eq!(
        set,
        Effect::Command {
            command: Command::Dev(DevCommand::SetHp {
                member: brenna,
                hp: i32::MAX
            }),
            close: false
        }
    );
    // A smaller number goes through, and the field reads the world again.
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Hp),
        Payload::Text("40".to_owned()),
    );
    let set = use_(&session, &mut forms, DebugAct::Set(Num::Hp), Payload::Press);
    send(&mut session, &mut forms, set);
    assert_eq!(session.views.party.members[0].hp, 40);
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Hp),
        Payload::Text(String::new()),
    );
    let s = shown(&session, &forms);
    assert_eq!(value(&s, Num::Hp), "", "an emptied field");
    assert_eq!(
        why(&s, DebugAct::Set(Num::Hp)).as_deref(),
        Some("type a number")
    );
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Hp),
        Payload::Text("007".to_owned()),
    );
    let set = use_(&session, &mut forms, DebugAct::Set(Num::Hp), Payload::Press);
    send(&mut session, &mut forms, set);
    assert_eq!(
        value(&shown(&session, &forms), Num::Hp),
        "7",
        "the world's, not the edit"
    );
    // Only digits.
    let refused = screen::apply(
        ScreenId::Debug,
        &ctx(&session),
        &mut forms,
        ControlId::Debug(DebugAct::Field(Num::Hp)),
        &Payload::Text("4a".to_owned()),
        0,
    );
    assert_eq!(refused, Err(screen::Refused::NotDigits));
}

#[test]
fn the_member_switch_ability_and_condition_reach_the_chosen_member() {
    let mut session = town();
    let mut forms = Forms::default();
    use_(&session, &mut forms, DebugAct::Member, Payload::Right);
    let durin = session.views.party.members[1].clone();
    assert_eq!(
        value(&shown(&session, &forms), Num::Xp),
        durin.xp.to_string()
    );
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Xp),
        Payload::Text("300".to_owned()),
    );
    let set = use_(&session, &mut forms, DebugAct::Set(Num::Xp), Payload::Press);
    send(&mut session, &mut forms, set);
    assert_eq!(
        session.views.party.members[1].level, 2,
        "Durin levels at 300"
    );
    // The third ability is Constitution.
    use_(&session, &mut forms, DebugAct::Ability, Payload::Right);
    use_(&session, &mut forms, DebugAct::Ability, Payload::Right);
    assert_eq!(
        value(&shown(&session, &forms), Num::Score),
        durin.scores[2].to_string()
    );
    let toggle = use_(&session, &mut forms, DebugAct::Toggle, Payload::Press);
    let Effect::Command {
        command: Command::Dev(DevCommand::SetCondition {
            member, applied, ..
        }),
        ..
    } = &toggle
    else {
        panic!("{toggle:?}");
    };
    assert_eq!((*member, *applied), (durin.member, true));
    send(&mut session, &mut forms, toggle);
    assert!(
        !session.views.party.members[1].conditions.is_empty(),
        "now on"
    );
    let off = use_(&session, &mut forms, DebugAct::Toggle, Payload::Press);
    send(&mut session, &mut forms, off);
    assert!(
        session.views.party.members[1].conditions.is_empty(),
        "and off"
    );
}

#[test]
fn the_party_page_sets_money_and_gives_items_to_the_member_or_the_stores() {
    let mut session = town();
    let mut forms = Forms::default();
    page(&mut forms, Page::Party);
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Copper),
        Payload::Text("12345".to_owned()),
    );
    let set = use_(
        &session,
        &mut forms,
        DebugAct::Set(Num::Copper),
        Payload::Press,
    );
    send(&mut session, &mut forms, set);
    assert_eq!(session.views.party.gold, 12345);
    // The spyglass, by the item choice.
    let ids: Vec<String> = session
        .data
        .registry
        .items
        .names()
        .map(str::to_owned)
        .collect();
    let spyglass = ids
        .iter()
        .position(|i| i == "base:item:spyglass")
        .expect("a spyglass");
    for _ in 0..spyglass {
        use_(&session, &mut forms, DebugAct::Item, Payload::Right);
    }
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Count),
        Payload::Text("3".to_owned()),
    );
    let to_member = use_(&session, &mut forms, DebugAct::ToMember, Payload::Press);
    let brenna = session.views.party.members[0].member;
    assert_eq!(
        to_member,
        Effect::Command {
            command: Command::Dev(DevCommand::GiveItem {
                member: Some(brenna),
                item: "base:item:spyglass".to_owned(),
                count: 3
            }),
            close: false
        }
    );
    send(&mut session, &mut forms, to_member);
    // A count of none gives one.
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::Count),
        Payload::Text("0".to_owned()),
    );
    let to_stores = use_(&session, &mut forms, DebugAct::ToStores, Payload::Press);
    let Effect::Command {
        command: Command::Dev(DevCommand::GiveItem { member, .. }),
        ..
    } = &to_stores
    else {
        panic!("{to_stores:?}");
    };
    assert_eq!(*member, None, "to the stores");
    let Effect::Command {
        command: Command::Dev(DevCommand::GiveItem { count, .. }),
        ..
    } = &to_stores
    else {
        panic!("{to_stores:?}");
    };
    assert_eq!(*count, 1, "at least one");
    send(&mut session, &mut forms, to_stores);
    assert!(
        session
            .views
            .party
            .inventory
            .iter()
            .any(|i| i.item == "base:item:spyglass"),
        "in the stores"
    );
}

#[test]
fn the_world_page_teleports_with_the_cell_held_inside_the_map() {
    let mut session = town();
    let mut forms = Forms::default();
    page(&mut forms, Page::World);
    let s = shown(&session, &forms);
    assert!(
        s.panels[0].rows.iter().any(|r| r
            .spans
            .iter()
            .any(|sp| sp.text == "The packs declare no flags")),
        "the shipped packs declare none"
    );
    assert_eq!(
        (value(&s, Num::X), value(&s, Num::Y)),
        ("10".to_owned(), "2".to_owned())
    );
    // Another map: the cell is held inside it.
    let here = session.views.here.position.map.clone();
    use_(&session, &mut forms, DebugAct::Map, Payload::Right);
    use_(
        &session,
        &mut forms,
        DebugAct::Field(Num::X),
        Payload::Text("9999".to_owned()),
    );
    use_(&session, &mut forms, DebugAct::Map, Payload::Right);
    use_(&session, &mut forms, DebugAct::Map, Payload::Left);
    let go = use_(&session, &mut forms, DebugAct::Go, Payload::Press);
    let Effect::Command {
        command: Command::Dev(DevCommand::Teleport { map, x, .. }),
        ..
    } = &go
    else {
        panic!("{go:?}");
    };
    assert_ne!(*map, here);
    let width = session
        .data
        .registry
        .maps
        .get(map)
        .and_then(|m| session.data.maps.get(&m))
        .expect("the map")
        .def
        .width;
    assert!(*x < width, "{x} inside {width}");
    let to = map.clone();
    send(&mut session, &mut forms, go);
    assert_eq!(session.views.here.position.map, to);
    let s = shown(&session, &forms);
    assert_eq!(
        value(&s, Num::X),
        session.views.here.position.x.to_string(),
        "back to the party's place"
    );
}

#[test]
fn in_a_fight_go_is_dim_with_the_engine_s_reason_and_the_fight_page_kills_a_stack() {
    let mut session = turn_of("Durin");
    let mut forms = Forms::default();
    page(&mut forms, Page::World);
    let go = why(&shown(&session, &forms), DebugAct::Go);
    assert!(go.is_some(), "no teleport in a fight");
    page(&mut forms, Page::Fight);
    let s = shown(&session, &forms);
    assert_eq!(why(&s, DebugAct::Kill), None);
    assert!(!value(&s, Num::Lead).is_empty(), "the lead's hit points");
    let kill = use_(&session, &mut forms, DebugAct::Kill, Payload::Press);
    send(&mut session, &mut forms, kill);
    assert!(
        session.views.combat.is_none(),
        "the only stack slain ends the fight"
    );
    let s = shown(&session, &forms);
    assert!(
        s.panels[0]
            .rows
            .iter()
            .any(|r| r.spans.iter().any(|sp| sp.text == "No fight is on")),
        "the page says so"
    );
}

#[test]
fn every_lit_button_on_every_page_sends_a_command_the_engine_accepts() {
    for session in [town(), turn_of("Brenna")] {
        let mut forms = Forms::default();
        for p in Page::ALL {
            page(&mut forms, p);
            let s = shown(&session, &forms);
            for c in s.controls() {
                let ControlId::Debug(act) = c.id else {
                    continue;
                };
                if c.why_not.is_some() || c.kind != Kind::Button {
                    continue;
                }
                let effect = use_(&session, &mut forms.clone(), act, Payload::Press);
                if act == DebugAct::Split {
                    assert_eq!(effect, Effect::None, "Split reports and sends nothing");
                    continue;
                }
                if act == DebugAct::Tidy {
                    assert_eq!(
                        effect,
                        Effect::Host(HostOp::Tidy { delete: false }),
                        "Tidy is the host's: it lists"
                    );
                    continue;
                }
                let Effect::Command { command, .. } = effect else {
                    panic!("{p:?} {act:?}: {effect:?}");
                };
                assert!(
                    omnis_vector::trial::accepted(&session.world, &session.data, &command),
                    "{p:?} {act:?}: {command:?}"
                );
            }
        }
    }
}

#[test]
fn the_pause_s_debug_is_dim_outside_a_devtools_world() {
    let data = common::data();
    let world = SimWorld::start(&data, 1, Settings::default()).expect("a world");
    let views = common::views(&world, &data);
    let ctx = common::ctx(&data, &world, &views);
    let pause = screen::content(ScreenId::Pause, &ctx, &Forms::default());
    assert_eq!(
        pause
            .control(ControlId::OpenDebug)
            .and_then(|c| c.why_not.clone()),
        Some("Devtools are off in this game".to_owned())
    );
    let nav = screen::nav::Nav::default();
    assert!(!nav.allowed(ScreenId::Debug, Some(&views)));
    let session = town();
    let pause = screen::content(ScreenId::Pause, &ctx_of(&session), &Forms::default());
    assert_eq!(
        pause
            .control(ControlId::OpenDebug)
            .and_then(|c| c.why_not.clone()),
        None
    );
}

fn ctx_of(session: &Session) -> screen::Ctx<'_> {
    ctx(session)
}

fn top(app: &App) -> Option<ScreenId> {
    let session = app.world().resource::<Session>();
    app.world()
        .resource::<Screens>()
        .nav
        .top(Some(&session.views))
}

#[test]
fn backtick_opens_the_panel_with_mouse_look_on_esc_closes_it_and_an_edit_logs_a_line() {
    let mut app = common::app::app_in_game(".omnis/test-saves/none.ron");
    app.update();
    app.update();
    common::app::click(&mut app, (800.0, 450.0));
    assert!(
        app.world().resource::<Intent>().looking,
        "a click in the view looks"
    );
    tap(&mut app, KeyCode::Backquote);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Debug));
    assert!(
        !app.world().resource::<Intent>().looking,
        "the pointer is given back"
    );
    let session = app.world().resource::<Session>();
    assert!(
        app.world()
            .resource::<Screens>()
            .nav
            .paused(Some(&session.views)),
        "the game waits"
    );
    // Type Brenna's hit points, then press Set.
    app.world_mut()
        .resource_scope(|world, mut screens: Mut<Screens>| {
            let session = world.resource::<Session>();
            screen::apply(
                ScreenId::Debug,
                &ctx(session),
                &mut screens.forms,
                ControlId::Debug(DebugAct::Field(Num::Hp)),
                &Payload::Text("5".to_owned()),
                0,
            )
            .expect("typed");
        });
    press_ui(
        &mut app,
        Pressable(ControlId::Debug(DebugAct::Set(Num::Hp))),
    );
    let session = app.world().resource::<Session>();
    assert_eq!(session.views.party.members[0].hp, 5);
    assert!(
        session.lines.iter().any(|l| l == "dev: Brenna hp 5"),
        "{:?}",
        session.lines
    );
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None, "Esc closes it");
    tap(&mut app, KeyCode::Backquote);
    tap(&mut app, KeyCode::Backquote);
    assert_eq!(top(&app), None, "backtick closes it too");
    // From the pause.
    tap(&mut app, KeyCode::Escape);
    press_ui(&mut app, Pressable(ControlId::OpenDebug));
    assert_eq!(top(&app), Some(ScreenId::Debug));
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause), "back to the pause");
}

/// Every span's words on the screen's first panel.
fn texts(screen: &screen::Screen) -> Vec<String> {
    screen.panels[0]
        .rows
        .iter()
        .flat_map(|r| r.spans.iter().map(|s| s.text.clone()))
        .collect()
}

#[test]
fn the_world_page_counts_the_journal_and_turning_adds_nothing() {
    let mut session = town();
    let mut forms = Forms::default();
    page(&mut forms, Page::World);
    let before = session.views.journal;
    let line = |n: usize| {
        let word = if n == 1 {
            "interaction"
        } else {
            "interactions"
        };
        format!("Journal: {n} {word} since the last fold; archive: 0 segments")
    };
    assert!(texts(&shown(&session, &forms)).contains(&line(before)));
    for _ in 0..3 {
        session
            .ordered(Command::Turn(Rotation::Left))
            .expect("a turn in town");
    }
    assert_eq!(
        session.views.journal, before,
        "turning changes only the party"
    );
    assert!(texts(&shown(&session, &forms)).contains(&line(before)));
}
#[test]
fn split_shows_the_parts_and_the_round_trip_until_the_world_moves() {
    let mut session = town();
    let mut forms = Forms::default();
    page(&mut forms, Page::World);
    let lines = texts(&shown(&session, &forms));
    assert!(
        !lines.iter().any(|l| l.starts_with("World base:")),
        "nothing before the press: {lines:?}"
    );
    use_(&session, &mut forms, DebugAct::Split, Payload::Press);
    let lines = texts(&shown(&session, &forms));
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("World base: ") && l.contains("; checkpoint: ")),
        "{lines:?}"
    );
    assert!(lines.contains(&"Round trip: ok".to_owned()), "{lines:?}");
    session
        .ordered(Command::Turn(Rotation::Left))
        .expect("a turn in town");
    let lines = texts(&shown(&session, &forms));
    assert!(
        !lines.iter().any(|l| l.starts_with("Round trip")),
        "a report of an older world is not shown: {lines:?}"
    );
}

#[test]
fn the_flag_choice_names_whose_each_flag_is_and_split_counts_them() {
    let mut config = config();
    config.packs.push(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../omnis-sim/tests/packs/objects"),
    );
    let session = Session::start(&config).expect("a session with the objects pack");
    let mut forms = Forms::default();
    page(&mut forms, Page::World);
    let s = shown(&session, &forms);
    let options = s
        .controls()
        .into_iter()
        .find(|c| c.id == ControlId::Debug(DebugAct::Flag))
        .map(|c| match &c.kind {
            Kind::Choice { options, .. } => options.clone(),
            other => panic!("{other:?}"),
        })
        .expect("a flag choice");
    assert!(
        options.contains(&"objects:flag:debt (party, now 0)".to_owned()),
        "{options:?}"
    );
    assert!(
        options.contains(&"objects:flag:lever (world, now 0)".to_owned()),
        "{options:?}"
    );
    use_(&session, &mut forms, DebugAct::Split, Payload::Press);
    let lines = texts(&shown(&session, &forms));
    assert!(
        lines.contains(&"Flags set: 0 the party's, 0 the world's".to_owned()),
        "{lines:?}"
    );
}

#[test]
fn fold_folds_and_says_the_world_has_moved_on() {
    let mut session = town();
    let mut forms = Forms::default();
    page(&mut forms, Page::World);
    assert_eq!(why(&shown(&session, &forms), DebugAct::Fold), None);
    let effect = use_(&session, &mut forms, DebugAct::Fold, Payload::Press);
    assert_eq!(
        effect,
        Effect::Command {
            command: Command::Dev(DevCommand::Fold),
            close: false
        }
    );
    send(&mut session, &mut forms, effect);
    assert!(
        session
            .history
            .iter()
            .any(|e| e.text == "The world has moved on"),
        "the player is told"
    );
    let lines = texts(&shown(&session, &forms));
    assert!(
        lines.contains(
            &"Journal: 0 interactions since the last fold; archive: 0 segments".to_owned()
        ),
        "{lines:?}"
    );
}

#[test]
fn delete_is_dim_until_tidy_found_something() {
    let session = town();
    let mut forms = Forms::default();
    page(&mut forms, Page::World);
    assert_eq!(
        why(&shown(&session, &forms), DebugAct::Delete).as_deref(),
        Some("tidy first")
    );
    forms.debug.tidied(Some(Vec::new()));
    assert_eq!(
        why(&shown(&session, &forms), DebugAct::Delete).as_deref(),
        Some("nothing to delete")
    );
    let stray = ".omnis/worlds/0123456789abcdef/manifest.ron".to_owned();
    forms.debug.tidied(Some(vec![stray.clone()]));
    let screen = shown(&session, &forms);
    assert_eq!(why(&screen, DebugAct::Delete), None);
    let lines = texts(&screen);
    assert!(
        lines.contains(&"1 file no save names".to_owned()),
        "{lines:?}"
    );
    assert!(lines.contains(&stray), "{lines:?}");
    assert_eq!(
        use_(&session, &mut forms, DebugAct::Delete, Payload::Press),
        Effect::Host(HostOp::Tidy { delete: true })
    );
}
