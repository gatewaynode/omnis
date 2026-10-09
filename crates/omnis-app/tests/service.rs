//! The service panel (M7 step 7), driven as a person does: every service opens its panel,
//! every offer's button sends the command the view promised (by event and by pointer through
//! real layout and picking), a refused offer's button is dim and sends nothing, the bank's
//! refusals show on the message line, leaving asks first, the tool pad's overlays come back to
//! the panel, and the panel lies inside the map at both window sizes.

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use common::feathers::{
    activate, change, click_node, control, keys, layout_faults, press, settle, shown, ultrawide,
};
use common::{feathers_app, fighter_draft, place, play_state, world};
use omnis_app::confirm_panel::ConfirmId;
use omnis_app::service_panel::{ServiceLabelId, ServicePanelId, money_line};
use omnis_app::sim::{PackData, PlayState, PlayerCommand, SimWorld};
use omnis_app::tool_bar::ToolButton;
use omnis_app::ui::RollLog;
use omnis_app::ui_kit::{Control, UiId};
use omnis_app::ui_text::screen_text;
use omnis_sim::omnis_core::Facing;
use omnis_sim::party::PartyCommand;
use omnis_sim::{Command, ModeKind, Rejection, ServiceCommand, ServiceView, service_view};

const SERVICES: [&str; 7] = [
    "inn", "temple", "trainer", "guild", "smith", "tavern", "bank",
];

fn site(name: &str) -> (u16, u16) {
    match name {
        "inn" => (1, 1),
        "temple" => (4, 1),
        "trainer" => (7, 1),
        "guild" => (10, 1),
        "smith" => (1, 3),
        "tavern" => (4, 3),
        "bank" => (7, 3),
        _ => panic!("no {name}"),
    }
}

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    settle(app);
}

/// A new game in town with Brenna and Corin and plenty of gold.
fn town(save: &str) -> App {
    let mut app = feathers_app(save, true);
    settle(&mut app);
    for name in ["Brenna", "Corin"] {
        let mut draft = fighter_draft();
        draft.name = name.into();
        send(&mut app, Command::Party(PartyCommand::Create(draft)));
    }
    app.world_mut()
        .resource_mut::<SimWorld>()
        .fixture_mut()
        .party
        .gold = 1_000_000;
    settle(&mut app);
    assert_eq!(play_state(&app), PlayState::Explore);
    app
}

/// Into a service by its door, as `Use` on the site does.
fn enter(app: &mut App, name: &str) {
    let (x, y) = site(name);
    place(app, "test:map:town", x, y, Facing::North);
    send(app, Command::Interact);
    assert_eq!(world(app).mode.kind(), ModeKind::Town, "inside the {name}");
    assert_eq!(play_state(app), PlayState::Service, "the {name}'s panel");
}

fn leave(app: &mut App) {
    send(app, Command::Service(ServiceCommand::Leave));
    assert_eq!(play_state(app), PlayState::Explore);
}

fn view(app: &App) -> ServiceView {
    let data = &app.world().resource::<PackData>().0;
    service_view(world(app), data).expect("inside a service")
}

fn offer(app: &App, wanted: impl Fn(&ServiceCommand) -> bool) -> usize {
    view(app)
        .offers
        .iter()
        .position(|o| wanted(&o.command))
        .expect("on offer")
}

fn offer_buttons(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&Control>();
    query
        .iter(app.world())
        .filter(|c| matches!(c.0, UiId::Service(ServicePanelId::Offer(_))))
        .count()
}

fn log(app: &App) -> Vec<String> {
    app.world().resource::<RollLog>().0.clone()
}

fn logged(app: &App, start: &str) -> bool {
    log(app).iter().any(|l| l.starts_with(start))
}

fn gold(app: &App) -> u32 {
    world(app).party.gold
}

