//! Combat events as text, with the roll math the simulation traced (PRD §7.3): a long form
//! for the band's message line and a short form for the roll log under the viewport. The
//! simulation sends keys, ids, and traces; this file is where they become English. Bevy-free.

use crate::font::fit;
use omnis_sim::omnis_core::{CharacterId, ConditionId, RollTrace, SpellId};
use omnis_sim::omnis_data::{DamageType, Data};
use omnis_sim::omnis_rules::{DamageAdjust, DeathSaveResult, Roll, RollMode};
use omnis_sim::{ActorRef, CheckKind, CombatOutcome, Event, Mode, Surprise, World};
use std::collections::BTreeMap;

/// Cells a long line may take: the band's message line.
pub const LONG_CELLS: usize = 100;
/// Cells a short line may take: a roll-log row under the viewport.
pub const SHORT_CELLS: usize = 39;
// A long line fits the band's log and its message line.
const _: () = assert!(LONG_CELLS <= crate::band::LOG_CELLS);
const _: () = assert!(LONG_CELLS <= crate::band::BAND_COLUMNS);

/// The names events refer to by id. Members are remembered by id after they leave the party
/// and stacks after a fight ends, so the batch that ends a fight still reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    members: BTreeMap<CharacterId, String>,
    /// Label and initial count per stack index.
    stacks: Vec<(String, u8)>,
    conditions: BTreeMap<ConditionId, String>,
    spells: BTreeMap<SpellId, String>,
}

impl Names {
    /// Names for the world as it is.
    #[must_use]
    pub fn new(world: &World, data: &Data) -> Names {
        let mut names = Names::default();
        names.refresh(world, data);
        names
    }

    /// Learn the current members and, while monsters stand there, the current stacks.
    pub fn refresh(&mut self, world: &World, data: &Data) {
        for member in &world.party.members {
            self.members.insert(member.id, member.name.clone());
        }
        let encounter = match &world.mode {
            Mode::Explore => None,
            Mode::Encounter(e) => Some(e),
            Mode::Combat(c) => Some(&c.encounter),
        };
        if let Some(encounter) = encounter {
            self.stacks = encounter
                .stacks
                .iter()
                .map(|s| {
                    let label = data
                        .monsters
                        .get(&s.monster)
                        .map_or("?", |m| data.label("en", &m.name));
                    (label.to_owned(), s.initial)
                })
                .collect();
        }
        if self.conditions.is_empty() {
            for (id, condition) in &data.conditions {
                self.conditions
                    .insert(*id, data.label("en", &condition.name).to_owned());
            }
        }
        if self.spells.is_empty() {
            for (id, spell) in &data.spells {
                self.spells
                    .insert(*id, data.label("en", &spell.name).to_owned());
            }
        }
    }

    /// A member's name.
    #[must_use]
    pub fn member(&self, id: CharacterId) -> &str {
        self.members.get(&id).map_or("?", String::as_str)
    }

    /// Who an actor is: `Brenna`, `Goblins` for a stack, `Goblin 2` for one of several.
    #[must_use]
    pub fn actor(&self, actor: &ActorRef) -> String {
        match actor {
            ActorRef::Member(id) => self.member(*id).to_owned(),
            ActorRef::Stack(stack) => match self.stacks.get(usize::from(*stack)) {
                Some((label, 1)) => label.clone(),
                Some((label, _)) => format!("{label}s"),
                None => format!("Stack {stack}"),
            },
            ActorRef::Monster { stack, index } => match self.stacks.get(usize::from(*stack)) {
                Some((label, 1)) => label.clone(),
                Some((label, _)) => format!("{label} {}", index + 1),
                None => format!("Stack {stack} #{}", index + 1),
            },
        }
    }

    /// A condition's name.
    #[must_use]
    pub fn condition(&self, id: ConditionId) -> &str {
        self.conditions.get(&id).map_or("?", String::as_str)
    }

    /// A spell's name.
    #[must_use]
    pub fn spell(&self, id: SpellId) -> &str {
        self.spells.get(&id).map_or("?", String::as_str)
    }
}

/// One event as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// With the math, at most `LONG_CELLS`.
    pub long: String,
    /// The outcome alone, at most `SHORT_CELLS`.
    pub short: String,
}

impl Line {
    pub(crate) fn new(long: String, short: String) -> Line {
        Line {
            long: fit(&long, LONG_CELLS),
            short: fit(&short, SHORT_CELLS),
        }
    }

