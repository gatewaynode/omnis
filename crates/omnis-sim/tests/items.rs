//! Items on real pack data: counts in a member's kit and the party's stores, the components a
//! cast consumes, the gem item by name, healing shared by potions and spells, and the item
//! commands: equipping with its minutes, moves between kits and the stores, a potion on the
//! road and in a fight, the rejections that leave the world untouched, the script words.

mod common;

use common::{act, data, encounter, party_of, world};
use omnis_core::{CharacterId, ItemId, StreamName};
use omnis_data::{Data, Disposition, EquipSlot};
use omnis_sim::items::{consume, count_of, has_all, item_id};
use omnis_sim::omnis_rules::{DeathSaves, armor_class, condition_id};
use omnis_sim::party::heal;
use omnis_sim::{
    ActorRef, CombatCommand, Command, Event, ItemCommand, ItemPlace, Mode, Party, Rejection,
    Surprise, Word, World, apply, combat, parse_script,
};

/// The id of an item a member's kit holds, as the item commands take it.
fn kit_row(world: &World, slot: usize, item: ItemId) -> String {
    assert!(
        count_of(&world.party.members[slot].equipment, item) > 0,
        "carried"
    );
    world_item(item)
}

/// An item's string id, from the base pack's registry.
fn world_item(item: ItemId) -> String {
    data().registry.items.name(item).unwrap().to_owned()
}

/// An id no pack defines.
const NOPE: &str = "base:item:nope";

fn item(command: ItemCommand) -> Command {
    Command::Item(command)
}

/// A rejection leaves the world exactly as it was.
fn refused(world: &mut World, data: &Data, command: Command, expected: Rejection) {
    let before = world.clone();
    assert_eq!(apply(world, data, command).unwrap_err(), expected);
    assert_eq!(*world, before, "{expected:?} changed the world");
}

fn minutes_passed(events: &[Event]) -> u32 {
    events
        .iter()
        .map(|e| match e {
            Event::TimeAdvanced { minutes, .. } => *minutes,
            _ => 0,
        })
        .sum()
}

#[test]
fn stores_are_counted_and_consumed_exactly_or_not_at_all() {
    let data = data();
    let gem = item_id(&data, "gem").unwrap();
    assert_eq!(data.registry.items.name(gem), Some("base:item:gem"));
    assert_eq!(item_id(&data, "philosopher_stone"), None);
    let potion = item_id(&data, "potion_of_healing").unwrap();
    let mut party = Party {
        inventory: vec![(gem, 3), (potion, 1)],
        ..Party::default()
    };
    assert_eq!(count_of(&party.inventory, gem), 3);
    assert_eq!(
        count_of(&party.inventory, item_id(&data, "dagger").unwrap()),
        0
    );
    assert!(has_all(&party, &[(gem, 3), (potion, 1)]));
    assert!(!has_all(&party, &[(gem, 4)]));
    let before = party.clone();
    let refused = consume(&mut party, &[(gem, 2), (potion, 2)]).unwrap_err();
    assert_eq!(
        refused,
        omnis_sim::items::Short {
            item: potion,
            have: 1
        }
    );
    assert_eq!(
        refused.rejection(&data),
        Rejection::NotEnough {
            item: "base:item:potion_of_healing".to_owned(),
            have: 1
        },
        "the refusal names the item by its id"
    );
    assert_eq!(party, before, "a refusal takes nothing, not even the gems");
    consume(&mut party, &[(gem, 2)]).unwrap();
    assert_eq!(party.inventory, [(gem, 1), (potion, 1)]);
    consume(&mut party, &[(gem, 1), (potion, 1)]).unwrap();
    assert!(party.inventory.is_empty(), "empty entries are dropped");
    assert!(has_all(&party, &[]));
}

