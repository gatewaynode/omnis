//! Tactics (PRD §7.9, D22, D23; ARCHITECTURE.md §4.7): what a combatant does when the player is
//! not choosing. The whole data shape is here from the start (the reactions switch, the auto
//! flag, the criteria library, runbooks and the default), so later work needs no migration;
//! M7c builds the reactions. A criteria tree is data the simulation walks over integers the
//! world supplies (`Facts`), never script, so tactics add no attack surface (PRD R8).

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Debug;
use omnis_core::{ConditionId, ItemId, MonsterId, SpellId};
use omnis_data::Data;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// What a criteria set names things by: the registry's numbers in the world and its save
/// ([`Ids`], the default), or the packs' string ids on the protocol ([`Named`]; protocol 2 lets
/// no registry number cross it). [`CriteriaSet::map`] turns one into the other.
pub trait Names {
    /// A spell.
    type Spell: Clone + Debug + Ord + Serialize + DeserializeOwned;
    /// An item.
    type Item: Clone + Debug + Ord + Serialize + DeserializeOwned;
    /// A monster.
    type Monster: Clone + Debug + Ord + Serialize + DeserializeOwned;
    /// A condition.
    type Condition: Clone + Debug + Ord + Serialize + DeserializeOwned;
}

/// The registry's numbers: what the world keeps and walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ids;

impl Names for Ids {
    type Spell = SpellId;
    type Item = ItemId;
    type Monster = MonsterId;
    type Condition = ConditionId;
}

/// The packs' string ids (`base:spell:shield`): what a command carries and a view shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Named;

impl Names for Named {
    type Spell = String;
    type Item = String;
    type Monster = String;
    type Condition = String;
}

/// How each kind of name in a criteria set becomes the other kind; `Err` carries the name that
/// did not, as text.
pub trait Rename<N: Names, M: Names> {
    /// A spell's.
    fn spell(&self, spell: &N::Spell) -> Result<M::Spell, String>;
    /// An item's.
    fn item(&self, item: &N::Item) -> Result<M::Item, String>;
    /// A monster's.
    fn monster(&self, monster: &N::Monster) -> Result<M::Monster, String>;
    /// A condition's.
    fn condition(&self, condition: &N::Condition) -> Result<M::Condition, String>;
}

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

/// Both renamings over the loaded packs: registry numbers to string ids for a view, string ids
/// to registry numbers for a command. A name the packs do not hold does not rename.
#[derive(Debug, Clone, Copy)]
pub struct Naming<'a>(pub &'a Data);

/// A registry number as text, for a name that does not rename.
fn unnamed(kind: &str, id: u32) -> String {
    alloc::format!("{kind} {id}")
}

impl Rename<Ids, Named> for Naming<'_> {
    fn spell(&self, spell: &SpellId) -> Result<String, String> {
        let reg = &self.0.registry.spells;
        reg.name(*spell)
            .map(String::from)
            .ok_or_else(|| unnamed("spell", spell.0))
    }
    fn item(&self, item: &ItemId) -> Result<String, String> {
        let reg = &self.0.registry.items;
        reg.name(*item)
            .map(String::from)
            .ok_or_else(|| unnamed("item", item.0))
    }
    fn monster(&self, monster: &MonsterId) -> Result<String, String> {
        let reg = &self.0.registry.monsters;
        reg.name(*monster)
            .map(String::from)
            .ok_or_else(|| unnamed("monster", monster.0))
    }
    fn condition(&self, condition: &ConditionId) -> Result<String, String> {
        let reg = &self.0.registry.conditions;
        reg.name(*condition)
            .map(String::from)
            .ok_or_else(|| unnamed("condition", condition.0))
    }
}

impl Rename<Named, Ids> for Naming<'_> {
    fn spell(&self, spell: &String) -> Result<SpellId, String> {
        self.0
            .registry
            .spells
            .get(spell)
            .ok_or_else(|| spell.clone())
    }
    fn item(&self, item: &String) -> Result<ItemId, String> {
        self.0.registry.items.get(item).ok_or_else(|| item.clone())
    }
    fn monster(&self, monster: &String) -> Result<MonsterId, String> {
        self.0
            .registry
            .monsters
            .get(monster)
            .ok_or_else(|| monster.clone())
    }
    fn condition(&self, condition: &String) -> Result<ConditionId, String> {
        self.0
            .registry
            .conditions
            .get(condition)
            .ok_or_else(|| condition.clone())
    }
}

/// A named trigger and condition for one action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct CriteriaSet<N: Names = Ids> {
    /// Player text.
    pub name: String,
    /// What it does.
    pub action: ActionRef<N>,
    /// When it is considered.
    pub trigger: Trigger,
    /// What must hold.
    pub when: Criteria<N>,
}

