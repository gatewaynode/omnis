//! Tactics (PRD §7.9, D22, D23; ARCHITECTURE.md §4.7): what a combatant does when the player is
//! not choosing. The whole data shape is here from the start (the reactions switch, the auto
//! flag, the criteria library, runbooks and the default), so later work needs no migration;
//! M7c builds the reactions. A criteria tree is data the simulation walks over integers the
//! world supplies (`Facts`), never script, so tactics add no attack surface (PRD R8).

use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{ConditionId, ItemId, MonsterId, SpellId};
use serde::{Deserialize, Serialize};

/// Bytes in a criteria set's or a runbook's name.
pub const TACTICS_NAME_BYTES: usize = 32;
/// Nesting depth of a criteria tree, `Always` or `Is` at depth 1.
pub const CRITERIA_DEPTH: usize = 4;
/// Nodes in a criteria tree.
pub const CRITERIA_NODES: usize = 16;
/// Criteria sets a member keeps.
pub const LIBRARY_SETS: usize = 32;
/// Runbooks a member keeps.
pub const RUNBOOKS: usize = 8;
/// Entries in one runbook.
pub const RUNBOOK_ENTRIES: usize = 16;

/// A member's tactics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tactics {
    /// The in-fight switch: off, no reaction fires. Flipping it costs nothing.
    pub reactions_on: bool,
    /// The runbook takes this member's turns (stored; not built in M7c).
    #[serde(default)]
    pub auto: bool,
    /// Every criteria set built so far, offered again in every runbook.
    #[serde(default)]
    pub library: Vec<CriteriaSet>,
    /// The runbooks; never empty.
    pub runbooks: Vec<Runbook>,
    /// Index of the default runbook.
    #[serde(default)]
    pub default_runbook: u8,
}

impl Default for Tactics {
    /// Reactions on, nothing declared, one empty default runbook.
    fn default() -> Tactics {
        Tactics {
            reactions_on: true,
            auto: false,
            library: Vec::new(),
            runbooks: alloc::vec![Runbook {
                name: String::from("Default"),
                when: None,
                entries: Vec::new(),
            }],
            default_runbook: 0,
        }
    }
}

impl Tactics {
    /// The runbook in use: the default (encounter criteria are not built).
    #[must_use]
    pub fn active(&self) -> Option<&Runbook> {
        self.runbooks.get(usize::from(self.default_runbook))
    }

    /// The criteria sets the active runbook names, in order.
    #[must_use]
    pub fn reactions(&self) -> Vec<&CriteriaSet> {
        self.active()
            .map(|book| {
                book.entries
                    .iter()
                    .filter_map(|(_, set)| self.library.get(usize::from(*set)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether the shape holds: a default that exists and the caps of this file.
    pub fn check(&self) -> Result<(), TacticsFault> {
        if self.library.len() > LIBRARY_SETS {
            return Err(TacticsFault::TooMany);
        }
        if self.runbooks.is_empty() || self.runbooks.len() > RUNBOOKS {
            return Err(TacticsFault::TooMany);
        }
        if usize::from(self.default_runbook) >= self.runbooks.len() {
            return Err(TacticsFault::NoDefault);
        }
        for set in &self.library {
            set.check()?;
        }
        for book in &self.runbooks {
            check_name(&book.name)?;
            if book.entries.len() > RUNBOOK_ENTRIES {
                return Err(TacticsFault::TooMany);
            }
            if let Some(when) = &book.when {
                when.check()?;
            }
            if book
                .entries
                .iter()
                .any(|(_, set)| usize::from(*set) >= self.library.len())
            {
                return Err(TacticsFault::NoSuchSet);
            }
        }
        Ok(())
    }
}

/// A named plan: for each action considered, in order, the one criteria set evaluated for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runbook {
    /// Player text.
    pub name: String,
    /// Encounter criteria choosing this runbook (not built; the default is used).
    #[serde(default)]
    pub when: Option<Criteria>,
    /// The actions in order of consideration, each with its set's index in the library.
    #[serde(default)]
    pub entries: Vec<(ActionRef, u16)>,
}

/// Why a tactics shape is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TacticsFault {
    /// A name is empty, longer than `TACTICS_NAME_BYTES`, or carries a control character.
    Name,
    /// A tree deeper than `CRITERIA_DEPTH` or larger than `CRITERIA_NODES`.
    TooDeep,
    /// More sets, runbooks or entries than the caps allow.
    TooMany,
    /// A percentage above 100.
    Percent,
    /// An entry names a set the library does not hold.
    NoSuchSet,
    /// The default names no runbook.
    NoDefault,
}

fn check_name(name: &str) -> Result<(), TacticsFault> {
    if name.trim().is_empty()
        || name.len() > TACTICS_NAME_BYTES
        || name.chars().any(char::is_control)
    {
        return Err(TacticsFault::Name);
    }
    Ok(())
}

/// A named trigger and condition for one action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriteriaSet {
    /// Player text.
    pub name: String,
    /// What it does.
    pub action: ActionRef,
    /// When it is considered.
    pub trigger: Trigger,
    /// What must hold.
    pub when: Criteria,
}

impl CriteriaSet {
    /// The name and the tree's shape.
    pub fn check(&self) -> Result<(), TacticsFault> {
        check_name(&self.name)?;
        self.when.check()
    }
}

/// An action a criteria set fires.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionRef {
    /// A weapon attack (an opportunity attack as a reaction).
    Attack,
    /// A known spell.
    Spell(SpellId),
    /// A carried item.
    Item(ItemId),
    /// A class feature, by its name key.
    Feature(String),
}

/// The closed list of moments the simulation raises (ARCHITECTURE.md §4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Trigger {
    /// A member cast a spell.
    SpellCast,
    /// This member is attacked, before the roll is judged.
    Attacked,
    /// A member in this member's row is attacked.
    MemberAttacked,
    /// A member in this member's row took damage.
    MemberWounded,
    /// A member in this member's row fell to zero hit points.
    MemberDying,
    /// A stack runs (no monster flees yet).
    EnemyFlees,
    /// A monster casts (no monster casts yet).
    EnemyCasts,
    /// The member's own turn (auto play; not built).
    OwnTurn,
}

