//! The inventory (parity P5): the words for the kit and its refusals, and the HUD lines for what
//! the party does with its items. Real packs, the fixed party, the town's start.

use crate::common;

use omnis_sim::omnis_data::{Data, EquipSlot};
use omnis_sim::{Command, Event, ItemCommand, Rejection};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;

/// Each command's HUD lines, in order, applied to `world`.
fn lines(world: &mut SimWorld, data: &Data, commands: Vec<ItemCommand>) -> Vec<String> {
    let mut names = Names::default();
    names.observe(&common::views(world, data).party);
    let mut events: Vec<Event> = Vec::new();
    for command in commands {
        events.extend(world.apply(data, Command::Item(command)).expect("accepted"));
    }
    events
        .iter()
        .filter_map(|e| text::describe(e, &names, data))
        .collect()
}

#[test]
fn the_kit_s_events_read_as_the_other_client_s_lines() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let (brenna, durin, pip) = (
        common::id(&world, 0),
        common::id(&world, 1),
        common::id(&world, 3),
    );
    let potion = data
        .registry
        .items
        .get("base:item:potion_of_healing")
        .unwrap();
    world.party.members[1].equipment.push((potion, 1));
    world.party.members[0].hp = 2;
    let item = |id: &str| labels::item(&data, id).to_owned();
    let (sword, mail, bolts) = (
        "base:item:longsword",
        "base:item:chain_mail",
        "base:item:crossbow_bolts",
    );
    let got = lines(
        &mut world,
        &data,
        vec![
            ItemCommand::Unequip {
                member: brenna,
                slot: EquipSlot::MainHand,
            },
            ItemCommand::Equip {
                member: brenna,
                item: sword.into(),
            },
            ItemCommand::Unequip {
                member: brenna,
                slot: EquipSlot::Body,
            },
            ItemCommand::Equip {
                member: brenna,
                item: mail.into(),
            },
            ItemCommand::Stow {
                member: brenna,
                item: bolts.into(),
                count: 1,
            },
            ItemCommand::Take {
                member: pip,
                item: bolts.into(),
                count: 1,
            },
            ItemCommand::Give {
                giver: brenna,
                receiver: pip,
                item: bolts.into(),
                count: 2,
            },
            ItemCommand::Use {
                member: durin,
                item: "base:item:potion_of_healing".into(),
                receiver: Some(brenna),
            },
        ],
    );
    let expected = [
        format!("Brenna takes off {}", item(sword)),
        format!("Brenna wields {}", item(sword)),
        format!("Brenna takes off {}", item(mail)),
        format!("Brenna wears {}", item(mail)),
        format!("Brenna stows {}", item(bolts)),
        format!("Pip takes {}", item(bolts)),
        format!("Brenna gives 2 {} to Pip", item(bolts)),
        format!(
            "Durin uses {} on Brenna",
            item("base:item:potion_of_healing")
        ),
    ];
    assert_eq!(got[..expected.len()], expected);
    let healed = &got[expected.len()];
    assert!(
        healed.starts_with("Brenna regains ") && healed.contains(" hp, now "),
        "{healed}"
    );
}

/// A refusal names the member, where the engine's words give a number.
#[test]
fn a_refusal_reads_in_a_few_words_with_the_member_s_name() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let party = common::views(&world, &data).party;
    let brenna = common::id(&world, 0);
    let reason = |r: Rejection| labels::reason(&party, &r);
    assert_eq!(
        reason(Rejection::MemberDown { member: brenna }),
        "Brenna is down"
    );
    assert_eq!(
        reason(Rejection::TargetDead { member: brenna }),
        "Brenna is dead"
    );
    assert_eq!(reason(Rejection::HandsFull), "no hand free for a shield");
    assert_eq!(
        reason(Rejection::NotEquippable),
        "cannot be worn or wielded"
    );
    assert_eq!(
        reason(Rejection::NotEnoughPoints { need: 2, have: 0 }),
        "need 2 pt"
    );
    assert_eq!(
        reason(Rejection::MissingComponents {
            spell: "base:spell:x".into()
        }),
        "needs components"
    );
    assert_eq!(
        reason(Rejection::SameMember),
        "a member cannot exchange with themselves"
    );
    assert_eq!(labels::slot(EquipSlot::OffHand), "Off hand");
    assert_eq!(labels::count_label("Rations", 5), "Rations x5");
    assert_eq!(labels::count_label("Rations", 1), "Rations");
}
