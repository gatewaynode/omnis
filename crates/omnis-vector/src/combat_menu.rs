//! The fight's choices (alt-ARCHITECTURE.md §9): the encounter's four, then a member's turn
//! (Attack, Cast, Use, Dodge, Swap, Flee) with the spell and item lists and the targets.
//! Bevy-free.
//!
//! Every command the menu offers is tried on a copy of the world first (`trial`), so a choice
//! the simulation would refuse is shown blocked with its reason and never sent. Targets are
//! found the same way: a stack or member is offered for an action only when the simulation
//! accepts that action on it, so the menu holds no rules of its own.

use crate::trial::{accepted, refusal};
use omnis_sim::omnis_data::Data;
use omnis_sim::{
    ActorRef, CombatCommand, CombatView, Command, EncounterChoice, ModeKind, Target, World,
    bribe_cost, combat_view,
};

/// An action waiting for its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Attack a stack.
    Attack,
    /// Cast the spell at this index of the caster's list.
    Cast(u8),
    /// Use the item at this row of the acting member's kit.
    Use(u8),
    /// Swap places with another member.
    Swap,
}

/// Where the menu is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Step {
    /// The turn's actions, or the encounter's choices.
    #[default]
    Top,
    /// The acting member's spells.
    Spells,
    /// The acting member's usable items.
    Items,
    /// The targets for an action.
    Target(Action),
}

/// What a menu entry does.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    /// Send this command.
    Command(Command),
    /// Go to another step.
    Open(Step),
    /// Go back a step.
    Back,
}

/// One entry of the menu.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The label.
    pub label: String,
    /// What it does.
    pub act: Act,
    /// Why it cannot be chosen now, if it cannot.
    pub blocked: Option<String>,
}

/// Something clicked on the screen: a stack by its index in the encounter, or a member by
/// marching-order slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pick {
    /// A stack.
    Stack(u8),
    /// A member.
    Member(u8),
}

/// The command `action` makes on `pick`, if that kind of target fits the action.
#[must_use]
pub fn command(action: Action, pick: Pick) -> Option<Command> {
    let combat = match (action, pick) {
        (Action::Attack, Pick::Stack(stack)) => CombatCommand::Attack { stack },
        (Action::Cast(spell), Pick::Stack(s)) => CombatCommand::Cast {
            spell,
            target: Target::Stack(s),
        },
        (Action::Cast(spell), Pick::Member(m)) => CombatCommand::Cast {
            spell,
            target: Target::Member(m),
        },
        (Action::Use(item), Pick::Member(m)) => CombatCommand::Use {
            item,
            target: Some(m),
        },
        (Action::Swap, Pick::Member(with)) => CombatCommand::Exchange { with },
        _ => return None,
    };
    Some(Command::Combat(combat))
}

/// Every stack still standing and every member, the candidates for any action.
fn candidates(world: &World, view: &CombatView) -> Vec<Pick> {
    let stacks = view.stacks.iter().filter(|s| s.alive).map(|s| s.index);
    let members = 0..u8::try_from(world.party.members.len()).unwrap_or(u8::MAX);
    stacks
        .map(Pick::Stack)
        .chain(members.map(Pick::Member))
        .collect()
}

/// The targets the simulation accepts `action` on now, stacks first.
#[must_use]
pub fn targets(world: &World, data: &Data, action: Action) -> Vec<Pick> {
    let Some(view) = combat_view(world, data) else {
        return Vec::new();
    };
    candidates(world, &view)
        .into_iter()
        .filter(|&pick| command(action, pick).is_some_and(|c| accepted(world, data, &c)))
        .collect()
}

/// The acting member's marching-order slot, on a member's turn.
fn acting(world: &World, view: &CombatView) -> Option<u8> {
    let Some(ActorRef::Member(id)) = view.current else {
        return None;
    };
    let slot = world.party.members.iter().position(|m| m.id == id)?;
    u8::try_from(slot).ok()
}

/// A spell whose target does not matter: the simulation takes it on a stack and on a member
/// alike (light). It is cast on the caster without asking.
fn untargeted(picks: &[Pick]) -> bool {
    picks.iter().any(|p| matches!(p, Pick::Stack(_)))
        && picks.iter().any(|p| matches!(p, Pick::Member(_)))
}

/// The fight's menu: which step it is on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatMenu {
    /// The step.
    pub step: Step,
}

