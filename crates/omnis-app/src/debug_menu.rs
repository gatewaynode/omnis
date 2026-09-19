//! The debug menu as a pure state machine: the world's numbers as rows a developer can push
//! up and down, items to hand out, conditions and flags to set, a teleport, and the stacks of
//! a fight. Every change is a `Dev` command through the simulation, so replays carry it and a
//! release world refuses it. Bevy-free; `debug.rs` (feature `devtools`) feeds keys and applies
//! intents.

use crate::menu::{MenuKey, cycle};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::{Ability, Data};
use omnis_sim::{DevCommand, Mode, World};

/// One member's numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberDebug {
    /// The name.
    pub name: String,
    /// Hit points and maximum.
    pub hp: (i32, i32),
    /// Spell points and maximum.
    pub sp: (u32, u32),
    /// Experience.
    pub xp: u32,
    /// The six scores in SRD order.
    pub scores: [u8; 6],
    /// Condition ids in effect.
    pub conditions: Vec<String>,
}

/// One stack of a fight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDebug {
    /// Its index in the encounter.
    pub index: u8,
    /// The monster's name.
    pub name: String,
    /// The living and the initial count.
    pub count: (u8, u8),
    /// The lead individual's hit points; zero when none stands.
    pub lead_hp: i32,
}

/// What the menu reads, built from the world every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugView {
    /// Whether a fight is on: stack edits apply, teleport does not.
    pub fighting: bool,
    /// Whether the world accepts dev commands at all.
    pub devtools: bool,
    /// The members in marching order.
    pub members: Vec<MemberDebug>,
    /// The party's gold.
    pub gold: u32,
    /// The party's food.
    pub food: u32,
    /// Every item id with its label, in id order.
    pub items: Vec<(String, String)>,
    /// Every condition id with its label, in id order.
    pub conditions: Vec<(String, String)>,
    /// Every flag id, in id order.
    pub flags: Vec<String>,
    /// Every map id with its size, in id order.
    pub maps: Vec<(String, u16, u16)>,
    /// Where the party stands: the map's index in `maps`, the tile, the facing.
    pub position: (usize, u16, u16, Facing),
    /// The stacks of the fight, when one is on.
    pub stacks: Vec<StackDebug>,
}

/// The view of a world.
#[must_use]
pub fn debug_view(world: &World, data: &Data) -> DebugView {
    let members = world
        .party
        .members
        .iter()
        .map(|m| MemberDebug {
            name: m.name.clone(),
            hp: (m.hp, m.hp_max),
            sp: (m.spell_points, m.spell_points_max),
            xp: m.xp,
            scores: m.scores,
            conditions: m
                .conditions
                .iter()
                .map(|c| data.registry.conditions.name(*c).unwrap_or("?").to_owned())
                .collect(),
        })
        .collect();
    let items = data
        .items
        .iter()
        .filter_map(|(id, item)| {
            let name = data.registry.items.name(*id)?;
            Some((name.to_owned(), data.label("en", &item.name).to_owned()))
        })
        .collect();
    let conditions = data
        .conditions
        .iter()
        .filter_map(|(id, c)| {
            let name = data.registry.conditions.name(*id)?;
            Some((name.to_owned(), data.label("en", &c.name).to_owned()))
        })
        .collect();
    let maps: Vec<(String, u16, u16)> = data
        .maps
        .iter()
        .filter_map(|(id, map)| {
            let name = data.registry.maps.name(*id)?;
            Some((name.to_owned(), map.def.width, map.def.height))
        })
        .collect();
    let p = world.position;
    let here = data.registry.maps.name(p.map).unwrap_or("?");
    let map_index = maps.iter().position(|(m, _, _)| m == here).unwrap_or(0);
    let stacks = match &world.mode {
        Mode::Combat(state) => state
            .encounter
            .stacks
            .iter()
            .enumerate()
            .map(|(i, s)| StackDebug {
                index: u8::try_from(i).unwrap_or(u8::MAX),
                name: data
                    .monsters
                    .get(&s.monster)
                    .map_or("?", |m| data.label("en", &m.name))
                    .to_owned(),
                count: (s.count(), s.initial),
                lead_hp: s.hp.first().copied().unwrap_or(0),
            })
            .collect(),
        _ => Vec::new(),
    };
    DebugView {
        fighting: matches!(world.mode, Mode::Combat(_)),
        devtools: world.settings.devtools,
        members,
        gold: world.party.gold,
        food: world.party.food,
        items,
        conditions,
        flags: data.registry.flags.names().map(str::to_owned).collect(),
        maps,
        position: (map_index, p.x, p.y, p.facing),
        stacks,
    }
}

