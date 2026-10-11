//! Town services (parity P4): the service view the shell reads, the words each offer is shown
//! with, and the HUD lines for what the party does inside. Real packs, the fixed party, the
//! test pack's town.

use crate::common;

use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_rules::Gains;
use omnis_sim::{Command, Event, Rejection, ServiceCommand};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::shell::text;
use omnis_vector::sim::{SimWorld, Views};

/// The fixed party inside the service whose site tile is `(x, y)` in the town.
fn inside(data: &Data, (x, y): (u16, u16)) -> SimWorld {
    let (mut world, _) = common::town_session(data);
    let town = common::map(data, "test:map:town");
    common::place(&mut world, town, x, y, Facing::North);
    world
        .apply(data, Command::Interact)
        .expect("a site tile opens its service");
    world
}

const SMITH: (u16, u16) = (1, 3);
const TEMPLE: (u16, u16) = (4, 1);
const TRAINER: (u16, u16) = (7, 1);
const TAVERN: (u16, u16) = (4, 3);

/// Every offer's row label, caption, note and reason, in the view's order.
fn rows(data: &Data, views: &Views) -> Vec<(String, &'static str, String, Option<String>)> {
    let service = views.service.as_ref().expect("inside a service");
    service
        .offers
        .iter()
        .map(|o| {
            (
                labels::offer_label(data, &views.party, o),
                labels::offer_caption(&o.command),
                labels::offer_note(o),
                o.refusal.as_ref().map(labels::service_reason),
            )
        })
        .collect()
}

#[test]
fn the_views_hold_the_service_inside_and_none_outside() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    assert_eq!(common::views(&world, &data).service, None);
    let views = common::views(&inside(&data, SMITH), &data);
    let service = views.service.expect("the smith's view");
    assert_eq!(service.service, "base:service:smith");
    assert_eq!(
        service.offers.last().map(|o| &o.command),
        Some(&ServiceCommand::Leave)
    );
}

/// A purchase the party cannot pay keeps its price beside the reason, and a sale names the
/// stores' count (the other client's `service_panel` words).
#[test]
fn the_smith_s_rows_name_the_item_its_price_and_why_not() {
    let data = common::data();
    let mut world = inside(&data, SMITH);
    world.party.gold = 0;
    let views = common::views(&world, &data);
    let dagger = views.service.as_ref().unwrap().offers[0].clone();
    let (label, caption, note, why) = rows(&data, &views).remove(0);
    assert_eq!((label.as_str(), caption), ("Dagger", "Buy"));
    assert_eq!(note, labels::coins(dagger.price.expect("a price")));
    assert_eq!(why.as_deref(), Some("not enough money"));

    world.party.gold = 10_000;
    for _ in 0..2 {
        let buy = ServiceCommand::Buy {
            item: "base:item:dagger".to_owned(),
            count: 1,
        };
        world.apply(&data, Command::Service(buy)).expect("bought");
    }
    let views = common::views(&world, &data);
    let held = views
        .party
        .inventory
        .iter()
        .find(|i| i.item == "base:item:dagger")
        .map_or(0, |i| i.count);
    assert!(held >= 2, "two daggers bought");
    let sell = rows(&data, &views)
        .into_iter()
        .find(|(label, caption, ..)| *caption == "Sell" && label.starts_with("Dagger"))
        .expect("the daggers can be sold");
    assert_eq!(sell.0, format!("Dagger ×{held}"));
    assert!(sell.2.starts_with("pays "), "{}", sell.2);
    assert_eq!(sell.3, None);
}

#[test]
fn the_temple_names_what_it_treats_and_the_trainer_the_next_level() {
    let data = common::data();
    let mut world = inside(&data, TEMPLE);
    world.party.members[0].hp = 3;
    let views = common::views(&world, &data);
    let brenna: Vec<_> = rows(&data, &views)
        .into_iter()
        .filter(|(label, ..)| label.starts_with("Brenna"))
        .collect();
    let max = views.party.members[0].hp_max;
    assert_eq!(brenna[0].0, format!("Brenna, 3 of {max} HP"));
    assert_eq!(brenna[0].1, "Heal");
    assert_eq!(
        (brenna[1].0.as_str(), brenna[1].3.as_deref()),
        ("Brenna, no conditions", Some("nothing to treat"))
    );
    assert_eq!(
        (brenna[2].0.as_str(), brenna[2].3.as_deref()),
        ("Brenna, alive", Some("not dead"))
    );

    let views = common::views(&inside(&data, TRAINER), &data);
    let (label, caption, ..) = rows(&data, &views).remove(0);
    assert_eq!((label.as_str(), caption), ("Brenna, level 1 to 2", "Train"));
}

#[test]
fn a_rest_refused_too_soon_says_how_long_to_wait() {
    assert_eq!(
        labels::service_reason(&Rejection::RestTooSoon { minutes: 65 }),
        "rested too recently (1h 05m)"
    );
}

/// The HUD's lines for a tavern visit, from the door to the price (the other client's
/// `a_night_at_the_inn_reads_from_the_door_to_the_price`).
#[test]
fn a_tavern_visit_reads_from_the_door_to_the_rumor() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    world.party.gold = 1537;
    let town = common::map(&data, "test:map:town");
    common::place(&mut world, town, TAVERN.0, TAVERN.1, Facing::North);
    let mut events = world.apply(&data, Command::Interact).unwrap();
    for command in [
        ServiceCommand::BuyFood { count: 3 },
        ServiceCommand::Rumor,
        ServiceCommand::Leave,
    ] {
        events.extend(world.apply(&data, Command::Service(command)).unwrap());
    }
    let names = Names::default();
    let lines: Vec<String> = events
        .iter()
        .filter_map(|e| text::describe(e, &names, &data))
        .collect();
    assert_eq!(
        lines,
        [
            "The party enters the Tavern",
            "Bought 3 food for 1 gp 5 sp 0 cp",
            "\"Talk from today: the rats below grow bolder every week.\"",
            "The party leaves the Tavern",
        ]
    );
}

#[test]
fn a_level_a_spell_and_the_bank_read_with_names_and_coins() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let mut names = Names::default();
    names.observe(&common::views(&world, &data).party);
    let (brenna, ilvara) = (common::id(&world, 0), common::id(&world, 2));
    let line = |event: Event| text::describe(&event, &names, &data).expect("a town line");
    assert_eq!(
        line(Event::LevelUp {
            member: brenna,
            level: 2,
            cost: 2000,
            gains: Gains {
                hp: 8,
                spell_points: 0,
                picks: 0,
                proficiency: 2,
                features: vec!["base:text:class.fighter.action_surge".to_owned()],
            },
        }),
        "Brenna reaches level 2: +8 HP, Action Surge"
    );
    let shatter = "base:spell:shatter".to_owned();
    assert_eq!(
        line(Event::SpellLearned {
            member: ilvara,
            spell: shatter.clone(),
            cost: 0,
        }),
        "Ilvara chooses Shatter"
    );
    assert_eq!(
        line(Event::SpellLearned {
            member: ilvara,
            spell: shatter,
            cost: 10_000,
        }),
        "Ilvara learns Shatter for 100 gp 0 sp 0 cp"
    );
    assert_eq!(
        line(Event::Banked {
            amount: 250,
            deposit: true,
        }),
        "Deposited 2 gp 5 sp 0 cp"
    );
    assert_eq!(
        line(Event::Banked {
            amount: 100,
            deposit: false,
        }),
        "Withdrew 1 gp 0 sp 0 cp"
    );
}
