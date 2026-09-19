//! The encounter, combat, and defeat menus as pure state machines (ARCHITECTURE.md §8.1),
//! and the view they read: stack rows with labels, the acting member, the bribe price. Each
//! menu takes a key and answers with an intent; `combat.rs` feeds keys and applies intents.
//! Bevy-free, so every transition is unit-tested against a real fight.

use crate::actors::Actor;
use crate::menu::{MenuKey, cycle};
use omnis_sim::combat::weapon_for;
use omnis_sim::omnis_data::{Data, Disposition, Size, SpellEffect};

pub use crate::spell_menu::SpellRow;
use crate::spell_menu::blocked_note;
pub use crate::use_menu::UseRow;
use crate::use_menu::use_rows;
use omnis_sim::{
    ActorRef, CombatCommand, EncounterChoice, Event, Mode, ModeKind, World, bribe_cost, combat_view,
};

/// One stack as the rows show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRow {
    /// Its index in the encounter, the number `attack` takes.
    pub index: u8,
    /// The monster's name.
    pub name: String,
    /// How many still stand.
    pub count: u8,
    /// How many there were.
    pub initial: u8,
    /// The monster's size, for its silhouette.
    pub size: Size,
    /// Whether it stands in front.
    pub front: bool,
    /// Whether anyone in it still stands.
    pub alive: bool,
    /// Why the acting member cannot attack it, when they cannot.
    pub blocked: Option<String>,
}

impl StackRow {
    /// Whether the acting member can attack it.
    #[must_use]
    pub fn reachable(&self) -> bool {
        self.alive && self.blocked.is_none()
    }

    /// The word the row ends with.
    #[must_use]
    pub const fn state(&self) -> &'static str {
        if !self.alive {
            "slain"
        } else if self.front {
            "front"
        } else {
            "back"
        }
    }
}

/// The encounter or fight as the screens show it, built from the world every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FightView {
    /// `Encounter` before the choice, `Combat` during the fight.
    pub phase: ModeKind,
    /// The round, zero before the fight.
    pub round: u32,
    /// The acting member's slot.
    pub own: Option<usize>,
    /// How the monsters feel about the party.
    pub disposition: Disposition,
    /// The stacks, in encounter order.
    pub stacks: Vec<StackRow>,
    /// What a bribe costs; `None` when the rule cannot say.
    pub bribe: Option<u32>,
    /// The party's purse.
    pub gold: u32,
    /// The acting member's spells, in cast order; empty when no member acts or none known.
    pub spells: Vec<SpellRow>,
    /// The acting member's spell points and maximum.
    pub points: (u32, u32),
    /// The acting member's usable kit rows; empty when no member acts or none has a use.
    pub usable: Vec<UseRow>,
}

impl FightView {
    /// Whether the party can pay the bribe.
    #[must_use]
    pub fn bribe_allowed(&self) -> bool {
        self.bribe.is_some_and(|cost| cost <= self.gold)
    }

    /// The bribe button's text: `Bribe 12g`, or `Bribe free`.
    #[must_use]
    pub fn bribe_label(&self) -> String {
        match self.bribe {
            Some(0) => "Bribe free".to_owned(),
            Some(cost) => format!("Bribe {cost}g"),
            None => "Bribe".to_owned(),
        }
    }

    /// The stack rows that still stand.
    fn living(&self) -> impl Iterator<Item = &StackRow> {
        self.stacks.iter().filter(|s| s.alive)
    }

    /// The living stacks as the silhouettes need them.
    #[must_use]
    pub fn actors(&self) -> Vec<Actor> {
        self.living()
            .map(|s| Actor {
                index: s.index,
                size: s.size,
                front: s.front,
            })
            .collect()
    }
}