impl CombatMenu {
    /// The line above the entries: the encounter, whose turn it is, or what the step asks.
    #[must_use]
    pub fn prompt(&self, world: &World, data: &Data) -> String {
        let Some(view) = combat_view(world, data) else {
            return String::new();
        };
        if view.phase == ModeKind::Encounter {
            return format!("Monsters ahead ({:?})", view.disposition);
        }
        let name = |slot: Option<u8>| {
            slot.and_then(|s| world.party.members.get(usize::from(s)))
                .map_or("?", |m| m.name.as_str())
                .to_owned()
        };
        let who = name(acting(world, &view));
        match self.step {
            Step::Top => format!("Round {}: {who}'s turn", view.round),
            Step::Spells => format!("{who} casts which spell?"),
            Step::Items => format!("{who} uses what?"),
            Step::Target(Action::Attack) => format!("{who} attacks whom?"),
            Step::Target(Action::Cast(i)) => {
                let spell = view.spells.iter().find(|s| s.index == i);
                let spell = spell.map_or("the spell", |s| data.label("en", &s.name));
                format!("{who} casts {spell} on whom?")
            }
            Step::Target(Action::Use(row)) => {
                format!("{who} uses {} on whom?", item_name(world, data, &view, row))
            }
            Step::Target(Action::Swap) => format!("{who} swaps places with whom?"),
        }
    }

    /// The entries for the world as it stands. An empty list outside a fight or while no
    /// member acts.
    #[must_use]
    pub fn entries(&self, world: &World, data: &Data) -> Vec<Entry> {
        let Some(view) = combat_view(world, data) else {
            return Vec::new();
        };
        if view.phase == ModeKind::Encounter {
            return encounter(world, data);
        }
        let Some(own) = acting(world, &view) else {
            return Vec::new();
        };
        let mut entries = match self.step {
            Step::Top => return turn(world, data, &view),
            Step::Spells => spells(world, data, &view, own),
            Step::Items => items(world, data, own),
            Step::Target(action) => target_entries(world, data, &view, action),
        };
        entries.push(Entry {
            label: "Back".to_owned(),
            act: Act::Back,
            blocked: None,
        });
        entries
    }

    /// The targets a click can choose now: the stacks an attack reaches on the turn's first
    /// step, or the targets of the action waiting for one.
    #[must_use]
    pub fn clickable(&self, world: &World, data: &Data) -> Vec<Pick> {
        match self.step {
            Step::Top => targets(world, data, Action::Attack),
            Step::Target(action) => targets(world, data, action),
            Step::Spells | Step::Items => Vec::new(),
        }
    }

    /// Carry out an entry: a command is returned to send and the menu goes back to its first
    /// step; otherwise the menu moves.
    pub fn choose(&mut self, act: &Act) -> Option<Command> {
        match act {
            Act::Command(command) => {
                self.step = Step::Top;
                return Some(command.clone());
            }
            Act::Open(step) => self.step = *step,
            Act::Back => self.back(),
        }
        None
    }

    /// Go back a step: from a spell's or item's targets to its list, otherwise to the top.
    pub fn back(&mut self) {
        self.step = match self.step {
            Step::Target(Action::Cast(_)) => Step::Spells,
            Step::Target(Action::Use(_)) => Step::Items,
            _ => Step::Top,
        };
    }

    /// A click on `pick`: the command to send when it is one of the clickable targets, and the
    /// menu goes back to its first step.
    pub fn pick(&mut self, world: &World, data: &Data, pick: Pick) -> Option<Command> {
        if !self.clickable(world, data).contains(&pick) {
            return None;
        }
        let action = match self.step {
            Step::Target(action) => action,
            _ => Action::Attack,
        };
        let command = command(action, pick)?;
        self.step = Step::Top;
        Some(command)
    }
}

/// An entry for a command, blocked with the simulation's reason if it would be refused.
fn order(world: &World, data: &Data, label: String, command: Command) -> Entry {
    Entry {
        blocked: refusal(world, data, &command),
        label,
        act: Act::Command(command),
    }
}

/// An entry that opens a step, blocked with `why` when the step would offer nothing.
fn open(label: &str, step: Step, why: Option<&str>) -> Entry {
    Entry {
        label: label.to_owned(),
        act: Act::Open(step),
        blocked: why.map(str::to_owned),
    }
}

fn encounter(world: &World, data: &Data) -> Vec<Entry> {
    let bribe = bribe_cost(world, data).map_or_else(
        |_| "Bribe".to_owned(),
        |cost| format!("Bribe ({cost} gold)"),
    );
    [
        ("Fight".to_owned(), EncounterChoice::Attack),
        (bribe, EncounterChoice::Bribe),
        ("Hide".to_owned(), EncounterChoice::Hide),
        ("Run".to_owned(), EncounterChoice::Run),
    ]
    .into_iter()
    .map(|(label, c)| order(world, data, label, Command::Encounter(c)))
    .collect()
}

