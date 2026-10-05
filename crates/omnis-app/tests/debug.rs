//! The debug panel (feature `devtools`), driven as a person does: the backtick and the pause
//! overlay's item open it, Escape and Close go back to the world; a number typed and committed
//! reaches the world once, with no limit at the maximum; every field holds after the panel
//! closes (B3); experience levels on the spot; items and a condition are set; in a
//! fight the stack rows work and the teleport is grey; the panel lies inside the map at both
//! window sizes.
#![cfg(feature = "devtools")]

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use common::feathers::{
    activate, change, click_node, control, controls, keys, layout_faults, press, settle, ultrawide,
};
use common::{click, feathers_app, key, place, play_state, seen, world};
use omnis_app::debug_panel::DebugPanelId;
use omnis_app::dev::recruit;
use omnis_app::menu::Pause;
use omnis_app::sim::{PackData, PlayState, PlayerCommand};
use omnis_app::ui_kit::UiId;
use omnis_app::widget::{Part, WidgetId};
use omnis_sim::items::count_of;
use omnis_sim::omnis_core::{Direction, Facing};
use omnis_sim::omnis_data::Ability;
use omnis_sim::{CombatOutcome, Command, EncounterChoice, Event, PartyCommand};

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    settle(app);
}

/// An autostarted dev game with two recruits, standing one tile north of the rats.
fn before_the_rats(save: &str) -> App {
    let mut app = feathers_app(save, true);
    settle(&mut app);
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
    settle(&mut app);
    app
}

fn backtick(app: &mut App) {
    key(app, Key::Character("`".into()));
    settle(app);
}

fn dim(app: &mut App, id: DebugPanelId) -> bool {
    let entity = control(app, id);
    app.world().get::<InteractionDisabled>(entity).is_some()
}

fn dev_commands(app: &App) -> usize {
    seen(app)
        .events
        .iter()
        .filter(|e| matches!(e, Event::Dev { .. }))
        .count()
}

/// Type `text` into a number field and press Enter.
fn type_into(app: &mut App, id: DebugPanelId, text: &str) {
    let outer = control(app, id);
    let input = app
        .world()
        .get::<Children>(outer)
        .expect("children")
        .iter()
        .find(|child| app.world().get::<EditableText>(*child).is_some())
        .expect("a text input in the number input");
    click_node(app, input);
    keys(app, text);
    press(app, KeyCode::Enter, Key::Enter);
    settle(app);
}