/// The view, or `None` while exploring.
#[must_use]
pub fn fight_view(world: &World, data: &Data) -> Option<FightView> {
    let view = combat_view(world, data)?;
    let own = match view.current {
        Some(ActorRef::Member(id)) => world.party.members.iter().position(|m| m.id == id),
        _ => None,
    };
    let fight = match &world.mode {
        Mode::Combat(state) => Some(state),
        _ => None,
    };
    let stacks = view
        .stacks
        .iter()
        .map(|s| StackRow {
            index: s.index,
            name: data.label("en", &s.name).to_owned(),
            count: u8::try_from(s.hp.len()).unwrap_or(u8::MAX),
            initial: s.initial,
            size: data
                .registry
                .monsters
                .get(&s.monster)
                .and_then(|id| data.monsters.get(&id))
                .map_or(Size::Medium, |m| m.size),
            front: s.front,
            alive: s.alive,
            blocked: fight.zip(own).and_then(|(state, own)| {
                weapon_for(state, world, data, own, s.index)
                    .err()
                    .map(|r| r.to_string())
            }),
        })
        .collect();
    let caster = own.map(|i| &world.party.members[i]);
    let spells = view
        .spells
        .iter()
        .filter_map(|s| {
            let caster = caster?;
            let id = *caster.known_spells.get(usize::from(s.index))?;
            let spell = data.spells.get(&id)?;
            let reaction = matches!(spell.effect, Some(SpellEffect::Reaction { .. }));
            let active = world
                .party
                .members
                .iter()
                .flat_map(|m| m.effects.iter())
                .chain(world.party.effects.iter())
                .any(|e| e.source == id && e.caster == caster.id);
            Some(SpellRow {
                index: s.index,
                name: data.label("en", &s.name).to_owned(),
                cost: s.cost,
                targets_members: s.targets_members,
                auto: reaction.then(|| caster.auto_cast.contains(&id)),
                active,
                blocked: s.blocked.as_ref().map(blocked_note),
            })
        })
        .collect();
    Some(FightView {
        phase: view.phase,
        round: view.round,
        own,
        disposition: view.disposition,
        stacks,
        bribe: (view.phase == ModeKind::Encounter)
            .then(|| bribe_cost(world, data).ok())
            .flatten(),
        gold: world.party.gold,
        spells,
        points: caster.map_or((0, 0), |c| (c.spell_points, c.spell_points_max)),
        usable: own.map_or_else(Vec::new, |own| use_rows(world, data, own)),
    })
}

// ---------------------------------------------------------------- combat

/// What the combat menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatIntent {
    /// A command for the acting member.
    Command(CombatCommand),
    /// Switch a reaction spell's auto-cast for the acting member.
    AutoCast {
        /// The spell's index in the caster's list.
        spell: u8,
        /// On or off.
        on: bool,
    },
    /// Open the pause overlay.
    Pause,
}

/// The action row's cursor position of Cast.
pub const ACTION_CAST: usize = 1;
/// The action row's cursor position of Use.
pub const ACTION_USE: usize = 2;

/// The fight's action row and target: Up/Down choose the action, Left/Right the stack, Enter
/// confirms, `a c u d e r` are hotkeys, Escape pauses. Cast opens the spell picker over the
/// action row: Up/Down choose the spell, Enter casts it at the target (or the band's selected
/// member for a heal or a buff), Escape closes it. Use opens the item picker the same way:
/// Enter uses the item on the band's selected member, or the user.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombatMenu {
    /// The action under the cursor.
    pub cursor: usize,
    /// The stack an attack goes to.
    pub target: u8,
    /// Why the last confirmation did nothing; empty when it did something.
    pub message: String,
    /// The spell picker's cursor while it is open.
    pub picker: Option<usize>,
    /// The item picker's cursor while it is open.
    pub use_picker: Option<usize>,
}

