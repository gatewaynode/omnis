//! `creation.get` (parity P3): the view says what point buy allows and what each race, class
//! and background brings, and it agrees with the check `party.create` makes: every draft built
//! from the view alone is accepted, and one step past any limit it states is refused.

use crate::common;

use common::{data, world};
use omnis_data::{Alignment, Skill};
use omnis_rules::{CreationError, Draft};
use omnis_sim::{
    Command, CreationView, Op, PartyCommand, Rejection, Reply, apply, creation_view, dispatch,
};

/// A draft from the view alone: all scores at the minimum, and the class's first legal picks.
fn draft(view: &CreationView, race: usize, class: usize, background: usize) -> Draft {
    let (r, c, b) = (
        &view.races[race],
        &view.classes[class],
        &view.backgrounds[background],
    );
    let skills = c
        .skill_list
        .iter()
        .filter(|s| !r.grants.contains(s) && !b.grants.contains(s))
        .take(usize::from(c.choose))
        .copied()
        .collect();
    Draft {
        name: "Ash".to_owned(),
        race: r.id.clone(),
        class: c.id.clone(),
        background: b.id.clone(),
        alignment: view.alignments[0],
        scores: [view.score_min; 6],
        skills,
    }
}

/// `d` with the budget spent on the highest scores it buys, first score first.
fn full(view: &CreationView, mut d: Draft) -> Draft {
    let cost = |s: u8| view.costs[usize::from(s - view.score_min)];
    let mut left = view.points;
    for score in &mut d.scores {
        let best = (view.score_min..=view.score_max)
            .rev()
            .find(|s| cost(*s) <= left)
            .unwrap();
        left -= cost(best);
        *score = best;
    }
    assert_eq!(left, 0, "the base pack's budget is spent exactly");
    d
}

fn create(draft: Draft) -> Result<(), Rejection> {
    let data = data();
    let mut world = world(&data);
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Create(draft)),
    )
    .map(|_| ())
}

#[test]
fn the_view_states_the_base_pack_s_point_buy() {
    let view = creation_view(&data());
    assert_eq!(view.slots, 6);
    assert_eq!(view.name_max, 32);
    assert_eq!((view.score_min, view.score_max, view.points), (8, 15, 27));
    assert_eq!(view.costs, vec![0, 1, 2, 3, 4, 5, 7, 9]);
    let ids = |list: Vec<&str>| list.into_iter().map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(
        view.races.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        ids(vec![
            "base:race:dwarf",
            "base:race:elf",
            "base:race:halfling",
            "base:race:human"
        ])
    );
    assert_eq!(view.classes.len(), 4);
    assert_eq!(view.alignments.len(), 9);
    assert_eq!(view.alignments, Alignment::ALL.to_vec());
    let rogue = view
        .classes
        .iter()
        .find(|c| c.id == "base:class:rogue")
        .unwrap();
    assert_eq!(rogue.choose, 4);
    let acolyte = view
        .backgrounds
        .iter()
        .find(|b| b.id == "base:background:acolyte")
        .unwrap();
    assert_eq!(acolyte.grants, vec![Skill::Insight, Skill::Religion]);
    let human = view
        .races
        .iter()
        .find(|r| r.id == "base:race:human")
        .unwrap();
    assert_eq!(human.bonuses, [1; 6], "the SRD human adds 1 to every score");
    assert_eq!(human.name, "base:text:race.human.name");
}

#[test]
fn every_draft_built_from_the_view_is_accepted() {
    let view = creation_view(&data());
    for race in 0..view.races.len() {
        for class in 0..view.classes.len() {
            for background in 0..view.backgrounds.len() {
                let d = draft(&view, race, class, background);
                let label = format!("{} {} {}", d.race, d.class, d.background);
                assert_eq!(create(d), Ok(()), "{label}");
            }
        }
    }
    // The whole budget, spent on the costliest scores the range allows, is accepted too.
    let d = full(&view, draft(&view, 0, 0, 0));
    assert_eq!(create(d), Ok(()));
}

#[test]
fn one_step_past_each_limit_the_view_states_is_refused() {
    let view = creation_view(&data());
    let rogue = view
        .classes
        .iter()
        .position(|c| c.id == "base:class:rogue")
        .unwrap();
    let acolyte = view
        .backgrounds
        .iter()
        .position(|b| b.id == "base:background:acolyte")
        .unwrap();
    let base = || draft(&view, 0, rogue, acolyte);
    let refused = |d: Draft| match create(d) {
        Err(Rejection::Character(e)) => e,
        other => panic!("expected a creation refusal, got {other:?}"),
    };

    let mut d = base();
    d.scores[0] = view.score_max + 1;
    assert!(matches!(refused(d), CreationError::ScoreRange { .. }));
    let mut d = base();
    d.scores[0] = view.score_min - 1;
    assert!(matches!(refused(d), CreationError::ScoreRange { .. }));

    // The whole budget spent, then one score a step higher.
    let mut d = full(&view, base());
    let raise = d.scores.iter().position(|s| *s < view.score_max).unwrap();
    d.scores[raise] += 1;
    assert!(matches!(
        refused(d),
        CreationError::Points { spent, budget } if budget == view.points && spent > budget
    ));

    let mut d = base();
    d.name = "x".repeat(view.name_max + 1);
    assert_eq!(refused(d), CreationError::Name);

    let mut d = base();
    let extra = view.classes[rogue]
        .skill_list
        .iter()
        .find(|s| !d.skills.contains(s) && !view.backgrounds[acolyte].grants.contains(s))
        .copied()
        .unwrap();
    d.skills.push(extra);
    assert!(matches!(refused(d), CreationError::Skills(_)));

    // The acolyte grants Insight, which is on the rogue's list: picking it is refused.
    assert!(view.classes[rogue].skill_list.contains(&Skill::Insight));
    let mut d = base();
    d.skills[0] = Skill::Insight;
    assert!(matches!(refused(d), CreationError::Skills(_)));

    // One member past the slots.
    let data = data();
    let mut world = world(&data);
    for _ in 0..view.slots {
        apply(
            &mut world,
            &data,
            Command::Party(PartyCommand::Create(base())),
        )
        .unwrap();
    }
    assert_eq!(
        apply(
            &mut world,
            &data,
            Command::Party(PartyCommand::Create(base()))
        ),
        Err(Rejection::PartyFull)
    );
}

#[test]
fn creation_get_answers_the_view() {
    // The JSON round trip is `omnis-mcp`'s replies test.
    let data = data();
    let mut world = world(&data);
    assert_eq!(
        dispatch(&mut world, &data, &Op::CreationGet),
        Ok(Reply::Creation {
            creation: creation_view(&data)
        })
    );
}
