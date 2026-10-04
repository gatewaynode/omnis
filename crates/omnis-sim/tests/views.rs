//! What `combat.get` and `party.get` show of the turn budget and tactics (M7c step 6): the
//! budget, features with their uses and why each is blocked, the spell rows for the action and
//! for the bonus action under the one-spell rule, the reactions switch and reactions left, a
//! caster's points and raised shields, and each member's declared reactions with the actions
//! that could be declared. Party: Brenna (human fighter), Durin (dwarf cleric), Ilvara (elf
//! wizard).

mod common;

use common::{act, data, encounter, party_of, world};
use omnis_data::ron_io::{parse, to_string};
use omnis_data::{Cost, Data, Disposition};
use omnis_sim::command::parse_script;
use omnis_sim::omnis_rules::{ActionRef, Criteria, CriteriaSet, Predicate, Trigger};
use omnis_sim::tactics::TacticsCommand;
use omnis_sim::{
    ActorRef, Budget, CombatCommand, CombatView, Command, EncounterSource, EncounterState,
    FeatureChoice, Mode, Op, PartyCommand, Pay, Rejection, Reply, Stack, Surprise, Target, World,
    apply, combat, combat_view, dispatch, party_view,
};

const BRENNA: usize = 0;
const DURIN: usize = 1;
const ILVARA: usize = 2;
const PIP: usize = 3;

fn fight(world: &mut World, data: &Data, stacks: &[(&str, u8)]) {
    let here = world.position;
    let monsters = encounter(data, stacks, Disposition::Hostile, here);
    combat::start(world, data, monsters, Surprise::None, &mut Vec::new()).unwrap();
}

fn view(world: &World, data: &Data) -> CombatView {
    combat_view(world, data).expect("a fight")
}

fn state(world: &mut World) -> &mut omnis_sim::CombatState {
    match &mut world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

/// Dodge whole turns until the member in `slot` acts.
fn until_turn_of(world: &mut World, data: &Data, slot: usize) {
    for _ in 0..200 {
        let id = world.party.members[slot].id;
        if view(world, data).current == Some(ActorRef::Member(id)) {
            return;
        }
        act(world, data, Command::Combat(CombatCommand::Dodge)).unwrap();
    }
    panic!("the turn never came");
}

fn ask(world: &mut World, data: &Data, command: CombatCommand) {
    apply(world, data, Command::Combat(command)).unwrap_or_else(|r| panic!("{command:?}: {r}"));
}

fn spell_id(data: &Data, name: &str) -> omnis_core::SpellId {
    data.registry
        .spells
        .get(&format!("base:spell:{name}"))
        .unwrap()
}

fn spell_row(world: &World, data: &Data, slot: usize, name: &str) -> u8 {
    let id = spell_id(data, name);
    let at = world.party.members[slot]
        .known_spells
        .iter()
        .position(|s| *s == id)
        .unwrap_or_else(|| panic!("{name}"));
    u8::try_from(at).unwrap()
}

#[test]
fn the_view_shows_the_budget_and_each_feature_s_uses_and_cost() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    world.party.members[BRENNA].level = 2;
    fight(&mut world, &data, &[("goblin", 3)]);
    until_turn_of(&mut world, &data, BRENNA);

    let seen = view(&world, &data);
    assert_eq!(
        seen.budget,
        Budget {
            actions: 1,
            bonus_actions: 1
        }
    );
    let brenna = &seen.members[BRENNA];
    assert_eq!(brenna.index, 0);
    let rows: Vec<_> = brenna
        .features
        .iter()
        .map(|f| {
            (
                f.index,
                f.name.as_str(),
                f.cost,
                f.uses_left,
                f.blocked.clone(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                0,
                "base:text:class.fighter.second_wind",
                Cost::BonusAction,
                Some(1),
                None
            ),
            (
                1,
                "base:text:class.fighter.action_surge",
                Cost::Free,
                Some(1),
                None
            ),
        ],
        "SRD: Second Wind at level 1 and Action Surge at 2, once a rest each"
    );
    assert!(
        seen.members[DURIN]
            .features
            .iter()
            .all(|f| f.blocked == Some(Rejection::NotYourTurn)),
        "only the acting member's features can be used"
    );

    ask(
        &mut world,
        &data,
        CombatCommand::Feature {
            feature: 1,
            choice: FeatureChoice::None,
        },
    );
    let seen = view(&world, &data);
    assert_eq!(seen.budget.actions, 2, "Action Surge adds an action");
    let surge = &seen.members[BRENNA].features[1];
    assert_eq!(surge.uses_left, Some(0));
    assert_eq!(surge.blocked, Some(Rejection::NoUsesLeft { feature: 1 }));

    ask(&mut world, &data, CombatCommand::Attack { stack: 0 });
    assert_eq!(view(&world, &data).budget.actions, 1);
    ask(
        &mut world,
        &data,
        CombatCommand::Feature {
            feature: 0,
            choice: FeatureChoice::None,
        },
    );
    let seen = view(&world, &data);
    assert_eq!(seen.budget.bonus_actions, 0);
    assert_eq!(
        seen.members[BRENNA].features[0].blocked,
        Some(Rejection::NoUsesLeft { feature: 0 }),
        "Second Wind's use is spent before the bonus action is asked"
    );
}

#[test]
fn spell_rows_answer_for_the_action_and_the_bonus_action_under_the_one_spell_rule() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let healing_word = spell_id(&data, "healing_word");
    world.party.members[DURIN].known_spells.push(healing_word);
    fight(&mut world, &data, &[("goblin", 3)]);
    until_turn_of(&mut world, &data, DURIN);
    let word = spell_row(&world, &data, DURIN, "healing_word");
    let flame = spell_row(&world, &data, DURIN, "sacred_flame");
    let row = |seen: &CombatView, at: u8| {
        let row = seen.spells.iter().find(|s| s.index == at).unwrap();
        (row.blocked.clone(), row.bonus.clone())
    };

    let seen = view(&world, &data);
    assert_eq!(row(&seen, word), (None, None), "Healing Word takes either");
    assert_eq!(
        row(&seen, flame),
        (None, Some(Rejection::NotABonusAction { spell: flame })),
        "Sacred Flame takes the action only"
    );
    assert_eq!(seen.spells_cast, omnis_sim::SpellsCast::default());

    ask(
        &mut world,
        &data,
        CombatCommand::Cast {
            spell: word,
            target: Target::Member(0),
            pay: Pay::BonusAction,
        },
    );
    let seen = view(&world, &data);
    assert_eq!(
        seen.budget,
        Budget {
            actions: 1,
            bonus_actions: 0
        }
    );
    assert!(seen.spells_cast.bonus);
    assert_eq!(
        row(&seen, word),
        (
            Some(Rejection::OneSpellATurn { spell: word }),
            Some(Rejection::NoBonusActionLeft)
        ),
        "SRD: after a bonus-action spell, only a cantrip with the action"
    );
    assert_eq!(
        row(&seen, flame),
        (None, Some(Rejection::NotABonusAction { spell: flame })),
        "the cantrip stays open"
    );
}

