//! The verbs screen (M8.5 step 2b item 3; presentation-ARCHITECTURE §6.7): every object in reach
//! with a button per interaction its state offers, dim with the engine's refusal worded short,
//! its checks and minutes after it, and the faced door first. Reached from the square's actions:
//! Use opens it when the engine would ask which (`ChooseVerb`) or refuse the one thing there is,
//! so the reason shows; Shift+Space does the same. Search here stands in the actions too. A verb
//! sends `Act` and the screen stays open with its line.

use crate::common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{click, config, key, press_ui, shell, tap, top};
use common::{ctx, objects_data, place, town_session};
use omnis_sim::objects::Refusal;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::{Ability, Data, School, Skill, Verb};
use omnis_sim::{CheckKind, Command};
use omnis_vector::labels;
use omnis_vector::screen::actions::{Does, available, default_action};
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, Screen, ScreenId};
use omnis_vector::shell::screen::Pressable;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::{Config, Session};
use omnis_vector::sim::SimWorld;
use std::path::PathBuf;

fn yard(data: &Data, x: u16, y: u16, facing: Facing) -> SimWorld {
    let (mut world, _) = town_session(data);
    let map = common::map(data, "objects:map:yard");
    place(&mut world, map, x, y, facing);
    world
}

fn shown(data: &Data, world: &SimWorld) -> Screen {
    let views = common::views(world, data);
    screen::content(
        ScreenId::Verbs,
        &ctx(data, world, &views),
        &Forms::default(),
    )
}

fn press(data: &Data, world: &SimWorld, id: ControlId) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    screen::apply(
        ScreenId::Verbs,
        &ctx(data, world, &views),
        &mut Forms::default(),
        id,
        &Payload::Press,
        0,
    )
}

/// Every row's words, spans then controls (a dim one as "label: reason") then the text after.
fn lines(screen: &Screen) -> Vec<String> {
    screen.panels[0]
        .rows
        .iter()
        .map(|r| {
            let mut words: Vec<String> = r.spans.iter().map(|s| s.text.clone()).collect();
            words.extend(r.controls.iter().map(|c| match &c.why_not {
                Some(why) => format!("[{}: {why}]", c.label),
                None => format!("[{}]", c.label),
            }));
            words.extend(r.after.iter().map(|s| s.text.clone()));
            words.join("  ")
        })
        .collect()
}

/// The control labelled `label`.
fn labelled(screen: &Screen, label: &str) -> ControlId {
    screen
        .controls()
        .find(|c| c.label == label)
        .unwrap_or_else(|| panic!("{label} in {:?}", lines(screen)))
        .id
}

/// Every live verb's command is one the engine accepts, and the screen stays open.
fn live_ones_are_accepted(data: &Data, world: &SimWorld) {
    let screen = shown(data, world);
    for c in screen.controls().filter(|c| c.id != ControlId::Close) {
        match press(data, world, c.id) {
            Ok(Effect::Command { command, close }) => {
                assert!(c.why_not.is_none(), "{} is dim", c.label);
                assert!(!close, "the screen stays open");
                assert!(
                    world.trial(data, &command).is_ok(),
                    "{}: {command:?}",
                    c.label
                );
            }
            Err(Refused::Dim(why)) => assert_eq!(Some(why), c.why_not.clone()),
            other => panic!("{}: {other:?}", c.label),
        }
    }
}

#[test]
fn the_chest_lists_both_ways_to_open_it_with_their_checks_and_minutes() {
    let data = objects_data();
    let world = yard(&data, 5, 4, Facing::North);
    let screen = shown(&data, &world);
    assert_eq!(screen.title.text, "Within reach");
    assert_eq!(
        lines(&screen),
        [
            "[Close]",
            "Chest  closed",
            "[Force it open]  Athletics DC 15  2 min",
            // The fixed party's rogue carries thieves' tools.
            "[Pick the lock]  Sleight of Hand DC 10  5 min",
        ]
    );
    // Force is the first verb, so key 1; Close has none.
    let force = labelled(&screen, "Force it open");
    assert_eq!(screen.key(screen::Key::Digit(1)), Some(force));
    assert_eq!(
        press(&data, &world, force),
        Ok(Effect::Command {
            command: Command::Act {
                object: "chest_1".to_owned(),
                verb: Verb::Open,
                option: 0,
            },
            close: false,
        })
    );
    assert_eq!(
        press(&data, &world, labelled(&screen, "Pick the lock")),
        Ok(Effect::Command {
            command: Command::Act {
                object: "chest_1".to_owned(),
                verb: Verb::Open,
                option: 1,
            },
            close: false,
        })
    );
    assert_eq!(press(&data, &world, ControlId::Close), Ok(Effect::Back));
    live_ones_are_accepted(&data, &world);
}

