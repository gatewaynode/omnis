//! The camp panel (M7 step 8b), driven as a person does: CAMP on the tool bar and R open it on
//! the map and nowhere else; a slider chooses the hit dice and the short rest spends them; the
//! long rest eats and then is dim with its reason; with too little food the reason is the
//! food; an ambush takes the game into the fight; Close and Escape go back to the map; the
//! panel lies inside the map at both window sizes; the sheet shows the hit dice.

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use common::feathers::{
    activate, change, click_node, control, layout_faults, press, settle, shown, ultrawide,
};
use common::{feathers_app, fighter_draft, place, play_state, world};
use omnis_app::camp_panel::{CampLabelId, CampPanelId};
use omnis_app::sim::{PlayState, PlayerCommand, SimWorld};
use omnis_app::tool_bar::ToolButton;
use omnis_app::ui::RollLog;
use omnis_app::ui_text::screen_text;
use omnis_sim::omnis_core::Facing;
use omnis_sim::party::PartyCommand;
use omnis_sim::{Command, ModeKind};

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    settle(app);
}

/// A new game on the meadow with Brenna and Corin, both hurt, food in the stores.
fn meadow(save: &str) -> App {
    let mut app = feathers_app(save, true);
    settle(&mut app);
    for name in ["Brenna", "Corin"] {
        let mut draft = fighter_draft();
        draft.name = name.into();
        send(&mut app, Command::Party(PartyCommand::Create(draft)));
    }
    place(&mut app, "test:map:meadow", 16, 16, Facing::North);
    {
        let world = app
            .world_mut()
            .resource_mut::<SimWorld>()
            .into_inner()
            .fixture_mut();
        world.party.food = 10;
        for member in &mut world.party.members {
            member.hp = 1;
        }
    }
    settle(&mut app);
    assert_eq!(play_state(&app), PlayState::Explore);
    app
}

fn open_by_bar(app: &mut App) {
    let camp = control(app, ToolButton::Camp);
    click_node(app, camp);
    assert_eq!(play_state(app), PlayState::Camp);
}

fn dim(app: &mut App, id: CampPanelId) -> bool {
    let entity = control(app, id);
    app.world().get::<InteractionDisabled>(entity).is_some()
}

fn logged(app: &App, text: &str) -> bool {
    app.world()
        .resource::<RollLog>()
        .0
        .iter()
        .any(|line| line.contains(text))
}