/// The turn's first step.
fn turn(world: &World, data: &Data, view: &CombatView) -> Vec<Entry> {
    let own = acting(world, view).unwrap_or(0);
    let none = |empty: bool, why| empty.then_some(why);
    let castable = spells(world, data, view, own)
        .iter()
        .any(|e| e.blocked.is_none());
    let usable = items(world, data, own).iter().any(|e| e.blocked.is_none());
    vec![
        open(
            "Attack",
            Step::Target(Action::Attack),
            none(
                targets(world, data, Action::Attack).is_empty(),
                "no foe in reach",
            ),
        ),
        open(
            "Cast",
            Step::Spells,
            none(
                !castable,
                if view.spells.is_empty() {
                    "knows no spells"
                } else {
                    "no spell can be cast now"
                },
            ),
        ),
        open("Use", Step::Items, none(!usable, "nothing to use now")),
        order(
            world,
            data,
            "Dodge".to_owned(),
            Command::Combat(CombatCommand::Dodge),
        ),
        open(
            "Swap",
            Step::Target(Action::Swap),
            none(
                targets(world, data, Action::Swap).is_empty(),
                "no one to swap with",
            ),
        ),
        order(
            world,
            data,
            "Flee".to_owned(),
            Command::Combat(CombatCommand::Run),
        ),
    ]
}

/// The acting member's spells. A spell whose target does not matter is cast at once on the
/// caster; the others ask for a target.
fn spells(world: &World, data: &Data, view: &CombatView, own: u8) -> Vec<Entry> {
    view.spells
        .iter()
        .map(|s| {
            let label = format!("{} ({} pt)", data.label("en", &s.name), s.cost);
            if let Some(why) = &s.blocked {
                return Entry {
                    label,
                    act: Act::Open(Step::Target(Action::Cast(s.index))),
                    blocked: Some(why.to_string()),
                };
            }
            let picks = targets(world, data, Action::Cast(s.index));
            if untargeted(&picks) {
                let cast = command(Action::Cast(s.index), Pick::Member(own));
                return order(
                    world,
                    data,
                    label,
                    cast.expect("a member target fits a cast"),
                );
            }
            Entry {
                label,
                act: Act::Open(Step::Target(Action::Cast(s.index))),
                blocked: picks.is_empty().then(|| "no target".to_owned()),
            }
        })
        .collect()
}

fn item_name(world: &World, data: &Data, view: &CombatView, row: u8) -> String {
    acting(world, view)
        .and_then(|own| world.party.members.get(usize::from(own)))
        .and_then(|m| m.equipment.get(usize::from(row)))
        .and_then(|(id, _)| data.items.get(id))
        .map_or_else(
            || "the item".to_owned(),
            |i| data.label("en", &i.name).to_owned(),
        )
}

/// The acting member's kit rows that have a use, each blocked with the simulation's reason
/// when it can go to no one.
fn items(world: &World, data: &Data, own: u8) -> Vec<Entry> {
    let Some(member) = world.party.members.get(usize::from(own)) else {
        return Vec::new();
    };
    member
        .equipment
        .iter()
        .enumerate()
        .filter_map(|(i, (id, count))| {
            let item = data.items.get(id)?;
            item.use_effect.as_ref()?;
            let row = u8::try_from(i).ok()?;
            let label = format!("{} x{count}", data.label("en", &item.name));
            let blocked = if targets(world, data, Action::Use(row)).is_empty() {
                let alone = Command::Combat(CombatCommand::Use {
                    item: row,
                    target: None,
                });
                Some(
                    refusal(world, data, &alone)
                        .unwrap_or_else(|| "no one to use it on".to_owned()),
                )
            } else {
                None
            };
            Some(Entry {
                label,
                act: Act::Open(Step::Target(Action::Use(row))),
                blocked,
            })
        })
        .collect()
}

/// One entry per target the simulation accepts the action on.
fn target_entries(world: &World, data: &Data, view: &CombatView, action: Action) -> Vec<Entry> {
    targets(world, data, action)
        .into_iter()
        .filter_map(|pick| {
            let label = match pick {
                Pick::Stack(i) => {
                    let s = view.stacks.iter().find(|s| s.index == i)?;
                    format!("{} x{}", data.label("en", &s.name), s.hp.len())
                }
                Pick::Member(m) => world.party.members.get(usize::from(m))?.name.clone(),
            };
            Some(Entry {
                label,
                act: Act::Command(command(action, pick)?),
                blocked: None,
            })
        })
        .collect()
}
