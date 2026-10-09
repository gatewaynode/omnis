//! The service view (M7 step 6): every offer is the command that asks for it, priced as that
//! command would be priced if it were sent next, with the refusal the rules would give; the
//! view changes nothing; and the party view and status carry the town and rest numbers.

mod common;

use common::{data, inside, inside_with, script};
use omnis_core::CharacterId;
use omnis_data::ServiceKind;
use omnis_sim::ops::party_view;
use omnis_sim::rest::HitDiceSpend;
use omnis_sim::{
    Command, OfferView, Op, OpError, Rejection, Reply, RestCommand, ServiceCommand, apply,
    dispatch, service_view,
};

fn offer(offers: &[OfferView], command: ServiceCommand) -> &OfferView {
    offers
        .iter()
        .find(|o| o.command == command)
        .unwrap_or_else(|| panic!("no offer {command:?}"))
}

#[test]
fn every_smith_price_is_what_the_command_charges() {
    let data = data();
    let mut world = inside(&data, "smith");
    world.party.gold = 1_000_000;
    let view = service_view(&world, &data).expect("inside the smith");
    assert_eq!(view.kind, ServiceKind::Smith);
    assert_eq!(view.service, "base:service:smith");
    assert_eq!(view.gold, world.party.gold);
    let buys: Vec<OfferView> = view
        .offers
        .iter()
        .filter(|o| matches!(o.command, ServiceCommand::Buy { .. }))
        .cloned()
        .collect();
    assert!(!buys.is_empty(), "the smith has stock");
    let stock = &data.services[&data.registry.services.get("base:service:smith").unwrap()].items;
    assert_eq!(buys.len(), stock.len(), "one offer for each item stocked");
    for (id, buy) in stock.iter().zip(&buys) {
        assert_eq!(
            buy.command,
            ServiceCommand::Buy {
                item: id.clone(),
                count: 1
            },
            "the offer names the item by its id, in stock order"
        );
        assert_eq!(buy.subject.as_ref(), Some(id));
        assert_eq!(buy.refusal, None, "{buy:?}");
        let mut after = world.clone();
        let gold = after.party.gold;
        apply(&mut after, &data, Command::Service(buy.command.clone())).unwrap();
        assert_eq!(Some(gold - after.party.gold), buy.price, "{buy:?}");
    }

    apply(&mut world, &data, Command::Service(buys[0].command.clone())).unwrap();
    let view = service_view(&world, &data).unwrap();
    let sale = view
        .offers
        .iter()
        .find(|o| matches!(o.command, ServiceCommand::Sell { .. }))
        .expect("the stores can be sold");
    assert_eq!(sale.price, None);
    let gold = world.party.gold;
    apply(&mut world, &data, Command::Service(sale.command.clone())).unwrap();
    assert_eq!(Some(world.party.gold - gold), sale.pays);
    assert_eq!(
        view.offers.last().map(|o| o.command.clone()),
        Some(ServiceCommand::Leave)
    );
}

#[test]
fn an_offer_the_party_cannot_pay_keeps_its_price() {
    let data = data();
    let mut world = inside(&data, "smith");
    world.party.gold = 1;
    let view = service_view(&world, &data).unwrap();
    let buy = offer(
        &view.offers,
        ServiceCommand::Buy {
            item: "base:item:dagger".to_owned(),
            count: 1,
        },
    );
    let cost = buy.price.expect("a price");
    assert!(cost > 1);
    assert_eq!(buy.refusal, Some(Rejection::CannotAfford { cost, gold: 1 }));
    assert_eq!(
        apply(&mut world, &data, Command::Service(buy.command.clone())),
        Err(buy.refusal.clone().unwrap()),
        "the refusal is the one the command gets"
    );
}