#[test]
fn the_view_shows_the_reactions_switch_and_reactions_left_and_the_word_flips_it() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 3);
    fight(&mut world, &data, &[("goblin", 3)]);
    let ilvara = ActorRef::Member(world.party.members[ILVARA].id);
    let left = |seen: &CombatView| {
        seen.reactions
            .iter()
            .find(|(actor, _)| *actor == ilvara)
            .map(|(_, n)| *n)
    };
    let seen = view(&world, &data);
    assert!(seen.members.iter().all(|m| m.reactions_on));
    assert_eq!(left(&seen), Some(1), "one reaction a round");

    state(&mut world).set_reactions(ilvara, 0);
    assert_eq!(left(&view(&world, &data)), Some(0));

    let words = parse_script("react-2-off react-2-on react-2-off").unwrap();
    assert_eq!(
        words[0],
        Command::Party(PartyCommand::Tactics(TacticsCommand::SetReactions {
            member: 2,
            on: false
        }))
    );
    assert_eq!(
        words[1],
        Command::Party(PartyCommand::Tactics(TacticsCommand::SetReactions {
            member: 2,
            on: true
        }))
    );
    assert!(parse_script("react-2-maybe").is_err());
    assert!(parse_script("react-x-on").is_err());
    for word in words {
        apply(&mut world, &data, word).unwrap();
    }
    let seen = view(&world, &data);
    assert!(!seen.members[ILVARA].reactions_on, "the switch, mid-fight");
    assert!(seen.members[BRENNA].reactions_on);
    assert!(
        !party_view(&world, &data).members[ILVARA]
            .tactics
            .reactions_on
    );
}

