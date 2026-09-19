//! The inventory overlay, headless: I and the ITEMS button open it, Enter takes armor off
//! and puts it back on, S stows, the stores tab and Enter take back, U drinks, G without a
//! second member says so, an action button acts, and Escape or I closes.

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use common::{click, key, play_state, seen, ui_app_saving_to, world};
use omnis_app::menus::Screens;
use omnis_app::sim::{PlayState, PlayerCommand};
use omnis_app::widget::{Part, ToolButton, WidgetId};
use omnis_sim::omnis_data::{Alignment, EquipSlot, Skill};
use omnis_sim::omnis_rules::Draft;
use omnis_sim::{Command, Event, ItemPlace, PartyCommand};

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
}

fn events_since(app: &App, from: usize) -> Vec<Event> {
    seen(app).events[from..].to_vec()
}

fn pane_and_cursor(app: &App) -> (usize, usize) {
    let menu = &app.world().resource::<Screens>().inventory;
    (menu.pane, menu.cursor)
}

fn brenna() -> Draft {
    Draft {
        name: "Brenna".to_owned(),
        race: "base:race:human".to_owned(),
        class: "base:class:fighter".to_owned(),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::LawfulGood,
        scores: [15, 14, 13, 12, 10, 8],
        skills: vec![Skill::Athletics, Skill::Perception],
    }
}

#[test]
fn keys_take_armor_off_stow_take_back_and_drink() {
    let mut app = ui_app_saving_to("inventory-keys.ron", true);
    app.update();
    app.update();
    send(&mut app, Command::Party(PartyCommand::Create(brenna())));
    assert_eq!(play_state(&app), PlayState::Explore);
    let brenna = world(&app).party.members[0].id;
    press(&mut app, KeyCode::KeyI);
    assert_eq!(
        play_state(&app),
        PlayState::Inventory,
        "I opens the overlay"
    );
    assert_eq!(pane_and_cursor(&app), (0, 0));
    // Row 0 is the chain mail, worn: Enter takes it off, Enter puts it back on.
    let mark = seen(&app).events.len();
    key(&mut app, Key::Enter);
    assert_eq!(
        play_state(&app),
        PlayState::Inventory,
        "the overlay stays open"
    );
    assert!(
        events_since(&app, mark).iter().any(|e| matches!(
            e,
            Event::Unequipped { member, slot: EquipSlot::Body, .. } if *member == brenna
        )),
        "{:?}",
        events_since(&app, mark)
    );
    assert!(
        !world(&app).party.members[0]
            .equipped
            .contains_key(&EquipSlot::Body)
    );
    let mark = seen(&app).events.len();
    key(&mut app, Key::Enter);
    assert!(events_since(&app, mark).iter().any(|e| matches!(
        e,
        Event::Equipped {
            slot: EquipSlot::Body,
            ..
        }
    )));
    // Down to the potion (row 6), S stows it, the stores tab shows it, Enter takes it back.
    for _ in 0..6 {
        key(&mut app, Key::ArrowDown);
    }
    assert_eq!(pane_and_cursor(&app), (0, 6));
    let mark = seen(&app).events.len();
    key(&mut app, Key::Character("s".into()));
    assert!(events_since(&app, mark).iter().any(|e| matches!(
        e,
        Event::ItemMoved {
            to: ItemPlace::Stores,
            count: 1,
            ..
        }
    )));
    assert_eq!(world(&app).party.inventory.len(), 1);
    key(&mut app, Key::ArrowRight);
    assert_eq!(pane_and_cursor(&app), (1, 0), "the stores pane");
    let mark = seen(&app).events.len();
    key(&mut app, Key::Enter);
    assert!(events_since(&app, mark).iter().any(|e| matches!(
        e,
        Event::ItemMoved { from: ItemPlace::Stores, to: ItemPlace::Member(m), .. } if *m == brenna
    )));
    assert!(world(&app).party.inventory.is_empty());
    // Back on Brenna's pane the potion is the last row again; U drinks it.
    key(&mut app, Key::ArrowLeft);
    key(&mut app, Key::ArrowUp);
    assert_eq!(pane_and_cursor(&app), (0, 6));
    let mark = seen(&app).events.len();
    key(&mut app, Key::Character("u".into()));
    assert!(events_since(&app, mark).iter().any(|e| matches!(
        e,
        Event::ItemUsed { member, consumed: true, .. } if *member == brenna
    )));
    assert_eq!(world(&app).party.members[0].equipment.len(), 6);
    // G with nobody else selected does nothing but say so.
    let mark = seen(&app).events.len();
    key(&mut app, Key::Character("g".into()));
    assert!(events_since(&app, mark).is_empty());
    assert!(
        app.world()
            .resource::<Screens>()
            .inventory
            .message
            .starts_with("Click another member")
    );
    key(&mut app, Key::Escape);
    assert_eq!(play_state(&app), PlayState::Explore, "Escape closes");
}

#[test]
fn the_items_button_opens_it_a_tab_and_a_row_click_pick_and_an_action_button_acts() {
    let mut app = ui_app_saving_to("inventory-mouse.ron", true);
    app.update();
    app.update();
    send(&mut app, Command::Party(PartyCommand::Create(brenna())));
    click(&mut app, WidgetId::Tool(ToolButton::Items), Part::Body);
    assert_eq!(play_state(&app), PlayState::Inventory, "ITEMS opens it");
    click(&mut app, WidgetId::Row(1), Part::Body);
    assert_eq!(pane_and_cursor(&app), (1, 0), "the stores tab");
    click(&mut app, WidgetId::Row(0), Part::Body);
    // Two tabs, so kit row 1 (the longsword) is widget row 3; EQUIP takes it off.
    click(&mut app, WidgetId::Row(2 + 1), Part::Body);
    assert_eq!(pane_and_cursor(&app), (0, 1));
    let mark = seen(&app).events.len();
    click(&mut app, WidgetId::Action(0), Part::Body);
    assert!(
        events_since(&app, mark).iter().any(|e| matches!(
            e,
            Event::Unequipped {
                slot: EquipSlot::MainHand,
                ..
            }
        )),
        "{:?}",
        events_since(&app, mark)
    );
    assert!(
        !world(&app).party.members[0]
            .equipped
            .contains_key(&EquipSlot::MainHand)
    );
    // TAKE is dim on a kit row: the click is inert.
    let mark = seen(&app).events.len();
    click(&mut app, WidgetId::Action(3), Part::Body);
    assert!(events_since(&app, mark).is_empty());
    key(&mut app, Key::Character("i".into()));
    assert_eq!(play_state(&app), PlayState::Explore, "I closes");
}