impl CombatMenu {
    /// The actions, in cursor order.
    pub const ACTIONS: [&'static str; 6] = ["Attack", "Cast", "Use", "Dodge", "Exchange", "Run"];
    /// The hotkeys, in the same order.
    pub const HOTKEYS: [char; 6] = ['a', 'c', 'u', 'd', 'e', 'r'];

    /// Keep the target on a living stack: the first one when the current target fell; keep
    /// the picker's cursor on a spell, and close it when the acting member knows none.
    pub fn sync(&mut self, view: &FightView) {
        let on_living = view.living().any(|s| s.index == self.target);
        if !on_living && let Some(first) = view.living().next() {
            self.target = first.index;
        }
        if let Some(cursor) = self.picker {
            if view.spells.is_empty() {
                self.picker = None;
            } else {
                self.picker = Some(cursor.min(view.spells.len() - 1));
            }
        }
        if let Some(cursor) = self.use_picker {
            if view.usable.is_empty() {
                self.use_picker = None;
            } else {
                self.use_picker = Some(cursor.min(view.usable.len() - 1));
            }
        }
    }

    /// Handle a key. `selected` is the band row the mouse selected, the exchange partner or
    /// the member a heal goes to.
    pub fn key(
        &mut self,
        key: MenuKey,
        view: &FightView,
        selected: Option<usize>,
    ) -> Option<CombatIntent> {
        if let Some(cursor) = self.picker {
            return self.picker_key(cursor, key, view, selected);
        }
        if let Some(cursor) = self.use_picker {
            return self.use_key(cursor, key, view, selected);
        }
        match key {
            MenuKey::Up | MenuKey::Down => {
                self.cursor = cycle(self.cursor, Self::ACTIONS.len(), key);
            }
            MenuKey::Left | MenuKey::Right => self.step_target(view, key),
            MenuKey::Enter => return self.confirm(view, selected),
            MenuKey::Char(c) => {
                if let Some(at) = Self::HOTKEYS.iter().position(|h| *h == c) {
                    self.cursor = at;
                    return self.confirm(view, selected);
                }
            }
            MenuKey::Escape => return Some(CombatIntent::Pause),
            MenuKey::Backspace => {}
        }
        None
    }

    pub(crate) fn step_target(&mut self, view: &FightView, key: MenuKey) {
        let living: Vec<u8> = view.living().map(|s| s.index).collect();
        let at = living.iter().position(|i| *i == self.target).unwrap_or(0);
        if let Some(index) = living.get(cycle(at, living.len(), key)) {
            self.target = *index;
        }
    }

    fn confirm(&mut self, view: &FightView, selected: Option<usize>) -> Option<CombatIntent> {
        self.message.clear();
        let command = match self.cursor {
            0 => {
                let row = view.stacks.iter().find(|s| s.index == self.target);
                match row {
                    Some(row) if row.reachable() => CombatCommand::Attack { stack: row.index },
                    Some(row) if !row.alive => {
                        self.message = format!("{} are slain", row.name);
                        return None;
                    }
                    Some(row) => {
                        self.message = row.blocked.clone().unwrap_or_default();
                        return None;
                    }
                    None => {
                        self.message = "Nothing to attack".to_owned();
                        return None;
                    }
                }
            }
            ACTION_CAST => {
                if view.spells.is_empty() {
                    self.message = "No spells known".to_owned();
                } else {
                    self.picker = Some(0);
                }
                return None;
            }
            ACTION_USE => {
                if view.usable.is_empty() {
                    self.message = "Nothing to use".to_owned();
                } else {
                    self.use_picker = Some(0);
                }
                return None;
            }
            3 => CombatCommand::Dodge,
            4 => match (selected, view.own) {
                (Some(with), Some(own)) if with != own => CombatCommand::Exchange {
                    with: u8::try_from(with).unwrap_or(u8::MAX),
                },
                (Some(_), _) => {
                    self.message = "Select another member to exchange with".to_owned();
                    return None;
                }
                (None, _) => {
                    self.message = "Select a member to exchange with".to_owned();
                    return None;
                }
            },
            _ => CombatCommand::Run,
        };
        Some(CombatIntent::Command(command))
    }
}

// ---------------------------------------------------------------- encounter

/// What the encounter menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterIntent {
    /// One of the four choices.
    Choice(EncounterChoice),
    /// Open the pause overlay.
    Pause,
}

/// The choice before a fight: arrows cycle the four, Enter confirms, `a b h r` are hotkeys,
/// Escape pauses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EncounterMenu {
    /// The choice under the cursor.
    pub cursor: usize,
    /// Why the last confirmation did nothing; empty when it did something.
    pub message: String,
}

impl EncounterMenu {
    /// The choices, in cursor order (the bribe's text comes from the view).
    pub const ACTIONS: [&'static str; 4] = ["Attack", "Bribe", "Hide", "Run"];
    /// The hotkeys, in the same order.
    pub const HOTKEYS: [char; 4] = ['a', 'b', 'h', 'r'];
    /// The choices, in the same order.
    const CHOICES: [EncounterChoice; 4] = [
        EncounterChoice::Attack,
        EncounterChoice::Bribe,
        EncounterChoice::Hide,
        EncounterChoice::Run,
    ];

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey, view: &FightView) -> Option<EncounterIntent> {
        match key {
            MenuKey::Up | MenuKey::Down | MenuKey::Left | MenuKey::Right => {
                self.cursor = cycle(self.cursor, Self::ACTIONS.len(), key);
            }
            MenuKey::Enter => return self.confirm(view),
            MenuKey::Char(c) => {
                if let Some(at) = Self::HOTKEYS.iter().position(|h| *h == c) {
                    self.cursor = at;
                    return self.confirm(view);
                }
            }
            MenuKey::Escape => return Some(EncounterIntent::Pause),
            MenuKey::Backspace => {}
        }
        None
    }

