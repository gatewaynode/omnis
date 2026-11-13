//! The loot window (owner, 2026-11-12): what lies in a place within reach, for the party to
//! take. On the yard fixture: the arrow pile at (2, 2) from the cell beside it, the chest at
//! (5, 3) once open. The pure screen first (its rows, who receives, what each button sends,
//! when it has nothing left), then the app: Shift+Space and a left click on the heap open it,
//! a click beside the heap is the view's, and the window closes itself when emptied.

use crate::common;

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use common::app::{click, config, press_ui, shell, tap, top};
use common::{ctx, objects_data, place, town_session};
use omnis_sim::event::ItemPlace;
use omnis_sim::omnis_core::{CharacterId, Facing};
use omnis_sim::omnis_data::{Data, Verb};
use omnis_sim::{Command, ItemCommand};
use omnis_vector::aim::{self, Eye};
use omnis_vector::ground::{Ground, eye};
use omnis_vector::screen::loot::{self, LootAct};
use omnis_vector::screen::{self, ControlId, Effect, Forms, Kind, Payload, Screen, ScreenId};
use omnis_vector::shapes::{Body, PILE_LOOK};
use omnis_vector::shell::aim::Aimed;
use omnis_vector::shell::movement::Intent;
use omnis_vector::shell::screen::{Pressable, Screens};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::{Config, Session};
use omnis_vector::sim::SimWorld;
use std::path::PathBuf;

const DAGGER: &str = "base:item:dagger";
const ARROWS: &str = "base:item:arrows";

pub(crate) fn yard(data: &Data, x: u16, y: u16, facing: Facing) -> SimWorld {
    let (mut world, _) = town_session(data);
    let map = common::map(data, "objects:map:yard");
    place(&mut world, map, x, y, facing);
    world
}

fn shown(data: &Data, world: &SimWorld, forms: &Forms) -> Screen {
    let views = common::views(world, data);
    screen::content(ScreenId::Loot, &ctx(data, world, &views), forms)
}

fn press(
    data: &Data,
    world: &SimWorld,
    forms: &mut Forms,
    act: LootAct,
    payload: &Payload,
) -> Effect {
    let views = common::views(world, data);
    screen::apply(
        ScreenId::Loot,
        &ctx(data, world, &views),
        forms,
        ControlId::Loot(act),
        payload,
        0,
    )
    .unwrap_or_else(|why| panic!("{act:?}: {why:?}"))
}

fn gone(data: &Data, world: &SimWorld, forms: &Forms) -> bool {
    loot::gone(data, &common::views(world, data), forms)
}

/// The rows as words, a control as "[label]" or "[label: reason]".
fn rows(screen: &Screen) -> Vec<String> {
    screen.panels[0]
        .rows
        .iter()
        .map(|r| {
            let mut words: Vec<String> = r.spans.iter().map(|s| s.text.clone()).collect();
            words.extend(r.controls.iter().map(|c| match &c.why_not {
                Some(why) => format!("[{}: {why}]", c.label),
                None => format!("[{}]", c.label),
            }));
            words.join(" ")
        })
        .collect()
}

fn pick_up(member: CharacterId, item: &str, count: u16) -> Command {
    Command::Item(ItemCommand::PickUp {
        member,
        item: item.into(),
        count,
    })
}

#[test]
fn the_window_lists_the_pile_ahead_with_take_take_all_and_take_everything() {
    let data = objects_data();
    let world = yard(&data, 1, 2, Facing::East);
    let forms = Forms::default();
    let screen = shown(&data, &world, &forms);
    assert_eq!(screen.title.text, "On the ground");
    let arrows = omnis_vector::labels::item(&data, ARROWS);
    assert_eq!(
        rows(&screen),
        [
            "[Taken by] [Close]".to_owned(),
            format!("{arrows} x10 [Take] [Take all] [Store] [Destroy]"),
            "[Take everything]".to_owned(),
        ]
    );
    let views = common::views(&world, &data);
    let names: Vec<String> = views.party.members.iter().map(|m| m.name.clone()).collect();
    let chooser = screen.control(ControlId::Loot(LootAct::Receiver)).unwrap();
    assert_eq!(
        chooser.kind,
        Kind::Choice {
            options: names,
            at: 0
        },
        "everyone, starting on the first on their feet"
    );
    assert!(!gone(&data, &world, &forms));

    // Turned away, the pile is out of reach: nothing to show.
    let away = yard(&data, 1, 2, Facing::North);
    assert!(gone(&data, &away, &forms));
    assert_eq!(
        rows(&shown(&data, &away, &forms)),
        ["[Taken by] [Close]", "Nothing left"]
    );
}

