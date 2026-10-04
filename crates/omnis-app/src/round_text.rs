//! The fight's shape and its blood as text: the start, the order, rounds and turns that need no
//! die, features used and opportunity attacks, then damage, falling, death saves, conditions, deaths and the end. Split from
//! `combat_text.rs`, which chains these after the encounter lines; its tests cover them through
//! `batch_lines`. Bevy-free.

use crate::text::{Line, Names, trace_math};
use omnis_sim::omnis_core::money::gp_floor;
use omnis_sim::omnis_data::DamageType;
use omnis_sim::omnis_rules::{DamageAdjust, DeathSaveResult};
use omnis_sim::{ActorRef, CombatOutcome, Event, Surprise};

/// The shape of the fight: its start, the order, rounds, and turns that need no die.
pub(crate) fn round_line(event: &Event, names: &Names) -> Option<Line> {
    Some(match event {
        Event::CombatStarted { surprised } => Line::same(
            match surprised {
                Surprise::None => "Combat!",
                Surprise::Party => "Ambush! The party is surprised",
                Surprise::Monsters => "The monsters are surprised",
            }
            .to_owned(),
        ),
        Event::Initiative { order, .. } => {
            let list = order
                .iter()
                .map(|(actor, total)| format!("{} {total}", names.actor(actor)))
                .collect::<Vec<_>>()
                .join(", ");
            Line::same(format!("Initiative: {list}"))
        }
        Event::RoundStarted { round } => Line::same(format!("Round {round}")),
        Event::Waited { actor } => Line::same(format!("{} wait", names.actor(actor))),
        Event::Dodging { actor } => Line::same(format!("{} dodges", names.actor(actor))),
        Event::Exchanged { a, b } => Line::same(format!("Slots {} and {} exchange", a + 1, b + 1)),
        Event::FeatureUsed { member, feature } => Line::same(format!(
            "{} uses {}",
            names.member(*member),
            names.feature(feature)
        )),
        Event::OpportunityAttack { stack, member } => Line::new(
            format!(
                "Opportunity attack: {} on {}",
                names.actor(&ActorRef::Stack(*stack)),
                names.member(*member)
            ),
            format!("Opportunity attack on {}", names.member(*member)),
        ),
        _ => return None,
    })
}

/// Blood: damage on its own, falling, death saves, conditions, deaths, and the end.
pub(crate) fn wound_line(event: &Event, names: &Names) -> Option<Line> {
    Some(match event {
        Event::Damage {
            target,
            kind,
            rolls,
            amount,
            adjust,
            ..
        } => {
            let outcome = format!(
                "{} takes {amount} {}{}",
                names.actor(target),
                kind_word(*kind),
                adjust_word(*adjust)
            );
            let math = rolls.iter().map(trace_math).collect::<Vec<_>>().join(" + ");
            Line::new(format!("{outcome} ({math})"), outcome)
        }
        Event::Down { target } => Line::same(format!("{} falls", names.member(*target))),
        Event::Wounded { member, failures } => Line::same(format!(
            "{} is wounded: {failures} of 3 failures",
            names.member(*member)
        )),
        Event::DeathSave {
            member,
            roll,
            result,
            successes,
            failures,
        } => {
            let who = names.member(*member);
            let outcome = match result {
                DeathSaveResult::Success => format!("{who} death save: success {successes}/3"),
                DeathSaveResult::Failure => format!("{who} death save: failure {failures}/3"),
                DeathSaveResult::Stable => format!("{who} is stable"),
                DeathSaveResult::Revived => format!("{who} comes to at 1 hp"),
                DeathSaveResult::Died => format!("{who} dies"),
            };
            Line::new(format!("{outcome} ({})", trace_math(roll)), outcome)
        }
        Event::Condition {
            target,
            condition,
            applied,
        } => Line::same(format!(
            "{} is {}{}",
            names.actor(target),
            if *applied { "" } else { "no longer " },
            names.condition(*condition)
        )),
        Event::Death { target, gold } => {
            let who = names.actor(target);
            match gold {
                Some(trace) => Line::new(
                    format!(
                        "{who} dies, dropping {} gold ({})",
                        trace.total,
                        trace_math(trace)
                    ),
                    format!("{who} dies, dropping {} gold", trace.total),
                ),
                None => Line::same(format!("{who} dies")),
            }
        }
        Event::CombatEnded {
            outcome,
            xp,
            gold,
            fallen,
        } => {
            let mut text = match outcome {
                CombatOutcome::Victory => {
                    format!("Victory! {xp} XP each, {} gold", gp_floor(*gold))
                }
                CombatOutcome::Fled => "The party gets away".to_owned(),
                CombatOutcome::Defeat => "The party has fallen".to_owned(),
            };
            if !fallen.is_empty() {
                let lost = fallen
                    .iter()
                    .map(|id| names.member(*id))
                    .collect::<Vec<_>>()
                    .join(", ");
                text = format!("{text}; lost: {lost}");
            }
            Line::same(text)
        }
        _ => return None,
    })
}

pub(crate) const fn adjust_word(adjust: DamageAdjust) -> &'static str {
    match adjust {
        DamageAdjust::None => "",
        DamageAdjust::Resisted => " (resisted)",
        DamageAdjust::Vulnerable => " (doubled)",
        DamageAdjust::Immune => " (immune)",
    }
}

fn kind_word(kind: DamageType) -> String {
    format!("{kind:?}").to_lowercase()
}
