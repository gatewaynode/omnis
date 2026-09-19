//! Remote sensing headless: LOOK and L are dim or say so with no spyglass in the party, and
//! look through the first one carried once there is one, leaving a line in the log.

mod common;

use bevy::prelude::*;
use common::{click, play_state, seen, ui_app_saving_to, widget, world};
use omnis_app::sim::{Notice, PlayState, PlayerCommand};
use omnis_app::ui::RollLog;
use omnis_app::widget::{Part, ToolButton, WidgetId};
use omnis_sim::omnis_data::{Alignment, Skill};
use omnis_sim::omnis_rules::Draft;
use omnis_sim::world::layer;
use omnis_sim::{Command, DevCommand, Event, PartyCommand};

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    app.update();
    app.update();
}

fn press(app: &mut App, code: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(code);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    app.update();
}

fn sensed_since(app: &App, from: usize) -> usize {
    seen(app).events[from..]
        .iter()
        .filter(|e| matches!(e, Event::Sensed { .. }))
        .count()
}

#[test]
fn look_and_l_use_the_first_spyglass_carried_or_say_there_is_none() {
    let mut app = ui_app_saving_to("look.ron", true);
    app.update();
    app.update();
    send(
        &mut app,
        Command::Party(PartyCommand::Create(Draft {
            name: "Brenna".to_owned(),
            race: "base:race:human".to_owned(),
            class: "base:class:fighter".to_owned(),
            background: "base:background:acolyte".to_owned(),
            alignment: Alignment::LawfulGood,
            scores: [15, 14, 13, 12, 10, 8],
            skills: vec![Skill::Athletics, Skill::Perception],
        })),
    );
    assert_eq!(play_state(&app), PlayState::Explore);
    assert!(
        !widget(&app, WidgetId::Tool(ToolButton::Look)).enabled,
        "no spyglass yet"
    );
    let mark = seen(&app).events.len();
    press(&mut app, KeyCode::KeyL);
    assert_eq!(sensed_since(&app, mark), 0);
    assert_eq!(
        app.world().resource::<Notice>().0,
        "Nothing to look through"
    );
    // The autostarted game is a dev world: the debug command hands over a spyglass.
    send(
        &mut app,
        Command::Dev(DevCommand::GiveItem {
            member: Some(0),
            item: "base:item:spyglass".to_owned(),
            count: 1,
        }),
    );
    assert!(
        widget(&app, WidgetId::Tool(ToolButton::Look)).enabled,
        "LOOK lights up"
    );
    let mark = seen(&app).events.len();
    click(&mut app, WidgetId::Tool(ToolButton::Look), Part::Body);
    assert_eq!(
        sensed_since(&app, mark),
        1,
        "{:?}",
        &seen(&app).events[mark..]
    );
    assert_eq!(
        play_state(&app),
        PlayState::Explore,
        "a look is not a screen"
    );
    let mark = seen(&app).events.len();
    press(&mut app, KeyCode::KeyL);
    assert_eq!(sensed_since(&app, mark), 1);
    let log = app.world().resource::<RollLog>();
    assert!(
        log.0.iter().any(|l| l.contains("looks through Spyglass")),
        "{:?}",
        log.0
    );
    assert!(
        !log.0.iter().any(|l| l.contains("uses Spyglass")),
        "the look is the line, not the use: {:?}",
        log.0
    );
    // The automap carries the remote mark on a tile the party has not seen itself.
    let w = world(&app);
    let known = w.automap.map(w.position.map).unwrap();
    let remote = known
        .values()
        .filter(|t| t.layers & layer::REMOTE != 0)
        .count();
    let seen_now = seen(&app)
        .events
        .iter()
        .rev()
        .find_map(|e| match e {
            Event::Sensed { tiles, .. } => Some(tiles.len()),
            _ => None,
        })
        .unwrap();
    assert!(
        remote > 0 || seen_now == 0,
        "remote tiles {remote}, sensed {seen_now}"
    );
}