#[test]
fn healing_caps_at_the_maximum_and_gets_a_downed_member_up() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    let unconscious = data
        .registry
        .conditions
        .get("base:condition:unconscious")
        .unwrap();
    let max = world.party.members[0].hp_max;
    world.party.members[0].hp = max - 3;
    let mut events = Vec::new();
    heal(&mut world, &data, 0, Vec::new(), 10, &mut events);
    assert_eq!(world.party.members[0].hp, max);
    assert_eq!(
        events,
        [Event::Healed {
            target: world.party.members[0].id,
            rolls: Vec::new(),
            amount: 10,
            hp: max
        }]
    );
    let member = &mut world.party.members[0];
    member.hp = 0;
    member.conditions.push(unconscious);
    member.death_saves.failures = 2;
    let mut events = Vec::new();
    heal(&mut world, &data, 0, Vec::new(), 4, &mut events);
    let member = &world.party.members[0];
    assert_eq!(member.hp, 4);
    assert!(!member.conditions.contains(&unconscious));
    assert_eq!(member.death_saves, DeathSaves::default());
    assert!(matches!(events[1], Event::Condition { applied: false, .. }));
    let mut events = Vec::new();
    heal(&mut world, &data, 0, Vec::new(), -5, &mut events);
    assert_eq!(world.party.members[0].hp, 4, "healing never hurts");
}

/// The identity of the member in `slot`: every party here is made by `party_of` on a new world
/// and never reordered, so the ids follow the slots (0, 1, …); an id past the party names nobody.
fn id(slot: u8) -> CharacterId {
    CharacterId(u32::from(slot))
}

fn equip(member: u8, item: &str) -> Command {
    Command::Item(ItemCommand::Equip {
        member: id(member),
        item: item.to_owned(),
    })
}

fn unequip(member: u8, slot: EquipSlot) -> Command {
    Command::Item(ItemCommand::Unequip {
        member: id(member),
        slot,
    })
}

fn give(from: u8, to: u8, item: &str, count: u16) -> Command {
    Command::Item(ItemCommand::Give {
        giver: id(from),
        receiver: id(to),
        item: item.to_owned(),
        count,
    })
}

fn stow(member: u8, item: &str, count: u16) -> Command {
    Command::Item(ItemCommand::Stow {
        member: id(member),
        item: item.to_owned(),
        count,
    })
}

fn take(member: u8, item: &str, count: u16) -> Command {
    Command::Item(ItemCommand::Take {
        member: id(member),
        item: item.to_owned(),
        count,
    })
}

fn use_on(member: u8, item: &str, target: Option<u8>) -> Command {
    Command::Item(ItemCommand::Use {
        member: id(member),
        item: item.to_owned(),
        receiver: target.map(id),
    })
}

/// The equip and unequip events of a command, in order.
fn slot_events(events: &[Event]) -> Vec<&Event> {
    events
        .iter()
        .filter(|e| matches!(e, Event::Equipped { .. } | Event::Unequipped { .. }))
        .collect()
}

#[test]
fn equipping_swaps_the_slot_and_armor_takes_its_minutes() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    let brenna = world.party.members[0].id;
    let (chain, leather) = (
        item_id(&data, "chain_mail").unwrap(),
        item_id(&data, "leather").unwrap(),
    );
    let ac = |world: &World| armor_class(&world.party.members[0], &data);
    assert_eq!(
        world.party.members[0].equipped.get(&EquipSlot::Body),
        Some(&chain)
    );
    assert_eq!(ac(&world), 18);
    world.party.members[0].equipment.push((leather, 1));
    let row = kit_row(&world, 0, leather);
    let events = apply(&mut world, &data, equip(0, &row)).unwrap();
    assert_eq!(
        slot_events(&events),
        [
            &Event::Unequipped {
                member: brenna,
                slot: EquipSlot::Body,
                item: "base:item:chain_mail".to_owned()
            },
            &Event::Equipped {
                member: brenna,
                slot: EquipSlot::Body,
                item: "base:item:leather".to_owned()
            }
        ]
    );
    assert_eq!(minutes_passed(&events), 5, "don_armor_minutes");
    assert_eq!(
        world.party.members[0].equipped.get(&EquipSlot::Body),
        Some(&leather)
    );
    assert_eq!(count_of(&world.party.members[0].equipment, chain), 1);
    assert_eq!(ac(&world), 15);
    let events = apply(&mut world, &data, unequip(0, EquipSlot::OffHand)).unwrap();
    assert_eq!(
        slot_events(&events),
        [&Event::Unequipped {
            member: brenna,
            slot: EquipSlot::OffHand,
            item: "base:item:shield".to_owned()
        }]
    );
    assert_eq!(minutes_passed(&events), 0, "a shield comes off for free");
    assert_eq!(ac(&world), 13);
    let events = apply(&mut world, &data, unequip(0, EquipSlot::Body)).unwrap();
    assert_eq!(minutes_passed(&events), 5, "doffing armor takes the same");
    assert_eq!(ac(&world), 12);
    let row = kit_row(&world, 0, chain);
    let events = apply(&mut world, &data, equip(0, &row)).unwrap();
    assert_eq!(
        slot_events(&events).len(),
        1,
        "an empty slot displaces nothing"
    );
    assert_eq!(ac(&world), 16);
}