/// What the menu asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugIntent {
    /// A dev command.
    Command(DevCommand),
    /// Close the menu.
    Close,
}

/// The rows, in cursor order.
pub const ROW_MEMBER: usize = 0;
/// Hit points.
pub const ROW_HP: usize = 1;
/// Spell points.
pub const ROW_SP: usize = 2;
/// Experience.
pub const ROW_XP: usize = 3;
/// One ability score.
pub const ROW_SCORE: usize = 4;
/// A condition to toggle.
pub const ROW_CONDITION: usize = 5;
/// An item to give.
pub const ROW_ITEM: usize = 6;
/// Gold.
pub const ROW_GOLD: usize = 7;
/// Food.
pub const ROW_FOOD: usize = 8;
/// A flag to set.
pub const ROW_FLAG: usize = 9;
/// A teleport.
pub const ROW_MAP: usize = 10;
/// A stack of the fight.
pub const ROW_STACK: usize = 11;
/// How many rows.
pub const ROWS: usize = 12;

/// The four facings the map row cycles.
const FACINGS: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

/// The menu: a row cursor, a field cursor on rows with several, and what each selector
/// points at. Numbers bound to the world change at once (Left/Right by one, `-`/`+` by ten,
/// `[`/`]` by a hundred); selectors are staged until Enter acts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugMenu {
    /// The row under the cursor.
    pub row: usize,
    /// The field under the cursor on rows with several.
    pub field: usize,
    /// The member the member rows edit.
    pub member: usize,
    /// The ability the score row edits.
    pub ability: usize,
    /// The condition the condition row toggles.
    pub condition: usize,
    /// The item the item row gives.
    pub item: usize,
    /// How many the item row gives.
    pub count: u16,
    /// The flag the flag row sets.
    pub flag: usize,
    /// The map the teleport goes to.
    pub map: usize,
    /// The tile the teleport goes to.
    pub tile: (u16, u16),
    /// The facing the teleport ends with.
    pub facing: usize,
    /// The stack the stack row edits.
    pub stack: usize,
    /// Why the last key did nothing; empty when it did something.
    pub message: String,
}

impl Default for DebugMenu {
    fn default() -> Self {
        DebugMenu {
            row: 0,
            field: 0,
            member: 0,
            ability: 0,
            condition: 0,
            item: 0,
            count: 1,
            flag: 0,
            map: 0,
            tile: (0, 0),
            facing: 0,
            stack: 0,
            message: String::new(),
        }
    }
}

/// Which fields a row has.
const fn fields(row: usize) -> usize {
    match row {
        ROW_SCORE | ROW_CONDITION | ROW_ITEM | ROW_FLAG | ROW_STACK => 2,
        ROW_MAP => 4,
        _ => 1,
    }
}

impl DebugMenu {
    /// Seed the teleport from where the party stands; called when the menu opens.
    pub fn open(&mut self, view: &DebugView) {
        let (map, x, y, facing) = view.position;
        self.map = map;
        self.tile = (x, y);
        self.facing = FACINGS.iter().position(|f| *f == facing).unwrap_or(0);
        self.message.clear();
        self.sync(view);
    }