#[test]
fn the_backtick_and_the_pause_item_open_the_panel_and_escape_and_close_return() {
    let mut app = before_the_rats("debug-open.ron");
    backtick(&mut app);
    assert_eq!(play_state(&app), PlayState::Debug);
    assert!(controls(&mut app).contains(&UiId::Debug(DebugPanelId::Hp)));
    key(&mut app, Key::Escape);
    settle(&mut app);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert!(
        !controls(&mut app).contains(&UiId::Debug(DebugPanelId::Hp)),
        "the panel goes with the screen"
    );

    press(&mut app, KeyCode::Escape, Key::Escape);
    settle(&mut app);
    assert_eq!(play_state(&app), PlayState::Paused);
    click(&mut app, WidgetId::Row(Pause::DEBUG), Part::Body);
    settle(&mut app);
    assert_eq!(play_state(&app), PlayState::Debug);
    activate(&mut app, DebugPanelId::Close);
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
fn typed_numbers_reach_the_world_once_and_every_field_holds_after_close() {
    let mut app = before_the_rats("debug-numbers.ron");
    backtick(&mut app);
    let before = dev_commands(&app);
    type_into(&mut app, DebugPanelId::Hp, "120");
    assert_eq!(world(&app).party.members[0].hp, 120, "past the maximum");
    assert_eq!(
        dev_commands(&app),
        before + 1,
        "one command, not one a digit"
    );

    change(
        &mut app,
        DebugPanelId::Score(Ability::Strength.index()),
        25_i32,
    );
    change(&mut app, DebugPanelId::Sp, 7_i32);
    change(&mut app, DebugPanelId::Xp, 300_i32);
    let gained = seen(&app)
        .events
        .iter()
        .find_map(|e| match e {
            Event::LevelUp {
                level: 2,
                cost: 0,
                gains,
                ..
            } => Some(gains.hp),
            _ => None,
        })
        .expect("experience levels on the spot");
    change(&mut app, DebugPanelId::Gold, 4321_i32);
    change(&mut app, DebugPanelId::Food, 77_i32);
    let (spyglass, at) = {
        let data = &app.world().resource::<PackData>().0;
        let at = data
            .registry
            .items
            .names()
            .position(|n| n == "base:item:spyglass")
            .unwrap();
        (data.registry.items.get("base:item:spyglass").unwrap(), at)
    };
    activate(&mut app, DebugPanelId::Item(at));
    change(&mut app, DebugPanelId::Count, 3_i32);
    activate(&mut app, DebugPanelId::GiveMember);
    activate(&mut app, DebugPanelId::GiveStores);
    activate(&mut app, DebugPanelId::ConditionPick(0));
    activate(&mut app, DebugPanelId::Toggle);
    // The packs declare no flags yet: Set has nothing to write.
    activate(&mut app, DebugPanelId::SetFlag);
    activate(&mut app, DebugPanelId::Close);
    assert_eq!(play_state(&app), PlayState::Explore);
    for _ in 0..5 {
        app.update();
    }

    let w = world(&app);
    let brenna = &w.party.members[0];
    assert_eq!(brenna.hp, 120 + gained, "the level's hit points on top");
    assert_eq!(brenna.scores[Ability::Strength.index()], 25);
    assert_eq!(brenna.spell_points, 7);
    assert_eq!((brenna.xp, brenna.level), (300, 2));
    assert_eq!((w.party.gold, w.party.food), (4321, 77));
    assert_eq!(count_of(&brenna.equipment, spyglass), 3);
    assert_eq!(count_of(&w.party.inventory, spyglass), 3);
    assert_eq!(brenna.conditions.len(), 1, "the first condition on");
    assert!(w.flags.is_empty());
}

#[test]
fn in_a_fight_the_stack_rows_work_and_the_teleport_is_grey() {
    let mut app = before_the_rats("debug-fight.ron");
    backtick(&mut app);
    assert!(dim(&mut app, DebugPanelId::Kill), "no fight yet");
    change(&mut app, DebugPanelId::X, 2_i32);
    activate(&mut app, DebugPanelId::Go);
    assert_eq!(world(&app).position.x, 2, "a teleport while exploring");
    activate(&mut app, DebugPanelId::Close);
    place(&mut app, "test:map:dungeon", 3, 7, Facing::South);
    send(&mut app, Command::Step(Direction::Forward));
    if play_state(&app) == PlayState::Encounter {
        send(&mut app, Command::Encounter(EncounterChoice::Attack));
    }
    assert_eq!(play_state(&app), PlayState::Combat);
    backtick(&mut app);
    assert_eq!(play_state(&app), PlayState::Debug);
    assert!(dim(&mut app, DebugPanelId::Go), "no teleport in a fight");
    assert!(!dim(&mut app, DebugPanelId::Kill));
    change(&mut app, DebugPanelId::StackHp, 1_i32);
    let lead = match &world(&app).mode {
        omnis_sim::Mode::Combat(state) => state.encounter.stacks[0].hp[0],
        other => panic!("{other:?}"),
    };
    assert_eq!(lead, 1);
    for _ in 0..4 {
        if !matches!(world(&app).mode, omnis_sim::Mode::Combat(_)) {
            break;
        }
        activate(&mut app, DebugPanelId::Kill);
    }
    let ended = seen(&app).events.iter().find_map(|e| match e {
        Event::CombatEnded { outcome, .. } => Some(*outcome),
        _ => None,
    });
    assert_eq!(ended, Some(CombatOutcome::Victory), "every stack fell");
    assert_eq!(play_state(&app), PlayState::Debug, "the panel stays up");
    backtick(&mut app);
    assert_eq!(
        play_state(&app),
        PlayState::Explore,
        "the backtick closes it where the world is"
    );
}

#[test]
fn the_panel_lies_inside_the_map_at_both_window_sizes() {
    let mut app = before_the_rats("debug-layout.ron");
    backtick(&mut app);
    for size in ["1280 by 720", "5120 by 1440"] {
        if size.starts_with("5120") {
            ultrawide(&mut app);
            settle(&mut app);
        }
        assert_eq!(layout_faults(&mut app), Vec::new(), "{size}");
        if let Ok(dir) = std::env::var("OMNIS_DUMP_SCREENS")
            && size.starts_with("1280")
        {
            let path = std::path::Path::new(&dir).join("debug.txt");
            std::fs::write(&path, omnis_app::ui_text::screen_text(app.world()))
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
    }
}