#[test]
fn each_button_sends_its_take_to_the_one_who_receives() {
    let data = objects_data();
    let world = yard(&data, 1, 2, Facing::East);
    let first = common::id(&world, 0);
    let second = common::id(&world, 1);
    let rogue = common::id(&world, 3);
    let mut forms = Forms::default();
    let sent = |forms: &mut Forms, act| match press(&data, &world, forms, act, &Payload::Press) {
        Effect::Command { command, close } => {
            assert!(!close, "the window stays open");
            assert!(world.trial(&data, &command).is_ok(), "{command:?}");
            command
        }
        other => panic!("{act:?} sends one command: {other:?}"),
    };
    assert_eq!(
        sent(&mut forms, LootAct::Take(0)),
        pick_up(first, ARROWS, 1)
    );
    assert_eq!(
        sent(&mut forms, LootAct::TakeAll(0)),
        pick_up(first, ARROWS, 10)
    );
    assert_eq!(
        sent(&mut forms, LootAct::Everything),
        pick_up(first, ARROWS, 10),
        "one kind of item, one command"
    );

    // The member selected in the party column receives.
    forms.selected = Some(rogue);
    assert_eq!(
        sent(&mut forms, LootAct::Take(0)),
        pick_up(rogue, ARROWS, 1)
    );
    // The window's own choice wins over the column's: a step right of the rogue wraps to the
    // first, a step left of them is the rogue again.
    assert_eq!(
        press(
            &data,
            &world,
            &mut forms,
            LootAct::Receiver,
            &Payload::Right
        ),
        Effect::None
    );
    assert_eq!(
        sent(&mut forms, LootAct::Take(0)),
        pick_up(first, ARROWS, 1)
    );
    press(
        &data,
        &world,
        &mut forms,
        LootAct::Receiver,
        &Payload::Press,
    );
    assert_eq!(
        sent(&mut forms, LootAct::Take(0)),
        pick_up(second, ARROWS, 1)
    );
    press(&data, &world, &mut forms, LootAct::Receiver, &Payload::Left);
    press(&data, &world, &mut forms, LootAct::Receiver, &Payload::Left);
    assert_eq!(
        sent(&mut forms, LootAct::Take(0)),
        pick_up(rogue, ARROWS, 1)
    );
}

#[test]
fn take_everything_sends_one_command_an_item_and_a_fallen_receiver_dims_the_buttons() {
    let data = objects_data();
    let mut world = yard(&data, 1, 2, Facing::East);
    let first = common::id(&world, 0);
    let rogue = common::id(&world, 3);
    world
        .apply(
            &data,
            Command::Item(ItemCommand::Drop {
                member: rogue,
                item: DAGGER.into(),
                count: 1,
            }),
        )
        .expect("the rogue lays a dagger down");
    let mut forms = Forms::default();
    assert_eq!(
        press(
            &data,
            &world,
            &mut forms,
            LootAct::Everything,
            &Payload::Press
        ),
        Effect::Commands(vec![pick_up(first, DAGGER, 1), pick_up(first, ARROWS, 10)]),
        "underfoot first, then the cell ahead"
    );
    let dagger = omnis_vector::labels::item(&data, DAGGER);
    assert_eq!(
        rows(&shown(&data, &world, &forms))[1],
        format!("{dagger} [Take] [Store] [Destroy]"),
        "one of it: no Take all"
    );

    // Someone who cannot act takes nothing: the engine's reason is on every button.
    let dead = data.registry.conditions.get("base:condition:dead").unwrap();
    world.party.members[0].hp = 0;
    world.party.members[0].conditions.push(dead);
    let second = common::id(&world, 1);
    assert_eq!(
        press(&data, &world, &mut forms, LootAct::Take(0), &Payload::Press),
        Effect::Command {
            command: pick_up(second, DAGGER, 1),
            close: false
        },
        "the first on their feet receives when no one is chosen"
    );
    forms.selected = Some(first);
    let screen = shown(&data, &world, &forms);
    for act in [
        LootAct::Take(0),
        LootAct::Take(1),
        LootAct::TakeAll(1),
        LootAct::Everything,
    ] {
        let control = screen.control(ControlId::Loot(act)).expect("on the window");
        assert!(control.why_not.is_some(), "{act:?} is dim: {control:?}");
    }
    // The stores need no hands, and neither does an end: those stay lit.
    for act in [LootAct::Store(0), LootAct::Destroy(1)] {
        let control = screen.control(ControlId::Loot(act)).expect("on the window");
        assert_eq!(control.why_not, None, "{act:?}");
    }
    let views = common::views(&world, &data);
    assert_eq!(
        screen::apply(
            ScreenId::Loot,
            &ctx(&data, &world, &views),
            &mut forms,
            ControlId::Loot(LootAct::Everything),
            &Payload::Press,
            0
        )
        .map_err(|_| ()),
        Err(()),
        "a dim button sends nothing"
    );
}

