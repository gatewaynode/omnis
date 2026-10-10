//! The fight's choices (presentation-ARCHITECTURE.md §9): the encounter's four, then a member's turn
//! (Attack, Cast, Use, Dodge, Swap, End turn, Flee) with the spell and item lists and the
//! targets. Bevy-free.
//!
//! Members are named by their `CharacterId` and spells and items by their string ids, as the
//! simulation's commands name them (protocol 2, ARCHITECTURE §4.9); the menu keeps only rows of
//! the lists it shows.
//!
//! Every command the menu offers is tried on a copy of the world first (`trial`), so a choice
//! the simulation would refuse is shown blocked with its reason and never sent. Targets are
//! found the same way: a stack or member is offered for an action only when the simulation
//! accepts that action on it, so the menu holds no rules of its own.

use crate::trial::{accepted, refusal};
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_sim::{
    ActorRef, Budget, CombatCommand, CombatView, Command, EncounterChoice, ItemView, ModeKind, Pay,
    SpellView, Target, World, bribe_cost, combat_view, party_view,
};

/// An action waiting for its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Attack a stack.
    Attack,
    /// Cast the spell at this row of the caster's list (`CombatView.spells`).
    Cast(u8),
    /// Use the item at this row of the acting member's kit (`MemberView.equipment`).
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
/// identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pick {
    /// A stack.
    Stack(u8),
    /// A member.
    Member(CharacterId),
}

/// An action with its row read out of the views: what a command needs besides the target.
enum Resolved {
    Attack,
    Cast { spell: String, pay: Pay },
    Use { item: String },
    Swap,
}

/// What pays for a cast from the menu: the bonus action when it can, leaving the action.
fn pay(spell: &SpellView) -> Pay {
    if spell.bonus.is_none() {
        Pay::BonusAction
    } else {
        Pay::Action
    }
}

/// The acting member's kit as the party view shows it.
fn kit(world: &World, data: &Data, own: CharacterId) -> Vec<ItemView> {
    party_view(world, data)
        .members
        .into_iter()
        .find(|m| m.member == own)
        .map(|m| m.equipment)
        .unwrap_or_default()
}

/// Read `action`'s row out of the views; `None` when the row is gone.
fn resolve(world: &World, data: &Data, view: &CombatView, action: Action) -> Option<Resolved> {
    Some(match action {
        Action::Attack => Resolved::Attack,
        Action::Cast(row) => {
            let spell = view.spells.get(usize::from(row))?;
            Resolved::Cast {
                spell: spell.spell.clone(),
                pay: pay(spell),
            }
        }
        Action::Use(row) => {
            let own = acting(view)?;
            let item = kit(world, data, own).into_iter().nth(usize::from(row))?;
            Resolved::Use { item: item.item }
        }
        Action::Swap => Resolved::Swap,
    })
}

/// The command a resolved action makes on `pick`, if that kind of target fits it.
fn build(action: &Resolved, pick: Pick) -> Option<Command> {
    let combat = match (action, pick) {
        (Resolved::Attack, Pick::Stack(stack)) => CombatCommand::Attack { stack },
        (Resolved::Cast { spell, pay }, Pick::Stack(s)) => CombatCommand::Cast {
            spell: spell.clone(),
            target: Target::Stack(s),
            pay: *pay,
        },
        (Resolved::Cast { spell, pay }, Pick::Member(m)) => CombatCommand::Cast {
            spell: spell.clone(),
            target: Target::Member(m),
            pay: *pay,
        },
        (Resolved::Use { item }, Pick::Member(m)) => CombatCommand::Use {
            item: item.clone(),
            receiver: Some(m),
        },
        (Resolved::Swap, Pick::Member(with)) => CombatCommand::Exchange { with },
        _ => return None,
    };
    Some(Command::Combat(combat))
}

/// The command `action` makes on `pick` now, if that kind of target fits the action.
#[must_use]
pub fn command(world: &World, data: &Data, action: Action, pick: Pick) -> Option<Command> {
    let view = combat_view(world, data)?;
    build(&resolve(world, data, &view, action)?, pick)
}

/// Every stack still standing and every member, the candidates for any action.
fn candidates(world: &World, view: &CombatView) -> Vec<Pick> {
    let stacks = view.stacks.iter().filter(|s| s.alive).map(|s| s.stack);
    let members = world.party.members.iter().map(|m| m.id);
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
    let Some(resolved) = resolve(world, data, &view, action) else {
        return Vec::new();
    };
    candidates(world, &view)
        .into_iter()
        .filter(|&pick| build(&resolved, pick).is_some_and(|c| accepted(world, data, &c)))
        .collect()
}