    fn confirm(&mut self, view: &FightView) -> Option<EncounterIntent> {
        self.message.clear();
        let choice = Self::CHOICES[self.cursor.min(3)];
        if choice == EncounterChoice::Bribe && !view.bribe_allowed() {
            self.message = match view.bribe {
                Some(cost) => format!("Not enough gold: {cost} needed, {} carried", view.gold),
                None => "They cannot be bribed".to_owned(),
            };
            return None;
        }
        Some(EncounterIntent::Choice(choice))
    }
}

// ---------------------------------------------------------------- defeat

/// What the defeat modal asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefeatAction {
    /// Load the last save.
    Load,
    /// Drop the game and return to the title.
    QuitToTitle,
}

/// "The party has fallen": two choices, no escape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefeatMenu {
    /// The choice under the cursor.
    pub cursor: usize,
}

impl DefeatMenu {
    /// The choices, in cursor order.
    pub const ITEMS: [&'static str; 2] = ["Load last save", "Quit to title"];

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<DefeatAction> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, 2, key),
            MenuKey::Enter => {
                return Some(if self.cursor == 0 {
                    DefeatAction::Load
                } else {
                    DefeatAction::QuitToTitle
                });
            }
            _ => {}
        }
        None
    }
}

/// Whether an event changes what the viewport shows behind the panel: who stands there.
#[must_use]
pub const fn redraws(event: &Event) -> bool {
    matches!(
        event,
        Event::EncounterStarted { .. }
            | Event::Death { .. }
            | Event::CombatEnded { .. }
            | Event::Bribed { .. }
            | Event::Check { .. }
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use omnis_sim::omnis_core::{Pcg32, StreamName};
    use omnis_sim::omnis_data::{Alignment, Skill, load_packs};
    use omnis_sim::omnis_rules::{Draft, monster_hit_points};
    use omnis_sim::{
        Command, EncounterSource, EncounterState, PartyCommand, Settings, Stack, apply,
    };
    use std::path::PathBuf;

    pub(crate) fn data() -> Data {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")])
            .unwrap_or_else(|r| panic!("{r}"))
    }

    fn draft(name: &str, class: &str, scores: [u8; 6]) -> Draft {
        Draft {
            name: name.to_owned(),
            race: "base:race:human".to_owned(),
            class: format!("base:class:{class}"),
            background: "base:background:acolyte".to_owned(),
            alignment: Alignment::LawfulGood,
            scores,
            skills: match class {
                "wizard" => vec![Skill::Arcana, Skill::History],
                "cleric" => vec![Skill::Medicine, Skill::History],
                _ => vec![Skill::Athletics, Skill::Perception],
            },
        }
    }

    /// A party of the given classes facing test-pack monsters, still choosing. The
    /// encounter is installed where the party stands, so no random table on the way can
    /// change what the view shows.
    pub(crate) fn facing(data: &Data, classes: &[&str], stacks: &[(&str, u8)]) -> World {
        let mut world = World::new(data, 0x0123_4567_89ab_cdef, Settings::default()).unwrap();
        let names = ["Brenna", "Gorm", "Wren", "Pip", "Durin", "Ilvara"];
        for (name, class) in names.iter().zip(classes) {
            let d = draft(name, class, [15, 14, 13, 12, 10, 8]);
            apply(&mut world, data, Command::Party(PartyCommand::Create(d))).unwrap();
        }
        let stream = StreamName::new("combat");
        let mut rng = Pcg32::for_stream(0, &stream);
        let stacks = stacks
            .iter()
            .map(|(name, count)| {
                let id = data
                    .registry
                    .monsters
                    .get(&format!("test:monster:{name}"))
                    .unwrap_or_else(|| panic!("{name}"));
                let hp = monster_hit_points(&data.monsters[&id], data, &mut rng, &stream).unwrap();
                Stack {
                    monster: id,
                    initial: *count,
                    hp: vec![hp; usize::from(*count)],
                }
            })
            .collect();
        world.mode = Mode::Encounter(EncounterState {
            source: EncounterSource::Fixed(0),
            stacks,
            disposition: Disposition::Hostile,
            retreat: world.position,
        });
        world
    }

    /// Four fighters facing the goblins of the test dungeon's first placement.
    pub(crate) fn facing_goblins(data: &Data) -> World {
        facing(data, &["fighter"; 4], &[("goblin", 3), ("giant_rat", 2)])
    }

    #[test]
    fn the_view_names_the_stacks_and_prices_the_bribe() {
        let data = data();
        let mut world = facing_goblins(&data);
        let view = fight_view(&world, &data).unwrap();
        assert_eq!(view.phase, ModeKind::Encounter);
        assert_eq!(view.disposition, Disposition::Hostile);
        let rows: Vec<(&str, u8, &str)> = view
            .stacks
            .iter()
            .map(|s| (s.name.as_str(), s.count, s.state()))
            .collect();
        assert_eq!(rows, [("Goblin", 3, "front"), ("Giant Rat", 2, "front")]);
        assert_eq!(
            view.stacks.iter().map(|s| s.size).collect::<Vec<_>>(),
            [Size::Small, Size::Small]
        );
        assert_eq!(view.actors().len(), 2);
        assert_eq!(view.own, None);
        // Goblins 50 xp × 3 + rats 25 xp × 2 = 200; hostile pays it all.
        assert_eq!(view.bribe, Some(200));
        assert_eq!(view.bribe_label(), "Bribe 200g");
        assert!(
            !view.bribe_allowed(),
            "two acolytes carry {} gold",
            view.gold
        );

        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        let view = fight_view(&world, &data).unwrap();
        assert_eq!(view.phase, ModeKind::Combat);
        assert_eq!(view.round, 1);
        assert!(view.own.is_some(), "a member is to act");
        assert_eq!(view.bribe, None);
        assert!(
            view.stacks.iter().all(StackRow::reachable),
            "front row melee"
        );
        assert!(fight_view(&World::new(&data, 1, Settings::default()).unwrap(), &data).is_none());
    }

    #[test]
    fn the_view_explains_an_unreachable_stack() {
        let data = data();
        // A lone wizard has no ranged weapon; with two stacks in front, the third is behind.
        let mut world = facing(
            &data,
            &["wizard"],
            &[("giant_rat", 1), ("giant_rat", 1), ("goblin", 1)],
        );
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        let view = fight_view(&world, &data).unwrap();
        assert_eq!(view.own, Some(0), "Brenna acts: {view:#?}");
        let back = &view.stacks[2];
        assert!(back.alive && !back.front && !back.reachable(), "{view:#?}");
        assert_eq!(
            back.blocked.as_deref(),
            Some("stack 2 is behind the front; a ranged weapon reaches it")
        );
        assert!(view.stacks[0].reachable() && view.stacks[1].reachable());
        let mut menu = CombatMenu {
            target: 2,
            ..CombatMenu::default()
        };
        assert_eq!(menu.key(MenuKey::Enter, &view, None), None);
        assert_eq!(
            menu.message,
            "stack 2 is behind the front; a ranged weapon reaches it"
        );
    }

    fn view_with(stacks: &[(u8, bool, Option<&str>)], own: Option<usize>) -> FightView {
        FightView {
            phase: ModeKind::Combat,
            round: 1,
            own,
            disposition: Disposition::Hostile,
            stacks: stacks
                .iter()
                .map(|(index, alive, blocked)| StackRow {
                    index: *index,
                    name: format!("Stack {index}"),
                    count: u8::from(*alive),
                    initial: 1,
                    size: Size::Medium,
                    front: *alive && *index < 2,
                    alive: *alive,
                    blocked: blocked.map(str::to_owned),
                })
                .collect(),
            bribe: None,
            gold: 0,
            spells: Vec::new(),
            points: (0, 0),
            usable: Vec::new(),
        }
    }

    #[test]
    fn combat_keys_choose_actions_and_living_targets() {
        let view = view_with(
            &[(0, false, None), (1, true, None), (2, true, Some("no"))],
            Some(0),
        );
        let mut menu = CombatMenu::default();
        menu.sync(&view);
        assert_eq!(menu.target, 1, "the first living stack");
        assert_eq!(menu.key(MenuKey::Right, &view, None), None);
        assert_eq!(menu.target, 2);
        menu.key(MenuKey::Right, &view, None);
        assert_eq!(menu.target, 1, "wraps over the living only");
        menu.key(MenuKey::Left, &view, None);
        assert_eq!(menu.target, 2);
        assert_eq!(menu.key(MenuKey::Enter, &view, None), None);
        assert_eq!(
            menu.message, "no",
            "the view's reason, before the sim rejects"
        );
        menu.key(MenuKey::Left, &view, None);
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Attack { stack: 1 }))
        );
        assert!(menu.message.is_empty());
        for _ in 0..3 {
            menu.key(MenuKey::Down, &view, None);
        }
        assert_eq!(menu.cursor, 3, "attack, cast, use, dodge");
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Dodge))
        );
        for _ in 0..4 {
            menu.key(MenuKey::Up, &view, None);
        }
        assert_eq!(menu.cursor, 5, "wraps");
        assert_eq!(
            menu.key(MenuKey::Char('r'), &view, None),
            Some(CombatIntent::Command(CombatCommand::Run))
        );
        assert_eq!(
            menu.key(MenuKey::Escape, &view, None),
            Some(CombatIntent::Pause)
        );
        assert_eq!(menu.key(MenuKey::Char('x'), &view, None), None);
    }

    #[test]
    fn exchange_needs_another_selected_member() {
        let view = view_with(&[(0, true, None)], Some(0));
        let mut menu = CombatMenu::default();
        assert_eq!(menu.key(MenuKey::Char('e'), &view, None), None);
        assert_eq!(menu.message, "Select a member to exchange with");
        assert_eq!(menu.key(MenuKey::Char('e'), &view, Some(0)), None);
        assert_eq!(menu.message, "Select another member to exchange with");
        assert_eq!(
            menu.key(MenuKey::Char('e'), &view, Some(3)),
            Some(CombatIntent::Command(CombatCommand::Exchange { with: 3 }))
        );
        let none = view_with(&[(0, false, None)], Some(0));
        menu.sync(&none);
        assert_eq!(menu.key(MenuKey::Char('a'), &none, None), None);
        assert_eq!(menu.message, "Stack 0 are slain");
    }

    #[test]
    fn encounter_keys_cycle_and_gate_the_bribe() {
        let mut view = view_with(&[(0, true, None)], None);
        view.phase = ModeKind::Encounter;
        view.bribe = Some(12);
        view.gold = 5;
        let mut menu = EncounterMenu::default();
        assert_eq!(menu.key(MenuKey::Right, &view), None);
        assert_eq!(menu.cursor, 1);
        assert_eq!(menu.key(MenuKey::Enter, &view), None);
        assert_eq!(menu.message, "Not enough gold: 12 needed, 5 carried");
        assert_eq!(view.bribe_label(), "Bribe 12g");
        view.gold = 12;
        assert_eq!(
            menu.key(MenuKey::Enter, &view),
            Some(EncounterIntent::Choice(EncounterChoice::Bribe))
        );
        assert!(menu.message.is_empty());
        view.bribe = Some(0);
        assert_eq!(view.bribe_label(), "Bribe free");
        assert!(view.bribe_allowed());
        view.bribe = None;
        assert_eq!(menu.key(MenuKey::Enter, &view), None);
        assert_eq!(menu.message, "They cannot be bribed");
        menu.key(MenuKey::Up, &view);
        assert_eq!(menu.cursor, 0);
        menu.key(MenuKey::Left, &view);
        assert_eq!(menu.cursor, 3, "wraps");
        assert_eq!(
            menu.key(MenuKey::Char('h'), &view),
            Some(EncounterIntent::Choice(EncounterChoice::Hide))
        );
        assert_eq!(
            menu.key(MenuKey::Char('r'), &view),
            Some(EncounterIntent::Choice(EncounterChoice::Run))
        );
        assert_eq!(
            menu.key(MenuKey::Char('a'), &view),
            Some(EncounterIntent::Choice(EncounterChoice::Attack))
        );
        assert_eq!(
            menu.key(MenuKey::Escape, &view),
            Some(EncounterIntent::Pause)
        );
    }

    #[test]
    fn the_defeat_menu_loads_or_quits() {
        let mut menu = DefeatMenu::default();
        assert_eq!(menu.key(MenuKey::Escape), None, "no way out but the two");
        assert_eq!(menu.key(MenuKey::Enter), Some(DefeatAction::Load));
        menu.key(MenuKey::Down);
        assert_eq!(menu.key(MenuKey::Enter), Some(DefeatAction::QuitToTitle));
        menu.key(MenuKey::Down);
        assert_eq!(menu.cursor, 0, "wraps");
    }

    #[test]
    fn redraws_follow_who_stands_there() {
        assert!(redraws(&Event::Bribed { cost: 1 }));
        assert!(redraws(&Event::Death {
            target: ActorRef::Stack(0),
            gold: None
        }));
        assert!(!redraws(&Event::RoundStarted { round: 2 }));
        assert!(!redraws(&Event::PartyChanged));
    }
}
