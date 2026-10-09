//! The service commands (M7 step 4b): each one's success and each refusal, prices from the
//! `services.ron` slots in copper, the clock, and a refusal that leaves the world (its dice
//! streams included) as it was.

mod common;

use common::{data, inside};
use omnis_core::{CharacterId, Clock, EraId, HolderId, StreamName};
use omnis_data::Data;
use omnis_sim::{Command, Event, ModeKind, Rejection, ServiceCommand, World, apply};

fn ask(world: &mut World, data: &Data, command: ServiceCommand) -> Vec<Event> {
    apply(world, data, Command::Service(command)).unwrap_or_else(|r| panic!("{command:?}: {r}"))
}

/// The command is refused with `why`, and nothing changed.
fn refused(world: &mut World, data: &Data, command: ServiceCommand, why: Rejection) {
    let before = world.clone();
    assert_eq!(
        apply(world, data, Command::Service(command)),
        Err(why),
        "{command:?}"
    );
    assert_eq!(*world, before, "{command:?} changed the world");
}

fn minutes(world: &World) -> i64 {
    world.party_clock().elapsed
}

#[test]
fn a_room_is_a_long_rest_once_a_day() {
    let data = data();
    let mut world = inside(&data, "inn");
    let brenna = &mut world.party.members[0];
    brenna.hp -= 5;
    brenna.hit_dice_spent = 1;
    world.party.members[1].spell_points = 0;
    let hp_max = world.party.members[0].hp_max;
    let sp_max = world.party.members[1].spell_points_max;
    let clock = minutes(&world);

    world.party.gold = 99;
    refused(
        &mut world,
        &data,
        ServiceCommand::Room,
        Rejection::CannotAfford {
            cost: 100,
            gold: 99,
        },
    );
    world.party.gold = 5000;
    let events = ask(&mut world, &data, ServiceCommand::Room);
    assert!(
        events.contains(&Event::RoomTaken { cost: 100 }),
        "5 sp a member"
    );
    assert_eq!(world.party.gold, 4900);
    assert_eq!(
        minutes(&world),
        clock + 480,
        "eight hours, no transaction time"
    );
    assert_eq!(world.party.members[0].hp, hp_max);
    assert_eq!(
        world.party.members[0].hit_dice_spent, 0,
        "half of one, at least one"
    );
    assert_eq!(world.party.members[1].spell_points, sp_max);
    assert_eq!(world.party.last_long_rest, Some(clock + 480));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Healed { amount: 5, .. })),
        "{events:?}"
    );
    assert_eq!(world.mode.kind(), ModeKind::Town, "still at the inn");

    refused(
        &mut world,
        &data,
        ServiceCommand::Room,
        Rejection::RestTooSoon { minutes: 960 },
    );
}

#[test]
fn a_room_wakes_the_downed_and_leaves_the_dead() {
    let data = data();
    let mut world = inside(&data, "inn");
    let dead = omnis_rules::condition_id(&data, "dead").unwrap();
    let unconscious = omnis_rules::condition_id(&data, "unconscious").unwrap();
    let brenna = &mut world.party.members[0];
    brenna.hp = 0;
    brenna.conditions.push(unconscious);
    brenna.hit_dice_spent = 1;
    let durin = &mut world.party.members[1];
    durin.hp = 0;
    durin.conditions.push(dead);
    ask(&mut world, &data, ServiceCommand::Room);
    let (brenna, durin) = (&world.party.members[0], &world.party.members[1]);
    assert_eq!(brenna.hp, 1, "stable, up at one hit point");
    assert!(!brenna.conditions.contains(&unconscious));
    assert_eq!(
        brenna.hit_dice_spent, 1,
        "no benefit from a rest begun at zero"
    );
    assert_eq!(durin.hp, 0);
    assert!(durin.conditions.contains(&dead));
}

