//! Declared reactions (M7c step 4, PRD D22, §7.9): the tactics commands check everything and
//! change nothing when refused; a declared reaction fires at its trigger for the member or the
//! row it concerns, once a round, and not at all with the switch off. Party: Brenna (fighter),
//! Durin (cleric), Ilvara (wizard) in front, Pip (rogue) behind.

mod common;

use common::{act, data, encounter, party_of};
use omnis_core::{CharacterId, MonsterId};
use omnis_data::{Data, Disposition};
use omnis_sim::omnis_rules::tactics::{LIBRARY_SETS, RUNBOOK_ENTRIES};
use omnis_sim::omnis_rules::{
    ActionRef, Cmp, Criteria, CriteriaSet, Predicate, TacticsFault, Trigger, Who,
};
use omnis_sim::tactics::TacticsCommand;
use omnis_sim::{
    ActorRef, CombatCommand, Command, Event, Mode, PartyCommand, Rejection, Settings, Surprise,
    World, apply, combat,
};

const DURIN: u8 = 1;
const ILVARA: u8 = 2;

fn spell(data: &Data, name: &str) -> omnis_core::SpellId {
    data.registry
        .spells
        .get(name)
        .unwrap_or_else(|| panic!("{name}"))
}

fn set(action: ActionRef, trigger: Trigger, when: Criteria) -> CriteriaSet {
    CriteriaSet {
        name: "Answer".to_owned(),
        action,
        trigger,
        when,
    }
}

fn put(member: u8, at: Option<u8>, set: CriteriaSet) -> Command {
    Command::Party(PartyCommand::Tactics(TacticsCommand::PutReaction {
        member,
        at,
        set,
    }))
}

fn party(data: &Data, seed: u64) -> World {
    let mut world = common::new_world(data, seed, Settings::default());
    party_of(&mut world, data, 4);
    let ward = spell(data, "test:spell:ward");
    world.party.members[usize::from(DURIN)]
        .known_spells
        .push(ward);
    world
}

fn refused(world: &mut World, data: &Data, command: Command, why: Rejection) {
    let before = world.clone();
    assert_eq!(apply(world, data, command.clone()), Err(why), "{command:?}");
    assert_eq!(*world, before, "{command:?} changed the world");
}

/// Dodge whole turns until the fight ends, every event kept.
fn fight_out(world: &mut World, data: &Data, stacks: &[(&str, u8)]) -> Vec<Event> {
    let here = world.position;
    let foes = encounter(data, stacks, Disposition::Hostile, here);
    let mut events = Vec::new();
    combat::start(world, data, foes, Surprise::None, &mut events).unwrap();
    for _ in 0..400 {
        if !matches!(world.mode, Mode::Combat(_)) {
            return events;
        }
        events.extend(act(world, data, Command::Combat(CombatCommand::Dodge)).unwrap());
    }
    panic!("the fight did not end");
}

/// Reactions by `who` between two of their own turns: at most one each time.
fn once_a_round(events: &[Event], who: CharacterId) -> usize {
    let mut fired = 0;
    let mut since_turn = 0;
    for event in events {
        match event {
            Event::Turn { actor } if *actor == ActorRef::Member(who) => since_turn = 0,
            Event::Reaction { actor, .. } if *actor == who => {
                since_turn += 1;
                fired += 1;
                assert!(since_turn <= 1, "two reactions before {who:?}'s turn");
            }
            _ => {}
        }
    }
    fired
}

