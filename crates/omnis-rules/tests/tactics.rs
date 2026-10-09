//! Tactics as data (M7c step 4): the criteria walk over integers, every predicate and both
//! combinators, and the shape caps that keep player text and trees bounded (PRD R8).

use omnis_core::{ConditionId, MonsterId, SpellId};
use omnis_rules::tactics::{CRITERIA_DEPTH, CRITERIA_NODES, TACTICS_NAME_BYTES};
use omnis_rules::{
    ActionRef, Cmp, Criteria, CriteriaSet, Facts, MemberFacts, Predicate, Row, Runbook, Tactics,
    TacticsFault, Trigger, Who,
};

const RAT: MonsterId = MonsterId(0);
const GOBLIN: MonsterId = MonsterId(1);
const POISONED: ConditionId = ConditionId(3);

fn facts() -> Facts {
    Facts {
        me: MemberFacts {
            hp_percent: 80,
            sp_percent: 50,
            conditions: Vec::new(),
            row: Row::Front,
        },
        subject: MemberFacts {
            hp_percent: 20,
            sp_percent: 0,
            conditions: vec![POISONED],
            row: Row::Back,
        },
        monsters: vec![(RAT, 3), (GOBLIN, 1)],
        round: 2,
        would_change: true,
    }
}

fn is(p: Predicate) -> Criteria {
    Criteria::Is(p)
}

#[test]
fn every_predicate_reads_its_integer() {
    let f = facts();
    let holds = |p: Predicate| is(p).holds(&f);
    assert!(holds(Predicate::MonsterCount {
        monster: RAT,
        cmp: Cmp::Eq,
        n: 3
    }));
    assert!(!holds(Predicate::MonsterCount {
        monster: GOBLIN,
        cmp: Cmp::Gt,
        n: 1
    }));
    assert!(holds(Predicate::MonsterShare {
        monster: RAT,
        cmp: Cmp::Ge,
        percent: 75
    }));
    assert!(!holds(Predicate::MonsterShare {
        monster: RAT,
        cmp: Cmp::Gt,
        percent: 75
    }));
    assert!(holds(Predicate::Hp {
        who: Who::Subject,
        cmp: Cmp::Lt,
        percent: 25
    }));
    assert!(!holds(Predicate::Hp {
        who: Who::Me,
        cmp: Cmp::Lt,
        percent: 25
    }));
    assert!(holds(Predicate::SpellPoints {
        who: Who::Me,
        cmp: Cmp::Le,
        percent: 50
    }));
    assert!(holds(Predicate::HasCondition {
        who: Who::Subject,
        condition: POISONED
    }));
    assert!(!holds(Predicate::HasCondition {
        who: Who::Me,
        condition: POISONED
    }));
    assert!(holds(Predicate::Row {
        who: Who::Me,
        row: Row::Front
    }));
    assert!(holds(Predicate::Row {
        who: Who::Subject,
        row: Row::Back
    }));
    assert!(holds(Predicate::Round { cmp: Cmp::Ge, n: 2 }));
    assert!(!holds(Predicate::Round { cmp: Cmp::Lt, n: 2 }));
    assert!(holds(Predicate::WouldChangeOutcome));
    let mut none = facts();
    none.monsters.clear();
    none.would_change = false;
    assert!(!is(Predicate::WouldChangeOutcome).holds(&none));
    assert!(
        is(Predicate::MonsterShare {
            monster: RAT,
            cmp: Cmp::Eq,
            percent: 0
        })
        .holds(&none),
        "no monsters: a share of nothing is 0"
    );
}