    /// Keep every selector inside its list.
    pub fn sync(&mut self, view: &DebugView) {
        let clamp = |at: &mut usize, len: usize| *at = (*at).min(len.saturating_sub(1));
        clamp(&mut self.member, view.members.len());
        clamp(&mut self.condition, view.conditions.len());
        clamp(&mut self.item, view.items.len());
        clamp(&mut self.flag, view.flags.len());
        clamp(&mut self.map, view.maps.len());
        clamp(&mut self.stack, view.stacks.len());
        self.field = self.field.min(fields(self.row) - 1);
        self.count = self.count.max(1);
    }

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey, view: &DebugView) -> Option<DebugIntent> {
        self.message.clear();
        match key {
            MenuKey::Up | MenuKey::Down => {
                self.row = cycle(self.row, ROWS, key);
                self.field = 0;
            }
            MenuKey::Char('\t') => self.field = (self.field + 1) % fields(self.row),
            MenuKey::Left => return self.adjust(-1, view),
            MenuKey::Right => return self.adjust(1, view),
            MenuKey::Char('-') => return self.adjust(-10, view),
            MenuKey::Char('+' | '=') => return self.adjust(10, view),
            MenuKey::Char('[') => return self.adjust(-100, view),
            MenuKey::Char(']') => return self.adjust(100, view),
            MenuKey::Enter => return self.act(view, false),
            MenuKey::Char('s') => return self.act(view, true),
            MenuKey::Char('k') if self.row == ROW_STACK => {
                return self.stack_command(view, |stack, _| DevCommand::KillStack { stack });
            }
            MenuKey::Escape => return Some(DebugIntent::Close),
            MenuKey::Char(_) | MenuKey::Backspace => {}
        }
        None
    }

    /// The member's slot as the commands want it.
    fn slot(&self) -> u8 {
        u8::try_from(self.member).unwrap_or(u8::MAX)
    }

    /// Move a selector, or push a number bound to the world by `by`.
    fn adjust(&mut self, by: i64, view: &DebugView) -> Option<DebugIntent> {
        let step = |at: &mut usize, len: usize| {
            if len > 0 {
                let key = if by < 0 {
                    MenuKey::Left
                } else {
                    MenuKey::Right
                };
                *at = cycle(*at, len, key);
            }
        };
        let member = view.members.get(self.member);
        let command = match (self.row, self.field) {
            (ROW_MEMBER, _) => {
                step(&mut self.member, view.members.len());
                return None;
            }
            (ROW_HP, _) => DevCommand::SetHp {
                member: self.slot(),
                hp: bump(i64::from(member?.hp.0), by, i64::from(member?.hp.1)),
            },
            (ROW_SP, _) => DevCommand::SetSpellPoints {
                member: self.slot(),
                points: bump(i64::from(member?.sp.0), by, i64::from(member?.sp.1)),
            },
            (ROW_XP, _) => DevCommand::SetXp {
                member: self.slot(),
                xp: bump(i64::from(member?.xp), by, i64::from(u32::MAX)),
            },
            (ROW_SCORE, 0) => {
                step(&mut self.ability, Ability::ALL.len());
                return None;
            }
            (ROW_SCORE, _) => DevCommand::SetScore {
                member: self.slot(),
                ability: Ability::ALL[self.ability],
                score: bump::<u8>(i64::from(member?.scores[self.ability]), by, 30).max(1),
            },
            (ROW_CONDITION, _) => {
                step(&mut self.condition, view.conditions.len());
                return None;
            }
            (ROW_ITEM, 0) => {
                step(&mut self.item, view.items.len());
                return None;
            }
            (ROW_ITEM, _) => {
                self.count = bump::<u16>(i64::from(self.count), by, i64::from(u16::MAX)).max(1);
                return None;
            }
            (ROW_GOLD, _) => DevCommand::SetGold {
                gold: bump(i64::from(view.gold), by, i64::from(u32::MAX)),
            },
            (ROW_FOOD, _) => DevCommand::SetFood {
                food: bump(i64::from(view.food), by, i64::from(u32::MAX)),
            },
            (ROW_FLAG, _) => {
                step(&mut self.flag, view.flags.len());
                return None;
            }
            (ROW_MAP, 0) => {
                step(&mut self.map, view.maps.len());
                return None;
            }
            (ROW_MAP, 1 | 2) => {
                let (w, h) = view.maps.get(self.map).map_or((1, 1), |m| (m.1, m.2));
                let (x, y) = self.tile;
                self.tile = if self.field == 1 {
                    (bump(i64::from(x), by, i64::from(w) - 1), y)
                } else {
                    (x, bump(i64::from(y), by, i64::from(h) - 1))
                };
                return None;
            }
            (ROW_MAP, _) => {
                step(&mut self.facing, FACINGS.len());
                return None;
            }
            (ROW_STACK, 0) => {
                step(&mut self.stack, view.stacks.len());
                return None;
            }
            (ROW_STACK, _) => {
                return self.stack_command(view, |stack, lead| DevCommand::SetMonsterHp {
                    stack,
                    index: 0,
                    hp: bump::<i32>(i64::from(lead), by, i64::from(i32::MAX)).max(0),
                });
            }
            _ => return None,
        };
        Some(DebugIntent::Command(command))
    }

    /// Enter on a row with a staged choice: toggle the condition, give the item (to the
    /// stores when `to_stores`), set the flag, teleport, or set the stack's lead hit points.
    fn act(&mut self, view: &DebugView, to_stores: bool) -> Option<DebugIntent> {
        let command = match self.row {
            ROW_CONDITION => {
                let (id, _) = view.conditions.get(self.condition)?;
                let member = view.members.get(self.member)?;
                DevCommand::SetCondition {
                    member: self.slot(),
                    condition: id.clone(),
                    applied: !member.conditions.contains(id),
                }
            }
            ROW_ITEM => DevCommand::GiveItem {
                member: (!to_stores).then_some(self.slot()),
                item: view.items.get(self.item)?.0.clone(),
                count: self.count,
            },
            ROW_FLAG => DevCommand::SetFlag {
                flag: view.flags.get(self.flag)?.clone(),
                value: 1,
            },
            ROW_MAP if view.fighting => {
                self.message = "No teleport in a fight".to_owned();
                return None;
            }
            ROW_MAP => DevCommand::Teleport {
                map: view.maps.get(self.map)?.0.clone(),
                x: self.tile.0,
                y: self.tile.1,
                facing: FACINGS[self.facing],
            },
            _ => return None,
        };
        Some(DebugIntent::Command(command))
    }

    /// A command on the selected stack, or why not.
    fn stack_command(
        &mut self,
        view: &DebugView,
        make: impl Fn(u8, i32) -> DevCommand,
    ) -> Option<DebugIntent> {
        if !view.fighting {
            self.message = "No fight is on".to_owned();
            return None;
        }
        let stack = view.stacks.get(self.stack)?;
        Some(DebugIntent::Command(make(stack.index, stack.lead_hp)))
    }
}