impl Trigger {
    /// Every trigger, in order.
    pub const ALL: [Trigger; 8] = [
        Trigger::SpellCast,
        Trigger::Attacked,
        Trigger::MemberAttacked,
        Trigger::MemberWounded,
        Trigger::MemberDying,
        Trigger::EnemyFlees,
        Trigger::EnemyCasts,
        Trigger::OwnTurn,
    ];
}

/// A condition tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Criteria {
    /// Holds.
    Always,
    /// Every child holds.
    All(Vec<Criteria>),
    /// Some child holds.
    Any(Vec<Criteria>),
    /// One predicate.
    Is(Predicate),
}

impl Criteria {
    /// Depth, size and percentages within the caps.
    pub fn check(&self) -> Result<(), TacticsFault> {
        let mut nodes = 0;
        self.walk(1, &mut nodes)
    }

    fn walk(&self, depth: usize, nodes: &mut usize) -> Result<(), TacticsFault> {
        *nodes += 1;
        if depth > CRITERIA_DEPTH || *nodes > CRITERIA_NODES {
            return Err(TacticsFault::TooDeep);
        }
        match self {
            Criteria::Always => Ok(()),
            Criteria::All(children) | Criteria::Any(children) => {
                for child in children {
                    child.walk(depth + 1, nodes)?;
                }
                Ok(())
            }
            Criteria::Is(predicate) => predicate.check(),
        }
    }

    /// Whether the tree holds over `facts`.
    #[must_use]
    pub fn holds(&self, facts: &Facts) -> bool {
        match self {
            Criteria::Always => true,
            Criteria::All(children) => children.iter().all(|c| c.holds(facts)),
            Criteria::Any(children) => children.iter().any(|c| c.holds(facts)),
            Criteria::Is(predicate) => predicate.holds(facts),
        }
    }
}

/// A comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cmp {
    /// Less than.
    Lt,
    /// At most.
    Le,
    /// Equal.
    Eq,
    /// At least.
    Ge,
    /// More than.
    Gt,
}

impl Cmp {
    /// `left` against `right`.
    #[must_use]
    pub const fn test(self, left: i64, right: i64) -> bool {
        match self {
            Cmp::Lt => left < right,
            Cmp::Le => left <= right,
            Cmp::Eq => left == right,
            Cmp::Ge => left >= right,
            Cmp::Gt => left > right,
        }
    }
}

/// Whom a predicate reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Who {
    /// The member whose tactics these are.
    Me,
    /// The member the trigger is about (attacked, wounded, dying, casting); the member when the
    /// trigger is about no one else.
    Subject,
}