#[test]
fn the_tactics_commands_check_everything_and_change_nothing_when_refused() {
    let data = data();
    let mut world = party(&data, 1);
    let shield = ActionRef::Spell(spell(&data, "base:spell:shield"));
    let bolt = ActionRef::Spell(spell(&data, "base:spell:fire_bolt"));
    let ok = set(shield.clone(), Trigger::Attacked, Criteria::Always);
    refused(
        &mut world,
        &data,
        put(9, None, ok.clone()),
        Rejection::NoSuchMember { index: 9 },
    );
    for (action, trigger) in [
        (bolt, Trigger::Attacked),
        (shield.clone(), Trigger::MemberWounded),
        (ActionRef::Attack, Trigger::Attacked),
        (ActionRef::Item(omnis_core::ItemId(0)), Trigger::Attacked),
        (shield.clone(), Trigger::OwnTurn),
    ] {
        refused(
            &mut world,
            &data,
            put(ILVARA, None, set(action, trigger, Criteria::Always)),
            Rejection::CannotReact,
        );
    }
    refused(
        &mut world,
        &data,
        put(DURIN, None, ok.clone()),
        Rejection::CannotReact,
    );
    let mut nameless = ok.clone();
    nameless.name = String::new();
    refused(
        &mut world,
        &data,
        put(ILVARA, None, nameless),
        Rejection::Tactics(TacticsFault::Name),
    );
    let unknown = Criteria::Is(Predicate::MonsterCount {
        monster: MonsterId(999),
        cmp: Cmp::Ge,
        n: 1,
    });
    refused(
        &mut world,
        &data,
        put(
            ILVARA,
            None,
            set(shield.clone(), Trigger::Attacked, unknown),
        ),
        Rejection::UnknownId {
            id: "monster 999".to_owned(),
        },
    );
    refused(
        &mut world,
        &data,
        put(ILVARA, Some(0), ok.clone()),
        Rejection::NoSuchEntry { at: 0 },
    );

    let events = apply(&mut world, &data, put(ILVARA, None, ok.clone())).unwrap();
    assert!(matches!(events[0], Event::TacticsChanged { .. }));
    apply(&mut world, &data, put(ILVARA, None, ok.clone())).unwrap();
    let tactics = &world.party.members[usize::from(ILVARA)].tactics;
    assert_eq!(
        (tactics.library.len(), tactics.runbooks[0].entries.len()),
        (1, 2),
        "the same set is kept once"
    );
    apply(
        &mut world,
        &data,
        put(
            ILVARA,
            None,
            set(ActionRef::Attack, Trigger::EnemyFlees, Criteria::Always),
        ),
    )
    .unwrap();
    refused(
        &mut world,
        &data,
        Command::Party(PartyCommand::Tactics(TacticsCommand::RemoveReaction {
            member: ILVARA,
            at: 3,
        })),
        Rejection::NoSuchEntry { at: 3 },
    );
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Tactics(TacticsCommand::RemoveReaction {
            member: ILVARA,
            at: 1,
        })),
    )
    .unwrap();
    let tactics = &world.party.members[usize::from(ILVARA)].tactics;
    assert_eq!(tactics.runbooks[0].entries.len(), 2);
    assert_eq!(tactics.library.len(), 2, "the library keeps every set");

    // The caps: entries in a runbook, then sets in the library.
    for _ in 2..RUNBOOK_ENTRIES {
        apply(&mut world, &data, put(ILVARA, None, ok.clone())).unwrap();
    }
    refused(
        &mut world,
        &data,
        put(ILVARA, None, ok.clone()),
        Rejection::Tactics(TacticsFault::TooMany),
    );
    for round in 2..LIBRARY_SETS {
        let mut new = ok.clone();
        new.name = format!("Set {round}");
        apply(&mut world, &data, put(ILVARA, Some(0), new)).unwrap();
    }
    let mut one_more = ok;
    one_more.name = "One more".to_owned();
    refused(
        &mut world,
        &data,
        put(ILVARA, Some(0), one_more),
        Rejection::Tactics(TacticsFault::TooMany),
    );
}

#[test]
fn shield_declared_for_every_hit_fires_on_hits_once_a_round() {
    let data = data();
    let shield = spell(&data, "base:spell:shield");
    let mut fired = 0;
    for seed in 0..30u64 {
        let mut world = party(&data, seed);
        let ilvara = world.party.members[usize::from(ILVARA)].id;
        world.party.members[usize::from(ILVARA)].hp_max = 200;
        world.party.members[usize::from(ILVARA)].hp = 200;
        world.party.members[usize::from(ILVARA)].spell_points = 50;
        apply(
            &mut world,
            &data,
            put(
                ILVARA,
                None,
                set(
                    ActionRef::Spell(shield),
                    Trigger::Attacked,
                    Criteria::Always,
                ),
            ),
        )
        .unwrap();
        let events = fight_out(&mut world, &data, &[("goblin", 3), ("goblin", 3)]);
        fired += once_a_round(&events, ilvara);
        for (i, event) in events.iter().enumerate() {
            if let Event::Reaction { actor, trigger, .. } = event
                && *actor == ilvara
            {
                assert_eq!(*trigger, Trigger::Attacked);
                assert!(
                    matches!(events[i + 1], Event::SpellCast { caster, spell, .. } if caster == ilvara && spell == shield),
                    "the cast follows the reaction"
                );
                // Every hit, and only hits: the judged roll would have reached the armor class
                // without the shield's +5.
                let judged = events[i..]
                    .iter()
                    .find_map(|e| match e {
                        Event::AttackResolved { roll, ac, .. } => {
                            Some((roll.total, *ac, roll.trace.rolls[0].value))
                        }
                        _ => None,
                    })
                    .unwrap();
                assert!(
                    judged.0 >= judged.1 - 5 || judged.2 == 20,
                    "{judged:?} was a miss before the shield"
                );
            }
        }
        if fired >= 3 {
            return;
        }
    }
    panic!("shield fired {fired} times in 30 fights");
}