#[test]
fn every_service_opens_its_panel() {
    let mut app = town("service-census.ron");
    for name in SERVICES {
        enter(&mut app, name);
        let view = view(&app);
        let label = app
            .world()
            .resource::<PackData>()
            .0
            .label("en", &view.name)
            .to_owned();
        let text = screen_text(app.world());
        assert!(text.starts_with("Service panel\n"), "{name}: {text}");
        assert!(text.contains(&format!("\"{label}\"")), "{name}: {text}");
        assert_eq!(
            shown(&mut app, ServiceLabelId::Money),
            money_line(&view),
            "{name}"
        );
        assert_eq!(
            offer_buttons(&mut app),
            view.offers.len() - 1,
            "{name}: a button per offer, and Leave"
        );
        control(&mut app, ServicePanelId::Leave);
        let lists: &[&str] = match name {
            "trainer" => &["Levels", "Spell picks"],
            "guild" => &["Spells"],
            "temple" => &["On offer", "Spells"],
            _ => &[],
        };
        for title in lists {
            assert!(text.contains(&format!("\"{title}\"")), "{name}: {text}");
        }
        if name == "bank" {
            control(&mut app, ServicePanelId::Amount);
        }
        leave(&mut app);
        assert_eq!(screen_text(app.world()), omnis_app::ui_text::NO_PANEL);
    }
}

#[test]
fn every_offer_sends_what_the_view_promised() {
    let mut app = town("service-events.ron");
    enter(&mut app, "inn");
    let room = offer(&app, |c| *c == ServiceCommand::Room);
    let price = view(&app).offers[room].price.unwrap();
    let before = gold(&app);
    activate(&mut app, ServicePanelId::Offer(room));
    assert_eq!(before - gold(&app), price);
    assert!(logged(&app, "A night's rest for"));
    assert!(
        world(&app).party.last_long_rest.is_some(),
        "the room is a rest"
    );
    leave(&mut app);

    enter(&mut app, "tavern");
    let rumor = offer(&app, |c| *c == ServiceCommand::Rumor);
    activate(&mut app, ServicePanelId::Offer(rumor));
    assert!(
        logged(&app, "\"Talk from today: the rats below"),
        "the rumor is in the log, its age filled on the town's clock: {:?}",
        log(&app)
    );
    let food = world(&app).party.food;
    let buy_food = offer(&app, |c| matches!(c, ServiceCommand::BuyFood { .. }));
    activate(&mut app, ServicePanelId::Offer(buy_food));
    assert_eq!(world(&app).party.food, food + 1);
    assert!(logged(&app, "Bought 1 food for"));
    leave(&mut app);

    enter(&mut app, "smith");
    let buy = offer(
        &app,
        |c| matches!(c, ServiceCommand::Buy { item, .. } if item == "base:item:dagger"),
    );
    let price = view(&app).offers[buy].price.unwrap();
    let before = gold(&app);
    activate(&mut app, ServicePanelId::Offer(buy));
    assert_eq!(before - gold(&app), price);
    let sell = offer(&app, |c| matches!(c, ServiceCommand::Sell { .. }));
    let pays = view(&app).offers[sell].pays.unwrap();
    let before = gold(&app);
    activate(&mut app, ServicePanelId::Offer(sell));
    assert_eq!(gold(&app) - before, pays);
    assert!(logged(&app, "Bought 1 ") && logged(&app, "Sold 1 "));
    leave(&mut app);

    app.world_mut()
        .resource_mut::<SimWorld>()
        .fixture_mut()
        .party
        .members[1]
        .hp -= 3;
    enter(&mut app, "temple");
    let corin_id = world(&app).party.members[1].id;
    let heal = offer(&app, |c| *c == ServiceCommand::Heal { member: corin_id });
    activate(&mut app, ServicePanelId::Offer(heal));
    let corin = &world(&app).party.members[1];
    assert_eq!(corin.hp, corin.hp_max);
    assert!(logged(&app, "Corin is treated for"));
    let row = |app: &App, wanted: ServiceCommand| ServiceLabelId::Row(offer(app, |c| *c == wanted));
    for (command, label) in [
        (
            ServiceCommand::Heal { member: corin_id },
            "Corin, 12 of 12 HP",
        ),
        (
            ServiceCommand::Cure { member: corin_id },
            "Corin, no conditions",
        ),
        (ServiceCommand::Raise { member: corin_id }, "Corin, alive"),
    ] {
        let id = row(&app, command);
        assert_eq!(shown(&mut app, id), label);
    }
    leave(&mut app);

    enter(&mut app, "bank");
    let before = gold(&app);
    change(&mut app, ServicePanelId::Amount, 12_i32);
    activate(&mut app, ServicePanelId::Deposit);
    assert_eq!((before - gold(&app), world(&app).party.bank), (1200, 1200));
    change(&mut app, ServicePanelId::Amount, 5_i32);
    activate(&mut app, ServicePanelId::Withdraw);
    assert_eq!(world(&app).party.bank, 700);
    assert!(logged(&app, "Deposited 12 gp") && logged(&app, "Withdrew 5 gp"));
    change(&mut app, ServicePanelId::Amount, -40_i32);
    activate(&mut app, ServicePanelId::Withdraw);
    assert_eq!(
        world(&app).party.bank,
        700,
        "a negative amount sends nothing"
    );
}

