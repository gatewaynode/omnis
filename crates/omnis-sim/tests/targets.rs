//! The possible targets of an option (owner, 2026-11-14; PRD §7.3, §8.3): the target is chosen
//! after the option, from a list the engine gives, and the list never disagrees with the
//! command. On the road every member is within reach of a touch; a spell of the caster's own
//! lists the caster alone; in a fight a touch reaches the front stacks only.

use crate::common;

use common::{act, data, encounter, party_of, world};
use omnis_data::{Data, Disposition, Range, Targeting};
use omnis_sim::items::item_id;
use omnis_sim::{
    ActorRef, Chosen, CombatCommand, Command, ItemCommand, Mode, Op, Pay, Rejection, Reply, Sight,
    Surprise, Target, TargetView, World, apply, combat, dispatch, targets,
};

/// Durin's slot in the six drafts, and Ilvara's.
const CLERIC: usize = 1;
const WIZARD: usize = 2;

fn spell(name: &str) -> Chosen {
    Chosen::Spell {
        spell: format!("base:spell:{name}"),
        pay: Pay::Action,
    }
}

fn six(data: &Data) -> World {
    let mut world = world(data);
    party_of(&mut world, data, 6);
    world
}

fn fight(world: &mut World, data: &Data, stacks: &[(&str, u8)], slot: usize) {
    let here = world.position;
    let encounter = encounter(data, stacks, Disposition::Hostile, here);
    combat::start(world, data, encounter, Surprise::None, &mut Vec::new()).unwrap();
    let id = world.party.members[slot].id;
    for _ in 0..200 {
        let Mode::Combat(state) = &world.mode else {
            panic!("the fight ended before the turn came");
        };
        if state.current_actor() == Some(ActorRef::Member(id)) {
            return;
        }
        act(world, data, Command::Combat(CombatCommand::Dodge)).unwrap();
    }
    panic!("the turn never came");
}

/// The command a client sends for `chosen` on `target`, as the mode takes it.
fn command(world: &World, member: usize, chosen: &Chosen, target: Target) -> Command {
    let id = world.party.members[member].id;
    let fighting = matches!(world.mode, Mode::Combat(_));
    match (chosen, target) {
        (Chosen::Spell { spell, pay }, target) if fighting => {
            Command::Combat(CombatCommand::Cast {
                spell: spell.clone(),
                target,
                pay: *pay,
            })
        }
        (Chosen::Spell { spell, .. }, target) => Command::Cast {
            caster: id,
            spell: spell.clone(),
            target,
        },
        (Chosen::Item { item }, Target::Member(receiver)) if fighting => {
            Command::Combat(CombatCommand::Use {
                item: item.clone(),
                receiver: Some(receiver),
            })
        }
        (Chosen::Item { item }, Target::Member(receiver)) => Command::Item(ItemCommand::Use {
            member: id,
            item: item.clone(),
            receiver: Some(receiver),
        }),
        (Chosen::Attack, Target::Stack(stack)) => Command::Combat(CombatCommand::Attack { stack }),
        (Chosen::Exchange, Target::Member(with)) => {
            Command::Combat(CombatCommand::Exchange { with })
        }
        other => panic!("no command for {other:?}"),
    }
}

/// The list for `chosen`, each row checked against the command itself on a copy of the world:
/// a row with no refusal is accepted, a row with one is rejected with exactly it and changes
/// nothing.
fn listed(world: &World, data: &Data, member: usize, chosen: &Chosen) -> Vec<TargetView> {
    let id = world.party.members[member].id;
    let rows = targets(world, data, id, chosen).expect("a list");
    for row in &rows {
        let mut copy = world.clone();
        let result = apply(
            &mut copy,
            data,
            command(world, member, chosen, row.target.clone()),
        );
        assert_eq!(
            result.as_ref().err(),
            row.refusal.as_ref(),
            "{chosen:?} {row:?}"
        );
        if row.refusal.is_some() {
            assert_eq!(&copy, world, "a refusal changes nothing: {row:?}");
        }
        assert_eq!((row.distance, row.sight), (0, Sight::Clear));
    }
    rows
}

