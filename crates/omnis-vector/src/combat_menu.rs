//! The fight's choices (presentation-ARCHITECTURE.md §9): the encounter's four, then a member's turn
//! (Attack, Cast, Features, Use, Dodge, Swap, End turn, Flee) with the spell, feature and item
//! lists and the targets. Bevy-free.
//!
//! Members are named by their `CharacterId` and spells and items by their string ids, as the
//! simulation's commands name them (protocol 2, ARCHITECTURE §4.9); the menu keeps only rows of
//! the lists it shows.
//!
//! Every command the menu offers is tried on a copy of the world first (`trial`), so a choice
//! the simulation would refuse is shown blocked with its reason and never sent. Targets are
//! found the same way: a stack or member is offered for an action only when the simulation
//! accepts that action on it. What a row costs and why it is refused come from the engine's
//! views (`SpellView`, `FeatureView`, `FightItemView`), so the menu holds no rules of its own.

use crate::labels;
use crate::sim::{SimWorld, Views};
use crate::trial::{accepted, why};
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_core::money::gp_floor;
use omnis_sim::omnis_data::{Cost, Data};
use omnis_sim::{
    ActorRef, ChoiceKind, CombatCommand, CombatView, Command, EncounterChoice, FeatureChoice,
    FeatureView, ModeKind, PartyView, Pay, SpellView, Target,
};

/// An action waiting for its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Attack a stack.
    Attack,
    /// Cast the spell at this row of the caster's list (`CombatView.spells`).
    Cast(u8),
    /// Use the item at this row of the acting member's usable items (`CombatView.items`).
    Use(u8),
    /// Swap places with another member.
    Swap,
    /// Use the feature at this row of the acting member's features (`FighterView.features`) to
    /// swap places: Cunning Action's exchange.
    Feature(u8),
}

/// Where the menu is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Step {
    /// The turn's actions, or the encounter's choices.
    #[default]
    Top,
    /// The acting member's spells.
    Spells,
    /// The acting member's class features.
    Features,
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
    /// Words after it in a picker: a spell's "bonus action", a feature's uses.
    pub note: Option<String>,
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
    Exchange { feature: String },
}

/// What pays for a cast from the menu: the bonus action when it can, leaving the action.
fn pay(spell: &SpellView) -> Pay {
    if spell.bonus.is_none() {
        Pay::BonusAction
    } else {
        Pay::Action
    }
}

/// The acting member's features, as the fight view shows them.
fn features_of(view: &CombatView, own: CharacterId) -> &[FeatureView] {
    view.members
        .iter()
        .find(|f| f.member == own)
        .map_or(&[], |f| f.features.as_slice())
}