#[test]
fn a_heal_declared_for_a_wounded_ally_answers_its_row_only() {
    let data = data();
    let ward = spell(&data, "test:spell:ward");
    let (mut fired, mut back_wounds) = (0, 0);
    for seed in 0..40u64 {
        let mut world = party(&data, seed);
        let durin = world.party.members[usize::from(DURIN)].id;
        let front: Vec<CharacterId> = world.party.members[..3].iter().map(|m| m.id).collect();
        let pip = world.party.members[3].id;
        world.party.members[usize::from(DURIN)].spell_points = 50;
        let when = Criteria::Is(Predicate::Hp {
            who: Who::Subject,
            cmp: Cmp::Lt,
            percent: 100,
        });
        apply(
            &mut world,
            &data,
            put(
                DURIN,
                None,
                set(ActionRef::Spell(ward), Trigger::MemberWounded, when),
            ),
        )
        .unwrap();
        // A third stack stands behind and shoots at anyone, Pip in the back row too.
        let events = fight_out(
            &mut world,
            &data,
            &[("goblin", 2), ("goblin", 2), ("goblin", 2)],
        );
        fired += once_a_round(&events, durin);
        let mut last_hurt = None;
        for (i, event) in events.iter().enumerate() {
            match event {
                Event::Damage {
                    target: ActorRef::Member(id),
                    amount,
                    ..
                } if *amount > 0 => {
                    last_hurt = Some(*id);
                    back_wounds += usize::from(*id == pip);
                }
                Event::Reaction { actor, .. } if *actor == durin => {
                    let hurt = last_hurt.expect("a wound came first");
                    assert!(front.contains(&hurt) && hurt != durin, "a front-row ally");
                    let healed = events[i..]
                        .iter()
                        .find_map(|e| match e {
                            Event::Healed { target, .. } => Some(*target),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(healed, hurt, "the wounded ally is healed");
                }
                _ => {}
            }
        }
    }
    assert!(fired >= 2, "ward fired {fired} times in 40 fights");
    assert!(
        back_wounds > 0,
        "the back row was never wounded: the row rule went untried"
    );
}

#[test]
fn the_switch_turns_every_reaction_off_and_is_the_only_tactics_command_in_a_fight() {
    let data = data();
    let shield = spell(&data, "base:spell:shield");
    for seed in 0..10u64 {
        let mut world = party(&data, seed);
        let ilvara = world.party.members[usize::from(ILVARA)].id;
        world.party.members[usize::from(ILVARA)].hp_max = 200;
        world.party.members[usize::from(ILVARA)].hp = 200;
        let always = set(
            ActionRef::Spell(shield),
            Trigger::Attacked,
            Criteria::Always,
        );
        apply(&mut world, &data, put(ILVARA, None, always.clone())).unwrap();
        let here = world.position;
        let foes = encounter(&data, &[("goblin", 3)], Disposition::Hostile, here);
        combat::start(&mut world, &data, foes, Surprise::None, &mut Vec::new()).unwrap();
        if !matches!(world.mode, Mode::Combat(_)) {
            continue;
        }
        refused(
            &mut world,
            &data,
            put(ILVARA, None, always),
            Rejection::WrongMode,
        );
        let off = Command::Party(PartyCommand::Tactics(TacticsCommand::SetReactions {
            member: ILVARA,
            on: false,
        }));
        let events = apply(&mut world, &data, off).unwrap();
        assert_eq!(
            events[0],
            Event::ReactionsSwitched {
                member: ilvara,
                on: false
            }
        );
        assert!(
            !events.iter().any(|e| matches!(e, Event::Turn { .. })),
            "free: no turn passes"
        );
        let mut events = Vec::new();
        for _ in 0..400 {
            if !matches!(world.mode, Mode::Combat(_)) {
                break;
            }
            events.extend(act(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap());
        }
        assert!(
            !events.iter().any(|e| matches!(e, Event::Reaction { .. })),
            "{events:?}"
        );
    }
}