#[test]
fn the_gate_says_why_it_will_not_move_until_the_lever_is_pulled() {
    let data = objects_data();
    let mut world = yard(&data, 3, 1, Facing::South);
    assert_eq!(
        lines(&shown(&data, &world)),
        [
            "[Close]",
            "Gate  shut",
            "[Push: something else must be done first]  1 min",
        ]
    );
    // The lever pulled (its flag set), the gate's Push is live.
    let lever = data
        .registry
        .flags
        .get("objects:flag:lever")
        .expect("a flag");
    world.flags.insert(lever, 1);
    assert_eq!(
        lines(&shown(&data, &world)),
        ["[Close]", "Gate  shut", "[Push]  1 min"]
    );
    let gate = labelled(&shown(&data, &world), "Push");
    world.flags.clear();
    assert_eq!(
        press(&data, &world, gate),
        Err(Refused::Dim("something else must be done first".to_owned()))
    );
    world.flags.insert(lever, 1);
    live_ones_are_accepted(&data, &world);
}

#[test]
fn the_faced_door_comes_first_then_what_lies_beyond_it() {
    let data = objects_data();
    let mut world = yard(&data, 6, 3, Facing::South);
    // Shut, the door hides the torch: the door alone.
    assert_eq!(
        lines(&shown(&data, &world)),
        ["[Close]", "Door", "[Open the door]"]
    );
    let open = labelled(&shown(&data, &world), "Open the door");
    let Ok(Effect::Command { command, .. }) = press(&data, &world, open) else {
        panic!("the door's button sends a command");
    };
    assert_eq!(command, Command::Door);
    world.apply(&data, command).expect("the door opens");
    assert_eq!(
        lines(&shown(&data, &world)),
        [
            "[Close]",
            "Door",
            "[Close the door]",
            "Torch  lit",
            "[Extinguish]  1 min"
        ]
    );
    live_ones_are_accepted(&data, &world);
}

#[test]
fn nothing_in_reach_says_so() {
    let data = objects_data();
    // On the arrows' cell, facing the empty north half of the yard.
    let world = yard(&data, 2, 2, Facing::North);
    assert_eq!(
        lines(&shown(&data, &world)),
        ["[Close]", "Nothing within reach"]
    );
}

fn labels(world: &SimWorld, data: &Data) -> Vec<String> {
    available(world, data)
        .into_iter()
        .map(|o| o.label)
        .collect()
}

#[test]
fn use_opens_the_verbs_when_the_engine_would_ask_or_refuse_and_search_is_always_there() {
    let data = objects_data();
    // The crate offers one thing: Use does it.
    let world = yard(&data, 1, 2, Facing::North);
    assert_eq!(labels(&world, &data), ["Use", "Search here"]);
    assert_eq!(
        default_action(&world, &data),
        Some(Does::Command(Command::Interact))
    );
    // Door and torch: Use opens the verbs.
    let mut world = yard(&data, 6, 3, Facing::South);
    world.apply(&data, Command::Door).expect("the door opens");
    assert_eq!(labels(&world, &data), ["Use...", "Search here"]);
    assert_eq!(default_action(&world, &data), Some(Does::Choose));
    // The shut gate refuses the one thing there is: Use opens the verbs, which say why.
    let world = yard(&data, 3, 1, Facing::South);
    assert_eq!(labels(&world, &data), ["Use...", "Search here"]);
    // Nothing to use: Search alone, and it is never the default.
    let world = yard(&data, 2, 2, Facing::North);
    assert_eq!(labels(&world, &data), ["Search here"]);
    assert_eq!(default_action(&world, &data), None);
    let views = common::views(&world, &data);
    let c = ctx(&data, &world, &views);
    let actions = screen::content(ScreenId::Actions, &c, &Forms::default());
    let search = labelled(&actions, "Search here");
    assert_eq!(
        screen::apply(
            ScreenId::Actions,
            &c,
            &mut Forms::default(),
            search,
            &Payload::Press,
            0
        ),
        Ok(Effect::Command {
            command: Command::Search,
            close: true
        })
    );
}

/// The shell on the shipped packs and the yard, with the party at `(x, y)` in it.
fn yard_app(x: u16, y: u16, facing: Facing) -> App {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let mut app = shell(Config { packs, ..config() }, true);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = place(&mut session.world, map, x, y, facing);
        session.refresh();
    }
    app.update();
    app
}

fn session(app: &App) -> &Session {
    app.world().resource::<Session>()
}

fn torch_state(app: &App) -> Option<String> {
    let s = session(app);
    s.views
        .objects
        .objects
        .iter()
        .find(|o| o.object == "torch_1")
        .map(|o| o.state.clone())
}