#[test]
fn equipping_refuses_gear_bad_rows_empty_slots_and_absent_members() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    let potion = kit_row(&world, 0, item_id(&data, "potion_of_healing").unwrap());
    refused(
        &mut world,
        &data,
        equip(0, &potion),
        Rejection::NotEquippable,
    );
    refused(
        &mut world,
        &data,
        equip(0, NOPE),
        Rejection::UnknownItem {
            item: NOPE.to_owned(),
        },
    );
    refused(
        &mut world,
        &data,
        equip(5, "base:item:longsword"),
        Rejection::NoSuchMember { member: id(5) },
    );
    apply(&mut world, &data, unequip(0, EquipSlot::OffHand)).unwrap();
    refused(
        &mut world,
        &data,
        unequip(0, EquipSlot::OffHand),
        Rejection::SlotEmpty {
            slot: EquipSlot::OffHand,
        },
    );
    world.party.members[0].hp = 0;
    refused(
        &mut world,
        &data,
        equip(0, "base:item:longsword"),
        Rejection::MemberDown { member: id(0) },
    );
}

#[test]
fn giving_moves_counts_and_refuses_zero_too_many_oneself_and_nobody() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let (brenna, durin) = (world.party.members[0].id, world.party.members[1].id);
    let bolts = item_id(&data, "crossbow_bolts").unwrap();
    let clock = world.party_clock().elapsed;
    let row = kit_row(&world, 0, bolts);
    let events = apply(&mut world, &data, give(0, 1, &row, 5)).unwrap();
    assert!(events.contains(&Event::ItemMoved {
        item: "base:item:crossbow_bolts".to_owned(),
        count: 5,
        from: ItemPlace::Member(brenna),
        to: ItemPlace::Member(durin),
    }));
    assert_eq!(count_of(&world.party.members[0].equipment, bolts), 15);
    assert_eq!(count_of(&world.party.members[1].equipment, bolts), 25);
    refused(&mut world, &data, give(0, 1, &row, 0), Rejection::ZeroCount);
    refused(
        &mut world,
        &data,
        give(0, 1, &row, 16),
        Rejection::NotEnough {
            item: "base:item:crossbow_bolts".to_owned(),
            have: 15,
        },
    );
    refused(
        &mut world,
        &data,
        give(0, 0, &row, 1),
        Rejection::SameMember,
    );
    refused(
        &mut world,
        &data,
        give(0, 7, &row, 1),
        Rejection::NoSuchMember { member: id(7) },
    );
    refused(
        &mut world,
        &data,
        give(0, 1, NOPE, 1),
        Rejection::UnknownItem {
            item: NOPE.to_owned(),
        },
    );
    assert_eq!(
        world.party_clock().elapsed,
        clock,
        "moving items takes no time"
    );
}

#[test]
fn stowing_a_worn_item_takes_it_off_and_taking_brings_it_back() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let (brenna, durin) = (world.party.members[0].id, world.party.members[1].id);
    let longsword = item_id(&data, "longsword").unwrap();
    assert_eq!(
        world.party.members[0].equipped.get(&EquipSlot::MainHand),
        Some(&longsword)
    );
    let row = kit_row(&world, 0, longsword);
    let events = apply(&mut world, &data, stow(0, &row, 1)).unwrap();
    let moved: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::Unequipped { .. } | Event::ItemMoved { .. }))
        .collect();
    assert_eq!(
        moved,
        [
            &Event::Unequipped {
                member: brenna,
                slot: EquipSlot::MainHand,
                item: "base:item:longsword".to_owned()
            },
            &Event::ItemMoved {
                item: "base:item:longsword".to_owned(),
                count: 1,
                from: ItemPlace::Member(brenna),
                to: ItemPlace::Stores,
            }
        ]
    );
    assert!(
        !world.party.members[0]
            .equipped
            .contains_key(&EquipSlot::MainHand)
    );
    assert_eq!(count_of(&world.party.members[0].equipment, longsword), 0);
    assert_eq!(world.party.inventory, [(longsword, 1)]);
    // Durin takes it from the stores; then the stores are empty.
    let sword = "base:item:longsword";
    let events = apply(&mut world, &data, take(1, sword, 1)).unwrap();
    assert!(events.contains(&Event::ItemMoved {
        item: "base:item:longsword".to_owned(),
        count: 1,
        from: ItemPlace::Stores,
        to: ItemPlace::Member(durin),
    }));
    assert!(world.party.inventory.is_empty());
    assert_eq!(count_of(&world.party.members[1].equipment, longsword), 1);
    assert!(
        !world.party.members[1]
            .equipped
            .values()
            .any(|i| *i == longsword),
        "taking does not wield"
    );
    refused(
        &mut world,
        &data,
        take(1, sword, 1),
        Rejection::NotInStores {
            item: sword.to_owned(),
        },
    );
    assert_eq!(minutes_passed(&events), 0);
}