fn open_chest(data: &Data, world: &mut SimWorld) {
    for _ in 0..50 {
        let _ = world.apply(
            data,
            Command::Act {
                object: "chest_1".to_owned(),
                verb: Verb::Open,
                option: 1,
            },
        );
        let views = common::views(world, data);
        let chest = views.objects.objects.iter().find(|o| o.object == "chest_1");
        if chest.is_some_and(|c| c.contents.is_some()) {
            return;
        }
    }
    panic!("the lock never gave");
}

#[test]
fn an_open_container_goes_through_the_same_window_and_a_shut_one_shows_nothing() {
    let data = objects_data();
    let mut world = yard(&data, 5, 3, Facing::North);
    let first = common::id(&world, 0);
    let mut forms = Forms::default();
    forms.loot.from = ItemPlace::Object("chest_1".to_owned());
    assert!(gone(&data, &world, &forms), "the chest is shut");
    open_chest(&data, &mut world);
    assert!(!gone(&data, &world, &forms));
    let screen = shown(&data, &world, &forms);
    let views = common::views(&world, &data);
    let chest = views
        .objects
        .objects
        .iter()
        .find(|o| o.object == "chest_1")
        .unwrap();
    assert_eq!(
        screen.title.text,
        omnis_vector::labels::text(&data, &chest.name)
    );
    let dagger = omnis_vector::labels::item(&data, DAGGER);
    assert_eq!(
        rows(&screen)[1],
        format!("{dagger} [Take] [Store] [Destroy]")
    );
    assert_eq!(
        press(
            &data,
            &world,
            &mut forms,
            LootAct::Store(0),
            &Payload::Press
        ),
        Effect::Command {
            command: Command::Item(ItemCommand::Stash {
                from: ItemPlace::Object("chest_1".into()),
                item: DAGGER.into(),
                count: 1,
            }),
            close: false
        },
        "the same Store, from the chest"
    );
    let take_out = Command::Item(ItemCommand::TakeOut {
        member: first,
        object: "chest_1".into(),
        item: DAGGER.into(),
        count: 1,
    });
    assert_eq!(
        press(&data, &world, &mut forms, LootAct::Take(0), &Payload::Press),
        Effect::Command {
            command: take_out.clone(),
            close: false
        }
    );
    world.apply(&data, take_out).expect("accepted");
    assert!(gone(&data, &world, &forms), "emptied");
    // A place nothing is taken from this way is never shown.
    forms.loot.from = ItemPlace::Stores;
    assert!(gone(&data, &world, &forms));
}