#[test]
fn the_tavern_sells_food_and_tells_rumors() {
    let data = data();
    let mut world = inside(&data, "tavern");
    let (food, clock) = (world.party.food, minutes(&world));
    let events = ask(&mut world, &data, ServiceCommand::BuyFood { count: 3 });
    assert!(events.contains(&Event::FoodBought {
        count: 3,
        cost: 150
    }));
    assert_eq!((world.party.food, world.party.gold), (food + 3, 5000 - 150));
    assert_eq!(minutes(&world), clock + 10, "service_minutes");
    refused(
        &mut world,
        &data,
        ServiceCommand::BuyFood { count: 0 },
        Rejection::ZeroCount,
    );

    let tavern = data.registry.services.get("base:service:tavern").unwrap();
    let town = StreamName::new("town");
    let region = HolderId::Region(data.maps[&world.position.map].region);
    // At the town's origin only the rats are talked of (M8: the others happen later).
    for _ in 0..10 {
        let events = ask(&mut world, &data, ServiceCommand::Rumor);
        assert!(
            events.contains(&Event::Rumor {
                service: tavern,
                index: 0,
                ago: 0
            }),
            "{events:?}"
        );
    }
    let now = 12 * 1440;
    world.clocks.insert(
        region,
        Clock {
            elapsed: now,
            era: EraId(0),
        },
    );
    let mut heard = std::collections::BTreeSet::new();
    for _ in 0..30 {
        let before = world.rngs.get(&town).copied();
        let events = ask(&mut world, &data, ServiceCommand::Rumor);
        let index = events
            .iter()
            .find_map(|e| match e {
                Event::Rumor {
                    service,
                    index,
                    ago,
                } if *service == tavern => {
                    let at = [0, 2 * 1440, 10 * 1440][usize::from(*index)];
                    assert_eq!(*ago, now - at, "told on the town's clock");
                    Some(*index)
                }
                _ => None,
            })
            .expect("a rumor");
        assert!(index < 3, "one of the three");
        heard.insert(index);
        assert_ne!(
            world.rngs.get(&town).copied(),
            before,
            "the die rolled on `town`"
        );
    }
    assert_eq!(heard.len(), 3, "every rumor comes up");
    assert_eq!(world.party.gold, 5000 - 150, "rumors are free");
}

#[test]
fn the_temple_heals_cures_and_raises_for_a_price() {
    let data = data();
    let mut world = inside(&data, "temple");
    let (brenna, durin) = (world.party.members[0].id, world.party.members[1].id);
    refused(
        &mut world,
        &data,
        ServiceCommand::Heal { member: brenna },
        Rejection::NothingToTreat { member: brenna },
    );
    world.party.members[0].hp -= 4;
    let events = ask(&mut world, &data, ServiceCommand::Heal { member: brenna });
    assert!(events.contains(&Event::Treated {
        member: brenna,
        cost: 400
    }));
    assert_eq!(world.party.members[0].hp, world.party.members[0].hp_max);
    assert_eq!(world.party.gold, 4600);

    refused(
        &mut world,
        &data,
        ServiceCommand::Cure { member: brenna },
        Rejection::NothingToTreat { member: brenna },
    );
    let poisoned = omnis_rules::condition_id(&data, "poisoned").unwrap();
    world.party.members[0].conditions.push(poisoned);
    let events = ask(&mut world, &data, ServiceCommand::Cure { member: brenna });
    assert!(events.contains(&Event::Condition {
        target: omnis_sim::ActorRef::Member(brenna),
        condition: poisoned,
        applied: false
    }));
    assert!(world.party.members[0].conditions.is_empty());
    assert_eq!(world.party.gold, 3600, "10 gp");

    refused(
        &mut world,
        &data,
        ServiceCommand::Raise { member: durin },
        Rejection::NotDead { member: durin },
    );
    let dead = omnis_rules::condition_id(&data, "dead").unwrap();
    world.party.members[1].hp = 0;
    world.party.members[1].conditions.push(dead);
    refused(
        &mut world,
        &data,
        ServiceCommand::Heal { member: durin },
        Rejection::MemberDead { member: durin },
    );
    world.party.gold = 2499;
    refused(
        &mut world,
        &data,
        ServiceCommand::Raise { member: durin },
        Rejection::CannotAfford {
            cost: 2500,
            gold: 2499,
        },
    );
    world.party.gold = 2500;
    let events = ask(&mut world, &data, ServiceCommand::Raise { member: durin });
    assert!(events.contains(&Event::Raised {
        member: durin,
        cost: 2500
    }));
    assert_eq!(world.party.members[1].hp, 1);
    assert!(world.party.members[1].conditions.is_empty());
    assert_eq!(world.party.gold, 0);
    refused(
        &mut world,
        &data,
        ServiceCommand::Heal {
            member: CharacterId(6),
        },
        Rejection::NoSuchMember {
            member: CharacterId(6),
        },
    );
}