    pub(crate) fn same(text: String) -> Line {
        Line::new(text.clone(), text)
    }
}

/// The lines for one command's events, in order. An attack and the damage that follows it
/// make one line; encounter checks, monster turns, and exploration events make none.
#[must_use]
pub fn batch_lines(events: &[Event], names: &Names) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut i = 0;
    while i < events.len() {
        if let Event::AttackResolved {
            attacker,
            target,
            roll,
            ac,
            hit,
            crit,
        } = &events[i]
        {
            let damage = match events.get(i + 1) {
                Some(Event::Damage {
                    target: hurt,
                    amount,
                    adjust,
                    ..
                }) if *hit && hurt == target => Some((*amount, *adjust)),
                _ => None,
            };
            if damage.is_some() {
                i += 1;
            }
            lines.push(attack_line(
                names,
                (attacker, target),
                roll,
                *ac,
                (*hit, *crit),
                damage,
            ));
        } else if let Some(line) = event_line(&events[i], names) {
            lines.push(line);
        }
        i += 1;
    }
    lines
}

fn attack_line(
    names: &Names,
    (attacker, target): (&ActorRef, &ActorRef),
    roll: &Roll,
    ac: i64,
    (hit, crit): (bool, bool),
    damage: Option<(i64, DamageAdjust)>,
) -> Line {
    let who = names.actor(attacker);
    let whom = names.actor(target);
    let math = format!("{} vs AC {ac}", roll_math(roll));
    let verb = if crit {
        "crits"
    } else if hit {
        "hits"
    } else {
        "misses"
    };
    let outcome = match damage {
        Some((amount, adjust)) => {
            format!("{who} {verb} {whom} for {amount}{}", adjust_word(adjust))
        }
        None => format!("{who} {verb} {whom}"),
    };
    Line::new(format!("{outcome} ({math})"), outcome)
}

/// One line for an event that stands alone, or `None` for the silent ones.
fn event_line(event: &Event, names: &Names) -> Option<Line> {
    before_fight_line(event, names)
        .or_else(|| round_line(event, names))
        .or_else(|| wound_line(event, names))
        .or_else(|| crate::spell_text::spell_line(event, names))
}