fn free(rows: &[TargetView]) -> Vec<Target> {
    let open = rows.iter().filter(|r| r.refusal.is_none());
    open.map(|r| r.target.clone()).collect()
}

/// The reported bug: on the road a heal could go to no one but its caster. The list is every
/// member, the caster among them, and a dead member is listed with the reason.
#[test]
fn on_the_road_a_touch_heal_lists_every_member() {
    let data = data();
    let mut world = six(&data);
    let everyone: Vec<Target> = world.party.ids().into_iter().map(Target::Member).collect();
    assert_eq!(everyone.len(), 6);
    for name in ["cure_wounds", "bless"] {
        let rows = listed(&world, &data, CLERIC, &spell(name));
        assert_eq!(free(&rows), everyone, "{name}");
    }
    // Brenna dies: still listed, refused as the command refuses her.
    let dead = data.registry.conditions.get("base:condition:dead").unwrap();
    let brenna = world.party.members[0].id;
    world.party.members[0].hp = 0;
    world.party.members[0].conditions.push(dead);
    let rows = listed(&world, &data, CLERIC, &spell("cure_wounds"));
    assert_eq!(rows.len(), 6);
    assert_eq!(
        rows[0].refusal,
        Some(Rejection::MemberDead { member: brenna })
    );
    assert_eq!(free(&rows), everyone[1..]);
}

/// "Self only shows self as the available option": a spell of the caster's own is one row.
#[test]
fn a_spell_of_the_caster_s_own_lists_the_caster_alone() {
    let data = data();
    let world = six(&data);
    for (slot, name) in [(WIZARD, "light"), (WIZARD, "detect_magic")] {
        let own = Target::Member(world.party.members[slot].id);
        let rows = listed(&world, &data, slot, &spell(name));
        assert_eq!(free(&rows), [own], "{name}");
        assert_eq!(rows.len(), 1, "{name}");
    }
}

/// A heal whose data says `Own` reaches its caster and nobody else, on the road and in a
/// fight, by the list and by the command.
#[test]
fn a_heal_written_as_the_caster_s_own_refuses_everyone_else() {
    let mut data = data();
    let cure = data.registry.spells.get("base:spell:cure_wounds").unwrap();
    data.spells.get_mut(&cure).unwrap().targeting = Some(Targeting::OWN);
    let mut world = six(&data);
    let durin = world.party.members[CLERIC].id;
    let brenna = world.party.members[0].id;
    for fighting in [false, true] {
        if fighting {
            fight(&mut world, &data, &[("giant_rat", 2)], CLERIC);
        }
        let rows = listed(&world, &data, CLERIC, &spell("cure_wounds"));
        assert_eq!(free(&rows), [Target::Member(durin)], "fighting: {fighting}");
        assert_eq!(rows.len(), 1);
        let other = command(
            &world,
            CLERIC,
            &spell("cure_wounds"),
            Target::Member(brenna),
        );
        let before = world.clone();
        assert_eq!(apply(&mut world, &data, other), Err(Rejection::BeyondRange));
        assert_eq!(world, before, "nothing is paid for a target out of range");
    }
}

/// In a fight a touch reaches the stacks in front; a spell with a range in tiles reaches every
/// stack, since a fight has no distances.
#[test]
fn in_a_fight_a_touch_reaches_the_front_stacks_and_a_range_every_stack() {
    let data = data();
    let mut world = six(&data);
    fight(
        &mut world,
        &data,
        &[("giant_rat", 2), ("giant_rat", 2), ("giant_rat", 2)],
        WIZARD,
    );
    let all = [Target::Stack(0), Target::Stack(1), Target::Stack(2)];
    let touch = listed(&world, &data, WIZARD, &spell("burning_hands"));
    assert_eq!(
        touch.iter().map(|r| r.target.clone()).collect::<Vec<_>>(),
        all
    );
    assert_eq!(free(&touch), all[..2], "two stacks stand in front");
    assert_eq!(touch[2].refusal, Some(Rejection::OutOfReach { stack: 2 }));
    for ranged in ["magic_missile", "fire_bolt"] {
        assert_eq!(
            free(&listed(&world, &data, WIZARD, &spell(ranged))),
            all,
            "{ranged}"
        );
    }
    // A spell of her own lists her, never a stack.
    let own = listed(&world, &data, WIZARD, &spell("light"));
    assert_eq!(free(&own), [Target::Member(world.party.members[WIZARD].id)]);
}

