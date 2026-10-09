//! The rest view (M7 step 8b): what the camp panel shows before the player asks. For every
//! state a party can be in, each answer the view gives is the answer the command gives: a
//! refusal it names is the refusal sent back, a count it allows is accepted and one more is
//! not, and reading it changes nothing.

mod common;

use common::{data, encounter, party_of};
use omnis_core::{Facing, Position};
use omnis_data::{Data, Disposition};
use omnis_sim::rest::HitDiceSpend;
use omnis_sim::{
    Command, Mode, Rejection, RestCommand, RestView, Settings, World, apply, rest_view,
};

fn at(data: &Data, members: usize, name: &str, x: u16, y: u16) -> World {
    let mut world = World::new(data, 5, Settings::default()).unwrap();
    party_of(&mut world, data, members);
    world.position = Position {
        map: data.registry.maps.get(&format!("test:map:{name}")).unwrap(),
        x,
        y,
        facing: Facing::North,
    };
    world
}

fn kill(world: &mut World, data: &Data, index: usize) {
    let dead = omnis_rules::condition_id(data, "dead").unwrap();
    let member = &mut world.party.members[index];
    member.hp = 0;
    member.conditions.push(dead);
}

/// The command's answer on a copy of the world.
fn answer(world: &World, data: &Data, command: RestCommand) -> Result<(), Rejection> {
    apply(&mut world.clone(), data, Command::Rest(command)).map(|_| ())
}

/// A short rest spending `count` of the dice of the member in slot `index`.
fn dice(world: &World, index: usize, count: u8) -> RestCommand {
    RestCommand::Short {
        dice: vec![HitDiceSpend {
            member: world.party.members[index].id,
            count,
        }],
    }
}

/// The view, checked against the command for the long rest and for every member's dice; and
/// reading it changes nothing.
fn agreed(world: &World, data: &Data) -> RestView {
    let before = world.clone();
    let view = rest_view(world, data);
    assert_eq!(*world, before, "reading the view changed the world");
    assert_eq!(
        answer(world, data, RestCommand::Long).err(),
        view.long,
        "the long rest"
    );
    assert_eq!(view.food, world.party.food);
    let members = world.party.members.len();
    assert_eq!(view.members.len(), members);
    if let Some(why) = &view.refusal {
        assert_eq!(
            answer(world, data, RestCommand::Short { dice: Vec::new() }),
            Err(why.clone())
        );
        return view;
    }
    for (index, member) in view.members.iter().enumerate() {
        assert!(member.spendable <= member.dice_left && member.dice_left <= member.dice);
        assert!([6, 8, 10, 12].contains(&member.die), "{member:?}");
        if member.spendable > 0 {
            assert_eq!(
                answer(world, data, dice(world, index, member.spendable)),
                Ok(()),
                "member {index} spends {}",
                member.spendable
            );
        }
        assert!(
            answer(world, data, dice(world, index, member.spendable + 1)).is_err(),
            "member {index} cannot spend {}",
            member.spendable + 1
        );
    }
    view
}

#[test]
fn the_view_agrees_with_the_command_in_every_state() {
    let data = data();
    let mut world = at(&data, 3, "meadow", 16, 16);
    let fresh = agreed(&world, &data);
    assert_eq!(fresh.refusal, None);
    assert_eq!(fresh.long, None, "a first night with food is allowed");
    assert_eq!(fresh.long_food, 3, "one each");
    assert!(
        fresh.members.iter().all(|m| m.spendable == 0),
        "nobody is hurt"
    );

    world.party.members[0].hp = 1;
    world.party.members[1].hp = 1;
    world.party.members[1].hit_dice_spent = world.party.members[1].level;
    kill(&mut world, &data, 2);
    let hurt = agreed(&world, &data);
    assert_eq!(hurt.members[0].spendable, hurt.members[0].dice_left);
    assert!(hurt.members[0].spendable > 0);
    assert_eq!(hurt.members[1].spendable, 0, "no dice left");
    assert_eq!(hurt.members[2].spendable, 0, "the dead spend none");
    assert_eq!(hurt.long_food, 2, "the dead do not eat");

    world.party.food = 1;
    assert_eq!(
        agreed(&world, &data).long,
        Some(Rejection::NoFood { need: 2, have: 1 })
    );

    world.party.food = 10;
    apply(&mut world, &data, Command::Rest(RestCommand::Long)).unwrap();
    assert_eq!(
        agreed(&world, &data).long,
        Some(Rejection::RestTooSoon { minutes: 960 })
    );
}

#[test]
fn no_rest_inside_a_service_or_a_fight() {
    let data = data();
    let mut world = at(&data, 2, "town", 1, 1);
    apply(&mut world, &data, Command::Interact).unwrap();
    assert!(matches!(world.mode, Mode::Town(_)));
    let inside = agreed(&world, &data);
    assert_eq!(inside.refusal, Some(Rejection::WrongMode));
    assert_eq!(inside.long, Some(Rejection::WrongMode));

    let mut world = at(&data, 2, "meadow", 16, 16);
    let retreat = world.position;
    world.mode = Mode::Encounter(encounter(
        &data,
        &[("giant_rat", 1)],
        Disposition::Hostile,
        retreat,
    ));
    assert_eq!(agreed(&world, &data).refusal, Some(Rejection::WrongMode));
}
