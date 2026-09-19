//! The party: creation from base data through commands, the slot limit, marching order,
//! rejections that leave the world untouched, saves and replays that carry members.

mod common;

use common::{data, step, world};
use omnis_core::Direction;
use omnis_data::{Alignment, EquipSlot, Skill};
use omnis_rules::{CreationError, Draft};
use omnis_sim::{
    Command, Event, Op, OpError, PartyCommand, Rejection, Replay, Reply, Settings, World, apply,
    dispatch, query,
};

fn draft(name: &str, race: &str, class: &str, scores: [u8; 6], skills: &[Skill]) -> Draft {
    Draft {
        name: name.to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::LawfulGood,
        scores,
        skills: skills.to_vec(),
    }
}

fn six() -> Vec<Draft> {
    let two = [Skill::Athletics, Skill::Perception];
    vec![
        draft("Brenna", "human", "fighter", [15, 14, 13, 12, 10, 8], &two),
        draft(
            "Durin",
            "dwarf",
            "cleric",
            [10, 8, 14, 10, 15, 8],
            &[Skill::Medicine, Skill::History],
        ),
        draft(
            "Ilvara",
            "elf",
            "wizard",
            [8, 14, 13, 15, 12, 10],
            &[Skill::Arcana, Skill::History],
        ),
        draft(
            "Pip",
            "halfling",
            "rogue",
            [8, 15, 12, 10, 13, 14],
            &[
                Skill::Stealth,
                Skill::Acrobatics,
                Skill::Deception,
                Skill::Perception,
            ],
        ),
        draft("Gorm", "dwarf", "fighter", [15, 10, 14, 8, 13, 8], &two),
        draft(
            "Wren",
            "human",
            "cleric",
            [10, 10, 12, 8, 15, 13],
            &[Skill::Medicine, Skill::Persuasion],
        ),
    ]
}

fn create(
    world: &mut World,
    data: &omnis_data::Data,
    draft: Draft,
) -> Result<Vec<Event>, Rejection> {
    apply(world, data, Command::Party(PartyCommand::Create(draft)))
}

#[test]
fn six_members_join_and_the_seventh_is_refused() {
    let data = data();
    let mut world = world(&data);
    for (i, d) in six().into_iter().enumerate() {
        let events = create(&mut world, &data, d).unwrap_or_else(|e| panic!("{e}"));
        assert!(events.contains(&Event::PartyChanged));
        assert!(
            events.iter().any(|e| matches!(e, Event::Visible { .. })),
            "every command ends with a look"
        );
        assert_eq!(world.party.members.len(), i + 1);
        assert_eq!(world.party.members[i].id.0, i as u32);
    }
    assert_eq!(world.turn, 6, "creation takes a turn but no time");
    assert_eq!(world.party_clock().elapsed, 0);
    assert_eq!(world.party.gold, 6 * 15, "the acolyte's purse, six times");
    assert_eq!(world.party.food, 60);
    assert_eq!(world.party.members[0].hp_max, 12);
    assert_eq!(world.party.members[2].spell_points_max, 4);
    let before = world.clone();
    let seventh = create(&mut world, &data, six().remove(0)).unwrap_err();
    assert_eq!(seventh, Rejection::PartyFull);
    assert_eq!(
        world, before,
        "a rejection changes nothing, not even the turn"
    );
    assert_eq!(
        query::path(&world, "party.members.0.hp").as_deref(),
        Some("12")
    );
    assert_eq!(
        query::path(&world, "party.members.1.name").as_deref(),
        Some("Durin")
    );
    assert_eq!(query::path(&world, "party.gold").as_deref(), Some("90"));
}

#[test]
fn bad_drafts_are_rejected_without_touching_the_world() {
    let data = data();
    let mut world = world(&data);
    let before = world.clone();
    let over = draft(
        "Over",
        "human",
        "fighter",
        [15, 15, 15, 9, 8, 8],
        &[Skill::Athletics, Skill::Perception],
    );
    assert_eq!(
        create(&mut world, &data, over).unwrap_err(),
        Rejection::Character(CreationError::Points {
            spent: 28,
            budget: 27
        })
    );
    let mage = draft("Mage", "human", "mage", [15, 14, 13, 12, 10, 8], &[]);
    assert!(matches!(
        create(&mut world, &data, mage).unwrap_err(),
        Rejection::Character(CreationError::UnknownClass(_))
    ));
    assert_eq!(world, before);
    assert!(
        world.rngs.is_empty(),
        "nothing was drawn for a refused draft"
    );
}