#[test]
fn store_sends_all_of_it_to_the_stores_and_destroy_asks_before_anything_is_sent() {
    let data = objects_data();
    let world = yard(&data, 1, 2, Facing::East);
    let mut forms = Forms::default();
    let arrows = omnis_vector::labels::item(&data, ARROWS);
    let gone_for_good = |keep: bool| {
        let (from, item, count) = (ItemPlace::Ground, ARROWS.to_owned(), 10);
        Effect::Command {
            command: Command::Item(if keep {
                ItemCommand::Stash { from, item, count }
            } else {
                ItemCommand::Destroy { from, item, count }
            }),
            close: false,
        }
    };
    assert_eq!(
        press(
            &data,
            &world,
            &mut forms,
            LootAct::Store(0),
            &Payload::Press
        ),
        gone_for_good(true)
    );
    let listed = rows(&shown(&data, &world, &forms));

    // Destroy sends nothing: the window asks, with nothing else to press.
    assert_eq!(
        press(
            &data,
            &world,
            &mut forms,
            LootAct::Destroy(0),
            &Payload::Press
        ),
        Effect::None
    );
    assert_eq!(forms.loot.asking, Some((ARROWS.to_owned(), 10)));
    let question = shown(&data, &world, &forms);
    assert_eq!(
        rows(&question),
        [
            format!("Destroy {arrows} x10? This cannot be undone."),
            "[Destroy] [Cancel]".to_owned(),
        ]
    );
    let views = common::views(&world, &data);
    for act in [LootAct::Take(0), LootAct::Store(0), LootAct::Everything] {
        assert!(
            screen::apply(
                ScreenId::Loot,
                &ctx(&data, &world, &views),
                &mut forms,
                ControlId::Loot(act),
                &Payload::Press,
                0
            )
            .is_err(),
            "{act:?} is not on the question"
        );
    }

    // Cancel: the rows are back as they were, and nothing was sent.
    assert_eq!(
        press(&data, &world, &mut forms, LootAct::Cancel, &Payload::Press),
        Effect::None
    );
    assert_eq!(forms.loot.asking, None);
    assert_eq!(rows(&shown(&data, &world, &forms)), listed);

    // Asked again and answered: the one command, and the question is over.
    press(
        &data,
        &world,
        &mut forms,
        LootAct::Destroy(0),
        &Payload::Press,
    );
    assert_eq!(
        press(&data, &world, &mut forms, LootAct::Confirm, &Payload::Press),
        gone_for_good(false)
    );
    assert_eq!(forms.loot.asking, None);
}

/// The shell on the yard, the party at `(x, y)` facing `facing`.
pub(crate) fn yard_app(x: u16, y: u16, facing: Facing) -> App {
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

fn ground_of(app: &App) -> Vec<(String, u16)> {
    let s = app.world().resource::<Session>();
    s.views
        .objects
        .ground
        .iter()
        .map(|i| (i.item.clone(), i.count))
        .collect()
}

#[test]
fn shift_space_opens_the_window_take_everything_empties_the_pile_and_it_closes() {
    let mut app = yard_app(1, 2, Facing::East);
    common::app::key(
        &mut app,
        KeyCode::ShiftLeft,
        bevy::input::ButtonState::Pressed,
    );
    tap(&mut app, KeyCode::Space);
    common::app::key(
        &mut app,
        KeyCode::ShiftLeft,
        bevy::input::ButtonState::Released,
    );
    assert_eq!(top(&app), Some(ScreenId::Loot), "the pile is the default");
    // What the window was left on does not outlive it: the next opening is the ground's, with
    // no one chosen.
    {
        let mut screens = app.world_mut().resource_mut::<Screens>();
        screens.forms.loot.receiver = Some(CharacterId(3));
    }
    tap(&mut app, KeyCode::Escape);
    app.update();
    assert_eq!(top(&app), None);
    assert_eq!(
        app.world().resource::<Screens>().forms.loot,
        loot::Loot::default()
    );
    common::app::key(
        &mut app,
        KeyCode::ShiftLeft,
        bevy::input::ButtonState::Pressed,
    );
    tap(&mut app, KeyCode::Space);
    common::app::key(
        &mut app,
        KeyCode::ShiftLeft,
        bevy::input::ButtonState::Released,
    );
    assert_eq!(top(&app), Some(ScreenId::Loot));
    let text = screen_text(app.world());
    for word in ["On the ground", "Take all", "Take everything", "Taken by"] {
        assert!(text.contains(word), "{word}: {text}");
    }
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Take(0))));
    assert_eq!(ground_of(&app), [(ARROWS.to_owned(), 9)]);
    assert_eq!(top(&app), Some(ScreenId::Loot), "the window stays open");
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Everything)));
    assert_eq!(ground_of(&app), []);
    app.update();
    assert_eq!(top(&app), None, "nothing left: the window closed itself");
    let s = app.world().resource::<Session>();
    assert!(
        s.lines.iter().any(|l| l.contains("picks up")),
        "{:?}",
        s.lines
    );
    assert_eq!(s.binder.refusals, 0);
    let first = s.views.party.members[0].member;
    let held = &s.views.party.members[0].equipment;
    assert_eq!(
        held.iter().find(|i| i.item == ARROWS).map(|i| i.count),
        Some(10),
        "{first:?} holds them"
    );
}

