//! The debug menu headless (feature `devtools`): the backtick opens it while exploring and
//! in a fight, its rows push the world's numbers through Dev commands, an item lands in the
//! kit, a kill ends the fight, and Escape returns to where the world is.
#![cfg(feature = "devtools")]

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use common::{click, key, place, play_state, seen, ui_app_saving_to, widget, world};
use omnis_app::debug::DebugPlugin;
use omnis_app::debug_menu::{ROW_HP, ROW_ITEM, ROW_STACK};
use omnis_app::dev::recruit;
use omnis_app::menus::Screens;
use omnis_app::sim::{PackData, PlayState, PlayerCommand};
use omnis_app::widget::{Part, WidgetId};
use omnis_sim::items::count_of;
use omnis_sim::omnis_core::{Direction, Facing};
use omnis_sim::{CombatOutcome, Command, Event, PartyCommand};

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    app.update();
    app.update();
}

/// An autostarted dev game with two recruits, standing one tile north of the rats.
fn before_the_rats(save: &str) -> App {
    let mut app = ui_app_saving_to(save, true);
    app.add_plugins(DebugPlugin);
    app.update();
    app.update();
    for _ in 0..2 {
        let draft = {
            let data = &app.world().resource::<PackData>().0;
            recruit(data, world(&app).party.members.len())
        };
        send(&mut app, Command::Party(PartyCommand::Create(draft)));
    }
    assert!(
        world(&app).settings.devtools,
        "a dev build starts a dev world"
    );
    place(&mut app, "test:map:dungeon", 3, 7, Facing::South);
    app
}

fn backtick(app: &mut App) {
    key(app, Key::Character("`".into()));
}

#[test]
fn the_menu_opens_while_exploring_and_its_rows_edit_the_world() {
    let mut app = before_the_rats("debug-explore.ron");
    assert_eq!(play_state(&app), PlayState::Explore);
    backtick(&mut app);
    assert_eq!(play_state(&app), PlayState::Debug);
    assert!(widget(&app, WidgetId::Row(ROW_HP)).enabled);
    let max = world(&app).party.members[0].hp_max;
    key(&mut app, Key::ArrowDown);
    assert_eq!(app.world().resource::<Screens>().debug.row, ROW_HP);
    key(&mut app, Key::ArrowLeft);
    assert_eq!(
        world(&app).party.members[0].hp,
        max - 1,
        "Left takes a point"
    );
    assert!(
        seen(&app)
            .events
            .iter()
            .any(|e| matches!(e, Event::Dev { .. }))
    );
    key(&mut app, Key::ArrowRight);
    assert_eq!(world(&app).party.members[0].hp, max);
    let spyglass = {
        let data = &app.world().resource::<PackData>().0;
        data.registry.items.get("base:item:spyglass").unwrap()
    };
    let items = {
        let data = &app.world().resource::<PackData>().0;
        data.registry.items.names().count()
    };
    app.world_mut().resource_mut::<Screens>().debug.row = ROW_ITEM;
    let at = {
        let data = &app.world().resource::<PackData>().0;
        data.registry
            .items
            .names()
            .position(|n| n == "base:item:spyglass")
            .unwrap()
    };
    assert!(at < items);
    app.world_mut().resource_mut::<Screens>().debug.item = at;
    key(&mut app, Key::Enter);
    assert_eq!(
        count_of(&world(&app).party.members[0].equipment, spyglass),
        1
    );
    key(&mut app, Key::Character("s".into()));
    assert_eq!(
        count_of(&world(&app).party.inventory, spyglass),
        1,
        "to the stores"
    );
    click(&mut app, WidgetId::Row(ROW_HP), Part::Left);
    assert_eq!(
        world(&app).party.members[0].hp,
        max - 1,
        "a click on the arrow"
    );
    key(&mut app, Key::Escape);
    assert_eq!(play_state(&app), PlayState::Explore);
    send(&mut app, Command::Step(Direction::Forward));
    assert_eq!(play_state(&app), PlayState::Encounter);
    backtick(&mut app);
    assert_eq!(
        play_state(&app),
        PlayState::Encounter,
        "the encounter choice comes first"
    );
}

#[test]
fn in_a_fight_the_stack_row_kills_and_escape_returns_to_the_world() {
    let mut app = before_the_rats("debug-fight.ron");
    send(&mut app, Command::Step(Direction::Forward));
    if play_state(&app) == PlayState::Encounter {
        send(
            &mut app,
            Command::Encounter(omnis_sim::EncounterChoice::Attack),
        );
    }
    assert_eq!(play_state(&app), PlayState::Combat);
    backtick(&mut app);
    assert_eq!(play_state(&app), PlayState::Debug);
    assert!(
        widget(&app, WidgetId::Row(ROW_STACK)).enabled,
        "a fight is on"
    );
    app.world_mut().resource_mut::<Screens>().debug.row = ROW_STACK;
    key(&mut app, Key::Character("k".into()));
    let ended = seen(&app).events.iter().find_map(|e| match e {
        Event::CombatEnded { outcome, .. } => Some(*outcome),
        _ => None,
    });
    assert_eq!(ended, Some(CombatOutcome::Victory), "the only stack fell");
    assert_eq!(play_state(&app), PlayState::Debug, "the menu stays up");
    key(&mut app, Key::Character("`".into()));
    assert_eq!(
        play_state(&app),
        PlayState::Explore,
        "the backtick closes it where the world is"
    );
}