/// A party row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Row {
    /// The front row.
    Front,
    /// The back row.
    Back,
}

/// What a criteria tree can ask. Integers only; the system offers these, the player composes
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Predicate {
    /// How many living monsters of a kind stand there.
    MonsterCount {
        /// The kind.
        monster: MonsterId,
        /// The comparison.
        cmp: Cmp,
        /// Against.
        n: u16,
    },
    /// A kind's share of the living monsters, in percent.
    MonsterShare {
        /// The kind.
        monster: MonsterId,
        /// The comparison.
        cmp: Cmp,
        /// Against, 0..=100.
        percent: u8,
    },
    /// Hit points as a percentage of the maximum.
    Hp {
        /// Whose.
        who: Who,
        /// The comparison.
        cmp: Cmp,
        /// Against, 0..=100.
        percent: u8,
    },
    /// Spell points as a percentage of the maximum (0 for a member with none).
    SpellPoints {
        /// Whose.
        who: Who,
        /// The comparison.
        cmp: Cmp,
        /// Against, 0..=100.
        percent: u8,
    },
    /// A condition is on.
    HasCondition {
        /// Whom.
        who: Who,
        /// Which.
        condition: ConditionId,
    },
    /// Standing in a row.
    Row {
        /// Who.
        who: Who,
        /// Which.
        row: Row,
    },
    /// The fight's round.
    Round {
        /// The comparison.
        cmp: Cmp,
        /// Against.
        n: u16,
    },
    /// The action would turn the attack that triggered it from a hit into a miss (the M6
    /// shield rule, now the player's choice).
    WouldChangeOutcome,
}

impl Predicate {
    fn check(&self) -> Result<(), TacticsFault> {
        match self {
            Predicate::MonsterShare { percent, .. }
            | Predicate::Hp { percent, .. }
            | Predicate::SpellPoints { percent, .. }
                if *percent > 100 =>
            {
                Err(TacticsFault::Percent)
            }
            _ => Ok(()),
        }
    }

    fn holds(&self, facts: &Facts) -> bool {
        match self {
            Predicate::MonsterCount { monster, cmp, n } => {
                cmp.test(facts.count(*monster), i64::from(*n))
            }
            Predicate::MonsterShare {
                monster,
                cmp,
                percent,
            } => {
                let total: i64 = facts.monsters.iter().map(|(_, n)| i64::from(*n)).sum();
                let share = if total == 0 {
                    0
                } else {
                    facts.count(*monster) * 100 / total
                };
                cmp.test(share, i64::from(*percent))
            }
            Predicate::Hp { who, cmp, percent } => {
                cmp.test(facts.of(*who).hp_percent, i64::from(*percent))
            }
            Predicate::SpellPoints { who, cmp, percent } => {
                cmp.test(facts.of(*who).sp_percent, i64::from(*percent))
            }
            Predicate::HasCondition { who, condition } => {
                facts.of(*who).conditions.contains(condition)
            }
            Predicate::Row { who, row } => facts.of(*who).row == *row,
            Predicate::Round { cmp, n } => cmp.test(i64::from(facts.round), i64::from(*n)),
            Predicate::WouldChangeOutcome => facts.would_change,
        }
    }
}

/// What a predicate reads about one member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFacts {
    /// Hit points as a percentage of the maximum.
    pub hp_percent: i64,
    /// Spell points as a percentage of the maximum.
    pub sp_percent: i64,
    /// Conditions on.
    pub conditions: Vec<ConditionId>,
    /// The row.
    pub row: Row,
}

/// The integers a criteria tree is walked over, built by the simulation at a trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The member whose tactics these are.
    pub me: MemberFacts,
    /// The member the trigger is about.
    pub subject: MemberFacts,
    /// Living monsters by kind.
    pub monsters: Vec<(MonsterId, u16)>,
    /// The round.
    pub round: u32,
    /// Whether the action turns the triggering hit into a miss.
    pub would_change: bool,
}

impl Facts {
    fn of(&self, who: Who) -> &MemberFacts {
        match who {
            Who::Me => &self.me,
            Who::Subject => &self.subject,
        }
    }

    fn count(&self, monster: MonsterId) -> i64 {
        self.monsters
            .iter()
            .filter(|(m, _)| *m == monster)
            .map(|(_, n)| i64::from(*n))
            .sum()
    }
}
