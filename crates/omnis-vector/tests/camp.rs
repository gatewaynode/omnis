//! Camp (parity P6; presentation-ARCHITECTURE §6.7): each member's hit points and hit dice with
//! a count to spend capped at what the rest view allows, a short rest that spends them, a long
//! rest with the food it eats or its refusal, every button lit by the view and a trial. In the
//! app: the Camp button is live only while exploring, opens the camp beside the party column,
//! pauses the party, and a rest leaves the screen open with its line.

use crate::common;

use bevy::prelude::*;
use common::app::{app_with, met_with, press_ui, tap, top};
use common::ctx;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Event};
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, Screen, ScreenId};
use omnis_vector::shell::controls::Action;
use omnis_vector::shell::roster::RosterRoot;
use omnis_vector::shell::screen::{Arrow, Beside, Pressable, Screens};
use omnis_vector::shell::session::Session;
use omnis_vector::sim::SimWorld;

fn shown(data: &Data, world: &SimWorld, forms: &Forms) -> Screen {
    let views = common::views(world, data);
    screen::content(ScreenId::Camp, &ctx(data, world, &views), forms)
}

fn press(
    data: &Data,
    world: &SimWorld,
    forms: &mut Forms,
    id: ControlId,
    payload: &Payload,
) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    screen::apply(
        ScreenId::Camp,
        &ctx(data, world, &views),
        forms,
        id,
        payload,
        0,
    )
}

/// The row whose text starts with `name`.
fn row<'s>(screen: &'s Screen, name: &str) -> &'s screen::Row {
    screen.panels[0]
        .rows
        .iter()
        .find(|r| r.spans.first().is_some_and(|s| s.text.starts_with(name)))
        .unwrap_or_else(|| panic!("{name}'s row: {screen:?}"))
}

fn why_not(screen: &Screen, id: ControlId) -> Option<String> {
    screen.control(id).expect("on the camp").why_not.clone()
}

/// Every live button's command is one the engine accepts.
fn live_ones_are_accepted(data: &Data, world: &SimWorld, forms: &Forms) {
    let screen = shown(data, world, forms);
    for id in [ControlId::ShortRest, ControlId::LongRest] {
        if why_not(&screen, id).is_some() {
            continue;
        }
        let mut f = forms.clone();
        let Ok(Effect::Command { command, close }) =
            press(data, world, &mut f, id, &Payload::Press)
        else {
            panic!("{id:?} sends a command");
        };
        assert!(!close, "the camp stays open");
        assert!(world.trial(data, &command).is_ok(), "{id:?}: {command:?}");
    }
}

#[test]
fn the_rows_read_the_rest_view_and_a_count_runs_to_what_may_be_spent() {
    let data = common::data();
    let (mut world, _) = common::session(&data);
    let dead = data.registry.conditions.get("base:condition:dead").unwrap();
    world.party.members[0].hp = 2;
    world.party.members[2].hp = 0;
    world.party.members[2].conditions.push(dead);
    world.party.members[3].hp = 1;
    world.party.members[3].hit_dice_spent = 1;
    let views = common::views(&world, &data);
    let mut forms = Forms::default();
    let screen = shown(&data, &world, &forms);
    let food = &screen.panels[0].rows[0];
    assert_eq!(
        food.spans[0].text,
        format!(
            "The night eats {} food; the stores hold {}",
            views.rest.long_food, views.rest.food
        )
    );
    for (m, shown_row) in views.rest.members.iter().zip(&views.party.members) {
        let r = row(&screen, &shown_row.name);
        assert_eq!(
            r.spans[0].text,
            format!(
                "{}  HP {}/{}  Hit dice {}/{} d{}",
                shown_row.name, m.hp, m.hp_max, m.hit_dice_left, m.hit_dice, m.die
            )
        );
    }
    let note = |name: &str| row(&screen, name).after[0].text.clone();
    assert_eq!(note("Brenna"), "spend 0");
    assert_eq!(note("Durin"), "unhurt");
    assert_eq!(note("Ilvara"), "cannot spend");
    assert_eq!(note("Pip"), "no hit dice left");
    assert!(
        row(&screen, "Durin").controls.is_empty(),
        "nothing to spend, no count"
    );

    let spend = ControlId::Dice(0);
    let max = i64::from(views.rest.members[0].spendable);
    assert_eq!(max, 1, "Brenna, level 1, has one die");
    press(&data, &world, &mut forms, spend, &Payload::Press).unwrap();
    press(&data, &world, &mut forms, spend, &Payload::Press).unwrap();
    assert_eq!(forms.dice[0], 1, "held at what may be spent");
    assert_eq!(
        press(&data, &world, &mut forms, spend, &Payload::Set(max + 1)),
        Err(Refused::OutOfRange)
    );
    assert_eq!(note_of(&shown(&data, &world, &forms), "Brenna"), "spend 1");
    // A count held by the form is capped again when the view allows less.
    world.party.members[0].hp = world.party.members[0].hp_max;
    assert_eq!(
        screen::camp::counts(
            ctx(&data, &world, &common::views(&world, &data))
                .game
                .unwrap(),
            &forms
        )[0],
        0
    );
}

fn note_of(screen: &Screen, name: &str) -> String {
    row(screen, name).after[0].text.clone()
}

