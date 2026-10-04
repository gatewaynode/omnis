//! A reaction's conditions as the tactics panel writes them, Bevy-free (M7c step 7): what a
//! condition asks about (`Kind`), the parts picked from lists (`Field`), the lists themselves
//! (`Choices`, read from the packs), one condition as the form holds it (`Draft`) and its way
//! to and from the rules' `Predicate`, and criteria in words. `tactics_panel.rs` holds the
//! panel's controls and form.

use omnis_sim::omnis_core::{ConditionId, MonsterId};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_rules::{ActionRef, Cmp, Criteria, Predicate, Row, Trigger, Who};

/// A part of a condition that is picked from a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// Whose numbers: me or the trigger's subject.
    Who,
    /// Which monster.
    Monster,
    /// Which condition.
    Condition,
    /// The comparison.
    Cmp,
    /// Which row.
    Row,
}

/// What a condition asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Kind {
    /// How many of a monster stand.
    MonsterCount,
    /// A monster's share of those standing.
    MonsterShare,
    /// Hit points in percent.
    #[default]
    Hp,
    /// Spell points in percent.
    SpellPoints,
    /// A condition is on.
    HasCondition,
    /// Standing in a row.
    Row,
    /// The fight's round.
    Round,
    /// The reaction would turn a hit into a miss.
    WouldChangeOutcome,
}

impl Kind {
    /// Every kind, in menu order.
    pub const ALL: [Kind; 8] = [
        Kind::MonsterCount,
        Kind::MonsterShare,
        Kind::Hp,
        Kind::SpellPoints,
        Kind::HasCondition,
        Kind::Row,
        Kind::Round,
        Kind::WouldChangeOutcome,
    ];

    /// The menu's words.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Kind::MonsterCount => "monsters standing",
            Kind::MonsterShare => "monster share %",
            Kind::Hp => "hit points %",
            Kind::SpellPoints => "spell points %",
            Kind::HasCondition => "has condition",
            Kind::Row => "stands in row",
            Kind::Round => "round",
            Kind::WouldChangeOutcome => "turns a hit to a miss",
        }
    }

    /// The fields its row shows, in order.
    #[must_use]
    pub const fn fields(self) -> &'static [Field] {
        match self {
            Kind::MonsterCount | Kind::MonsterShare => &[Field::Monster, Field::Cmp],
            Kind::Hp | Kind::SpellPoints => &[Field::Who, Field::Cmp],
            Kind::HasCondition => &[Field::Who, Field::Condition],
            Kind::Row => &[Field::Who, Field::Row],
            Kind::Round => &[Field::Cmp],
            Kind::WouldChangeOutcome => &[],
        }
    }

    /// The largest number it takes; `None` when it takes none.
    #[must_use]
    pub const fn number_max(self) -> Option<u16> {
        match self {
            Kind::MonsterCount | Kind::Round => Some(u16::MAX),
            Kind::MonsterShare | Kind::Hp | Kind::SpellPoints => Some(100),
            Kind::HasCondition | Kind::Row | Kind::WouldChangeOutcome => None,
        }
    }
}

const CMPS: [Cmp; 5] = [Cmp::Lt, Cmp::Le, Cmp::Eq, Cmp::Ge, Cmp::Gt];
const WHOS: [Who; 2] = [Who::Me, Who::Subject];
const ROWS: [Row; 2] = [Row::Front, Row::Back];

/// The monsters and conditions a condition may name, with their names, in id order; and the
/// names of the actions a member may declare (the views name them by key).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choices {
    /// Every loaded monster.
    pub monsters: Vec<(MonsterId, String)>,
    /// Every loaded condition.
    pub conditions: Vec<(ConditionId, String)>,
    /// Every spell and item as an action, with its name.
    pub actions: Vec<(ActionRef, String)>,
}

impl Choices {
    /// Read the lists from the packs.
    #[must_use]
    pub fn from_data(data: &Data) -> Self {
        Choices {
            monsters: data
                .monsters
                .iter()
                .map(|(id, m)| (*id, data.label("en", &m.name).to_owned()))
                .collect(),
            conditions: data
                .conditions
                .iter()
                .map(|(id, c)| (*id, data.label("en", &c.name).to_owned()))
                .collect(),
            actions: data
                .spells
                .iter()
                .map(|(id, s)| (ActionRef::Spell(*id), data.label("en", &s.name).to_owned()))
                .chain(
                    data.items.iter().map(|(id, i)| {
                        (ActionRef::Item(*id), data.label("en", &i.name).to_owned())
                    }),
                )
                .collect(),
        }
    }