/// The encounter phase: who stands there, the checks, the bribe.
fn before_fight_line(event: &Event, names: &Names) -> Option<Line> {
    Some(match event {
        Event::EncounterStarted {
            stacks,
            disposition,
            stealth,
            perception,
            noticed,
            ..
        } => {
            let who = stacks
                .iter()
                .enumerate()
                .map(|(i, (_, count))| {
                    let label = names.stacks.get(i).map_or("?", |(label, _)| label);
                    format!("{count} {label}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            let mood = format!("{disposition:?}").to_lowercase();
            let seen = match stealth {
                Some(roll) => format!(
                    " (stealth {} vs {perception}: {})",
                    roll_math(roll),
                    if *noticed { "noticed" } else { "surprised!" }
                ),
                None => String::new(),
            };
            Line::new(
                format!("{who}, {mood}{seen}"),
                if *noticed {
                    format!("{who}, {mood}")
                } else {
                    format!("Ambush! {who}")
                },
            )
        }
        Event::Check {
            actor,
            kind,
            roll,
            dc,
            success,
        } => {
            let who = names.actor(actor);
            let verb = match kind {
                CheckKind::Stealth => "sneaks",
                CheckKind::Hide => "hides",
                CheckKind::Run => "runs",
                CheckKind::Flee => "flees",
                CheckKind::Save(_) => "saves",
            };
            let result = if *success { "success" } else { "failure" };
            let math = roll.as_ref().map_or_else(
                || "no roll needed".to_owned(),
                |r| format!("{} vs DC {dc}", roll_math(r)),
            );
            Line::new(
                format!("{who} {verb}: {result} ({math})"),
                format!("{who} {verb}: {result}"),
            )
        }
        Event::Bribed { cost } => Line::same(format!("The party pays {cost} gold; they leave")),
        _ => return None,
    })
}

/// The shape of the fight: its start, the order, rounds, and turns that need no die.
fn round_line(event: &Event, names: &Names) -> Option<Line> {
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
        _ => return None,
    })
}

/// Blood: damage on its own, falling, death saves, conditions, deaths, and the end.
fn wound_line(event: &Event, names: &Names) -> Option<Line> {
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
                CombatOutcome::Victory => format!("Victory! {xp} XP each, {gold} gold"),
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

/// `1d20+4 [17]=21`, or `2d20 adv+4 [3, 17]=21` under a mode.
#[must_use]
pub fn roll_math(roll: &Roll) -> String {
    let dice = match roll.mode {
        RollMode::Normal => "1d20",
        RollMode::Advantage => "2d20 adv",
        RollMode::Disadvantage => "2d20 dis",
    };
    let bonus = roll.modifier + roll.proficiency;
    let bonus = if bonus == 0 {
        String::new()
    } else {
        format!("{bonus:+}")
    };
    format!("{dice}{bonus} {}={}", faces(&roll.trace), roll.total)
}

/// `1d8+2 [5]=7`: the trace without its stream name.
#[must_use]
pub fn trace_math(trace: &RollTrace) -> String {
    format!("{} {}={}", trace.dice, faces(trace), trace.total)
}

fn faces(trace: &RollTrace) -> String {
    let faces = trace
        .rolls
        .iter()
        .map(|r| r.value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{faces}]")
}

const fn adjust_word(adjust: DamageAdjust) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::tests::facing_goblins;
    use omnis_sim::omnis_core::{Dice, DieRoll, MonsterId, StreamName};
    use omnis_sim::omnis_data::{Disposition, load_packs};
    use omnis_sim::{
        CombatCommand, Command, EncounterChoice, EncounterSource, ModeKind, apply, combat_view,
    };
    use std::path::PathBuf;

    fn data() -> Data {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")])
            .unwrap_or_else(|r| panic!("{r}"))
    }

    fn trace(dice: Dice, values: &[u32]) -> RollTrace {
        let rolls = values
            .iter()
            .map(|v| DieRoll {
                index: 0,
                raw: 0,
                value: *v,
            })
            .collect();
        let sum: i32 = values.iter().map(|v| i32::try_from(*v).unwrap()).sum();
        RollTrace {
            stream: StreamName::new("combat"),
            dice,
            rolls,
            total: sum + dice.modifier,
        }
    }

    fn d20(mode: RollMode, values: &[u32], face: u32, modifier: i64, proficiency: i64) -> Roll {
        Roll {
            trace: trace(Dice::new(values.len() as u16, 20), values),
            mode,
            face,
            modifier,
            proficiency,
            bonus: None,
            total: i64::from(face) + modifier + proficiency,
        }
    }

    /// Two members of the longest names the creation form allows and a stack of the longest
    /// monster label the stack rows show, with no conditions known.
    fn wide_names() -> Names {
        let mut names = Names::default();
        names
            .members
            .insert(CharacterId(0), "Brennagh-of-the-Long-Hall".to_owned());
        names
            .members
            .insert(CharacterId(1), "Gormundsson-the-Younger".to_owned());
        names.stacks.push(("Ancient Red Dragon Wy".to_owned(), 3));
        names.stacks.push(("Goblin".to_owned(), 1));
        names
    }

    /// A line fits its cell limit and starts with what matters.
    fn clipped(text: &str, cells: usize, prefix: &str) {
        assert!(text.chars().count() <= cells, "{text}");
        assert!(text.starts_with(prefix), "{text}");
    }

    #[test]
    fn a_line_past_the_budget_is_cut_at_it() {
        let names = wide_names();
        let me = CharacterId(0);
        let order: Vec<(ActorRef, i64)> = (0..6)
            .map(|i| {
                let actor = if i % 2 == 0 {
                    ActorRef::Member(me)
                } else {
                    ActorRef::Stack(0)
                };
                (actor, 20 - i)
            })
            .collect();
        let lines = batch_lines(
            &[Event::Initiative {
                order,
                rolls: vec![],
            }],
            &names,
        );
        assert_eq!(
            lines[0].long.chars().count(),
            LONG_CELLS,
            "{}",
            lines[0].long
        );
        assert_eq!(lines[0].short.chars().count(), SHORT_CELLS);
        assert!(
            lines[0]
                .long
                .starts_with("Initiative: Brennagh-of-the-Long-Hall")
        );
    }

    #[test]
    fn names_follow_ids_and_stack_counts() {
        let names = wide_names();
        assert_eq!(names.member(CharacterId(1)), "Gormundsson-the-Younger");
        assert_eq!(names.member(CharacterId(9)), "?");
        assert_eq!(names.actor(&ActorRef::Stack(0)), "Ancient Red Dragon Wys");
        assert_eq!(names.actor(&ActorRef::Stack(1)), "Goblin");
        assert_eq!(names.actor(&ActorRef::Stack(2)), "Stack 2");
        assert_eq!(
            names.actor(&ActorRef::Monster { stack: 0, index: 2 }),
            "Ancient Red Dragon Wy 3"
        );
        assert_eq!(
            names.actor(&ActorRef::Monster { stack: 1, index: 0 }),
            "Goblin"
        );
    }

    fn attack_events() -> Vec<Event> {
        vec![
            Event::AttackResolved {
                attacker: ActorRef::Monster { stack: 1, index: 0 },
                target: ActorRef::Member(CharacterId(0)),
                roll: d20(RollMode::Normal, &[17], 17, 4, 0),
                ac: 16,
                hit: true,
                crit: false,
            },
            Event::Damage {
                target: ActorRef::Member(CharacterId(0)),
                kind: DamageType::Slashing,
                rolls: vec![trace(
                    Dice {
                        count: 1,
                        sides: 6,
                        modifier: 2,
                    },
                    &[3],
                )],
                raw: 5,
                amount: 5,
                adjust: DamageAdjust::None,
            },
            Event::AttackResolved {
                attacker: ActorRef::Member(CharacterId(1)),
                target: ActorRef::Monster { stack: 0, index: 1 },
                roll: d20(RollMode::Disadvantage, &[19, 2], 2, 3, 2),
                ac: 19,
                hit: false,
                crit: false,
            },
            Event::AttackResolved {
                attacker: ActorRef::Member(CharacterId(0)),
                target: ActorRef::Monster { stack: 1, index: 0 },
                roll: d20(RollMode::Advantage, &[20, 5], 20, 3, 2),
                ac: 15,
                hit: true,
                crit: true,
            },
            Event::Damage {
                target: ActorRef::Monster { stack: 1, index: 0 },
                kind: DamageType::Bludgeoning,
                rolls: vec![trace(Dice::new(1, 8), &[6]), trace(Dice::new(1, 8), &[2])],
                raw: 11,
                amount: 22,
                adjust: DamageAdjust::Vulnerable,
            },
        ]
    }

    #[test]
    fn an_attack_and_its_damage_make_one_line_with_the_math() {
        let events = attack_events();
        let lines = batch_lines(&events, &wide_names());
        assert_eq!(lines.len(), 3, "damage folds into its attack");
        clipped(
            &lines[0].long,
            LONG_CELLS,
            "Goblin hits Brennagh-of-the-Long-Hall for 5 (1d20+4",
        );
        clipped(
            &lines[0].short,
            SHORT_CELLS,
            "Goblin hits Brennagh-of-the-Long-Hall",
        );
        clipped(
            &lines[1].long,
            LONG_CELLS,
            "Gormundsson-the-Younger misses Ancient Red Dragon Wy",
        );
        clipped(
            &lines[2].short,
            SHORT_CELLS,
            "Brennagh-of-the-Long-Hall crits Goblin",
        );
    }

    #[test]
    fn short_names_show_the_whole_math() {
        let events = attack_events();
        let mut names = Names::default();
        names.members.insert(CharacterId(0), "Brenna".to_owned());
        names.members.insert(CharacterId(1), "Gorm".to_owned());
        names.stacks.push(("Wyrm".to_owned(), 2));
        names.stacks.push(("Goblin".to_owned(), 1));
        let lines = batch_lines(&events, &names);
        assert_eq!(
            lines[0].long,
            "Goblin hits Brenna for 5 (1d20+4 [17]=21 vs AC 16)"
        );
        assert_eq!(lines[0].short, "Goblin hits Brenna for 5");
        assert_eq!(
            lines[1].long,
            "Gorm misses Wyrm 2 (2d20 dis+5 [19, 2]=7 vs AC 19)"
        );
        assert_eq!(lines[1].short, "Gorm misses Wyrm 2");
        clipped(
            &lines[2].long,
            LONG_CELLS,
            "Brenna crits Goblin for 22 (doubled) (2d20 adv+5 [20",
        );
        assert_eq!(lines[2].short, "Brenna crits Goblin for 22 (doubled)");
        let alone = batch_lines(&events[4..], &names);
        clipped(
            &alone[0].long,
            LONG_CELLS,
            "Goblin takes 22 bludgeoning (doubled) (1d8 [6]=6 +",
        );
        assert_eq!(alone[0].short, "Goblin takes 22 bludgeoning (doubled)");
        let miss_then_damage = [events[2].clone(), events[4].clone()];
        assert_eq!(
            batch_lines(&miss_then_damage, &names).len(),
            2,
            "damage after a miss is not the miss's damage"
        );
    }

    /// Every event of the encounter phase, then of the fight, with the silent ones among them.
    fn arm_events() -> Vec<Event> {
        [encounter_arm_events(), fight_arm_events()].concat()
    }

    fn encounter_arm_events() -> Vec<Event> {
        let me = CharacterId(0);
        vec![
            Event::EncounterStarted {
                source: EncounterSource::Random,
                stacks: vec![(MonsterId(0), 3), (MonsterId(1), 1)],
                disposition: Disposition::Wary,
                counts: vec![],
                stealth: Some(d20(RollMode::Normal, &[14], 14, 2, 0)),
                perception: 13,
                noticed: false,
            },
            Event::Check {
                actor: ActorRef::Member(me),
                kind: CheckKind::Hide,
                roll: Some(d20(RollMode::Normal, &[12], 12, 3, 2)),
                dc: 14,
                success: true,
            },
            Event::Check {
                actor: ActorRef::Member(me),
                kind: CheckKind::Run,
                roll: None,
                dc: 0,
                success: true,
            },
            Event::Bribed { cost: 9999 },
        ]
    }

    fn fight_arm_events() -> Vec<Event> {
        let me = CharacterId(0);
        let dragon = ActorRef::Monster { stack: 0, index: 2 };
        vec![
            Event::CombatStarted {
                surprised: Surprise::Party,
            },
            Event::Initiative {
                order: vec![
                    (ActorRef::Member(me), 18),
                    (ActorRef::Stack(0), 15),
                    (ActorRef::Member(CharacterId(1)), 12),
                ],
                rolls: vec![],
            },
            Event::RoundStarted { round: 999 },
            Event::Turn {
                actor: ActorRef::Member(me),
            },
            Event::Turn {
                actor: ActorRef::Stack(0),
            },
            Event::EncounterCheck {
                roll: trace(Dice::new(1, 100), &[50]),
                chance: 3,
                fired: false,
            },
            Event::Waited {
                actor: ActorRef::Stack(0),
            },
            Event::Dodging {
                actor: ActorRef::Member(me),
            },
            Event::Exchanged { a: 0, b: 3 },
            Event::Down { target: me },
            Event::Wounded {
                member: me,
                failures: 2,
            },
            Event::DeathSave {
                member: me,
                roll: trace(Dice::new(1, 20), &[1]),
                result: DeathSaveResult::Failure,
                successes: 0,
                failures: 2,
            },
            Event::DeathSave {
                member: me,
                roll: trace(Dice::new(1, 20), &[20]),
                result: DeathSaveResult::Revived,
                successes: 0,
                failures: 0,
            },
            Event::Condition {
                target: ActorRef::Member(me),
                condition: ConditionId(0),
                applied: true,
            },
            Event::Death {
                target: dragon,
                gold: Some(trace(Dice::new(2, 4), &[4, 4])),
            },
            Event::Death {
                target: ActorRef::Member(me),
                gold: None,
            },
            Event::CombatEnded {
                outcome: CombatOutcome::Victory,
                xp: 99999,
                gold: 99999,
                fallen: vec![me, CharacterId(1)],
            },
            Event::CombatEnded {
                outcome: CombatOutcome::Defeat,
                xp: 0,
                gold: 0,
                fallen: vec![],
            },
            Event::PartyChanged,
        ]
    }

    #[test]
    fn every_arm_renders_within_its_cells() {
        let events = arm_events();
        let lines = batch_lines(&events, &wide_names());
        assert_eq!(
            lines.len(),
            events.len() - 4,
            "the turns, the encounter check, and PartyChanged are silent"
        );
        for line in &lines {
            assert!(line.long.chars().count() <= LONG_CELLS, "{}", line.long);
            assert!(line.short.chars().count() <= SHORT_CELLS, "{}", line.short);
            assert!(!line.short.is_empty());
            assert!(!line.long.ends_with(" to act"), "the header shows the turn");
        }
        clipped(
            &lines[0].short,
            SHORT_CELLS,
            "Ambush! 3 Ancient Red Dragon Wy, 1 Gob",
        );
        clipped(
            &lines[0].long,
            LONG_CELLS,
            "3 Ancient Red Dragon Wy, 1 Goblin, wary (stealth 1d",
        );
        clipped(
            &lines[1].long,
            LONG_CELLS,
            "Brennagh-of-the-Long-Hall hides: success (1d20+5",
        );
        clipped(
            &lines[2].long,
            LONG_CELLS,
            "Brennagh-of-the-Long-Hall runs: success (no roll",
        );
        assert_eq!(lines[3].long, "The party pays 9999 gold; they leave");
        assert_eq!(lines[4].long, "Ambush! The party is surprised");
        clipped(
            &lines[5].short,
            SHORT_CELLS,
            "Initiative: Brennagh-of-the-Long-Hall",
        );
        assert_eq!(lines[6].long, "Round 999");
        assert_eq!(lines[7].long, "Ancient Red Dragon Wys wait");
        assert_eq!(lines[8].long, "Brennagh-of-the-Long-Hall dodges");
        assert_eq!(lines[9].long, "Slots 1 and 4 exchange");
        assert_eq!(lines[10].long, "Brennagh-of-the-Long-Hall falls");
        clipped(
            &lines[11].long,
            LONG_CELLS,
            "Brennagh-of-the-Long-Hall is wounded: 2 of 3 fail",
        );
        clipped(
            &lines[12].long,
            LONG_CELLS,
            "Brennagh-of-the-Long-Hall death save: failure 2/3",
        );
        clipped(
            &lines[13].short,
            SHORT_CELLS,
            "Brennagh-of-the-Long-Hall comes to at",
        );
        assert_eq!(lines[14].long, "Brennagh-of-the-Long-Hall is ?");
        clipped(
            &lines[15].long,
            LONG_CELLS,
            "Ancient Red Dragon Wy 3 dies, dropping 8 gold (2d4 [",
        );
        clipped(
            &lines[15].short,
            SHORT_CELLS,
            "Ancient Red Dragon Wy 3 dies, dropping",
        );
        assert_eq!(lines[16].long, "Brennagh-of-the-Long-Hall dies");
        clipped(
            &lines[17].long,
            LONG_CELLS,
            "Victory! 99999 XP each, 99999 gold; lost: Brennagh",
        );
        assert_eq!(lines[18].long, "The party has fallen");
    }

    #[test]
    fn a_real_fight_reads_from_the_first_round_to_the_end() {
        let data = data();
        let mut world = facing_goblins(&data);
        let mut names = Names::new(&world, &data);
        let mut all = Vec::new();
        let attack = Command::Encounter(EncounterChoice::Attack);
        all.extend(batch_lines(
            &apply(&mut world, &data, attack).unwrap(),
            &names,
        ));
        for _ in 0..200 {
            if world.mode.kind() == ModeKind::Explore {
                break;
            }
            let stack = combat_view(&world, &data)
                .unwrap()
                .stacks
                .iter()
                .find(|s| s.alive && s.reachable)
                .map(|s| s.index)
                .expect("something to hit");
            let command = Command::Combat(CombatCommand::Attack { stack });
            let events = apply(&mut world, &data, command).unwrap_or_else(|r| panic!("{r}"));
            names.refresh(&world, &data);
            all.extend(batch_lines(&events, &names));
        }
        assert_eq!(world.mode.kind(), ModeKind::Explore, "the fight ended");
        let text: Vec<&str> = all.iter().map(|l| l.long.as_str()).collect();
        assert_eq!(text[0], "Combat!");
        assert!(text[1].starts_with("Initiative: "), "{}", text[1]);
        assert_eq!(text[2], "Round 1");
        assert!(
            text.iter()
                .any(|t| t.contains(" hits ") && t.contains(" vs AC ")),
            "{text:?}"
        );
        assert!(
            text.iter()
                .any(|t| t.starts_with("Goblin ") || t.starts_with("Giant Rat ")),
            "{text:?}"
        );
        assert!(text.iter().any(|t| t.contains(" dies")), "{text:?}");
        assert!(
            text.last()
                .is_some_and(|t| t.starts_with("Victory! ") || t == &"The party has fallen"),
            "{text:?}"
        );
        for line in &all {
            assert!(line.long.chars().count() <= LONG_CELLS);
            assert!(line.short.chars().count() <= SHORT_CELLS);
        }
        assert!(
            !text.iter().any(|t| t.contains('?')),
            "every id resolved to a name, including after the fight ended: {text:?}"
        );
    }
}
