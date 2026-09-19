//! The cast menu while exploring, headless: C opens it, Enter casts a spell for the road,
//! and Escape returns to the map.

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use common::{escape, key, play_state, seen, ui_app_saving_to, world};
use omnis_app::sim::{PackData, PlayState, PlayerCommand, ShellCommand};
use omnis_app::ui::MessageLine;
use omnis_sim::omnis_data::Alignment;
use omnis_sim::omnis_rules::Draft;
use omnis_sim::{Command, Event, PartyCommand};

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    app.update();
    app.update();
}

#[test]
fn c_opens_the_cast_menu_and_a_wizard_lights_the_way() {
    let mut app = ui_app_saving_to("cast-menu.ron", true);
    app.update();
    app.update();
    let draft = Draft {
        name: "Ilvara".to_owned(),
        race: "base:race:elf".to_owned(),
        class: "base:class:wizard".to_owned(),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::ChaoticGood,
        scores: [8, 14, 13, 15, 12, 10],
        skills: vec![
            omnis_sim::omnis_data::Skill::Arcana,
            omnis_sim::omnis_data::Skill::History,
        ],
    };
    send(&mut app, Command::Party(PartyCommand::Create(draft)));
    assert_eq!(play_state(&app), PlayState::Explore);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyC);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    assert_eq!(play_state(&app), PlayState::Cast, "C opens the cast menu");
    escape(&mut app);
    key(&mut app, Key::Escape);
    assert_eq!(
        play_state(&app),
        PlayState::Explore,
        "Escape returns to the map"
    );
    app.world_mut()
        .resource_mut::<Messages<ShellCommand>>()
        .write(ShellCommand::Cast);
    app.update();
    app.update();
    assert_eq!(play_state(&app), PlayState::Cast);
    // The wizard's road spells are light and mage hand; the first row is light.
    key(&mut app, Key::Enter);
    assert_eq!(
        play_state(&app),
        PlayState::Explore,
        "a cast closes the menu"
    );
    assert!(
        seen(&app)
            .events
            .iter()
            .any(|e| matches!(e, Event::SpellCast { points: 0, .. })),
        "light was cast for free: {:?}",
        seen(&app).events.len()
    );
    assert_eq!(
        world(&app).party.effects.len(),
        1,
        "the party carries the light"
    );
    let line = app.world().resource::<MessageLine>();
    assert!(line.0.text.contains("Light"), "{}", line.0.text);
    let _ = app.world().resource::<PackData>();
}