#[test]
fn a_potion_gets_a_downed_member_up_and_is_spent() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let (brenna, durin) = (world.party.members[0].id, world.party.members[1].id);
    let potion = item_id(&data, "potion_of_healing").unwrap();
    let unconscious = condition_id(&data, "unconscious").unwrap();
    let durin_max = world.party.members[1].hp_max;
    {
        let m = &mut world.party.members[1];
        m.hp = 0;
        m.conditions.push(unconscious);
        m.death_saves.failures = 2;
    }
    let row = kit_row(&world, 0, potion);
    let events = apply(&mut world, &data, use_on(0, &row, Some(1))).unwrap();
    let used = events
        .iter()
        .position(|e| {
            *e == Event::ItemUsed {
                member: brenna,
                item: "base:item:potion_of_healing".to_owned(),
                receiver: Some(durin),
                consumed: true,
            }
        })
        .expect("used");
    let Event::Healed {
        target,
        rolls,
        amount,
        hp,
    } = &events[used + 1]
    else {
        panic!("{events:?}")
    };
    assert_eq!((*target, rolls.len()), (durin, 1));
    assert_eq!(rolls[0].stream, StreamName::new("items"));
    let dice = rolls[0].dice;
    assert_eq!((dice.count, dice.sides, dice.modifier), (2, 4, 2));
    assert_eq!(i64::from(rolls[0].total), *amount);
    assert!((4..=10).contains(amount), "2d4+2: {amount}");
    assert_eq!(i64::from(*hp), (*amount).min(i64::from(durin_max)));
    assert_eq!(minutes_passed(&events), 1, "use_item_minutes");
    let m = &world.party.members[1];
    assert!(!m.conditions.contains(&unconscious) && m.death_saves == DeathSaves::default());
    assert_eq!(count_of(&world.party.members[0].equipment, potion), 0);
    // Durin drinks his own on himself: no target named.
    world.party.members[1].hp = 1;
    let row = kit_row(&world, 1, potion);
    let events = apply(&mut world, &data, use_on(1, &row, None)).unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::ItemUsed { member, receiver: Some(t), consumed: true, .. } if *member == durin && *t == durin
    )));
    assert!(world.party.members[1].hp > 1);
    assert_eq!(count_of(&world.party.members[1].equipment, potion), 0);
}

#[test]
fn a_use_refuses_useless_items_dead_targets_and_users_who_cannot_act() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let potion = item_id(&data, "potion_of_healing").unwrap();
    let glass = item_id(&data, "spyglass").unwrap();
    let sword = kit_row(&world, 0, item_id(&data, "longsword").unwrap());
    refused(
        &mut world,
        &data,
        use_on(0, &sword, None),
        Rejection::NotUsable,
    );
    // A spyglass is a use on the road: it looks (tests/sense.rs) and is not spent.
    world.party.members[0].equipment.push((glass, 1));
    let glass_row = kit_row(&world, 0, glass);
    let events = apply(&mut world, &data, use_on(0, &glass_row, None)).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::Sensed { .. })));
    assert_eq!(count_of(&world.party.members[0].equipment, glass), 1);
    refused(
        &mut world,
        &data,
        use_on(0, NOPE, None),
        Rejection::UnknownItem {
            item: NOPE.to_owned(),
        },
    );
    let row = kit_row(&world, 0, potion);
    let dead = condition_id(&data, "dead").unwrap();
    world.party.members[1].conditions.push(dead);
    refused(
        &mut world,
        &data,
        use_on(0, &row, Some(1)),
        Rejection::TargetDead { member: id(1) },
    );
    refused(
        &mut world,
        &data,
        use_on(1, "base:item:potion_of_healing", None),
        Rejection::MemberDead { member: id(1) },
    );
    refused(
        &mut world,
        &data,
        use_on(0, &row, Some(9)),
        Rejection::NoSuchMember { member: id(9) },
    );
    world.party.members[1].conditions.clear();
    world.party.members[0].hp = 0;
    refused(
        &mut world,
        &data,
        use_on(0, &row, None),
        Rejection::MemberDown { member: id(0) },
    );
}

