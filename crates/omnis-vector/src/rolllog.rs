//! The roll log: one short line per fight event (alt-ARCHITECTURE.md §9). Bevy-free.
//!
//! The simulation numbers a monster among its stack's living, so the second rat becomes the
//! first when the first dies. [`Names`] gives each individual the number it was met with and
//! keeps it, and it keeps the names of members a fight's last command buries. The session
//! shows it every event after describing it.

use omnis_sim::omnis_core::{CharacterId, SpellId};
use omnis_sim::omnis_data::{Data, Disposition};
use omnis_sim::omnis_rules::{DamageAdjust, DeathSaveResult};
use omnis_sim::{ActorRef, CheckKind, CombatOutcome, EffectTarget, Event, Surprise, World};
use std::collections::BTreeMap;

/// One stack as the log names it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stack {
    /// The monster's name.
    name: String,
    /// How many it started with.
    initial: u8,
    /// The numbers, from one, of those still standing, in the simulation's order.
    living: Vec<u8>,
}

/// Who is in the fight, by the references events carry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    /// Every member seen, kept after they leave the party.
    members: BTreeMap<CharacterId, String>,
    /// Marching order, slot to member.
    order: Vec<CharacterId>,
    /// The stacks of the current or last encounter.
    stacks: Vec<Stack>,
}

fn monster_name(data: &Data, id: omnis_sim::omnis_core::MonsterId) -> &str {
    data.monsters
        .get(&id)
        .map_or("?", |m| data.label("en", &m.name))
}

impl Names {
    /// Take in the party's members and marching order. Members are added, never removed.
    pub fn observe(&mut self, world: &World) {
        for m in &world.party.members {
            self.members.insert(m.id, m.name.clone());
        }
        self.order = world.party.members.iter().map(|m| m.id).collect();
    }

    /// Follow an event already described: meeting monsters numbers them afresh, and a death
    /// takes its number out of the living.
    pub fn follow(&mut self, event: &Event, data: &Data) {
        match event {
            Event::EncounterStarted { stacks, .. } => {
                self.stacks = stacks
                    .iter()
                    .map(|(monster, count)| Stack {
                        name: monster_name(data, *monster).to_owned(),
                        initial: *count,
                        living: (1..=*count).collect(),
                    })
                    .collect();
            }
            Event::Death {
                target: ActorRef::Monster { stack, index },
                ..
            } => {
                if let Some(s) = self.stacks.get_mut(usize::from(*stack))
                    && usize::from(*index) < s.living.len()
                {
                    s.living.remove(usize::from(*index));
                }
            }
            _ => {}
        }
    }

    fn member(&self, id: CharacterId) -> String {
        self.members
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "someone".to_owned())
    }

    fn slot(&self, slot: u8) -> String {
        self.order
            .get(usize::from(slot))
            .map_or_else(|| "someone".to_owned(), |id| self.member(*id))
    }

    /// A member's name, a stack's monster name, or one individual by the number it was met
    /// with ("Giant rat 2") when its stack started with more than one.
    #[must_use]
    pub fn actor(&self, actor: ActorRef) -> String {
        let Some(stack) = (match actor {
            ActorRef::Member(id) => return self.member(id),
            ActorRef::Stack(s) | ActorRef::Monster { stack: s, .. } => {
                self.stacks.get(usize::from(s))
            }
        }) else {
            return "a monster".to_owned();
        };
        match actor {
            ActorRef::Monster { index, .. } if stack.initial > 1 => {
                match stack.living.get(usize::from(index)) {
                    Some(n) => format!("{} {n}", stack.name),
                    None => stack.name.clone(),
                }
            }
            _ => stack.name.clone(),
        }
    }

    fn effect(&self, target: EffectTarget) -> String {
        match target {
            EffectTarget::Member(id) => self.member(id),
            EffectTarget::Party => "the party".to_owned(),
        }
    }
}

fn spell(data: &Data, id: SpellId) -> &str {
    data.spells
        .get(&id)
        .map_or("a spell", |s| data.label("en", &s.name))
}