#[test]
fn a_short_rest_spends_the_dice_chosen_and_heals() {
    let data = common::data();
    let (mut world, _) = common::session(&data);
    world.party.members[0].hp = 2;
    let mut forms = Forms::default();
    assert_eq!(
        why_not(&shown(&data, &world, &forms), ControlId::ShortRest).as_deref(),
        Some("Choose hit dice to spend")
    );
    live_ones_are_accepted(&data, &world, &forms);
    press(
        &data,
        &world,
        &mut forms,
        ControlId::Dice(0),
        &Payload::Press,
    )
    .unwrap();
    assert_eq!(
        why_not(&shown(&data, &world, &forms), ControlId::ShortRest),
        None
    );
    live_ones_are_accepted(&data, &world, &forms);
    let Ok(Effect::Command { command, .. }) = press(
        &data,
        &world,
        &mut forms,
        ControlId::ShortRest,
        &Payload::Press,
    ) else {
        panic!("a short rest");
    };
    assert!(forms.dice.is_empty(), "the counts start over");
    let brenna = world.party.members[0].id;
    let events = world.apply(&data, command).expect("accepted");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::HitDiceSpent { member, dice: 1 } if *member == brenna)),
        "{events:?}"
    );
    assert!(world.party.members[0].hp > 2, "Brenna is healed");
    assert_eq!(world.party.members[0].hit_dice_spent, 1);
}

#[test]
fn a_long_rest_shows_its_refusal_too_soon_and_without_food() {
    let data = common::data();
    let (mut world, _) = common::session(&data);
    let mut forms = Forms::default();
    assert_eq!(
        why_not(&shown(&data, &world, &forms), ControlId::LongRest),
        None
    );
    live_ones_are_accepted(&data, &world, &forms);

    let mut hungry = world.clone();
    hungry.party.food = 0;
    let need = common::views(&hungry, &data).rest.long_food;
    assert_eq!(
        why_not(&shown(&data, &hungry, &forms), ControlId::LongRest),
        Some(format!("the rest eats {need} food; the stores hold 0"))
    );
    assert_eq!(
        press(
            &data,
            &hungry,
            &mut forms,
            ControlId::LongRest,
            &Payload::Press
        ),
        Err(Refused::Dim(format!(
            "the rest eats {need} food; the stores hold 0"
        )))
    );

    world
        .apply(&data, Command::Rest(omnis_sim::RestCommand::Long))
        .expect("the night's rest");
    let why = why_not(&shown(&data, &world, &forms), ControlId::LongRest).unwrap_or_default();
    assert!(why.starts_with("rested too recently ("), "{why}");
}

/// Whether the Camp button can be pressed now.
fn camp_live(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut q = world.query::<(&Action, Has<Button>)>();
    q.iter(world)
        .find(|(a, _)| **a == Action::Camp)
        .expect("the Camp button")
        .1
}

fn column_shown(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut root = world.query_filtered::<&Node, With<RosterRoot>>();
    root.single(world).expect("the column").display != Display::None
}

#[test]
fn the_camp_button_is_live_exploring_and_opens_the_camp_beside_the_column() {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    app.update();
    assert!(camp_live(&mut app), "exploring the street");
    let here = app.world().resource::<Session>().views.position;
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.world.party.members[0].hp = 2;
        session.refresh();
    }
    press_ui(&mut app, Action::Camp);
    assert_eq!(top(&app), Some(ScreenId::Camp));
    assert!(column_shown(&mut app), "the column stands beside the camp");
    assert!(app.world().resource::<Beside>().0 > 0.0);

    // The party does not walk with the camp open.
    common::app::key(&mut app, KeyCode::KeyW, bevy::input::ButtonState::Pressed);
    for _ in 0..30 {
        app.update();
    }
    common::app::key(&mut app, KeyCode::KeyW, bevy::input::ButtonState::Released);
    app.update();
    assert_eq!(app.world().resource::<Session>().views.position, here);

    press_ui(&mut app, Arrow(ControlId::Dice(0), true));
    press_ui(&mut app, Pressable(ControlId::ShortRest));
    assert_eq!(top(&app), Some(ScreenId::Camp), "the camp stays open");
    let message = app.world().resource::<Screens>().forms.message.clone();
    let text = message.map(|m| m.text).unwrap_or_default();
    assert!(text.starts_with("Brenna regains "), "{text}");

    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None, "Esc closes it");
    tap(&mut app, KeyCode::KeyR);
    assert_eq!(top(&app), Some(ScreenId::Camp), "R opens it");
    tap(&mut app, KeyCode::KeyR);
    assert_eq!(top(&app), None, "and closes it");
}

#[test]
fn the_camp_button_is_dim_in_a_service_and_in_a_fight() {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let town = common::map(&session.data, "test:map:town");
        common::place(&mut session.world, town, 1, 3, Facing::North);
        session.refresh();
        session.order(Command::Interact);
    }
    app.update();
    assert!(
        app.world()
            .resource::<Session>()
            .views
            .here
            .service
            .is_some(),
        "inside the smith"
    );
    assert!(!camp_live(&mut app), "the room is the rest indoors");
    tap(&mut app, KeyCode::KeyR);
    assert_eq!(top(&app), Some(ScreenId::Service), "R does nothing here");

    let mut app = met_with(1);
    app.update();
    assert!(!camp_live(&mut app), "not in a fight");
}