#[test]
fn in_the_window_use_opens_the_verbs_and_a_verb_acts_with_its_line() {
    let mut app = yard_app(6, 3, Facing::South);
    // The door first, from the actions.
    tap(&mut app, KeyCode::Space);
    assert_eq!(top(&app), Some(ScreenId::Actions));
    press_ui(&mut app, Pressable(ControlId::Action(0)));
    assert!(
        session(&app)
            .views
            .objects
            .objects
            .iter()
            .any(|o| o.object == "torch_1")
    );
    // Use now opens the verbs over the actions.
    tap(&mut app, KeyCode::Space);
    press_ui(&mut app, Pressable(ControlId::Action(0)));
    assert_eq!(top(&app), Some(ScreenId::Verbs));
    let text = screen_text(app.world());
    for line in [
        "\"Within reach\"",
        "[button] \"Close\"",
        "[button] \"1  Close the door\"",
        "[button] \"2  Extinguish\"",
        "\"Torch\"",
    ] {
        assert!(text.contains(line), "{line} in:\n{text}");
    }
    // Key 2 puts the torch out; the screen stays open with the new state.
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(torch_state(&app).as_deref(), Some("out"));
    assert_eq!(top(&app), Some(ScreenId::Verbs));
    assert!(screen_text(app.world()).contains("[button] \"2  Light\""));
    // Esc goes back to the actions, and again to the view.
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Actions));
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None);
    assert_eq!(session(&app).binder.refusals, 0);
}

#[test]
fn a_dim_verb_shows_its_reason_in_the_window_and_takes_no_click() {
    let mut app = yard_app(3, 1, Facing::South);
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::Space);
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Released);
    app.update();
    assert_eq!(
        top(&app),
        Some(ScreenId::Verbs),
        "Shift+Space opens the verbs"
    );
    let text = screen_text(app.world());
    assert!(
        text.contains("\"Push: something else must be done first\""),
        "{text}"
    );
    let log = session(&app).binder.log.len();
    tap(&mut app, KeyCode::Digit1);
    assert_eq!(
        session(&app).binder.log.len(),
        log,
        "a dim verb does nothing"
    );
    assert_eq!(session(&app).binder.refusals, 0);
}

#[test]
fn a_verb_s_words_are_its_line_and_moving_closes_the_screen() {
    let mut app = yard_app(1, 2, Facing::North);
    // The crate's one verb runs from the actions' Use, and its words are the line.
    tap(&mut app, KeyCode::Space);
    press_ui(&mut app, Pressable(ControlId::Action(0)));
    let s = session(&app);
    assert!(
        s.lines
            .iter()
            .any(|l| l == "A crate of rough planks, nailed shut."),
        "{:?}",
        s.lines
    );
    // Opened at the gate, the verbs close when the party steps away.
    let mut app = yard_app(3, 1, Facing::South);
    tap(&mut app, KeyCode::Space);
    press_ui(&mut app, Pressable(ControlId::Action(0)));
    assert_eq!(top(&app), Some(ScreenId::Verbs));
    // E turns the party right: the square has changed, so the verbs close.
    tap(&mut app, KeyCode::KeyE);
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(session(&app).world.position.facing, Facing::West);
    assert_eq!(top(&app), None, "turning away closes the verbs");
    assert!(
        !session(&app)
            .binder
            .log
            .iter()
            .any(|c| matches!(c, Command::Act { .. })),
        "nothing was acted on"
    );
}

#[test]
fn an_object_s_refusals_are_worded_short_with_items_by_name() {
    let data = objects_data();
    let cases = [
        (Refusal::NotOffered, "not now"),
        (Refusal::Spent, "done already"),
        (
            Refusal::Lacks {
                item: "base:item:thieves_tools".to_owned(),
            },
            "needs Thieves' tools",
        ),
        (
            Refusal::Flag {
                flag: "objects:flag:lever".to_owned(),
            },
            "something else must be done first",
        ),
        (
            Refusal::School {
                school: School::Abjuration,
            },
            "needs someone who knows Abjuration magic",
        ),
        (
            Refusal::Rule {
                rule: "objects:rule:x".to_owned(),
            },
            "not allowed now",
        ),
        (Refusal::NoneCanAct, "nobody can act"),
        (Refusal::Closed, "it is shut"),
        (Refusal::Full, "it is full"),
    ];
    for (refusal, words) in cases {
        assert_eq!(labels::object_refusal(&data, &refusal), words);
    }
    assert_eq!(
        labels::check(CheckKind::Skill(Skill::SleightOfHand), 10),
        "Sleight of Hand DC 10"
    );
    assert_eq!(
        labels::check(CheckKind::Ability(Ability::Strength), 12),
        "Strength DC 12"
    );
}

#[test]
fn shift_space_opens_the_verbs_while_the_mouse_looks_and_gives_the_pointer_back() {
    use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
    use omnis_vector::shell::movement::Intent;
    let mut app = yard_app(3, 1, Facing::South);
    click(&mut app, (800.0, 450.0));
    assert!(
        app.world().resource::<Intent>().looking,
        "a click in the view looks"
    );
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Pressed);
    tap(&mut app, KeyCode::Space);
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Released);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Verbs));
    assert!(
        !app.world().resource::<Intent>().looking,
        "the verbs need the pointer"
    );
    let world = app.world_mut();
    let mut cursor = world.query_filtered::<&CursorOptions, With<PrimaryWindow>>();
    assert_eq!(
        cursor.single(world).expect("the window's cursor").grab_mode,
        CursorGrabMode::None
    );
}
