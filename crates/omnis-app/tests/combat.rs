//! Headless fight tests (ARCHITECTURE.md §11, tier 6): a fixed encounter of the test dungeon
//! fought by mouse to its end, running away, the defeat modal, and pausing a fight.

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use common::{
    click, draft_fighter_by_mouse, key, place, play_state, seen, start_new_game_by_mouse, ui_app,
    ui_app_saving_to, widget, world,
};
use omnis_app::dev::recruit;
use omnis_app::menu::ROW_BEGIN;
use omnis_app::sim::{
    AppState, PackData, PlayState, PlayerCommand, ShellCommand, SimEvent, SimWorld,
};
use omnis_app::ui::{MessageLine, RollLog};
use omnis_app::widget::{Part, WidgetId};
use omnis_sim::omnis_core::{Direction, Facing};
use omnis_sim::{CheckKind, CombatOutcome, Command, Event, PartyCommand};

/// The rat placement of the test dungeon: (3, 8), entered from the north.
const DUNGEON: &str = "test:map:dungeon";

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    app.update();
    app.update();
}

/// An autostarted game with four stock recruits, standing one tile north of the rats.
fn before_the_rats(save: &str) -> App {
    let mut app = ui_app_saving_to(save, true);
    app.update();
    app.update();
    for _ in 0..4 {
        let draft = {
            let data = &app.world().resource::<PackData>().0;
            recruit(data, world(&app).party.members.len())
        };
        send(&mut app, Command::Party(PartyCommand::Create(draft)));
    }
    assert_eq!(world(&app).party.members.len(), 4);
    place(&mut app, DUNGEON, 3, 7, Facing::South);
    app
}

/// Step onto the rats: the encounter screen (surprise is off in the base rules, so the
/// choice always comes first).
fn meet_the_rats(app: &mut App) {
    send(app, Command::Step(Direction::Forward));
    assert!(
        matches!(play_state(app), PlayState::Encounter | PlayState::Combat),
        "{:?}",
        play_state(app)
    );
    assert!(
        seen(app)
            .events
            .iter()
            .any(|e| matches!(e, Event::EncounterStarted { .. }))
    );
}

#[test]
fn a_fixed_encounter_is_fought_by_mouse_to_its_end() {
    let mut app = before_the_rats("combat-fight.ron");
    meet_the_rats(&mut app);
    if play_state(&app) == PlayState::Encounter {
        assert!(widget(&app, WidgetId::Action(0)).enabled, "Attack");
        click(&mut app, WidgetId::Action(0), Part::Body);
    }
    assert_eq!(play_state(&app), PlayState::Combat);
    assert!(
        frame_has(&app, WidgetId::Stack(0)) && frame_has(&app, WidgetId::Action(3)),
        "the fight screen is up"
    );
    let line = app.world().resource::<MessageLine>();
    assert!(
        !line.0.text.ends_with(" to act"),
        "the turn is shown, not said: {}",
        line.0.text
    );
    for _ in 0..400 {
        if play_state(&app) != PlayState::Combat {
            break;
        }
        // Attack whatever the menu targets; the model keeps the target on a living stack.
        click(&mut app, WidgetId::Action(0), Part::Body);
    }
    let ended = seen(&app).events.iter().find_map(|e| match e {
        Event::CombatEnded { outcome, xp, .. } => Some((*outcome, *xp)),
        _ => None,
    });
    let (outcome, xp) = ended.expect("the fight ended");
    assert_eq!(
        outcome,
        CombatOutcome::Victory,
        "four fighters clear the room"
    );
    assert!(xp > 0);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert!(
        world(&app).party.members.iter().any(|m| m.xp == xp),
        "the survivors were paid"
    );
    let log = app.world().resource::<RollLog>();
    assert!(log.0.len() >= 4, "{:?}", log.0);
    assert!(log.0.iter().all(|l| !l.ends_with(" to act")), "{:?}", log.0);
    assert!(
        log.0
            .iter()
            .any(|l| l.contains(" hits ") || l.contains(" misses "))
    );
    let line = app.world().resource::<MessageLine>();
    assert!(line.0.text.starts_with("Victory!"), "{}", line.0.text);
    assert!(
        !frame_has(&app, WidgetId::Action(0)),
        "the action row is gone"
    );
}