#[test]
fn the_temple_says_why_not_for_each_member() {
    let data = data();
    let mut world = inside(&data, "temple");
    world.party.members[1].hp -= 3;
    let (first, second) = (world.party.members[0].id, world.party.members[1].id);
    let view = service_view(&world, &data).unwrap();
    assert_eq!(
        view.offers.len(),
        2 * 3 + 5 + 1,
        "heal, cure, raise per member, the cleric's five spells, leave"
    );
    let heal = offer(&view.offers, ServiceCommand::Heal { member: first });
    assert_eq!(heal.member, Some(first));
    assert_eq!(
        heal.refusal,
        Some(Rejection::NothingToTreat { member: first })
    );
    assert_eq!(heal.price, None);
    let raise = offer(&view.offers, ServiceCommand::Raise { member: first });
    assert_eq!(raise.refusal, Some(Rejection::NotDead { member: first }));
    let wounded = offer(&view.offers, ServiceCommand::Heal { member: second });
    assert_eq!((wounded.price, wounded.refusal.clone()), (Some(300), None));
}

/// Every offer is what its command does: an open one charges its price and a refused one is
/// refused for the same reason.
fn offers_agree_with_their_commands(world: &omnis_sim::World, data: &omnis_data::Data) -> usize {
    let view = service_view(world, data).unwrap();
    for offer in view
        .offers
        .iter()
        .filter(|o| o.command != ServiceCommand::Leave)
    {
        let mut after = world.clone();
        let result = apply(&mut after, data, Command::Service(offer.command.clone()));
        match &offer.refusal {
            Some(refusal) => assert_eq!(result, Err(refusal.clone()), "{offer:?}"),
            None => {
                assert!(result.is_ok(), "{offer:?}: {result:?}");
                let paid = world.party.gold - after.party.gold;
                assert_eq!(Some(paid), offer.price, "{offer:?}");
            }
        }
    }
    view.offers.len()
}

#[test]
fn the_trainer_offers_levels_and_picks_and_the_sellers_their_spells() {
    let data = data();
    let mut world = inside_with(&data, "trainer", 3);
    world.party.gold = 100_000;
    world.party.members[1].xp = 300;
    let (first, second) = (world.party.members[0].id, world.party.members[1].id);
    let view = service_view(&world, &data).unwrap();
    let train = offer(&view.offers, ServiceCommand::Train { member: second });
    assert_eq!(
        (train.member, train.price, train.refusal.clone()),
        (Some(second), Some(2000), None)
    );
    let unready = offer(&view.offers, ServiceCommand::Train { member: first });
    assert_eq!(
        unready.refusal,
        Some(Rejection::NotReady {
            member: first,
            xp: 0,
            needed: 300
        })
    );
    assert_eq!(
        offers_agree_with_their_commands(&world, &data),
        3 + 1,
        "no picks owed yet"
    );

    apply(&mut world, &data, Command::Service(train.command.clone())).unwrap();
    let view = service_view(&world, &data).unwrap();
    let rows: Vec<(&str, bool)> = view
        .offers
        .iter()
        .filter_map(|o| match &o.command {
            ServiceCommand::Choose { member, spell } if *member == second => {
                Some((spell.as_str(), o.refusal.is_none()))
            }
            _ => None,
        })
        .collect();
    // Healing word, guiding bolt, inflict wounds now; spiritual weapon and prayer of healing at
    // level 3.
    assert_eq!(
        rows,
        [
            ("base:spell:healing_word", true),
            ("base:spell:guiding_bolt", true),
            ("base:spell:inflict_wounds", true),
            ("base:spell:spiritual_weapon", false),
            ("base:spell:prayer_of_healing", false)
        ]
    );
    assert_eq!(
        offer(
            &view.offers,
            ServiceCommand::Choose {
                member: second,
                spell: "base:spell:healing_word".to_owned(),
            }
        )
        .price,
        Some(0)
    );
    offers_agree_with_their_commands(&world, &data);

    for name in ["guild", "temple"] {
        let mut world = inside_with(&data, name, 3);
        world.party.gold = 100_000;
        let count = offers_agree_with_their_commands(&world, &data);
        let view = service_view(&world, &data).unwrap();
        let learners: Vec<Option<CharacterId>> = view
            .offers
            .iter()
            .filter(|o| matches!(o.command, ServiceCommand::Learn { .. }))
            .map(|o| o.member)
            .collect();
        let caster = world.party.members[if name == "guild" { 2 } else { 1 }].id;
        assert!(!learners.is_empty(), "{name}");
        assert!(
            learners.iter().all(|m| *m == Some(caster)),
            "{name}: {learners:?}"
        );
        assert!(count > learners.len(), "{name}");
    }
}

