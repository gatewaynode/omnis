//! The service view (M7 step 6): every offer is the command that asks for it, priced as that
//! command would be priced if it were sent next, with the refusal the rules would give; the
//! view changes nothing; and the party view and status carry the town and rest numbers.

mod common;

use common::{data, inside};
use omnis_data::ServiceKind;
use omnis_sim::ops::party_view;
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
    for (row, buy) in buys.iter().enumerate() {
        assert_eq!(buy.row, Some(u8::try_from(row).unwrap()));
        assert_eq!(buy.refusal, None, "{buy:?}");
        let mut after = world.clone();
        let gold = after.party.gold;
        apply(&mut after, &data, Command::Service(buy.command)).unwrap();
        assert_eq!(Some(gold - after.party.gold), buy.price, "{buy:?}");
    }

    apply(&mut world, &data, Command::Service(buys[0].command)).unwrap();
    let view = service_view(&world, &data).unwrap();
    let sale = view
        .offers
        .iter()
        .find(|o| matches!(o.command, ServiceCommand::Sell { .. }))
        .expect("the stores can be sold");
    assert_eq!(sale.price, None);
    let gold = world.party.gold;
    apply(&mut world, &data, Command::Service(sale.command)).unwrap();
    assert_eq!(Some(world.party.gold - gold), sale.pays);
    assert_eq!(
        view.offers.last().map(|o| o.command),
        Some(ServiceCommand::Leave)
    );
}

#[test]
fn an_offer_the_party_cannot_pay_keeps_its_price() {
    let data = data();
    let mut world = inside(&data, "smith");
    world.party.gold = 1;
    let view = service_view(&world, &data).unwrap();
    let buy = offer(&view.offers, ServiceCommand::Buy { item: 0, count: 1 });
    let cost = buy.price.expect("a price");
    assert!(cost > 1);
    assert_eq!(buy.refusal, Some(Rejection::CannotAfford { cost, gold: 1 }));
    assert_eq!(
        apply(&mut world, &data, Command::Service(buy.command)),
        Err(buy.refusal.clone().unwrap()),
        "the refusal is the one the command gets"
    );
}

#[test]
fn the_temple_says_why_not_for_each_member() {
    let data = data();
    let mut world = inside(&data, "temple");
    world.party.members[1].hp -= 3;
    let view = service_view(&world, &data).unwrap();
    assert_eq!(
        view.offers.len(),
        2 * 3 + 1,
        "heal, cure, raise per member, leave"
    );
    let heal = offer(&view.offers, ServiceCommand::Heal { member: 0 });
    assert_eq!(heal.member, Some(0));
    assert_eq!(heal.refusal, Some(Rejection::NothingToTreat { index: 0 }));
    assert_eq!(heal.price, None);
    let raise = offer(&view.offers, ServiceCommand::Raise { member: 0 });
    assert_eq!(raise.refusal, Some(Rejection::NotDead { index: 0 }));
    let wounded = offer(&view.offers, ServiceCommand::Heal { member: 1 });
    assert_eq!((wounded.price, wounded.refusal.clone()), (Some(300), None));
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
    let level = member.level;
    assert_eq!(party_view(&world, &data).members[0].hit_dice_left, level);
    apply(
        &mut world,
        &data,
        Command::Rest(RestCommand::Short { dice: vec![1] }),
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
    let commands = omnis_sim::command::parse_script("buy-0-2, sell-0, leave").unwrap();
    for command in commands {
        apply(&mut world, &data, command).unwrap();
    }
    assert!(world.party.gold < gold);
    assert_eq!(service_view(&world, &data), None, "left");
}