#[test]
fn a_caster_s_points_and_raised_shields_show_per_individual() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let stack = |name: &str, count: u8| Stack {
        monster: data
            .registry
            .monsters
            .get(&format!("test:monster:{name}"))
            .unwrap(),
        initial: count,
        hp: vec![30; usize::from(count)],
        spent: Vec::new(),
    };
    let monsters = EncounterState {
        source: EncounterSource::Random,
        stacks: vec![stack("giant_rat", 3), stack("bob_the_rat_king", 2)],
        disposition: Disposition::Hostile,
        retreat: world.position,
    };
    combat::start(&mut world, &data, monsters, Surprise::None, &mut Vec::new()).unwrap();
    let points = |world: &World| -> Vec<Vec<u8>> {
        view(world, &data)
            .stacks
            .iter()
            .map(|s| s.points_left.clone())
            .collect()
    };
    // Bob may have cast before the first member's turn: set the state the view reads.
    let fight = state(&mut world);
    fight.encounter.stacks[1].spent = vec![0, 2];
    fight.monster_shields = vec![(1, 1)];
    assert_eq!(
        points(&world),
        [vec![], vec![5, 3]],
        "five points each, less what each spent; rats cast nothing"
    );
    let seen = view(&world, &data);
    assert_eq!(seen.stacks[1].shielded, [1]);
    assert!(seen.stacks[0].shielded.is_empty());

    // An older save has no `spent`: every Bob has his five.
    state(&mut world).encounter.stacks[1].spent.clear();
    assert_eq!(points(&world), [vec![], vec![5, 5]]);
}

#[test]
fn party_get_lists_declared_reactions_and_what_each_member_could_declare() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 3);
    let shield = spell_id(&data, "shield");
    let set = CriteriaSet {
        name: "Shield".to_owned(),
        action: ActionRef::Spell(shield),
        trigger: Trigger::Attacked,
        when: Criteria::Is(Predicate::WouldChangeOutcome),
    };
    let put = TacticsCommand::PutReaction {
        member: 2,
        at: None,
        set: set.clone(),
    };
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Tactics(put)),
    )
    .unwrap();

    let party = party_view(&world, &data);
    let tactics = &party.members[ILVARA].tactics;
    assert!(tactics.reactions_on && !tactics.auto);
    assert_eq!(tactics.reactions.len(), 1);
    let declared = &tactics.reactions[0];
    assert_eq!(declared.index, 0);
    assert_eq!(declared.name, "Shield");
    assert_eq!(declared.action, ActionRef::Spell(shield));
    assert_eq!(declared.action_name, "base:spell:shield");
    assert_eq!(declared.trigger, Trigger::Attacked);
    assert_eq!(declared.when, set.when);
    let could: Vec<_> = tactics
        .answers
        .iter()
        .map(|a| (a.name.as_str(), a.triggers.clone()))
        .collect();
    assert_eq!(
        could,
        [
            ("attack", vec![Trigger::EnemyFlees]),
            ("base:spell:shield", vec![Trigger::Attacked]),
        ],
        "the weapon at a fleeing enemy; Shield when attacked"
    );
    let brenna = &party.members[BRENNA].tactics;
    assert!(brenna.reactions.is_empty());
    assert_eq!(brenna.answers.len(), 1, "a fighter has only the weapon");
    assert_eq!(brenna.answers[0].action, ActionRef::Attack);

    // Both replies go over the wire whole.
    let text = to_string(&Reply::Party {
        party: party.clone(),
    })
    .unwrap();
    assert_eq!(parse::<Reply>(&text).unwrap(), Reply::Party { party });
    fight(&mut world, &data, &[("goblin", 2)]);
    let Reply::Combat { combat } = dispatch(&mut world, &data, &Op::CombatGet).unwrap() else {
        panic!("not the fight");
    };
    assert_eq!(combat.members.len(), 3);
    let text = to_string(&Reply::Combat {
        combat: combat.clone(),
    })
    .unwrap();
    assert_eq!(parse::<Reply>(&text).unwrap(), Reply::Combat { combat });
}

#[test]
fn cunning_action_shows_open_on_the_rogue_s_turn_at_will() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    world.party.members[PIP].level = 2;
    fight(&mut world, &data, &[("goblin", 3)]);
    until_turn_of(&mut world, &data, PIP);
    let pip = &view(&world, &data).members[PIP];
    let cunning = pip
        .features
        .iter()
        .find(|f| f.name == "base:text:class.rogue.cunning_action")
        .unwrap_or_else(|| panic!("{pip:?}"));
    assert_eq!(
        (cunning.cost, cunning.uses_left, cunning.blocked.clone()),
        (Cost::BonusAction, None, None),
        "SRD: a bonus action, at will"
    );
    let brenna = &view(&world, &data).members[BRENNA].features;
    assert!(!brenna.is_empty());
    assert!(
        brenna
            .iter()
            .all(|f| f.blocked == Some(Rejection::NotYourTurn)),
        "Second Wind waits for Brenna's own turn"
    );
}