#[test]
fn the_marching_order_is_a_permutation() {
    let data = data();
    let mut world = world(&data);
    for d in six().into_iter().take(3) {
        create(&mut world, &data, d).unwrap();
    }
    let reorder = |world: &mut World, order: Vec<u8>| {
        apply(
            world,
            &data,
            Command::Party(PartyCommand::Reorder { order }),
        )
    };
    reorder(&mut world, vec![2, 0, 1]).unwrap();
    let names: Vec<&str> = world
        .party
        .members
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(names, ["Ilvara", "Brenna", "Durin"]);
    assert_eq!(
        world.party.members[1].id.0, 0,
        "identities travel with the members"
    );
    let before = world.clone();
    for bad in [vec![0, 1], vec![0, 1, 1], vec![0, 1, 3], vec![0, 1, 2, 2]] {
        assert_eq!(reorder(&mut world, bad).unwrap_err(), Rejection::BadOrder);
    }
    assert_eq!(world, before);
    assert_eq!(omnis_sim::party::slots(&data), 6);
    assert_eq!(omnis_sim::party::front_row(&data), 3);
}

#[test]
fn a_party_survives_a_save_and_a_replay() {
    let data = data();
    let mut world = world(&data);
    let mut commands = Vec::new();
    for d in six().into_iter().take(2) {
        commands.push(Command::Party(PartyCommand::Create(d)));
    }
    commands.push(Command::Party(PartyCommand::Reorder { order: vec![1, 0] }));
    commands.push(Command::Step(Direction::Forward));
    for command in &commands {
        apply(&mut world, &data, command.clone()).unwrap();
    }
    let text = world.to_ron().unwrap();
    assert!(text.contains("Brenna"));
    let loaded = World::from_ron(&text, &data, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(loaded.party, world.party);
    assert_eq!(loaded.fingerprint().unwrap(), world.fingerprint().unwrap());

    let seed = world.seed;
    let replay = Replay::record(&data, seed, Settings::default(), commands).unwrap();
    assert_eq!(replay.fingerprint, world.fingerprint().unwrap());
    assert_eq!(replay.check(&data), Ok(()));
    let mut again = world.clone();
    step(&mut again, &data);
    assert_ne!(again.fingerprint().unwrap(), replay.fingerprint);
}

#[test]
fn party_get_lists_the_kit_as_rows_with_the_worn_slots_and_the_stores() {
    let data = data();
    let mut world = world(&data);
    dispatch(
        &mut world,
        &data,
        &Op::PartyCreate {
            character: six().remove(0),
        },
    )
    .unwrap();
    let potion = omnis_sim::items::item_id(&data, "potion_of_healing").unwrap();
    world.party.inventory.push((potion, 2));
    let Reply::Party { party } = dispatch(&mut world, &data, &Op::PartyGet).unwrap() else {
        panic!("party.get answers with the party");
    };
    let brenna = &party.members[0];
    let ids: Vec<&str> = brenna.equipment.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "base:item:chain_mail",
            "base:item:longsword",
            "base:item:shield",
            "base:item:light_crossbow",
            "base:item:crossbow_bolts",
            "base:item:holy_symbol",
            "base:item:potion_of_healing"
        ]
    );
    let rows: Vec<u8> = brenna.equipment.iter().map(|i| i.index).collect();
    assert_eq!(rows, [0, 1, 2, 3, 4, 5, 6]);
    let bolts = &brenna.equipment[4];
    assert_eq!(
        (bolts.count, bolts.slot, bolts.usable, bolts.equipped),
        (20, None, false, false)
    );
    let sword = &brenna.equipment[1];
    assert_eq!(
        (sword.slot, sword.equipped, sword.name.as_str()),
        (
            Some(EquipSlot::MainHand),
            true,
            "base:text:item.longsword.name"
        )
    );
    let flask = &brenna.equipment[6];
    assert!(flask.usable && flask.consumable && !flask.equipped);
    assert_eq!(
        brenna.equipped,
        [
            (EquipSlot::MainHand, "base:item:longsword".to_owned()),
            (EquipSlot::OffHand, "base:item:shield".to_owned()),
            (EquipSlot::Ranged, "base:item:light_crossbow".to_owned()),
            (EquipSlot::Body, "base:item:chain_mail".to_owned()),
        ]
    );
    assert!(brenna.effects.is_empty() && party.effects.is_empty());
    assert_eq!(party.inventory.len(), 1);
    assert_eq!((party.inventory[0].index, party.inventory[0].count), (0, 2));
    let Reply::Events { events } = dispatch(
        &mut world,
        &data,
        &Op::SimCommand {
            command: Command::Item(omnis_sim::ItemCommand::Take {
                member: 0,
                item: 0,
                count: 2,
            }),
        },
    )
    .unwrap() else {
        panic!("sim.command answers with events");
    };
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ItemMoved { count: 2, .. }))
    );
    let Reply::Party { party } = dispatch(&mut world, &data, &Op::PartyGet).unwrap() else {
        panic!("party.get answers with the party");
    };
    assert!(party.inventory.is_empty());
    assert_eq!(party.members[0].equipment[6].count, 3);
}