#[test]
fn a_potion_in_a_fight_is_the_turn_and_a_spyglass_is_not_used_there() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let brenna = world.party.members[0].id;
    let potion = item_id(&data, "potion_of_healing").unwrap();
    let glass = item_id(&data, "spyglass").unwrap();
    world.party.members[0].equipment.push((glass, 1));
    world.party.members[0].hp = 1;
    let here = world.position;
    let encounter = encounter(&data, &[("giant_rat", 1)], Disposition::Hostile, here);
    let mut events = Vec::new();
    combat::start(&mut world, &data, encounter, Surprise::None, &mut events).unwrap();
    // Dodge until Brenna acts.
    for _ in 0..20 {
        let Mode::Combat(state) = &world.mode else {
            panic!("the rat won");
        };
        if state.current_actor() == Some(ActorRef::Member(brenna)) {
            break;
        }
        act(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap();
    }
    let glass_row = kit_row(&world, 0, glass);
    // A word counts the acting member's kit as the screen lists it; the command names the kind.
    let at = world.party.members[0]
        .equipment
        .iter()
        .position(|(id, _)| *id == glass)
        .unwrap();
    let durin = world.party.members[1].id;
    assert_eq!(
        Word::parse(&format!("use-item-{at}-m1")).and_then(|w| w.command(&world, &data)),
        Some(Command::Combat(CombatCommand::Use {
            item: "base:item:spyglass".to_owned(),
            receiver: Some(durin),
        }))
    );
    refused(
        &mut world,
        &data,
        Command::Combat(CombatCommand::Use {
            item: glass_row.clone(),
            receiver: None,
        }),
        Rejection::NotUsableHere,
    );
    refused(
        &mut world,
        &data,
        item(ItemCommand::Stow {
            member: brenna,
            item: glass_row,
            count: 1,
        }),
        Rejection::WrongMode,
    );
    let row = kit_row(&world, 0, potion);
    let hp = world.party.members[0].hp;
    let events = act(
        &mut world,
        &data,
        Command::Combat(CombatCommand::Use {
            item: row,
            receiver: None,
        }),
    )
    .unwrap();
    let used = events
        .iter()
        .position(|e| {
            *e == Event::ItemUsed {
                member: brenna,
                item: "base:item:potion_of_healing".to_owned(),
                receiver: Some(brenna),
                consumed: true,
            }
        })
        .expect("used");
    let Event::Healed { rolls, .. } = &events[used + 1] else {
        panic!("{events:?}")
    };
    assert_eq!(
        rolls[0].stream,
        StreamName::new("combat"),
        "the fight's dice"
    );
    assert!(
        world.party.members[0].hp > hp
            || world.party.members[0].hp == world.party.members[0].hp_max
    );
    assert!(
        events[used + 2..]
            .iter()
            .any(|e| matches!(e, Event::Turn { .. } | Event::CombatEnded { .. })),
        "the turn passed: {events:?}"
    );
    assert_eq!(count_of(&world.party.members[0].equipment, potion), 0);
}

#[test]
fn the_item_words_parse_and_print() {
    assert!(parse_script("use-item-0, use-item-2-m1").is_ok());
    assert_eq!(
        common::word("use-item-0"),
        None,
        "a kit row means the acting member's, and nobody acts outside a fight"
    );
    let used = Command::Combat(CombatCommand::Use {
        item: "base:item:potion_of_healing".to_owned(),
        receiver: Some(CharacterId(1)),
    });
    assert_eq!(used.word(), "use-item");
    assert_eq!(
        item(ItemCommand::Unequip {
            member: CharacterId(0),
            slot: EquipSlot::Body
        })
        .word(),
        "item"
    );
    for bad in [
        "item",
        "use-item",
        "use-item-x",
        "use-item-1-2",
        "use-item-1-m",
    ] {
        assert!(parse_script(bad).is_err(), "{bad}");
    }
}