#[test]
fn looking_changes_nothing() {
    let data = data();
    for name in [
        "inn", "tavern", "smith", "temple", "bank", "trainer", "guild",
    ] {
        let world = inside(&data, name);
        let before = world.clone();
        let print = world.fingerprint().unwrap();
        let view = service_view(&world, &data).unwrap();
        assert!(!view.offers.is_empty(), "{name}: leaving at least");
        let mut after = world.clone();
        dispatch(&mut after, &data, &Op::ServiceGet).unwrap();
        assert_eq!(after, before, "{name}");
        assert_eq!(after.fingerprint().unwrap(), print, "{name}");
    }
    let world = inside(&data, "tavern");
    let view = service_view(&world, &data).unwrap();
    let rumor = offer(&view.offers, ServiceCommand::Rumor);
    assert_eq!((rumor.price, rumor.refusal.clone()), (Some(0), None));
}

#[test]
fn outside_a_service_there_is_no_view() {
    let data = data();
    let mut world = inside(&data, "inn");
    apply(&mut world, &data, Command::Service(ServiceCommand::Leave)).unwrap();
    assert_eq!(service_view(&world, &data), None);
    assert_eq!(
        dispatch(&mut world, &data, &Op::ServiceGet),
        Err(OpError::NoService)
    );
}

#[test]
fn the_party_view_and_status_carry_the_town_and_rest() {
    let data = data();
    let mut world = inside(&data, "bank");
    let Ok(Reply::Status(status)) = dispatch(&mut world, &data, &Op::GameStatus) else {
        panic!("status");
    };
    assert_eq!(status.service.as_deref(), Some("base:service:bank"));
    apply(
        &mut world,
        &data,
        Command::Service(ServiceCommand::Deposit { amount: 250 }),
    )
    .unwrap();
    assert_eq!(party_view(&world, &data).bank, 250);

    let mut world = inside(&data, "inn");
    let before = party_view(&world, &data);
    assert_eq!((before.last_long_rest, before.long_rest_wait), (None, 0));
    apply(&mut world, &data, Command::Service(ServiceCommand::Room)).unwrap();
    let rested = party_view(&world, &data);
    let now = world.party_clock().elapsed;
    assert_eq!(rested.last_long_rest, Some(now));
    assert_eq!(
        rested.long_rest_wait,
        1440 - 480,
        "a day end to end, less the night"
    );
    apply(&mut world, &data, Command::Service(ServiceCommand::Leave)).unwrap();
    let Ok(Reply::Status(status)) = dispatch(&mut world, &data, &Op::GameStatus) else {
        panic!("status");
    };
    assert_eq!(status.service, None);

    let member = &mut world.party.members[0];
    member.hp = 1;
    let (level, id) = (member.level, member.id);
    assert_eq!(party_view(&world, &data).members[0].hit_dice_left, level);
    apply(
        &mut world,
        &data,
        Command::Rest(RestCommand::Short {
            dice: vec![HitDiceSpend {
                member: id,
                count: 1,
            }],
        }),
    )
    .unwrap();
    let view = party_view(&world, &data);
    assert_eq!(view.members[0].hit_dice, level);
    assert_eq!(view.members[0].hit_dice_left, level - 1);
    assert_eq!(
        view.long_rest_wait,
        1440 - 480 - 60,
        "an hour's rest counts"
    );
}

#[test]
fn a_town_script_runs_end_to_end() {
    let data = data();
    let mut world = inside(&data, "smith");
    let gold = world.party.gold;
    // Each word resolves against the world as it stands when it is applied: `sell-0` is the
    // first row of the stores after the purchase.
    for text in ["buy-0-2", "sell-0", "leave"] {
        let command = script(&world, &data, text).remove(0);
        apply(&mut world, &data, command).unwrap();
    }
    assert!(world.party.gold < gold);
    assert_eq!(service_view(&world, &data), None, "left");
}