/// An attack lists every standing stack with the weapon's own refusal, and an exchange every
/// other member.
#[test]
fn an_attack_lists_the_standing_stacks_and_an_exchange_the_others() {
    let data = data();
    let mut world = six(&data);
    fight(
        &mut world,
        &data,
        &[("giant_rat", 1), ("giant_rat", 1), ("giant_rat", 1)],
        0,
    );
    let attack = listed(&world, &data, 0, &Chosen::Attack);
    assert_eq!(attack.len(), 3);
    assert!(attack[0].refusal.is_none() && attack[1].refusal.is_none());
    let others = listed(&world, &data, 0, &Chosen::Exchange);
    let ids = world.party.ids();
    assert_eq!(others.len(), 5, "the member acting is not offered themself");
    assert_eq!(
        free(&others),
        ids[1..]
            .iter()
            .copied()
            .map(Target::Member)
            .collect::<Vec<_>>()
    );
    // Not the acting member's list to ask for.
    assert_eq!(
        targets(&world, &data, ids[1], &Chosen::Attack),
        Err(Rejection::NotYourTurn)
    );
    // Once the action is spent every row says so, as the command would.
    let Mode::Combat(state) = &mut world.mode else {
        panic!("the fight goes on");
    };
    state.budget.actions = 0;
    let spent = listed(&world, &data, 0, &Chosen::Attack);
    assert!(!spent.is_empty());
    for row in &spent {
        assert_eq!(row.refusal, Some(Rejection::NoActionLeft), "{row:?}");
    }
}

/// A potion lists every member on the road and in a fight; an item whose data says `Own`
/// lists its user alone and the command refuses anyone else.
#[test]
fn an_item_s_use_lists_who_its_targeting_reaches() {
    let mut data = data();
    let potion = item_id(&data, "potion_of_healing").unwrap();
    let mut world = six(&data);
    world.party.members[0].equipment.push((potion, 3));
    let chosen = Chosen::Item {
        item: "base:item:potion_of_healing".to_owned(),
    };
    let ids = world.party.ids();
    let everyone: Vec<Target> = ids.iter().copied().map(Target::Member).collect();
    assert_eq!(free(&listed(&world, &data, 0, &chosen)), everyone);
    // An item the member does not carry has nothing to aim.
    let absent = Chosen::Item {
        item: "base:item:spyglass".to_owned(),
    };
    assert!(targets(&world, &data, ids[0], &absent).unwrap().is_empty());

    data.items.get_mut(&potion).unwrap().targeting = Some(Targeting::of(Range::Own));
    let own = listed(&world, &data, 0, &chosen);
    assert_eq!(free(&own), everyone[..1]);
    assert_eq!(
        own.len(),
        1,
        "the user alone is listed, not the others refused"
    );
    let before = world.clone();
    let other = command(&world, 0, &chosen, everyone[1].clone());
    assert_eq!(apply(&mut world, &data, other), Err(Rejection::BeyondRange));
    assert_eq!(world, before, "the potion is not spent");
}

/// The op is the same list.
#[test]
fn the_op_answers_with_the_same_list() {
    let data = data();
    let mut world = six(&data);
    let member = world.party.members[CLERIC].id;
    let option = spell("cure_wounds");
    let expected = targets(&world, &data, member, &option).unwrap();
    let op = Op::TargetsGet {
        member,
        option: option.clone(),
    };
    assert_eq!(
        dispatch(&mut world, &data, &op),
        Ok(Reply::Targets { targets: expected })
    );
}
