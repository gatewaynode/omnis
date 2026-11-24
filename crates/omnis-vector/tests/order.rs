//! The marching order (owner, 2026-11-24): the party as it marches with an Up and a Down for
//! each member, every press the engine's `Reorder` with two neighbours swapped; opened from
//! the party column's Order button, which is dim in a fight with the engine's reason.

use crate::common;
use crate::common::app::Pointing;

use bevy::prelude::*;
use common::app::{app_with, met_with, set, top};
use common::ctx;
use omnis_sim::Command;
use omnis_sim::api::PartyCommand;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, Screen, ScreenId};
use omnis_vector::shell::screen::{Pressable, Side};
use omnis_vector::shell::session::Session;
use omnis_vector::sim::SimWorld;

fn shown(data: &Data, world: &SimWorld) -> Screen {
    let views = common::views(world, data);
    screen::content(
        ScreenId::Order,
        &ctx(data, world, &views),
        &Forms::default(),
    )
}

fn press(data: &Data, world: &SimWorld, id: ControlId) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    let mut forms = Forms::default();
    screen::apply(
        ScreenId::Order,
        &ctx(data, world, &views),
        &mut forms,
        id,
        &Payload::Press,
        0,
    )
}

fn order(world: &SimWorld, data: &Data) -> Vec<CharacterId> {
    world.party(data).members.iter().map(|m| m.member).collect()
}

fn shift(member: CharacterId, up: bool) -> ControlId {
    ControlId::Shift { member, up }
}

fn why_not(screen: &Screen, id: ControlId) -> Option<String> {
    screen.control(id).expect("on the order").why_not.clone()
}

/// The texts of the rows that are only text or a member's line, top to bottom after the head.
fn lines(screen: &Screen) -> Vec<String> {
    screen.panels[0].rows[1..]
        .iter()
        .map(|r| r.spans[0].text.clone())
        .collect()
}

#[test]
fn the_rows_are_the_party_as_it_marches_with_the_ends_dim() {
    let data = common::data();
    let (world, _) = common::session(&data);
    let party = world.party(&data);
    assert_eq!(party.members.len(), 4);
    assert_eq!(party.front_row, 3, "the fixture's rule");
    let screen = shown(&data, &world);
    let got = lines(&screen);
    assert_eq!(got[0], "front");
    assert!(got[1].starts_with("Brenna"), "{got:?}");
    assert!(got[2].starts_with("Durin"));
    assert!(got[3].starts_with("Ilvara"));
    assert_eq!(got[4], "back");
    assert!(got[5].starts_with("Pip"));
    assert_eq!(got.len(), 6);
    let ids = order(&world, &data);
    assert_eq!(
        why_not(&screen, shift(ids[0], true)).as_deref(),
        Some("Already first")
    );
    assert_eq!(
        why_not(&screen, shift(ids[3], false)).as_deref(),
        Some("Already last")
    );
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(why_not(&screen, shift(*id, true)).is_some(), i == 0);
        assert_eq!(why_not(&screen, shift(*id, false)).is_some(), i == 3);
    }
    assert_eq!(
        press(&data, &world, shift(ids[0], true)),
        Err(Refused::Dim("Already first".into()))
    );
}

#[test]
fn a_press_swaps_two_neighbours_and_the_line_between_the_rows_is_crossed() {
    let data = common::data();
    let (mut world, _) = common::session(&data);
    let ids = order(&world, &data);
    // Pip, last and alone in the back, goes up: Ilvara takes the back row.
    let Ok(Effect::Command { command, close }) = press(&data, &world, shift(ids[3], true)) else {
        panic!("Up sends a command");
    };
    assert!(!close, "the order stays open");
    assert_eq!(
        command,
        Command::Party(PartyCommand::Reorder {
            order: vec![ids[0], ids[1], ids[3], ids[2]]
        })
    );
    world.apply(&data, command).expect("the engine takes it");
    assert_eq!(order(&world, &data), [ids[0], ids[1], ids[3], ids[2]]);
    let got = lines(&shown(&data, &world));
    assert!(got[3].starts_with("Pip"), "{got:?}");
    assert_eq!(got[4], "back");
    assert!(got[5].starts_with("Ilvara"));
    let party = world.party(&data);
    assert!(party.members[2].in_front && !party.members[3].in_front);

    // Down from the front: Brenna and Durin change places.
    let Ok(Effect::Command { command, .. }) = press(&data, &world, shift(ids[0], false)) else {
        panic!("Down sends a command");
    };
    world.apply(&data, command).expect("the engine takes it");
    assert_eq!(order(&world, &data), [ids[1], ids[0], ids[3], ids[2]]);
    assert_eq!(press(&data, &world, ControlId::Close), Ok(Effect::Back));
}

fn press_side(app: &mut App, id: ControlId) {
    set(app, &Side(id), Pointing::Press);
    app.update();
    common::app::let_go(app, &Side(id));
    app.update();
}

fn press_screen(app: &mut App, id: ControlId) {
    set(app, &Pressable(id), Pointing::Press);
    app.update();
    common::app::let_go(app, &Pressable(id));
    app.update();
}

fn marching(app: &App) -> Vec<CharacterId> {
    let session = app.world().resource::<Session>();
    session
        .views
        .party
        .members
        .iter()
        .map(|m| m.member)
        .collect()
}

#[test]
fn the_column_s_button_opens_the_order_and_a_press_moves_the_member() {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    app.update();
    let ids = marching(&app);
    press_side(&mut app, ControlId::Order);
    assert_eq!(top(&app), Some(ScreenId::Order));
    press_screen(&mut app, shift(ids[2], true));
    assert_eq!(marching(&app), [ids[0], ids[2], ids[1], ids[3]]);
    assert_eq!(top(&app), Some(ScreenId::Order), "it stays open");
    assert_eq!(
        app.world().resource::<Session>().binder.refusals,
        0,
        "the engine took it"
    );
    press_screen(&mut app, ControlId::Close);
    assert_eq!(top(&app), None);
}

#[test]
fn the_order_is_not_set_in_a_fight() {
    let app = met_with(1);
    let before = marching(&app);
    let session = app.world().resource::<Session>();
    let game = screen::Game {
        world: &session.world,
        views: &session.views,
        log: &[],
        unreplayable: None,
        history: &session.history,
    };
    let panel = screen::roster::content(&session.data, game, None);
    let button = panel
        .rows
        .iter()
        .flat_map(|r| &r.controls)
        .find(|c| c.id == ControlId::Order)
        .expect("the column's Order button");
    let why = button
        .why_not
        .clone()
        .expect("dim with the engine's reason");
    assert_eq!(
        screen::roster::apply(&session.data, game, &mut None, ControlId::Order),
        Err(Refused::Dim(why))
    );
    assert!(
        !app.world()
            .resource::<omnis_vector::shell::screen::Screens>()
            .nav
            .allowed(ScreenId::Order, Some(&session.views)),
        "nor is the screen allowed"
    );
    assert_eq!(marching(&app), before);
}