impl<N: Names> CriteriaSet<N> {
    /// The name and the tree's shape.
    pub fn check(&self) -> Result<(), TacticsFault> {
        check_name(&self.name)?;
        self.when.check()
    }

    /// The same set naming things the other way.
    ///
    /// # Errors
    /// The first name that does not rename, as text.
    pub fn map<M: Names>(&self, rename: &impl Rename<N, M>) -> Result<CriteriaSet<M>, String> {
        Ok(CriteriaSet {
            name: self.name.clone(),
            action: self.action.map(rename)?,
            trigger: self.trigger,
            when: self.when.map(rename)?,
        })
    }
}

/// An action a criteria set fires.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum ActionRef<N: Names = Ids> {
    /// A weapon attack (an opportunity attack as a reaction).
    Attack,
    /// A known spell.
    Spell(N::Spell),
    /// A carried item.
    Item(N::Item),
    /// A class feature, by its name key.
    Feature(String),
}

impl<N: Names> ActionRef<N> {
    /// The same action naming things the other way.
    ///
    /// # Errors
    /// The name that does not rename, as text.
    pub fn map<M: Names>(&self, rename: &impl Rename<N, M>) -> Result<ActionRef<M>, String> {
        Ok(match self {
            ActionRef::Attack => ActionRef::Attack,
            ActionRef::Spell(spell) => ActionRef::Spell(rename.spell(spell)?),
            ActionRef::Item(item) => ActionRef::Item(rename.item(item)?),
            ActionRef::Feature(key) => ActionRef::Feature(key.clone()),
        })
    }
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
#[serde(bound = "")]
pub enum Criteria<N: Names = Ids> {
    /// Holds.
    Always,
    /// Every child holds.
    All(Vec<Criteria<N>>),
    /// Some child holds.
    Any(Vec<Criteria<N>>),
    /// One predicate.
    Is(Predicate<N>),
}

impl<N: Names> Criteria<N> {
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

    /// The same tree naming things the other way.
    ///
    /// # Errors
    /// The first name that does not rename, as text.
    pub fn map<M: Names>(&self, rename: &impl Rename<N, M>) -> Result<Criteria<M>, String> {
        let all = |children: &Vec<Criteria<N>>| {
            children
                .iter()
                .map(|c| c.map(rename))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(match self {
            Criteria::Always => Criteria::Always,
            Criteria::All(children) => Criteria::All(all(children)?),
            Criteria::Any(children) => Criteria::Any(all(children)?),
            Criteria::Is(predicate) => Criteria::Is(predicate.map(rename)?),
        })
    }
}

impl Criteria {
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
#[serde(bound = "")]
pub enum Predicate<N: Names = Ids> {
    /// How many living monsters of a kind stand there.
    MonsterCount {
        /// The kind.
        monster: N::Monster,
        /// The comparison.
        cmp: Cmp,
        /// Against.
        n: u16,
    },
    /// A kind's share of the living monsters, in percent.
    MonsterShare {
        /// The kind.
        monster: N::Monster,
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
        condition: N::Condition,
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

impl<N: Names> Predicate<N> {
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

    /// The same predicate naming things the other way.
    ///
    /// # Errors
    /// The name that does not rename, as text.
    pub fn map<M: Names>(&self, rename: &impl Rename<N, M>) -> Result<Predicate<M>, String> {
        Ok(match self {
            Predicate::MonsterCount { monster, cmp, n } => Predicate::MonsterCount {
                monster: rename.monster(monster)?,
                cmp: *cmp,
                n: *n,
            },
            Predicate::MonsterShare {
                monster,
                cmp,
                percent,
            } => Predicate::MonsterShare {
                monster: rename.monster(monster)?,
                cmp: *cmp,
                percent: *percent,
            },
            Predicate::Hp { who, cmp, percent } => Predicate::Hp {
                who: *who,
                cmp: *cmp,
                percent: *percent,
            },
            Predicate::SpellPoints { who, cmp, percent } => Predicate::SpellPoints {
                who: *who,
                cmp: *cmp,
                percent: *percent,
            },
            Predicate::HasCondition { who, condition } => Predicate::HasCondition {
                who: *who,
                condition: rename.condition(condition)?,
            },
            Predicate::Row { who, row } => Predicate::Row {
                who: *who,
                row: *row,
            },
            Predicate::Round { cmp, n } => Predicate::Round { cmp: *cmp, n: *n },
            Predicate::WouldChangeOutcome => Predicate::WouldChangeOutcome,
        })
    }
}

impl Predicate {
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