    /// An action's name: a spell's or item's own, otherwise what the view called it.
    #[must_use]
    pub fn action_name(&self, action: &ActionRef, called: &str) -> String {
        self.actions
            .iter()
            .find(|(a, _)| a == action)
            .map_or(called, |(_, name)| name.as_str())
            .to_owned()
    }

    /// A field's options, as the menu lists them.
    #[must_use]
    pub fn options(&self, field: Field) -> Vec<String> {
        match field {
            Field::Who => WHOS.iter().map(|w| who_name(*w).to_owned()).collect(),
            Field::Monster => self.monsters.iter().map(|(_, n)| n.clone()).collect(),
            Field::Condition => self.conditions.iter().map(|(_, n)| n.clone()).collect(),
            Field::Cmp => CMPS.iter().map(|c| cmp_name(*c).to_owned()).collect(),
            Field::Row => ROWS.iter().map(|r| row_name(*r).to_owned()).collect(),
        }
    }
}

/// One condition as the form holds it: every field any kind needs, so a kind change keeps
/// what still applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Draft {
    /// What it asks about.
    pub kind: Kind,
    /// Row in `WHOS`.
    pub who: usize,
    /// Row in `Choices::monsters`.
    pub monster: usize,
    /// Row in `Choices::conditions`.
    pub condition: usize,
    /// Row in `CMPS`.
    pub cmp: usize,
    /// Row in `ROWS`.
    pub row: usize,
    /// The number, within the kind's range.
    pub n: u16,
}

impl Draft {
    /// The predicate it stands for; `None` when a list it names is empty.
    #[must_use]
    pub fn predicate(&self, choices: &Choices) -> Option<Predicate> {
        let who = WHOS[self.who % WHOS.len()];
        let cmp = CMPS[self.cmp % CMPS.len()];
        let percent = u8::try_from(self.n.min(100)).unwrap_or(100);
        Some(match self.kind {
            Kind::MonsterCount => Predicate::MonsterCount {
                monster: choices.monsters.get(self.monster)?.0,
                cmp,
                n: self.n,
            },
            Kind::MonsterShare => Predicate::MonsterShare {
                monster: choices.monsters.get(self.monster)?.0,
                cmp,
                percent,
            },
            Kind::Hp => Predicate::Hp { who, cmp, percent },
            Kind::SpellPoints => Predicate::SpellPoints { who, cmp, percent },
            Kind::HasCondition => Predicate::HasCondition {
                who,
                condition: choices.conditions.get(self.condition)?.0,
            },
            Kind::Row => Predicate::Row {
                who,
                row: ROWS[self.row % ROWS.len()],
            },
            Kind::Round => Predicate::Round { cmp, n: self.n },
            Kind::WouldChangeOutcome => Predicate::WouldChangeOutcome,
        })
    }

    /// The draft a predicate loads as; `None` when it names a monster or condition not loaded.
    #[must_use]
    pub fn of(predicate: &Predicate, choices: &Choices) -> Option<Draft> {
        let monster = |id: MonsterId| choices.monsters.iter().position(|(m, _)| *m == id);
        let mut draft = Draft::default();
        match *predicate {
            Predicate::MonsterCount { monster: m, cmp, n } => {
                (draft.kind, draft.monster, draft.cmp, draft.n) =
                    (Kind::MonsterCount, monster(m)?, at(&CMPS, cmp), n);
            }
            Predicate::MonsterShare {
                monster: m,
                cmp,
                percent,
            } => {
                (draft.kind, draft.monster, draft.cmp, draft.n) = (
                    Kind::MonsterShare,
                    monster(m)?,
                    at(&CMPS, cmp),
                    u16::from(percent),
                );
            }
            Predicate::Hp { who, cmp, percent } | Predicate::SpellPoints { who, cmp, percent } => {
                draft.kind = if matches!(predicate, Predicate::Hp { .. }) {
                    Kind::Hp
                } else {
                    Kind::SpellPoints
                };
                (draft.who, draft.cmp, draft.n) =
                    (at(&WHOS, who), at(&CMPS, cmp), u16::from(percent));
            }
            Predicate::HasCondition { who, condition } => {
                draft.kind = Kind::HasCondition;
                draft.who = at(&WHOS, who);
                draft.condition = choices
                    .conditions
                    .iter()
                    .position(|(c, _)| *c == condition)?;
            }
            Predicate::Row { who, row } => {
                (draft.kind, draft.who, draft.row) = (Kind::Row, at(&WHOS, who), at(&ROWS, row));
            }
            Predicate::Round { cmp, n } => {
                (draft.kind, draft.cmp, draft.n) = (Kind::Round, at(&CMPS, cmp), n);
            }
            Predicate::WouldChangeOutcome => draft.kind = Kind::WouldChangeOutcome,
        }
        Some(draft)
    }