/// The acting member, on a member's turn.
fn acting(view: &CombatView) -> Option<CharacterId> {
    match view.current {
        Some(ActorRef::Member(id)) => Some(id),
        _ => None,
    }
}

/// What the turn has left to pay with, in words.
fn budget(left: Budget) -> String {
    let count = |n: u8, one: &str| match n {
        0 => format!("no {one}"),
        1 => format!("1 {one}"),
        n => format!("{n} {one}s"),
    };
    format!(
        "{}, {}",
        count(left.actions, "action"),
        count(left.bonus_actions, "bonus action")
    )
}

/// A member's name, or `?` for one the party no longer holds.
fn name_of(world: &World, id: Option<CharacterId>) -> String {
    id.and_then(|id| world.party.members.iter().find(|m| m.id == id))
        .map_or("?", |m| m.name.as_str())
        .to_owned()
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
        let who = name_of(world, acting(&view));
        match self.step {
            Step::Top => format!(
                "Round {}: {who}'s turn ({})",
                view.round,
                budget(view.budget)
            ),
            Step::Spells => format!("{who} casts which spell?"),
            Step::Items => format!("{who} uses what?"),
            Step::Target(Action::Attack) => format!("{who} attacks whom?"),
            Step::Target(Action::Cast(i)) => {
                let spell = view.spells.get(usize::from(i));
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
        let Some(own) = acting(&view) else {
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
        let command = command(world, data, action, pick)?;
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
    let Some(own) = acting(view) else {
        return Vec::new();
    };
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
            "End turn".to_owned(),
            Command::Combat(CombatCommand::EndTurn),
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
/// caster; the others ask for a target. A spell the bonus action can pay for is cast that way,
/// leaving the action; it is blocked only when neither can pay.
fn spells(world: &World, data: &Data, view: &CombatView, own: CharacterId) -> Vec<Entry> {
    (0..u8::try_from(view.spells.len()).unwrap_or(u8::MAX))
        .zip(&view.spells)
        .map(|(row, s)| {
            let note = if s.bonus.is_none() { ", bonus" } else { "" };
            let label = format!("{} ({} pt{note})", data.label("en", &s.name), s.cost);
            if let Some(why) = s.bonus.as_ref().and(s.blocked.as_ref()) {
                return Entry {
                    label,
                    act: Act::Open(Step::Target(Action::Cast(row))),
                    blocked: Some(why.to_string()),
                };
            }
            let picks = targets(world, data, Action::Cast(row));
            if untargeted(&picks) {
                let cast = command(world, data, Action::Cast(row), Pick::Member(own));
                return order(
                    world,
                    data,
                    label,
                    cast.expect("a member target fits a cast"),
                );
            }
            Entry {
                label,
                act: Act::Open(Step::Target(Action::Cast(row))),
                blocked: picks.is_empty().then(|| "no target".to_owned()),
            }
        })
        .collect()
}

fn item_name(world: &World, data: &Data, view: &CombatView, row: u8) -> String {
    acting(view)
        .and_then(|own| kit(world, data, own).into_iter().nth(usize::from(row)))
        .map_or_else(
            || "the item".to_owned(),
            |i| data.label("en", &i.name).to_owned(),
        )
}

/// The acting member's kit rows that have a use, each blocked with the simulation's reason
/// when it can go to no one.
fn items(world: &World, data: &Data, own: CharacterId) -> Vec<Entry> {
    kit(world, data, own)
        .into_iter()
        .enumerate()
        .filter_map(|(i, item)| {
            if !item.usable {
                return None;
            }
            let row = u8::try_from(i).ok()?;
            let label = format!("{} x{}", data.label("en", &item.name), item.count);
            let blocked = if targets(world, data, Action::Use(row)).is_empty() {
                let alone = Command::Combat(CombatCommand::Use {
                    item: item.item,
                    receiver: None,
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
    let Some(resolved) = resolve(world, data, view, action) else {
        return Vec::new();
    };
    targets(world, data, action)
        .into_iter()
        .filter_map(|pick| {
            let label = match pick {
                Pick::Stack(i) => {
                    let s = view.stacks.iter().find(|s| s.stack == i)?;
                    format!("{} x{}", data.label("en", &s.name), s.hps.len())
                }
                Pick::Member(m) => name_of(world, Some(m)),
            };
            Some(Entry {
                label,
                act: Act::Command(build(&resolved, pick)?),
                blocked: None,
            })
        })
        .collect()
}