/// Shift+Space with the pile ahead: the loot window.
fn open_window(app: &mut App) {
    let shift = |app: &mut App, state| common::app::key(app, KeyCode::ShiftLeft, state);
    shift(app, bevy::input::ButtonState::Pressed);
    tap(app, KeyCode::Space);
    shift(app, bevy::input::ButtonState::Released);
    assert_eq!(top(app), Some(ScreenId::Loot));
}

fn stored(app: &App, item: &str) -> u16 {
    let s = app.world().resource::<Session>();
    let stores = &s.views.party.inventory;
    stores
        .iter()
        .filter(|i| i.item == item)
        .map(|i| i.count)
        .sum()
}

#[test]
fn in_the_app_store_fills_the_stores_and_the_window_closes_on_the_empty_pile() {
    let mut app = yard_app(1, 2, Facing::East);
    let before = stored(&app, ARROWS);
    open_window(&mut app);
    let text = screen_text(app.world());
    for word in ["Store", "Destroy"] {
        assert!(text.contains(word), "{word}: {text}");
    }
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Store(0))));
    assert_eq!(ground_of(&app), []);
    assert_eq!(stored(&app, ARROWS), before + 10);
    app.update();
    assert_eq!(top(&app), None, "nothing left: the window closed itself");
    let s = app.world().resource::<Session>();
    assert!(
        s.lines.iter().any(|l| l.contains("goes into the stores")),
        "{:?}",
        s.lines
    );
    assert_eq!(s.binder.refusals, 0);
}

#[test]
fn in_the_app_destroy_asks_first_cancel_changes_nothing_and_yes_ends_the_pile() {
    let mut app = yard_app(1, 2, Facing::East);
    let before = stored(&app, ARROWS);
    open_window(&mut app);
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Destroy(0))));
    let text = screen_text(app.world());
    assert!(text.contains("This cannot be undone."), "{text}");
    assert!(!text.contains("Take everything"), "{text}");
    assert_eq!(ground_of(&app), [(ARROWS.to_owned(), 10)], "only asked");
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Cancel)));
    assert_eq!(ground_of(&app), [(ARROWS.to_owned(), 10)]);
    assert_eq!(top(&app), Some(ScreenId::Loot));
    let text = screen_text(app.world());
    assert!(text.contains("Take everything"), "{text}");

    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Destroy(0))));
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Confirm)));
    assert_eq!(ground_of(&app), []);
    assert_eq!(stored(&app, ARROWS), before, "into nobody's hands");
    app.update();
    assert_eq!(top(&app), None, "nothing left: the window closed itself");
    let s = app.world().resource::<Session>();
    assert!(
        s.lines.iter().any(|l| l.contains("are destroyed")),
        "{:?}",
        s.lines
    );
    assert_eq!(s.binder.refusals, 0);
    let held = &s.views.party.members[0].equipment;
    assert_eq!(
        held.iter().find(|i| i.item == ARROWS).map(|i| i.count),
        None
    );
}

/// Where the middle of the arrow pile is drawn in a 1600 by 900 window, from the session's pose.
fn heap_on_screen(app: &App) -> (f32, f32) {
    let s = app.world().resource::<Session>();
    let pos = s.world.exploring_at(&s.data).expect("exploring");
    let ground = Ground::from_map(s.data.maps.get(&pos.map).unwrap(), pos.level);
    let (_, piles) = s.world.objects_on(&s.data, pos.map, pos.level);
    let pile = &piles[0];
    let body = Body::of(pile.x, pile.y, pile.pos, Facing::North, &PILE_LOOK, &ground);
    let view = Eye {
        at: [s.pose.x, eye(&ground, s.pose), s.pose.z],
        yaw: s.pose.yaw,
        pitch: 0.0,
        fov: 0.96,
    };
    let [l, t, r, b] = aim::rect(&view, (1600.0, 900.0), &body).expect("ahead of the eye");
    assert!(
        l < 1600.0 && r > 0.0 && t < 900.0 && b > 0.0,
        "on the screen: {l} {t} {r} {b}"
    );
    // A heap near the eye runs off the bottom: the point is the middle of what shows.
    (
        (l.max(0.0) + r.min(1600.0)) / 2.0,
        (t.max(0.0) + b.min(900.0)) / 2.0,
    )
}