    /// The caption of one of its fields.
    #[must_use]
    pub fn caption(&self, field: Field, choices: &Choices) -> String {
        let pick = |list: Vec<String>, at: usize| list.get(at).cloned().unwrap_or_default();
        match field {
            Field::Who => pick(choices.options(field), self.who),
            Field::Monster => pick(choices.options(field), self.monster),
            Field::Condition => pick(choices.options(field), self.condition),
            Field::Cmp => pick(choices.options(field), self.cmp),
            Field::Row => pick(choices.options(field), self.row),
        }
    }
}

/// Where `x` stands in `list`, or the first row.
fn at<T: PartialEq + Copy>(list: &[T], x: T) -> usize {
    list.iter().position(|y| *y == x).unwrap_or(0)
}

/// A trigger in words.
#[must_use]
pub const fn trigger_name(trigger: Trigger) -> &'static str {
    match trigger {
        Trigger::SpellCast => "a member casts",
        Trigger::Attacked => "attacked",
        Trigger::MemberAttacked => "a member attacked",
        Trigger::MemberWounded => "a member wounded",
        Trigger::MemberDying => "a member dying",
        Trigger::EnemyFlees => "an enemy flees",
        Trigger::EnemyCasts => "an enemy casts",
        Trigger::OwnTurn => "own turn",
    }
}

const fn who_name(who: Who) -> &'static str {
    match who {
        Who::Me => "me",
        Who::Subject => "the subject",
    }
}

const fn cmp_name(cmp: Cmp) -> &'static str {
    match cmp {
        Cmp::Lt => "<",
        Cmp::Le => "<=",
        Cmp::Eq => "=",
        Cmp::Ge => ">=",
        Cmp::Gt => ">",
    }
}

const fn row_name(row: Row) -> &'static str {
    match row {
        Row::Front => "front",
        Row::Back => "back",
    }
}

/// A predicate in words.
fn predicate_text(predicate: &Predicate, choices: &Choices) -> String {
    let monster = |id: MonsterId| {
        choices
            .monsters
            .iter()
            .find(|(m, _)| *m == id)
            .map_or("?", |(_, n)| n.as_str())
            .to_owned()
    };
    match predicate {
        Predicate::MonsterCount { monster: m, cmp, n } => {
            format!("{} standing {} {n}", monster(*m), cmp_name(*cmp))
        }
        Predicate::MonsterShare {
            monster: m,
            cmp,
            percent,
        } => format!("{} share {} {percent}%", monster(*m), cmp_name(*cmp)),
        Predicate::Hp { who, cmp, percent } => {
            format!("{} HP {} {percent}%", who_name(*who), cmp_name(*cmp))
        }
        Predicate::SpellPoints { who, cmp, percent } => {
            format!("{} SP {} {percent}%", who_name(*who), cmp_name(*cmp))
        }
        Predicate::HasCondition { who, condition } => {
            let name = choices
                .conditions
                .iter()
                .find(|(c, _)| c == condition)
                .map_or("?", |(_, n)| n.as_str());
            format!("{} {name}", who_name(*who))
        }
        Predicate::Row { who, row } => format!("{} in the {} row", who_name(*who), row_name(*row)),
        Predicate::Round { cmp, n } => format!("round {} {n}", cmp_name(*cmp)),
        Predicate::WouldChangeOutcome => "it turns a hit to a miss".to_owned(),
    }
}

/// A criteria tree in words; a deeper one in brackets.
#[must_use]
pub fn criteria_text(criteria: &Criteria, choices: &Choices) -> String {
    match criteria {
        Criteria::Always => "always".to_owned(),
        Criteria::Is(p) => predicate_text(p, choices),
        Criteria::All(list) | Criteria::Any(list) => {
            let join = if matches!(criteria, Criteria::Any(_)) {
                " or "
            } else {
                " and "
            };
            list.iter()
                .map(|c| match c {
                    Criteria::Is(p) => predicate_text(p, choices),
                    deeper => format!("({})", criteria_text(deeper, choices)),
                })
                .collect::<Vec<_>>()
                .join(join)
        }
    }
}