/// A new game in town with Brenna (fighter) and Durin (cleric), plenty of gold, and Durin's
/// experience at level 2's threshold.
fn town_with_a_cleric(save: &str) -> App {
    let mut app = feathers_app(save, true);
    settle(&mut app);
    let mut cleric = fighter_draft();
    cleric.name = "Durin".into();
    cleric.class = "base:class:cleric".into();
    cleric.skills = vec![
        omnis_sim::omnis_data::Skill::Medicine,
        omnis_sim::omnis_data::Skill::History,
    ];
    for draft in [fighter_draft(), cleric] {
        send(&mut app, Command::Party(PartyCommand::Create(draft)));
    }
    let world = app
        .world_mut()
        .resource_mut::<SimWorld>()
        .into_inner()
        .fixture_mut();
    world.party.gold = 1_000_000;
    world.party.members[1].xp = 300;
    settle(&mut app);
    app
}

fn dim(app: &mut App, id: ServicePanelId) -> bool {
    let entity = control(app, id);
    app.world().get::<InteractionDisabled>(entity).is_some()
}

#[test]
fn the_trainer_grants_a_level_and_its_pick_and_the_temple_sells_a_spell() {
    let mut app = town_with_a_cleric("service-trainer.ron");
    enter(&mut app, "trainer");
    let (brenna_id, durin_id) = (
        world(&app).party.members[0].id,
        world(&app).party.members[1].id,
    );
    let brenna = offer(&app, |c| *c == ServiceCommand::Train { member: brenna_id });
    assert!(
        dim(&mut app, ServicePanelId::Offer(brenna)),
        "Brenna has no experience"
    );
    let durin = offer(&app, |c| *c == ServiceCommand::Train { member: durin_id });
    assert_eq!(
        shown(&mut app, ServiceLabelId::Row(durin)),
        "Durin, level 1 to 2"
    );
    let before = gold(&app);
    let button = control(&mut app, ServicePanelId::Offer(durin));
    click_node(&mut app, button);
    assert_eq!(before - gold(&app), 2000);
    assert_eq!(world(&app).party.members[1].level, 2);
    assert!(logged(&app, "Durin reaches level 2: +"), "{:?}", log(&app));

    // The level owes one pick: healing word, guiding bolt or inflict wounds now.
    let pick = offer(&app, |c| {
        *c == ServiceCommand::Choose {
            member: durin_id,
            spell: "base:spell:healing_word".to_owned(),
        }
    });
    assert_eq!(
        shown(&mut app, ServiceLabelId::Row(pick)),
        "Durin: Healing Word"
    );
    assert_eq!(shown(&mut app, ServiceLabelId::Note(pick)), "free");
    let later = offer(&app, |c| {
        *c == ServiceCommand::Choose {
            member: durin_id,
            spell: "base:spell:spiritual_weapon".to_owned(),
        }
    });
    assert!(
        dim(&mut app, ServicePanelId::Offer(later)),
        "spiritual weapon at level 3"
    );
    activate(&mut app, ServicePanelId::Offer(pick));
    assert!(
        logged(&app, "Durin chooses Healing Word"),
        "{:?}",
        log(&app)
    );
    assert_eq!(world(&app).party.members[1].spell_picks, 0);
    assert!(
        view(&app)
            .offers
            .iter()
            .all(|o| !matches!(o.command, ServiceCommand::Choose { .. })),
        "no picks are left to offer"
    );
    leave(&mut app);

    enter(&mut app, "temple");
    let bolt = offer(&app, |c| {
        *c == ServiceCommand::Learn {
            member: durin_id,
            spell: "base:spell:guiding_bolt".to_owned(),
        }
    });
    assert_eq!(
        shown(&mut app, ServiceLabelId::Row(bolt)),
        "Durin: Guiding Bolt"
    );
    let before = gold(&app);
    activate(&mut app, ServicePanelId::Offer(bolt));
    assert_eq!(before - gold(&app), 5000);
    assert!(
        logged(&app, "Durin learns Guiding Bolt for 50 gp"),
        "{:?}",
        log(&app)
    );
    leave(&mut app);

    enter(&mut app, "guild");
    assert!(
        view(&app)
            .offers
            .iter()
            .all(|o| !matches!(o.command, ServiceCommand::Learn { .. })),
        "the guild's spells are the wizard's"
    );
}