pub(crate) fn cursor(app: &mut App) -> Option<CursorIcon> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Option<&CursorIcon>, With<PrimaryWindow>>();
    q.single(world).ok().flatten().cloned()
}

pub(crate) fn hover(app: &mut App, point: (f32, f32)) {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    q.single_mut(world)
        .expect("the window")
        .set_cursor_position(Some(Vec2::new(point.0, point.1)));
    app.update();
    app.update();
}

#[test]
fn a_click_on_the_heap_opens_the_window_and_a_click_beside_it_is_the_view_s() {
    let mut app = yard_app(1, 2, Facing::East);
    app.world_mut().spawn((
        Window {
            resolution: bevy::window::WindowResolution::new(1600, 900),
            ..default()
        },
        PrimaryWindow,
    ));
    app.update();
    let heap = heap_on_screen(&app);
    let looking = |app: &App| app.world().resource::<Intent>().looking;
    let aimed = |app: &App| app.world().resource::<Aimed>().0.is_some();

    // Over the sky: nothing to click, and the click takes the mouse look as before.
    hover(&mut app, (800.0, 100.0));
    assert!(!aimed(&app));
    assert_ne!(
        cursor(&mut app),
        Some(CursorIcon::System(SystemCursorIcon::Pointer))
    );
    click(&mut app, (800.0, 100.0));
    assert!(looking(&app), "a click in the view takes the mouse look");
    assert_eq!(top(&app), None);
    tap(&mut app, KeyCode::Escape);
    tap(&mut app, KeyCode::Escape);
    assert!(!looking(&app));
    assert_eq!(
        top(&app),
        None,
        "Esc gave the pointer back and closed the pause"
    );

    // Over the heap: the hand, and the click is the pile's.
    hover(&mut app, heap);
    assert!(aimed(&app));
    assert_eq!(
        cursor(&mut app),
        Some(CursorIcon::System(SystemCursorIcon::Pointer))
    );
    // The frame the button goes down: the window is up and the mouse look never began.
    let window = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<PrimaryWindow>>();
        q.single(world).expect("the window")
    };
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: MouseButton::Left,
            state: bevy::input::ButtonState::Pressed,
            window,
        });
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Loot));
    assert!(!looking(&app), "the click was not the view's");
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: MouseButton::Left,
            state: bevy::input::ButtonState::Released,
            window,
        });
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Loot));
    assert!(!looking(&app));
    assert!(
        !aimed(&app),
        "a screen is up: nothing in the view is clicked"
    );
    assert_eq!(
        cursor(&mut app),
        Some(CursorIcon::System(SystemCursorIcon::Default))
    );

    // Turned away with the pointer where the heap was: nothing there.
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None);
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = place(&mut session.world, map, 1, 2, Facing::West);
        session.refresh();
    }
    hover(&mut app, heap);
    assert!(!aimed(&app), "the pile is behind the party, out of reach");

    // South of the hedge, facing it: the heap is drawn straight ahead, and is not in reach.
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = place(&mut session.world, map, 2, 3, Facing::North);
        session.refresh();
    }
    app.update();
    let heap = heap_on_screen(&app);
    hover(&mut app, heap);
    assert!(!aimed(&app), "a hedge stands between");
    click(&mut app, heap);
    assert_eq!(top(&app), None);
    assert!(looking(&app), "so the click is the view's");
}