/// `value + by`, clamped to `0..=max`, as the command's integer type.
fn bump<T: TryFrom<i64> + Default>(value: i64, by: i64, max: i64) -> T {
    T::try_from((value + by).clamp(0, max)).unwrap_or_default()
}

/// One line for a dev command, for the log.
#[must_use]
pub fn describe(command: &DevCommand) -> String {
    match command {
        DevCommand::GiveItem {
            member,
            item,
            count,
        } => match member {
            Some(m) => format!("dev: {count} x {item} to member {m}"),
            None => format!("dev: {count} x {item} to the stores"),
        },
        DevCommand::SetHp { member, hp } => format!("dev: member {member} hp {hp}"),
        DevCommand::SetSpellPoints { member, points } => {
            format!("dev: member {member} sp {points}")
        }
        DevCommand::SetGold { gold } => format!("dev: gold {gold}"),
        DevCommand::SetFood { food } => format!("dev: food {food}"),
        DevCommand::SetXp { member, xp } => format!("dev: member {member} xp {xp}"),
        DevCommand::SetScore {
            member,
            ability,
            score,
        } => format!("dev: member {member} {} {score}", ability.short()),
        DevCommand::SetCondition {
            member,
            condition,
            applied,
        } => format!(
            "dev: member {member} {condition} {}",
            if *applied { "on" } else { "off" }
        ),
        DevCommand::SetFlag { flag, value } => format!("dev: {flag} = {value}"),
        DevCommand::Teleport { map, x, y, facing } => {
            format!("dev: teleport to {map} ({x}, {y}) facing {facing}")
        }
        DevCommand::SetMonsterHp { stack, index, hp } => {
            format!("dev: stack {stack} #{} hp {hp}", index + 1)
        }
        DevCommand::KillStack { stack } => format!("dev: stack {stack} slain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_data::load_packs;
    use omnis_sim::{Command, PartyCommand, Settings, apply};
    use std::path::PathBuf;

    fn world_and_data() -> (World, Data) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")])
            .unwrap_or_else(|r| panic!("{r}"));
        let settings = Settings {
            devtools: true,
            ..Settings::default()
        };
        let mut world = World::new(&data, 1, settings).unwrap();
        let draft = omnis_sim::omnis_rules::Draft {
            name: "Brenna".to_owned(),
            race: "base:race:human".to_owned(),
            class: "base:class:fighter".to_owned(),
            background: "base:background:acolyte".to_owned(),
            alignment: omnis_sim::omnis_data::Alignment::LawfulGood,
            scores: [15, 14, 13, 12, 10, 8],
            skills: vec![
                omnis_sim::omnis_data::Skill::Athletics,
                omnis_sim::omnis_data::Skill::Perception,
            ],
        };
        apply(
            &mut world,
            &data,
            Command::Party(PartyCommand::Create(draft)),
        )
        .unwrap();
        (world, data)
    }

    #[test]
    fn the_view_reads_the_world_and_the_packs() {
        let (world, data) = world_and_data();
        let view = debug_view(&world, &data);
        assert!(view.devtools && !view.fighting);
        assert_eq!(view.members.len(), 1);
        assert_eq!(view.members[0].name, "Brenna");
        assert_eq!(view.items.len(), 24);
        assert_eq!(view.items[0].0, "base:item:arrows");
        assert_eq!(view.conditions.len(), 16);
        assert_eq!(view.maps.len(), 2);
        assert_eq!(view.position.1, 16, "the meadow start");
        assert!(view.stacks.is_empty());
    }

    #[test]
    fn numbers_move_at_once_and_choices_wait_for_enter() {
        let (world, data) = world_and_data();
        let view = debug_view(&world, &data);
        let mut menu = DebugMenu::default();
        menu.open(&view);
        assert_eq!(menu.map, view.position.0);
        assert_eq!(menu.tile, (view.position.1, view.position.2));
        menu.key(MenuKey::Down, &view);
        assert_eq!(menu.row, ROW_HP);
        let max = view.members[0].hp.1;
        assert_eq!(
            menu.key(MenuKey::Left, &view),
            Some(DebugIntent::Command(DevCommand::SetHp {
                member: 0,
                hp: max - 1
            }))
        );
        assert_eq!(
            menu.key(MenuKey::Char('['), &view),
            Some(DebugIntent::Command(DevCommand::SetHp { member: 0, hp: 0 })),
            "clamped at zero"
        );
        menu.row = ROW_GOLD;
        assert_eq!(
            menu.key(MenuKey::Char('+'), &view),
            Some(DebugIntent::Command(DevCommand::SetGold {
                gold: view.gold + 10
            }))
        );
        menu.row = ROW_ITEM;
        menu.key(MenuKey::Right, &view);
        assert_eq!(menu.item, 1);
        menu.key(MenuKey::Char('\t'), &view);
        assert_eq!(menu.field, 1);
        menu.key(MenuKey::Right, &view);
        assert_eq!(menu.count, 2);
        assert_eq!(
            menu.key(MenuKey::Enter, &view),
            Some(DebugIntent::Command(DevCommand::GiveItem {
                member: Some(0),
                item: view.items[1].0.clone(),
                count: 2
            }))
        );
        assert_eq!(
            menu.key(MenuKey::Char('s'), &view),
            Some(DebugIntent::Command(DevCommand::GiveItem {
                member: None,
                item: view.items[1].0.clone(),
                count: 2
            }))
        );
        menu.row = ROW_CONDITION;
        menu.field = 0;
        assert_eq!(
            menu.key(MenuKey::Enter, &view),
            Some(DebugIntent::Command(DevCommand::SetCondition {
                member: 0,
                condition: view.conditions[0].0.clone(),
                applied: true
            }))
        );
        menu.row = ROW_SCORE;
        menu.key(MenuKey::Right, &view);
        assert_eq!(menu.ability, 1, "Dexterity");
        menu.key(MenuKey::Char('\t'), &view);
        assert_eq!(
            menu.key(MenuKey::Right, &view),
            Some(DebugIntent::Command(DevCommand::SetScore {
                member: 0,
                ability: Ability::Dexterity,
                score: 16
            }))
        );
        menu.row = ROW_MAP;
        menu.field = 1;
        menu.key(MenuKey::Left, &view);
        assert_eq!(menu.tile.0, view.position.1 - 1);
        menu.field = 3;
        menu.key(MenuKey::Right, &view);
        assert_eq!(
            menu.key(MenuKey::Enter, &view),
            Some(DebugIntent::Command(DevCommand::Teleport {
                map: view.maps[view.position.0].0.clone(),
                x: view.position.1 - 1,
                y: view.position.2,
                facing: Facing::East
            }))
        );
        menu.row = ROW_STACK;
        assert_eq!(menu.key(MenuKey::Char('k'), &view), None);
        assert_eq!(menu.message, "No fight is on");
        assert_eq!(menu.key(MenuKey::Escape, &view), Some(DebugIntent::Close));
        assert_eq!(
            describe(&DevCommand::SetHp { member: 0, hp: 5 }),
            "dev: member 0 hp 5"
        );
    }
}