#[test]
fn the_smith_buys_at_list_and_sells_at_half() {
    let data = data();
    let mut world = inside(&data, "smith");
    let dagger = data.registry.items.get("base:item:dagger").unwrap();
    let events = ask(&mut world, &data, ServiceCommand::Buy { item: 0, count: 2 });
    assert!(events.contains(&Event::Bought {
        item: dagger,
        count: 2,
        cost: 400
    }));
    assert_eq!(world.party.gold, 4600);
    let row = world
        .party
        .inventory
        .iter()
        .position(|(i, _)| *i == dagger)
        .unwrap();
    assert_eq!(world.party.inventory[row], (dagger, 2));
    refused(
        &mut world,
        &data,
        ServiceCommand::Buy { item: 99, count: 1 },
        Rejection::NotOffered,
    );

    let row = u8::try_from(row).unwrap();
    refused(
        &mut world,
        &data,
        ServiceCommand::Sell {
            item: row,
            count: 3,
        },
        Rejection::NotEnough {
            item: dagger,
            have: 2,
        },
    );
    let events = ask(
        &mut world,
        &data,
        ServiceCommand::Sell {
            item: row,
            count: 2,
        },
    );
    assert!(events.contains(&Event::Sold {
        item: dagger,
        count: 2,
        price: 200
    }));
    assert_eq!(world.party.gold, 4800);
    assert!(!world.party.inventory.iter().any(|(i, _)| *i == dagger));
    refused(
        &mut world,
        &data,
        ServiceCommand::Sell { item: 99, count: 1 },
        Rejection::NotInStores { item: 99 },
    );
}

#[test]
fn the_bank_keeps_copper() {
    let data = data();
    let mut world = inside(&data, "bank");
    refused(
        &mut world,
        &data,
        ServiceCommand::Deposit { amount: 5001 },
        Rejection::CannotAfford {
            cost: 5001,
            gold: 5000,
        },
    );
    let events = ask(&mut world, &data, ServiceCommand::Deposit { amount: 1234 });
    assert!(events.contains(&Event::Banked {
        amount: 1234,
        deposit: true
    }));
    assert_eq!((world.party.gold, world.party.bank), (3766, 1234));
    refused(
        &mut world,
        &data,
        ServiceCommand::Withdraw { amount: 1235 },
        Rejection::BankShort {
            amount: 1235,
            bank: 1234,
        },
    );
    ask(&mut world, &data, ServiceCommand::Withdraw { amount: 1000 });
    assert_eq!((world.party.gold, world.party.bank), (4766, 234));
    refused(
        &mut world,
        &data,
        ServiceCommand::Withdraw { amount: 0 },
        Rejection::ZeroCount,
    );
}

#[test]
fn each_service_does_only_its_own_work() {
    let data = data();
    for name in [
        "inn", "temple", "trainer", "guild", "smith", "tavern", "bank",
    ] {
        let mut world = inside(&data, name);
        let (brenna, durin) = (world.party.members[0].id, world.party.members[1].id);
        let asks = [
            ServiceCommand::Room,
            ServiceCommand::Rumor,
            ServiceCommand::Heal { member: brenna },
            ServiceCommand::Buy { item: 0, count: 1 },
            ServiceCommand::Deposit { amount: 1 },
            ServiceCommand::Train { member: brenna },
            ServiceCommand::Learn {
                member: durin,
                spell: 2,
            },
        ];
        world.party.members[0].hp -= 1;
        world.party.members[0].xp = 300;
        for command in asks {
            let before = world.clone();
            let result = apply(&mut world, &data, Command::Service(command));
            let offered = matches!(
                (name, command),
                ("inn", ServiceCommand::Room)
                    | ("tavern", ServiceCommand::Rumor)
                    | (
                        "temple",
                        ServiceCommand::Heal { .. } | ServiceCommand::Learn { .. }
                    )
                    | ("smith", ServiceCommand::Buy { .. })
                    | ("bank", ServiceCommand::Deposit { .. })
                    | ("trainer", ServiceCommand::Train { .. })
                    | ("guild", ServiceCommand::Learn { .. })
            );
            if matches!((name, command), ("guild", ServiceCommand::Learn { .. })) {
                // The guild's shield is off the cleric's list: offered, and refused for that.
                assert_eq!(result, Err(Rejection::NotOnList { member: durin }));
                world = before;
            } else if offered {
                assert!(result.is_ok(), "{name} {command:?}: {result:?}");
                world = before;
            } else {
                assert_eq!(result, Err(Rejection::NotOffered), "{name} {command:?}");
                assert_eq!(world, before);
            }
        }
    }
}

#[test]
fn a_refusal_names_a_price_to_the_copper() {
    let why = Rejection::CannotAfford {
        cost: 1645,
        gold: 1609,
    };
    assert_eq!(
        why.to_string(),
        "that costs 16 gp 4 sp 5 cp; the party has 16 gp 0 sp 9 cp"
    );
}