/// Read `action`'s row out of the views; `None` when the row is gone.
fn resolve(view: &CombatView, action: Action) -> Option<Resolved> {
    Some(match action {
        Action::Attack => Resolved::Attack,
        Action::Cast(row) => {
            let spell = view.spells.get(usize::from(row))?;
            Resolved::Cast {
                spell: spell.spell.clone(),
                pay: pay(spell),
            }
        }
        Action::Use(row) => Resolved::Use {
            item: view.items.get(usize::from(row))?.item.clone(),
        },
        Action::Swap => Resolved::Swap,
        Action::Feature(row) => Resolved::Exchange {
            feature: features_of(view, acting(view)?)
                .get(usize::from(row))?
                .feature
                .clone(),
        },
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
        (Resolved::Exchange { feature }, Pick::Member(with)) => CombatCommand::Feature {
            feature: feature.clone(),
            choice: FeatureChoice::Exchange { with },
        },
        _ => return None,
    };
    Some(Command::Combat(combat))
}

/// The command `action` makes on `pick` now, if that kind of target fits the action.
#[must_use]
pub fn command(world: &SimWorld, data: &Data, action: Action, pick: Pick) -> Option<Command> {
    let view = world.combat(data)?;
    build(&resolve(&view, action)?, pick)
}

/// Every stack still standing and every member, the candidates for any action.
fn candidates(world: &SimWorld, data: &Data, view: &CombatView) -> Vec<Pick> {
    let stacks = view.stacks.iter().filter(|s| s.alive).map(|s| s.stack);
    let members = world.party(data).members.into_iter().map(|m| m.member);
    stacks
        .map(Pick::Stack)
        .chain(members.map(Pick::Member))
        .collect()
}

/// The targets the simulation accepts `action` on now, stacks first.
#[must_use]
pub fn targets(world: &SimWorld, data: &Data, action: Action) -> Vec<Pick> {
    let Some(view) = world.combat(data) else {
        return Vec::new();
    };
    let Some(resolved) = resolve(&view, action) else {
        return Vec::new();
    };
    candidates(world, data, &view)
        .into_iter()
        .filter(|&pick| build(&resolved, pick).is_some_and(|c| accepted(world, data, &c)))
        .collect()
}

/// The acting member, on a member's turn.
#[must_use]
pub fn acting(view: &CombatView) -> Option<CharacterId> {
    match view.current {
        Some(ActorRef::Member(id)) => Some(id),
        _ => None,
    }
}

/// What the acting member's turn has left to pay with: "Action 1   Bonus 1   Reaction 1 (on)",
/// the reactions and the switch the acting member's own. `None` when no member acts.
#[must_use]
pub fn budget_line(views: &Views) -> Option<String> {
    let view = views.combat.as_ref()?;
    let own = acting(view)?;
    let reactions = view
        .reactions
        .iter()
        .find(|(who, _)| *who == ActorRef::Member(own))
        .map_or(0, |(_, left)| *left);
    let on = view
        .members
        .iter()
        .find(|f| f.member == own)
        .is_some_and(|f| f.reactions_on);
    Some(format!(
        "Action {}   Bonus {}   Reaction {reactions} ({})",
        view.budget.actions,
        view.budget.bonus_actions,
        if on { "on" } else { "off" }
    ))
}

/// The acting member's spell points, "SP 3/5", for the pickers' head; `None` when no member
/// acts or the one acting has no spell points.
#[must_use]
pub fn points(views: &Views) -> Option<String> {
    let own = acting(views.combat.as_ref()?)?;
    let m = views.party.members.iter().find(|m| m.member == own)?;
    (m.spell_points_max > 0).then(|| format!("SP {}/{}", m.spell_points, m.spell_points_max))
}

/// A member's name, or `?` for one the party no longer holds.
fn name_of(world: &SimWorld, data: &Data, id: Option<CharacterId>) -> String {
    id.and_then(|id| {
        world
            .party(data)
            .members
            .into_iter()
            .find(|m| m.member == id)
    })
    .map_or_else(|| "?".to_owned(), |m| m.name)
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
    /// Whether the step is one of the pickers (spells, features, items), drawn in the panel over
    /// the picture.
    #[must_use]
    pub const fn picking(&self) -> bool {
        matches!(self.step, Step::Spells | Step::Features | Step::Items)
    }

    /// The line above the entries: the encounter, whose turn it is, or what the step asks.
    #[must_use]
    pub fn prompt(&self, world: &SimWorld, data: &Data) -> String {
        let Some(view) = world.combat(data) else {
            return String::new();
        };
        if view.phase == ModeKind::Encounter {
            return format!("Monsters ahead: {}", labels::disposition(view.disposition));
        }
        let who = name_of(world, data, acting(&view));
        match self.step {
            Step::Top => format!("Round {}: {who}'s turn", view.round),
            Step::Spells => format!("{who} casts which spell?"),
            Step::Features => format!("{who} uses which feature?"),
            Step::Items => format!("{who} uses what?"),
            Step::Target(Action::Attack) => format!("{who} attacks whom?"),
            Step::Target(Action::Cast(i)) => {
                let spell = view.spells.get(usize::from(i));
                let spell = spell.map_or("the spell", |s| labels::text(data, &s.name));
                format!("{who} casts {spell} on whom?")
            }
            Step::Target(Action::Use(row)) => {
                format!("{who} uses {} on whom?", item_name(data, &view, row))
            }
            Step::Target(Action::Swap) => format!("{who} swaps places with whom?"),
            Step::Target(Action::Feature(_)) => {
                format!("{who} swaps places with whom (Cunning Action)?")
            }
        }
    }

    /// The entries for the world as it stands. An empty list outside a fight or while no
    /// member acts.
    #[must_use]
    pub fn entries(&self, world: &SimWorld, data: &Data) -> Vec<Entry> {
        let Some(view) = world.combat(data) else {
            return Vec::new();
        };
        if view.phase == ModeKind::Encounter {
            return encounter(world, data, &view);
        }
        let Some(own) = acting(&view) else {
            return Vec::new();
        };
        let party = world.party(data);
        let mut entries = match self.step {
            Step::Top => return turn(world, data, &view, &party),
            Step::Spells => spells(world, data, &view, &party, own),
            Step::Features => features(world, data, &view, &party, own),
            Step::Items => items(world, data, &view, &party),
            Step::Target(action) => target_entries(world, data, &view, action),
        };
        entries.push(Entry {
            label: "Back".to_owned(),
            act: Act::Back,
            blocked: None,
            note: None,
        });
        entries
    }

    /// The targets a click can choose now: the stacks an attack reaches on the turn's first
    /// step, or the targets of the action waiting for one.
    #[must_use]
    pub fn clickable(&self, world: &SimWorld, data: &Data) -> Vec<Pick> {
        match self.step {
            Step::Top => targets(world, data, Action::Attack),
            Step::Target(action) => targets(world, data, action),
            Step::Spells | Step::Features | Step::Items => Vec::new(),
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
            Step::Target(Action::Feature(_)) => Step::Features,
            _ => Step::Top,
        };
    }

    /// A click on `pick`: the command to send when it is one of the clickable targets, and the
    /// menu goes back to its first step.
    pub fn pick(&mut self, world: &SimWorld, data: &Data, pick: Pick) -> Option<Command> {
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
fn order(
    world: &SimWorld,
    data: &Data,
    party: &PartyView,
    label: String,
    command: Command,
) -> Entry {
    Entry {
        blocked: why(world, data, party, &command),
        label,
        act: Act::Command(command),
        note: None,
    }
}

/// An entry that opens a step, blocked with `why` when the step would offer nothing.
fn open(label: &str, step: Step, why: Option<&str>) -> Entry {
    Entry {
        label: label.to_owned(),
        act: Act::Open(step),
        blocked: why.map(str::to_owned),
        note: None,
    }
}

fn encounter(world: &SimWorld, data: &Data, view: &CombatView) -> Vec<Entry> {
    let party = world.party(data);
    // The view's price is copper; the label says whole gold rounded down, as the retired client.
    let bribe = view.bribe.map_or_else(
        || "Bribe".to_owned(),
        |cost| format!("Bribe ({} gold)", gp_floor(cost)),
    );
    [
        ("Fight".to_owned(), EncounterChoice::Attack),
        (bribe, EncounterChoice::Bribe),
        ("Hide".to_owned(), EncounterChoice::Hide),
        ("Run".to_owned(), EncounterChoice::Run),
    ]
    .into_iter()
    .map(|(label, c)| order(world, data, &party, label, Command::Encounter(c)))
    .collect()
}

/// The turn's first step.
fn turn(world: &SimWorld, data: &Data, view: &CombatView, party: &PartyView) -> Vec<Entry> {
    let Some(own) = acting(view) else {
        return Vec::new();
    };
    let none = |empty: bool, why| empty.then_some(why);
    let castable = spells(world, data, view, party, own)
        .iter()
        .any(|e| e.blocked.is_none());
    let usable = features(world, data, view, party, own)
        .iter()
        .any(|e| e.blocked.is_none());
    let combat = |c| Command::Combat(c);
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
        open(
            "Features",
            Step::Features,
            none(
                !usable,
                if features_of(view, own).is_empty() {
                    "has no features"
                } else {
                    "no feature can be used now"
                },
            ),
        ),
        open(
            "Use",
            Step::Items,
            none(
                !view.items.iter().any(|i| i.blocked.is_none()),
                "nothing to use now",
            ),
        ),
        order(
            world,
            data,
            party,
            "Dodge".to_owned(),
            combat(CombatCommand::Dodge),
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
            party,
            "End turn".to_owned(),
            combat(CombatCommand::EndTurn),
        ),
        order(
            world,
            data,
            party,
            "Flee".to_owned(),
            combat(CombatCommand::Run),
        ),
    ]
}

/// Whether the spell is in force from this caster: an effect on a member or the party.
fn in_force(party: &PartyView, spell: &str, own: CharacterId) -> bool {
    party
        .members
        .iter()
        .flat_map(|m| &m.effects)
        .chain(&party.effects)
        .any(|e| e.spell == spell && e.caster == own)
}

/// The acting member's spells, each "Bless  1 pt". A spell whose target does not matter is cast
/// at once on the caster; the others ask for a target. A spell the bonus action can pay for is
/// cast that way, leaving the action; it is blocked only when neither can pay. A reaction is
/// never cast from here: it answers its trigger from the member's tactics. A live row's note
/// says the spell is in force from this caster, or that it takes the bonus action.
fn spells(
    world: &SimWorld,
    data: &Data,
    view: &CombatView,
    party: &PartyView,
    own: CharacterId,
) -> Vec<Entry> {
    (0..u8::try_from(view.spells.len()).unwrap_or(u8::MAX))
        .zip(&view.spells)
        .map(|(row, s)| {
            let label = format!("{}  {} pt", labels::text(data, &s.name), s.cost);
            let later = Act::Open(Step::Target(Action::Cast(row)));
            if s.pay == Cost::Reaction {
                return Entry {
                    label,
                    act: later,
                    blocked: Some("reaction".to_owned()),
                    note: Some("declare it in tactics".to_owned()),
                };
            }
            if let Some(refused) = s.bonus.as_ref().and(s.blocked.as_ref()) {
                return Entry {
                    label,
                    act: later,
                    blocked: Some(labels::reason(party, refused)),
                    note: None,
                };
            }
            let picks = targets(world, data, Action::Cast(row));
            let mut entry = if untargeted(&picks) {
                let cast = command(world, data, Action::Cast(row), Pick::Member(own));
                order(
                    world,
                    data,
                    party,
                    label,
                    cast.expect("a member target fits a cast"),
                )
            } else {
                Entry {
                    label,
                    act: later,
                    blocked: picks.is_empty().then(|| "no target".to_owned()),
                    note: None,
                }
            };
            if entry.blocked.is_none() {
                entry.note = if in_force(party, &s.spell, own) {
                    Some("in force".to_owned())
                } else {
                    (pay(s) == Pay::BonusAction).then(|| "bonus action".to_owned())
                };
            }
            entry
        })
        .collect()
}

/// A feature's uses and what it spends of the turn: "1 use, bonus action", "at will, action".
fn uses(f: &FeatureView) -> String {
    let left = match f.uses_left {
        None => "at will".to_owned(),
        Some(1) => "1 use".to_owned(),
        Some(n) => format!("{n} uses"),
    };
    format!("{left}, {}", labels::pay(f.pay))
}

/// The acting member's class features, each refused as the engine's view refuses it. Cunning
/// Action is two rows: a swap, which asks for the member, and a hide.
fn features(
    world: &SimWorld,
    data: &Data,
    view: &CombatView,
    party: &PartyView,
    own: CharacterId,
) -> Vec<Entry> {
    let mut rows = Vec::new();
    for (i, f) in features_of(view, own).iter().enumerate() {
        let Ok(row) = u8::try_from(i) else {
            break;
        };
        let name = labels::text(data, &f.feature);
        let refused = f.blocked.as_ref().map(|r| labels::reason(party, r));
        let feature = |choice| {
            Act::Command(Command::Combat(CombatCommand::Feature {
                feature: f.feature.clone(),
                choice,
            }))
        };
        if f.choices.contains(&ChoiceKind::Exchange) {
            let swap = targets(world, data, Action::Feature(row))
                .is_empty()
                .then(|| {
                    refused
                        .clone()
                        .unwrap_or_else(|| "no one to swap with".to_owned())
                });
            rows.push(Entry {
                label: format!("{name}: swap"),
                act: Act::Open(Step::Target(Action::Feature(row))),
                blocked: swap,
                note: Some(uses(f)),
            });
        }
        let choice = if f.choices.contains(&ChoiceKind::Hide) {
            Some((format!("{name}: hide"), FeatureChoice::Hide))
        } else if f.choices.is_empty() || f.choices.contains(&ChoiceKind::Plain) {
            Some((name.to_owned(), FeatureChoice::None))
        } else {
            None
        };
        if let Some((label, choice)) = choice {
            let act = feature(choice);
            let blocked = refused.clone().or_else(|| match &act {
                Act::Command(c) => why(world, data, party, c),
                _ => None,
            });
            rows.push(Entry {
                label,
                act,
                blocked,
                note: Some(uses(f)),
            });
        }
    }
    rows
}

fn item_name(data: &Data, view: &CombatView, row: u8) -> String {
    view.items.get(usize::from(row)).map_or_else(
        || "the item".to_owned(),
        |i| labels::text(data, &i.name).to_owned(),
    )
}

/// The acting member's items that have a use, each refused as the engine's view refuses it, or
/// when it can go to no one.
fn items(world: &SimWorld, data: &Data, view: &CombatView, party: &PartyView) -> Vec<Entry> {
    (0..u8::try_from(view.items.len()).unwrap_or(u8::MAX))
        .zip(&view.items)
        .map(|(row, item)| {
            let blocked = item
                .blocked
                .as_ref()
                .map(|r| labels::reason(party, r))
                .or_else(|| {
                    targets(world, data, Action::Use(row))
                        .is_empty()
                        .then(|| "no one to use it on".to_owned())
                });
            Entry {
                label: labels::count_label(labels::text(data, &item.name), item.count),
                act: Act::Open(Step::Target(Action::Use(row))),
                blocked,
                note: None,
            }
        })
        .collect()
}

/// One entry per target the simulation accepts the action on.
fn target_entries(world: &SimWorld, data: &Data, view: &CombatView, action: Action) -> Vec<Entry> {
    let Some(resolved) = resolve(view, action) else {
        return Vec::new();
    };
    targets(world, data, action)
        .into_iter()
        .filter_map(|pick| {
            let label = match pick {
                Pick::Stack(i) => {
                    let s = view.stacks.iter().find(|s| s.stack == i)?;
                    format!("{} x{}", labels::text(data, &s.name), s.hps.len())
                }
                Pick::Member(m) => name_of(world, data, Some(m)),
            };
            Some(Entry {
                label,
                act: Act::Command(build(&resolved, pick)?),
                blocked: None,
                note: None,
            })
        })
        .collect()
}