#[test]
fn a_point_ahead_is_drawn_in_the_middle_and_one_behind_is_nowhere() {
    use omnis_vector::shell::render::rotation;
    let size = (1600.0, 900.0);
    let view = Eye {
        at: [3.5, 0.6, 7.5],
        yaw: 0.0,
        pitch: 0.0,
        fov: 0.96,
    };
    // Yaw 0 looks north, which is −Z.
    let ahead = aim::project(&view, size, [3.5, 0.6, 5.5]).expect("in front");
    assert!(
        (ahead.0 - 800.0).abs() < 0.01 && (ahead.1 - 450.0).abs() < 0.01,
        "{ahead:?}"
    );
    assert_eq!(aim::project(&view, size, [3.5, 0.6, 9.5]), None, "behind");
    let right = aim::project(&view, size, [4.0, 0.6, 5.5]).unwrap();
    let low = aim::project(&view, size, [3.5, 0.0, 5.5]).unwrap();
    assert!(
        right.0 > 800.0 && (right.1 - 450.0).abs() < 0.01,
        "east is right: {right:?}"
    );
    assert!(
        low.1 > 450.0 && (low.0 - 800.0).abs() < 0.01,
        "the floor is below: {low:?}"
    );

    // Any turn and tilt: the same place the camera's own rotation and a perspective put it.
    for (yaw, pitch) in [
        (0.7_f32, 0.3_f32),
        (-2.1, -0.5),
        (3.0, 0.9),
        (core::f32::consts::FRAC_PI_2, 0.0),
    ] {
        let view = Eye { yaw, pitch, ..view };
        for point in [
            [4.1, 0.2, 6.0],
            [2.0, 1.4, 9.0],
            [5.5, 0.0, 7.4],
            [3.4, 0.9, 3.0],
        ] {
            let local = rotation(yaw, pitch).inverse()
                * (Vec3::from_array(point) - Vec3::from_array(view.at));
            let got = aim::project(&view, size, point);
            if local.z > -0.05 {
                assert_eq!(got, None, "behind at {yaw} {pitch}: {point:?}");
                continue;
            }
            let half = (0.96_f32 / 2.0).tan();
            let want = (
                (local.x / (-local.z * half * (16.0 / 9.0)) + 1.0) * 800.0,
                (1.0 - local.y / (-local.z * half)) * 450.0,
            );
            let got = got.expect("in front");
            assert!(
                // A point just in front of the eye lands far off the screen: the slack grows with it.
                (got.0 - want.0).abs() < 0.05 + want.0.abs() * 1e-4
                    && (got.1 - want.1).abs() < 0.05 + want.1.abs() * 1e-4,
                "{yaw} {pitch} {point:?}: {got:?} is not {want:?}"
            );
        }
    }
}

#[test]
fn the_pointer_is_on_a_pile_inside_its_rectangle_and_a_margin_and_not_beyond() {
    let data = objects_data();
    let world = yard(&data, 1, 2, Facing::East);
    let pos = world.exploring_at(&data).unwrap();
    let ground = Ground::from_map(data.maps.get(&pos.map).unwrap(), pos.level);
    let piles = world.piles_in_reach(&data);
    assert_eq!(piles.len(), 1);
    let pose = omnis_vector::pose::Pose::at(pos);
    let view = Eye {
        at: [pose.x, eye(&ground, pose), pose.z],
        yaw: pose.yaw,
        pitch: 0.0,
        fov: 0.96,
    };
    let size = (1600.0, 900.0);
    let body = Body::of(
        piles[0].x,
        piles[0].y,
        piles[0].pos,
        Facing::North,
        &PILE_LOOK,
        &ground,
    );
    let [l, t, r, b] = aim::rect(&view, size, &body).unwrap();
    assert!(
        r - l > 20.0 && b - t > 5.0,
        "a heap, not a dot: {l} {t} {r} {b}"
    );
    assert!(t > 450.0, "on the floor, below the horizon");
    // The heap's own rectangle; the floor of its cell is a target too (`pile_reach.rs`).
    let on = |x: f32, y: f32| aim::on_heap((x, y), size, &view, &body);
    let m = aim::MARGIN;
    assert_eq!(
        aim::pile_under(((l + r) / 2.0, (t + b) / 2.0), size, &view, &piles, &ground),
        Some(0)
    );
    assert!(on((l + r) / 2.0, (t + b) / 2.0));
    assert!(on(l - m + 1.0, t - m + 1.0), "inside the margin");
    assert!(on(r + m - 1.0, b + m - 1.0));
    assert!(!on(l - m - 1.0, (t + b) / 2.0), "left of it");
    assert!(!on(r + m + 1.0, (t + b) / 2.0), "right of it");
    assert!(!on((l + r) / 2.0, t - m - 1.0), "above it");
    assert!(!on((l + r) / 2.0, b + m + 1.0), "below it");
    // Looking the other way, the heap is behind the eye: never under the pointer.
    let back = Eye {
        yaw: view.yaw + core::f32::consts::PI,
        ..view
    };
    assert_eq!(aim::rect(&back, size, &body), None);
    assert_eq!(
        aim::pile_under((800.0, 450.0), size, &back, &piles, &ground),
        None
    );
}