/// A line for a fight event, or `None` for the ones the screen shows another way (whose turn
/// it is) and for events outside fights.
#[must_use]
pub fn describe(event: &Event, names: &Names, data: &Data) -> Option<String> {
    Some(match event {
        Event::EncounterStarted {
            stacks,
            disposition,
            noticed,
            ..
        } => {
            let met: Vec<String> = stacks
                .iter()
                .map(|(monster, count)| format!("{count} {}", monster_name(data, *monster)))
                .collect();
            let mood = match disposition {
                Disposition::Hostile => "hostile",
                Disposition::Wary => "wary",
                Disposition::Neutral => "neutral",
                Disposition::Friendly => "friendly",
            };
            let surprise = if *noticed {
                ""
            } else {
                "; they caught the party unaware"
            };
            format!("Met {} ({mood}){surprise}", met.join(", "))
        }
        Event::Check {
            actor,
            kind,
            roll,
            dc,
            success,
        } => {
            let what = match kind {
                CheckKind::Stealth => "Stealth".to_owned(),
                CheckKind::Hide => "Hide".to_owned(),
                CheckKind::Run => "Run".to_owned(),
                CheckKind::Flee => "Flee".to_owned(),
                CheckKind::Save(ability) => format!("{ability:?} save"),
            };
            let rolled = roll
                .as_ref()
                .map_or_else(String::new, |r| format!(" {} against {dc}", r.total));
            let outcome = if *success { "success" } else { "failure" };
            format!("{} {what}:{rolled}, {outcome}", names.actor(*actor))
        }
        Event::Bribed { cost } => format!("Paid {cost} gold; the monsters leave"),
        Event::CombatStarted { surprised } => match surprised {
            Surprise::None => "The fight begins",
            Surprise::Party => "The fight begins; the party is surprised",
            Surprise::Monsters => "The fight begins; the monsters are surprised",
        }
        .to_owned(),
        Event::Initiative { order, .. } => {
            let order: Vec<String> = order
                .iter()
                .map(|(actor, total)| format!("{} {total}", names.actor(*actor)))
                .collect();
            format!("Initiative: {}", order.join(", "))
        }
        Event::RoundStarted { round } => format!("Round {round}"),
        Event::Waited { actor } => format!("{} waits", names.actor(*actor)),
        Event::Dodging { actor } => format!("{} dodges", names.actor(*actor)),
        Event::Exchanged { a, b } => {
            format!("{} and {} swap places", names.slot(*a), names.slot(*b))
        }
        Event::AttackResolved {
            attacker,
            target,
            roll,
            ac,
            hit,
            crit,
        } => {
            let verb = match (hit, crit) {
                (true, true) => "lands a critical hit on",
                (true, false) => "hits",
                (false, _) => "misses",
            };
            format!(
                "{} {verb} {} ({} against AC {ac})",
                names.actor(*attacker),
                names.actor(*target),
                roll.total
            )
        }
        Event::Damage {
            target,
            kind,
            amount,
            adjust,
            ..
        } => {
            let adjust = match adjust {
                DamageAdjust::None => "",
                DamageAdjust::Resisted => " (resisted)",
                DamageAdjust::Vulnerable => " (vulnerable)",
                DamageAdjust::Immune => " (immune)",
            };
            let kind = format!("{kind:?}").to_lowercase();
            format!("{} takes {amount} {kind}{adjust}", names.actor(*target))
        }
        Event::SpellCast {
            caster,
            spell: id,
            points,
            ..
        } => {
            let cost = if *points == 0 {
                String::new()
            } else {
                format!(" ({points} points)")
            };
            format!("{} casts {}{cost}", names.member(*caster), spell(data, *id))
        }
        Event::EffectApplied {
            target, spell: id, ..
        } => {
            format!(
                "{} takes hold on {}",
                spell(data, *id),
                names.effect(*target)
            )
        }
        Event::EffectEnded {
            target, spell: id, ..
        } => {
            format!("{} on {} ends", spell(data, *id), names.effect(*target))
        }
        Event::Concentration {
            caster, spell: id, ..
        } => format!(
            "{} stops concentrating on {}",
            names.member(*caster),
            spell(data, *id)
        ),
        Event::ItemUsed {
            member,
            item,
            target,
            ..
        } => {
            let item = data
                .items
                .get(item)
                .map_or("an item", |i| data.label("en", &i.name));
            let on = target.map_or_else(String::new, |t| format!(" on {}", names.member(t)));
            format!("{} uses {item}{on}", names.member(*member))
        }
        Event::Healed { target, hp, .. } => {
            format!("{} is healed to {hp} hp", names.member(*target))
        }
        Event::Down { target } => format!("{} is down", names.member(*target)),
        Event::Wounded { member, failures } => format!(
            "{} is hurt while down ({failures} of 3 failures)",
            names.member(*member)
        ),
        Event::DeathSave { member, result, .. } => {
            let result = match result {
                DeathSaveResult::Success => "a success",
                DeathSaveResult::Failure => "a failure",
                DeathSaveResult::Stable => "stable",
                DeathSaveResult::Revived => "back on their feet",
                DeathSaveResult::Died => "dead",
            };
            format!("{} death save: {result}", names.member(*member))
        }
        Event::Condition {
            target,
            condition,
            applied,
        } => {
            let condition = data
                .conditions
                .get(condition)
                .map_or("a condition", |c| data.label("en", &c.name));
            let state = if *applied { "is now" } else { "is no longer" };
            format!("{} {state} {condition}", names.actor(*target))
        }
        Event::Death { target, gold } => {
            let gold = gold
                .as_ref()
                .filter(|g| g.total > 0)
                .map_or_else(String::new, |g| format!(", dropping {} gold", g.total));
            format!("{} dies{gold}", names.actor(*target))
        }
        Event::CombatEnded {
            outcome,
            xp,
            gold,
            fallen,
        } => {
            let line = match outcome {
                CombatOutcome::Victory => format!("Victory: {xp} xp each, {gold} gold"),
                CombatOutcome::Fled => "The party got away".to_owned(),
                CombatOutcome::Defeat => "The party has fallen".to_owned(),
            };
            if fallen.is_empty() {
                line
            } else {
                let lost: Vec<String> = fallen.iter().map(|id| names.member(*id)).collect();
                format!("{line}. Lost: {}", lost.join(", "))
            }
        }
        _ => return None,
    })
}