#[test]
fn the_bar_and_r_open_the_camp_on_the_map_only() {
    let mut app = meadow("camp-open.ron");
    open_by_bar(&mut app);
    press(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(play_state(&app), PlayState::Explore, "Escape closes it");
    press(&mut app, KeyCode::KeyR, Key::Character("r".into()));
    assert_eq!(play_state(&app), PlayState::Camp, "R opens it");
    activate(&mut app, CampPanelId::Close);
    assert_eq!(play_state(&app), PlayState::Explore, "Close closes it");

    // Inside a service the room is the rest: CAMP is dim and R does nothing.
    place(&mut app, "test:map:town", 1, 1, Facing::North);
    send(&mut app, Command::Interact);
    assert_eq!(play_state(&app), PlayState::Service);
    let camp = control(&mut app, ToolButton::Camp);
    assert!(app.world().get::<InteractionDisabled>(camp).is_some());
    click_node(&mut app, camp);
    press(&mut app, KeyCode::KeyR, Key::Character("r".into()));
    assert_eq!(play_state(&app), PlayState::Service);
}

#[test]
fn a_slider_chooses_the_dice_and_the_short_rest_spends_them() {
    let mut app = meadow("camp-short.ron");
    open_by_bar(&mut app);
    assert!(shown(&mut app, CampLabelId::Member(0)).starts_with("Brenna  HP 1/"));
    assert!(dim(&mut app, CampPanelId::Short), "no dice chosen yet");
    change(&mut app, CampPanelId::Dice(0), 1.0_f32);
    assert_eq!(shown(&mut app, CampLabelId::Dice(0)), "spend 1");
    assert!(!dim(&mut app, CampPanelId::Short));
    let hp = world(&app).party.members[0].hp;
    activate(&mut app, CampPanelId::Short);
    let brenna = &world(&app).party.members[0];
    assert_eq!(brenna.hit_dice_spent, 1);
    assert!(brenna.hp > hp, "healed by the die");
    assert!(
        logged(&app, "spends 1 hit die"),
        "{:?}",
        app.world().resource::<RollLog>().0
    );
    assert_eq!(play_state(&app), PlayState::Camp, "the camp stays open");
    assert_eq!(
        shown(&mut app, CampLabelId::Dice(0)),
        "no hit dice left",
        "a level-1 fighter has one"
    );
    assert!(dim(&mut app, CampPanelId::Short), "the choice is cleared");
}

#[test]
fn the_long_rest_eats_and_then_waits_a_day() {
    let mut app = meadow("camp-long.ron");
    open_by_bar(&mut app);
    assert_eq!(
        shown(&mut app, CampLabelId::Food),
        "The night eats 2 food; the stores hold 10"
    );
    assert!(!dim(&mut app, CampPanelId::Long));
    activate(&mut app, CampPanelId::Long);
    let party = &world(&app).party;
    assert_eq!(party.food, 8);
    assert!(party.members.iter().all(|m| m.hp == m.hp_max));
    assert!(logged(&app, "rests for the night"));
    assert!(dim(&mut app, CampPanelId::Long));
    assert!(
        shown(&mut app, CampLabelId::Long).starts_with("rested too recently"),
        "{}",
        shown(&mut app, CampLabelId::Long)
    );

    app.world_mut()
        .resource_mut::<SimWorld>()
        .fixture_mut()
        .party
        .last_long_rest = None;
    app.world_mut()
        .resource_mut::<SimWorld>()
        .fixture_mut()
        .party
        .food = 1;
    settle(&mut app);
    assert!(dim(&mut app, CampPanelId::Long));
    assert_eq!(
        shown(&mut app, CampLabelId::Long),
        "the rest eats 2 food; the stores hold 1"
    );
}

#[test]
fn an_ambush_leaves_the_camp_for_the_fight() {
    let mut app = meadow("camp-ambush.ron");
    place(&mut app, "test:map:dungeon", 1, 0, Facing::North);
    open_by_bar(&mut app);
    for _ in 0..200 {
        {
            let party = &mut app
                .world_mut()
                .resource_mut::<SimWorld>()
                .into_inner()
                .fixture_mut()
                .party;
            party.last_long_rest = None;
            party.food = 10;
        }
        settle(&mut app);
        activate(&mut app, CampPanelId::Long);
        if world(&app).mode.kind() != ModeKind::Explore {
            break;
        }
    }
    let kind = world(&app).mode.kind();
    assert!(
        matches!(kind, ModeKind::Encounter | ModeKind::Combat),
        "ambushed within 200 nights: {kind:?}"
    );
    assert!(logged(&app, "Ambushed after"));
    assert!(
        matches!(play_state(&app), PlayState::Encounter | PlayState::Combat),
        "{:?}",
        play_state(&app)
    );
    assert_eq!(screen_text(app.world()), omnis_app::ui_text::NO_PANEL);
}

#[test]
fn the_camp_lies_inside_the_map_at_both_window_sizes() {
    let mut app = meadow("camp-layout.ron");
    for size in ["1280 by 720", "5120 by 1440"] {
        if size.starts_with("5120") {
            ultrawide(&mut app);
        }
        open_by_bar(&mut app);
        assert_eq!(layout_faults(&mut app), Vec::new(), "{size}");
        if let Ok(dir) = std::env::var("OMNIS_DUMP_SCREENS")
            && size.starts_with("1280")
        {
            let path = std::path::Path::new(&dir).join("camp.txt");
            std::fs::write(&path, screen_text(app.world()))
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
        activate(&mut app, CampPanelId::Close);
    }
}