#[test]
fn buttons_are_clicked_and_a_refused_one_is_dim() {
    let mut app = town("service-pointer.ron");
    enter(&mut app, "smith");
    let stores = world(&app).party.inventory.len();
    let buy = offer(
        &app,
        |c| matches!(c, ServiceCommand::Buy { item, .. } if item == "base:item:dagger"),
    );
    let price = view(&app).offers[buy].price.unwrap();
    let before = gold(&app);
    let button = control(&mut app, ServicePanelId::Offer(buy));
    click_node(&mut app, button);
    assert_eq!(
        before - gold(&app),
        price,
        "the shown price is what it cost"
    );
    assert_eq!(world(&app).party.inventory.len(), stores + 1);
    let sells = |app: &App| {
        view(app)
            .offers
            .iter()
            .filter(|o| matches!(o.command, ServiceCommand::Sell { .. }))
            .count()
    };
    assert_eq!(sells(&app), stores + 1);
    assert_eq!(
        offer_buttons(&mut app),
        view(&app).offers.len() - 1,
        "the panel was built again with the new row"
    );

    app.world_mut()
        .resource_mut::<SimWorld>()
        .fixture_mut()
        .party
        .gold = 1;
    settle(&mut app);
    let button = control(&mut app, ServicePanelId::Offer(buy));
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
    assert!(
        shown(&mut app, ServiceLabelId::Note(buy)).ends_with("not enough money"),
        "the row says why"
    );
    let lines = log(&app).len();
    click_node(&mut app, button);
    assert_eq!(gold(&app), 1, "a dim button sends nothing");
    assert_eq!(log(&app).len(), lines);
    leave(&mut app);

    // The bank's amount typed by hand: a withdrawal beyond the account is refused in full on
    // the message line, and the next thing done clears it.
    app.world_mut()
        .resource_mut::<SimWorld>()
        .fixture_mut()
        .party
        .gold = 100_000;
    enter(&mut app, "bank");
    let amount = control(&mut app, ServicePanelId::Amount);
    let input = app
        .world()
        .get::<Children>(amount)
        .expect("children")
        .iter()
        .find(|child| app.world().get::<EditableText>(*child).is_some())
        .expect("a text input in the number input");
    click_node(&mut app, input);
    keys(&mut app, "5");
    press(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(
        play_state(&app),
        PlayState::Service,
        "Escape in the amount leaves the field, not the bank"
    );
    let withdraw = control(&mut app, ServicePanelId::Withdraw);
    click_node(&mut app, withdraw);
    assert_eq!(
        shown(&mut app, ServiceLabelId::Message),
        Rejection::BankShort {
            amount: 500,
            bank: 0
        }
        .to_string()
    );
    let deposit = control(&mut app, ServicePanelId::Deposit);
    click_node(&mut app, deposit);
    assert_eq!(world(&app).party.bank, 500);
    assert_eq!(shown(&mut app, ServiceLabelId::Message), "");
}

/// How many times the map came up.
#[derive(Resource, Default)]
struct MapFrames(u32);

#[test]
fn leaving_asks_first_by_button_and_by_escape() {
    let mut app = town("service-leave.ron");
    app.init_resource::<MapFrames>().add_systems(
        OnEnter(PlayState::Explore),
        |mut seen: ResMut<MapFrames>| {
            seen.0 += 1;
        },
    );
    enter(&mut app, "smith");
    let inside = world(&app).position;
    let on_the_map = app.world().resource::<MapFrames>().0;
    let leave_button = control(&mut app, ServicePanelId::Leave);
    click_node(&mut app, leave_button);
    assert_eq!(play_state(&app), PlayState::Confirm);
    assert!(screen_text(app.world()).contains("\"Leave the Smithy?\""));
    let stay = control(&mut app, ConfirmId::Stay);
    click_node(&mut app, stay);
    assert_eq!(
        play_state(&app),
        PlayState::Service,
        "Stay is the panel again"
    );
    assert_eq!(
        app.world().resource::<MapFrames>().0,
        on_the_map,
        "and never the map between, where the keys would walk"
    );
    assert_eq!(world(&app).mode.kind(), ModeKind::Town);

    press(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(play_state(&app), PlayState::Confirm, "Escape asks too");
    let go = control(&mut app, ConfirmId::Go);
    click_node(&mut app, go);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert_eq!(world(&app).mode.kind(), ModeKind::Explore);
    assert_eq!(world(&app).position, inside, "leaving is in place");
    assert!(logged(&app, "The party leaves the Smithy"));
    assert_eq!(screen_text(app.world()), omnis_app::ui_text::NO_PANEL);
}

#[test]
fn the_tool_bar_opens_over_the_panel_and_comes_back_to_it() {
    let mut app = town("service-tools.ron");
    enter(&mut app, "smith");
    let button = control(&mut app, ToolButton::Items);
    click_node(&mut app, button);
    assert_eq!(play_state(&app), PlayState::Inventory);
    press(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(
        play_state(&app),
        PlayState::Service,
        "the inventory closes back"
    );

    let button = control(&mut app, ToolButton::Menu);
    click_node(&mut app, button);
    assert_eq!(play_state(&app), PlayState::Paused);
    press(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(
        play_state(&app),
        PlayState::Service,
        "the pause resumes back"
    );
    control(&mut app, ServicePanelId::Leave);
}

#[test]
fn the_panels_lie_inside_the_map_at_both_window_sizes() {
    let mut app = town("service-layout.ron");
    app.world_mut()
        .resource_mut::<SimWorld>()
        .fixture_mut()
        .party
        .members[0]
        .hp -= 3;
    enter(&mut app, "smith");
    for _ in 0..3 {
        let buy = offer(
            &app,
            |c| matches!(c, ServiceCommand::Buy { item, .. } if item == "base:item:handaxe"),
        );
        activate(&mut app, ServicePanelId::Offer(buy));
    }
    // The smith's two lists each scroll on their own.
    let mut bars = app.world_mut().query::<&bevy::ui_widgets::Scrollbar>();
    let mut targets: Vec<Entity> = bars.iter(app.world()).map(|b| b.target).collect();
    targets.sort();
    targets.dedup();
    assert_eq!(targets.len(), 2, "a scrollbar for each list");
    for target in targets {
        assert!(
            app.world()
                .get::<bevy::ui_widgets::ScrollArea>(target)
                .is_some()
        );
    }
    leave(&mut app);
    for size in ["1280 by 720", "5120 by 1440"] {
        if size.starts_with("5120") {
            ultrawide(&mut app);
        }
        for name in ["smith", "temple", "bank", "tavern", "guild"] {
            enter(&mut app, name);
            assert_eq!(layout_faults(&mut app), Vec::new(), "{name} at {size}");
            if let Ok(dir) = std::env::var("OMNIS_DUMP_SCREENS")
                && size.starts_with("1280")
            {
                let path = std::path::Path::new(&dir).join(format!("service_{name}.txt"));
                std::fs::write(&path, screen_text(app.world()))
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            }
            leave(&mut app);
        }
    }
}
