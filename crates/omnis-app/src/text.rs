//! The vocabulary every event-text file shares: the names events refer to by id, the two
//! lengths of a line, and the roll math as text. `combat_text.rs`, `spell_text.rs` and
//! `item_text.rs` build on it. Bevy-free.

use crate::font::fit;
use omnis_sim::omnis_core::{CharacterId, ConditionId, ItemId, RollTrace, SpellId};
use omnis_sim::omnis_data::Data;
use omnis_sim::{ActorRef, Mode, World};
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
    pub(crate) members: BTreeMap<CharacterId, String>,
    /// Label and initial count per stack index.
    pub(crate) stacks: Vec<(String, u8)>,
    conditions: BTreeMap<ConditionId, String>,
    spells: BTreeMap<SpellId, String>,
    items: BTreeMap<ItemId, String>,
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
        if self.items.is_empty() {
            for (id, item) in &data.items {
                self.items
                    .insert(*id, data.label("en", &item.name).to_owned());
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

    /// An item's name.
    #[must_use]
    pub fn item(&self, id: ItemId) -> &str {
        self.items.get(&id).map_or("?", String::as_str)
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

/// `1d8+2 [5]=7`: the trace without its stream name.
#[must_use]
pub fn trace_math(trace: &RollTrace) -> String {
    format!("{} {}={}", trace.dice, faces(trace), trace.total)
}

pub(crate) fn faces(trace: &RollTrace) -> String {
    let faces = trace
        .rolls
        .iter()
        .map(|r| r.value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{faces}]")
}