#[test]
fn all_and_any_combine_and_always_holds() {
    let f = facts();
    let yes = is(Predicate::Round { cmp: Cmp::Eq, n: 2 });
    let no = is(Predicate::Round { cmp: Cmp::Eq, n: 9 });
    assert!(Criteria::Always.holds(&f));
    assert!(Criteria::All(vec![yes.clone(), yes.clone()]).holds(&f));
    assert!(!Criteria::All(vec![yes.clone(), no.clone()]).holds(&f));
    assert!(Criteria::Any(vec![no.clone(), yes.clone()]).holds(&f));
    assert!(!Criteria::Any(vec![no.clone(), no.clone()]).holds(&f));
    assert!(Criteria::All(Vec::new()).holds(&f), "an empty All holds");
    assert!(
        !Criteria::Any(Vec::new()).holds(&f),
        "an empty Any does not"
    );
    assert!(Criteria::Any(vec![no, Criteria::All(vec![yes, Criteria::Always])]).holds(&f));
}

fn set(name: &str, when: Criteria) -> CriteriaSet {
    CriteriaSet {
        name: name.to_owned(),
        action: ActionRef::Spell(SpellId(0)),
        trigger: Trigger::Attacked,
        when,
    }
}

#[test]
fn names_trees_and_percentages_are_capped() {
    assert_eq!(set("Shield", Criteria::Always).check(), Ok(()));
    assert_eq!(
        set(&"x".repeat(TACTICS_NAME_BYTES), Criteria::Always).check(),
        Ok(())
    );
    for name in ["", "   ", &"x".repeat(TACTICS_NAME_BYTES + 1), "bell\u{7}"] {
        assert_eq!(
            set(name, Criteria::Always).check(),
            Err(TacticsFault::Name),
            "{name:?}"
        );
    }
    let mut deep: Criteria = Criteria::Always;
    for _ in 1..CRITERIA_DEPTH {
        deep = Criteria::All(vec![deep]);
    }
    assert_eq!(deep.check(), Ok(()), "depth {CRITERIA_DEPTH}");
    assert_eq!(
        Criteria::Any(vec![deep]).check(),
        Err(TacticsFault::TooDeep)
    );
    let wide = |n| Criteria::<omnis_rules::Ids>::Any(vec![Criteria::Always; n]);
    assert_eq!(
        wide(CRITERIA_NODES - 1).check(),
        Ok(()),
        "{CRITERIA_NODES} nodes"
    );
    assert_eq!(wide(CRITERIA_NODES).check(), Err(TacticsFault::TooDeep));
    let hp = |percent| {
        is(Predicate::Hp {
            who: Who::Me,
            cmp: Cmp::Lt,
            percent,
        })
    };
    assert_eq!(hp(100).check(), Ok(()));
    assert_eq!(hp(101).check(), Err(TacticsFault::Percent));
    let share = is(Predicate::MonsterShare {
        monster: RAT,
        cmp: Cmp::Gt,
        percent: 200,
    });
    assert_eq!(share.check(), Err(TacticsFault::Percent));
}

#[test]
fn a_new_member_reacts_to_nothing_with_one_default_runbook() {
    let mut tactics = Tactics::default();
    assert!(tactics.reactions_on && !tactics.auto);
    assert_eq!(tactics.runbooks.len(), 1);
    assert!(tactics.reactions().is_empty());
    assert_eq!(tactics.check(), Ok(()));
    tactics.library.push(set("Shield", Criteria::Always));
    tactics.runbooks[0]
        .entries
        .push((ActionRef::Spell(SpellId(0)), 0));
    assert_eq!(tactics.reactions().len(), 1);
    tactics.runbooks[0].entries.push((ActionRef::Attack, 1));
    assert_eq!(tactics.check(), Err(TacticsFault::NoSuchSet));
    tactics.runbooks[0].entries.pop();
    tactics.default_runbook = 1;
    assert_eq!(tactics.check(), Err(TacticsFault::NoDefault));
    tactics.default_runbook = 0;
    tactics.runbooks.push(Runbook {
        name: String::new(),
        when: None,
        entries: Vec::new(),
    });
    assert_eq!(tactics.check(), Err(TacticsFault::Name));
}