fn frame_has(app: &App, id: WidgetId) -> bool {
    common::frame(app).frame.widget(id).is_some()
}

#[test]
fn running_resolves_as_its_check_says() {
    let mut app = before_the_rats("combat-run.ron");
    meet_the_rats(&mut app);
    let (run_action, kind) = if play_state(&app) == PlayState::Encounter {
        (3, CheckKind::Run)
    } else {
        (3, CheckKind::Flee)
    };
    click(&mut app, WidgetId::Action(run_action), Part::Body);
    let success = seen(&app)
        .events
        .iter()
        .find_map(|e| match e {
            Event::Check {
                kind: k, success, ..
            } if *k == kind => Some(*success),
            _ => None,
        })
        .expect("a run check was rolled");
    if success {
        assert_eq!(play_state(&app), PlayState::Explore);
        let p = world(&app).position;
        assert_eq!(
            (p.x, p.y, p.facing),
            (3, 7, Facing::North),
            "back where it came from"
        );
    } else {
        assert_eq!(play_state(&app), PlayState::Combat);
    }
}

#[test]
fn the_defeat_modal_loads_the_last_save_or_quits() {
    let mut app = before_the_rats("combat-defeat.ron");
    app.world_mut()
        .resource_mut::<Messages<ShellCommand>>()
        .write(ShellCommand::Save);
    app.update();
    app.update();
    let wipe = SimEvent(Event::CombatEnded {
        outcome: CombatOutcome::Defeat,
        xp: 0,
        gold: 0,
        fallen: vec![],
    });
    app.world_mut()
        .resource_mut::<Messages<SimEvent>>()
        .write(wipe.clone());
    app.update();
    app.update();
    assert_eq!(play_state(&app), PlayState::Defeat);
    assert!(frame_has(&app, WidgetId::Row(0)) && frame_has(&app, WidgetId::Row(1)));
    assert!(
        !widget(&app, WidgetId::Pad(omnis_app::widget::PadButton::Use)).enabled,
        "the pad is inert"
    );
    let line = app.world().resource::<MessageLine>();
    assert_eq!(line.0.text, "The party has fallen");
    let before = seen(&app).replaced;
    click(&mut app, WidgetId::Row(0), Part::Body);
    assert_eq!(seen(&app).replaced, before + 1, "the save was loaded");
    assert_eq!(play_state(&app), PlayState::Explore);
    assert_eq!(world(&app).party.members.len(), 4);

    app.world_mut()
        .resource_mut::<Messages<SimEvent>>()
        .write(wipe);
    app.update();
    app.update();
    assert_eq!(play_state(&app), PlayState::Defeat);
    click(&mut app, WidgetId::Row(1), Part::Body);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
    assert!(app.world().get_resource::<SimWorld>().is_none());
}

#[test]
fn escape_pauses_a_fight_and_resume_returns_to_it() {
    let mut app = before_the_rats("combat-pause.ron");
    meet_the_rats(&mut app);
    let before = play_state(&app);
    key(&mut app, Key::Escape);
    assert_eq!(play_state(&app), PlayState::Paused);
    click(&mut app, WidgetId::Row(0), Part::Body);
    assert_eq!(play_state(&app), before, "Resume returns to the fight");
    assert!(frame_has(&app, WidgetId::Action(0)));
    // The fight itself is untouched by the detour.
    assert!(
        !seen(&app)
            .events
            .iter()
            .any(|e| matches!(e, Event::CombatEnded { .. }))
    );
}

#[test]
fn a_party_built_by_mouse_reaches_the_encounter_screen() {
    let mut app = ui_app(false);
    start_new_game_by_mouse(&mut app);
    draft_fighter_by_mouse(&mut app);
    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    place(&mut app, DUNGEON, 15, 7, Facing::South);
    // The friendly rat at (15, 8): a bribe is free and every choice is on the row.
    send(&mut app, Command::Step(Direction::Forward));
    assert_eq!(play_state(&app), PlayState::Encounter);
    let bribe = widget(&app, WidgetId::Action(1));
    assert!(bribe.enabled, "a friendly bribe is free");
    click(&mut app, WidgetId::Action(1), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert!(
        seen(&app)
            .events
            .iter()
            .any(|e| matches!(e, Event::Bribed { cost: 0 }))
    );
}