#[test]
fn the_ops_expose_the_party_and_the_rules() {
    let data = data();
    let mut world = world(&data);
    let reply = dispatch(
        &mut world,
        &data,
        &Op::PartyCreate {
            character: six().remove(0),
        },
    )
    .unwrap();
    assert!(matches!(&reply, Reply::Events { events } if events.contains(&Event::PartyChanged)));
    let Reply::Party { party } = dispatch(&mut world, &data, &Op::PartyGet).unwrap() else {
        panic!("party.get answers with the party");
    };
    assert_eq!(
        (party.slots, party.front_row, party.gold, party.food),
        (6, 3, 15, 10)
    );
    let member = &party.members[0];
    assert_eq!(
        (member.index, member.name.as_str(), member.race.as_str()),
        (0, "Brenna", "base:race:human")
    );
    assert_eq!(
        (
            member.class.as_str(),
            member.level,
            member.hp_max,
            member.ac
        ),
        ("base:class:fighter", 1, 12, 18)
    );
    assert!(member.front && member.conditions.is_empty());
    let text = omnis_data::ron_io::to_string(&Reply::Party {
        party: party.clone(),
    })
    .unwrap();
    assert_eq!(
        omnis_data::ron_io::parse::<Reply>(&text).unwrap(),
        Reply::Party { party }
    );

    let Reply::Rules { rules } = dispatch(&mut world, &data, &Op::RulesList).unwrap() else {
        panic!("rules.list answers with the rules");
    };
    assert!(rules.slots.iter().any(|s| s.name == "spell_points.pool"));
    assert_eq!(rules.values["point_budget"], 27);
    assert_eq!(rules.tables["point_cost"].len(), 8);
    let get =
        |world: &mut World, slot: &str| dispatch(world, &data, &Op::RulesGet { slot: slot.into() });
    assert!(
        matches!(get(&mut world, "spell_points.pool").unwrap(), Reply::Rule { rule } if rule.inputs.len() == 4 && rule.source.starts_with("max("))
    );
    assert_eq!(
        get(&mut world, "nope").unwrap_err(),
        OpError::UnknownSlot {
            slot: "nope".into()
        }
    );
    assert_eq!(
        dispatch(
            &mut world,
            &data,
            &Op::RulesSet {
                slot: "spell_points.pool".into(),
                source: "1".into()
            }
        )
        .unwrap_err(),
        OpError::HostOnly
    );
    let bad = Op::PartyCreate {
        character: draft("", "human", "fighter", [8; 6], &[]),
    };
    assert!(matches!(
        dispatch(&mut world, &data, &bad).unwrap_err(),
        OpError::Rejected {
            rejection: Rejection::Character(CreationError::Name)
        }
    ));
}
