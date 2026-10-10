//! The character sheet as a value: what one member's three pages show (`sheet_view`), the
//! member and page under the cursor (`SheetMenu`), and the keys that move them. Bevy-free;
//! `sheet_screen.rs` paints it and `sheet.rs` wires it to the world.

use crate::defs;
use crate::menu::{MenuKey, cycle, words};
use crate::sim::Views;
use omnis_sim::api::{MemberView, ModeKind, PartyView};
use omnis_sim::omnis_data::{Ability, Data, EquipSlot};

/// A page of the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetPage {
    /// Scores, saves, skills, conditions.
    #[default]
    Stats,
    /// Spells known, effects in force, the casting ability.
    Magic,
    /// What is worn and wielded, what is carried.
    Gear,
}

impl SheetPage {
    /// Every page, in tab order.
    pub const ALL: [SheetPage; 3] = [SheetPage::Stats, SheetPage::Magic, SheetPage::Gear];

    /// The tab's word.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            SheetPage::Stats => "Stats",
            SheetPage::Magic => "Magic",
            SheetPage::Gear => "Gear",
        }
    }

    /// The page's index in tab order.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A number with its sign, as the sheet shows bonuses.
#[must_use]
pub fn signed(n: i64) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// The magic page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SheetMagic {
    /// The casting ability's name, for a caster.
    pub casting: Option<String>,
    /// Spell points, current and maximum.
    pub points: (u32, u32),
    /// Spells known as `(name, cost)`; the cost is "cantrip" or "n pt".
    pub spells: Vec<(String, String)>,
    /// Effects in force on the member, then on the party, as `(spell, time left)`.
    pub effects: Vec<(String, String)>,
}

/// The gear page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SheetGear {
    /// The four slots as `(slot, item)`; `-` when empty.
    pub slots: Vec<(String, String)>,
    /// Everything carried as `(item, count)`.
    pub carried: Vec<(String, u16)>,
}

/// One member's sheet, every number already derived.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SheetView {
    /// The name.
    pub name: String,
    /// The race's display name.
    pub race: String,
    /// The class's display name.
    pub class: String,
    /// The level.
    pub level: u8,
    /// The background's display name.
    pub background: String,
    /// The alignment as words.
    pub alignment: String,
    /// Age in years: at creation plus the party's subjective time (PRD §7.5).
    pub age_years: i64,
    /// Hit points, current and maximum.
    pub hp: (i32, i32),
    /// Spell points, current and maximum.
    pub sp: (u32, u32),
    /// Hit dice as `(left, total, sides)`: one per level, spent on short rests.
    pub hit_dice: (u8, u8, u8),
    /// The experience has reached a level a trainer has not granted.
    pub ready: bool,
    /// Spells owed by levels, chosen at a trainer.
    pub spell_picks: u8,
    /// Armour class.
    pub ac: i64,
    /// Proficiency bonus.
    pub proficiency: i64,
    /// Experience points.
    pub xp: u32,
    /// The next level's threshold, when there is one.
    pub next_xp: Option<i64>,
    /// The six scores as `(short name, score, modifier)`.
    pub scores: [(&'static str, u8, i64); 6],
    /// The class's two saving throws as `(short name, bonus)`.
    pub saves: Vec<(&'static str, i64)>,
    /// Proficient skills as `(name, bonus)`.
    pub skills: Vec<(String, i64)>,
    /// Conditions in effect.
    pub conditions: Vec<String>,
    /// The magic page.
    pub magic: SheetMagic,
    /// The gear page.
    pub gear: SheetGear,
    /// Whether the tactics panel may open: declaring is refused in a fight.
    pub tactics: bool,
}

/// The sheet of the member in this slot, or `None` when the slot is empty.
#[must_use]
pub fn sheet_view(views: &Views, data: &Data, member: usize) -> Option<SheetView> {
    let m = views.party.members.get(member)?;
    let mut scores = [("", 0, 0); 6];
    for (i, ability) in Ability::ALL.iter().enumerate() {
        scores[i] = (ability.short(), m.scores[i], m.modifiers[i]);
    }
    let saves = m
        .saves
        .iter()
        .map(|(a, bonus)| (a.short(), *bonus))
        .collect();
    let skills = m
        .skills
        .iter()
        .map(|(s, bonus)| (words(&format!("{s:?}")), *bonus))
        .collect();
    let named = |name: Option<&str>| name.map_or("?", |n| data.label("en", n)).to_owned();
    Some(SheetView {
        name: m.name.clone(),
        race: named(defs::race(data, &m.race).map(|r| r.name.as_str())),
        class: named(defs::class(data, &m.class).map(|c| c.name.as_str())),
        level: m.level,
        background: named(defs::background(data, &m.background).map(|b| b.name.as_str())),
        alignment: words(&format!("{:?}", m.alignment)),
        age_years: m.age_years,
        hp: (m.hp, m.hp_max),
        sp: (m.spell_points, m.spell_points_max),
        hit_dice: (m.hit_dice_left, m.hit_dice, m.hit_die),
        ready: m.ready,
        spell_picks: m.spell_picks,
        ac: m.ac,
        proficiency: m.proficiency,
        xp: m.xp,
        next_xp: m.next_xp,
        scores,
        saves,
        skills,
        conditions: m
            .conditions
            .iter()
            .map(|c| named(defs::condition(data, c).map(|c| c.name.as_str())))
            .collect(),
        magic: magic_page(&views.party, data, m),
        gear: gear_page(data, m),
        tactics: !matches!(views.here.mode, ModeKind::Encounter | ModeKind::Combat),
    })
}

/// How long an effect has left, in words.
fn time_left(minutes_left: Option<i64>) -> String {
    match minutes_left {
        Some(m) => format!("{m} min"),
        None => "next turn".to_owned(),
    }
}

fn magic_page(party: &PartyView, data: &Data, m: &MemberView) -> SheetMagic {
    let spell_name = |id: &str| {
        defs::spell(data, id)
            .map_or("?", |s| data.label("en", &s.name))
            .to_owned()
    };
    let spells = m
        .spells
        .iter()
        .map(|id| {
            let cost = defs::spell(data, id).map_or(0, |s| s.point_cost());
            let cost = if cost == 0 {
                "cantrip".to_owned()
            } else {
                format!("{cost} pt")
            };
            (spell_name(id), cost)
        })
        .collect();
    let effects = m
        .effects
        .iter()
        .chain(party.effects.iter())
        .map(|e| (spell_name(&e.spell), time_left(e.minutes_left)))
        .collect();
    SheetMagic {
        casting: m.casting.map(|a| words(&format!("{a:?}"))),
        points: (m.spell_points, m.spell_points_max),
        spells,
        effects,
    }
}

/// The slot's label on the sheet.
#[must_use]
pub const fn slot_label(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::MainHand => "Main hand",
        EquipSlot::OffHand => "Off hand",
        EquipSlot::Ranged => "Ranged",
        EquipSlot::Body => "Body",
    }
}

fn gear_page(data: &Data, m: &MemberView) -> SheetGear {
    let item_name = |id: &str| {
        defs::item(data, id)
            .map_or("?", |i| data.label("en", &i.name))
            .to_owned()
    };
    SheetGear {
        slots: EquipSlot::ALL
            .iter()
            .map(|slot| {
                let worn = m
                    .worn
                    .iter()
                    .find(|(s, _)| s == slot)
                    .map_or("-".to_owned(), |(_, id)| item_name(id));
                (slot_label(*slot).to_owned(), worn)
            })
            .collect(),
        carried: m
            .equipment
            .iter()
            .map(|i| (item_name(&i.item), i.count))
            .collect(),
    }
}

/// The widget row of the TACTICS button, after the member row.
pub const ROW_TACTICS: usize = 4;

/// What the sheet asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetIntent {
    /// Close the sheet.
    Close,
    /// Open the tactics panel on the member shown.
    Tactics,
}

/// The sheet's cursor: which member, which page. Left/Right change the member, Tab or
/// Up/Down the page, T or the TACTICS button open the tactics panel, Escape or P close.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SheetMenu {
    /// The member shown.
    pub member: usize,
    /// The page shown.
    pub page: SheetPage,
    /// TACTICS was clicked: the Enter that follows the click opens the panel.
    pub armed: bool,
    /// Why the last key did nothing; empty when it did something.
    pub message: String,
}

impl SheetMenu {
    /// Open on the band's selected member, or the first.
    pub fn open(&mut self, selected: Option<usize>, members: usize) {
        self.member = selected.unwrap_or(0);
        self.message.clear();
        self.sync(members);
    }

    /// Keep the member inside the party.
    pub fn sync(&mut self, members: usize) {
        self.member = self.member.min(members.saturating_sub(1));
    }

    /// A click on a row: the three tabs choose a page; the member row's arrows arrive as
    /// keys.
    pub fn click_row(&mut self, row: usize) {
        if let Some(page) = SheetPage::ALL.get(row) {
            self.page = *page;
        }
        self.armed = row == ROW_TACTICS;
    }

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey, members: usize) -> Option<SheetIntent> {
        self.message.clear();
        match key {
            MenuKey::Left | MenuKey::Right => self.member = cycle(self.member, members, key),
            MenuKey::Up | MenuKey::Down => {
                self.page = SheetPage::ALL[cycle(self.page.index(), 3, key)];
            }
            MenuKey::Char('\t') => {
                self.page = SheetPage::ALL[cycle(self.page.index(), 3, MenuKey::Right)];
            }
            MenuKey::Escape | MenuKey::Char('p') => return Some(SheetIntent::Close),
            MenuKey::Char('t') => return Some(SheetIntent::Tactics),
            MenuKey::Enter if std::mem::take(&mut self.armed) => {
                return Some(SheetIntent::Tactics);
            }
            MenuKey::Enter | MenuKey::Char(_) | MenuKey::Backspace => {}
        }
        None
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::combat_menu::tests::{data, facing};
    use omnis_sim::Mode;
    use omnis_sim::World;
    use omnis_sim::omnis_core::Dice;
    use omnis_sim::omnis_data::BuffOn;
    use omnis_sim::omnis_rules::{ActiveEffect, EffectKind, Expiry, armor_class};

    fn sheet_view(world: &World, data: &Data, member: usize) -> Option<SheetView> {
        super::sheet_view(&Views::of(world, data), data, member)
    }

    /// A fighter's sheet with something on every page, for the screen dumps.
    pub(crate) fn sample() -> SheetView {
        SheetView {
            name: "Brenna".to_owned(),
            race: "Human".to_owned(),
            class: "Fighter".to_owned(),
            level: 1,
            background: "Acolyte".to_owned(),
            alignment: "Lawful Good".to_owned(),
            age_years: 24,
            hp: (12, 12),
            sp: (0, 0),
            hit_dice: (1, 1, 10),
            ready: false,
            spell_picks: 0,
            ac: 18,
            proficiency: 2,
            xp: 25,
            next_xp: Some(300),
            scores: [
                ("STR", 16, 3),
                ("DEX", 15, 2),
                ("CON", 14, 2),
                ("INT", 13, 1),
                ("WIS", 11, 0),
                ("CHA", 9, -1),
            ],
            saves: vec![("STR", 5), ("CON", 4)],
            skills: vec![
                ("Athletics".to_owned(), 5),
                ("Insight".to_owned(), 2),
                ("Perception".to_owned(), 2),
                ("Religion".to_owned(), 3),
            ],
            conditions: vec!["Wounded".to_owned()],
            magic: SheetMagic {
                casting: Some("Intelligence".to_owned()),
                points: (1, 1),
                spells: vec![
                    ("Fire Bolt".to_owned(), "cantrip".to_owned()),
                    ("Light".to_owned(), "cantrip".to_owned()),
                    ("Magic Missile".to_owned(), "1 pt".to_owned()),
                    ("Shield".to_owned(), "1 pt".to_owned()),
                ],
                effects: vec![
                    ("Light".to_owned(), "42 min".to_owned()),
                    ("Shield".to_owned(), "next turn".to_owned()),
                ],
            },
            gear: SheetGear {
                slots: vec![
                    ("Main hand".to_owned(), "Longsword".to_owned()),
                    ("Off hand".to_owned(), "Shield".to_owned()),
                    ("Ranged".to_owned(), "Light crossbow".to_owned()),
                    ("Body".to_owned(), "Chain mail".to_owned()),
                ],
                carried: vec![
                    ("Chain mail".to_owned(), 1),
                    ("Longsword".to_owned(), 1),
                    ("Shield".to_owned(), 1),
                    ("Light crossbow".to_owned(), 1),
                    ("Crossbow bolt".to_owned(), 20),
                    ("Potion of healing".to_owned(), 1),
                ],
            },
            tactics: true,
        }
    }

    #[test]
    fn the_fighter_and_the_wizard_have_their_numbers() {
        let data = data();
        let mut world = facing(&data, &["fighter", "wizard"], &[("giant_rat", 1)]);
        world.mode = Mode::Explore;
        let brenna = sheet_view(&world, &data, 0).expect("the fighter");
        assert_eq!(
            (brenna.name.as_str(), brenna.class.as_str()),
            ("Brenna", "Fighter")
        );
        assert_eq!(brenna.race, "Human");
        assert_eq!(brenna.background, "Acolyte");
        assert_eq!(brenna.alignment, "Lawful Good");
        assert_eq!((brenna.level, brenna.proficiency, brenna.xp), (1, 2, 0));
        assert_eq!(brenna.next_xp, Some(300));
        // Human: every score one up from [15, 14, 13, 12, 10, 8].
        assert_eq!(brenna.scores[0], ("STR", 16, 3));
        assert_eq!(brenna.scores[5], ("CHA", 9, -1));
        assert_eq!(brenna.saves, [("STR", 5), ("CON", 4)]);
        // The class picks plus the acolyte's Insight and Religion.
        assert_eq!(
            brenna.skills,
            [
                ("Athletics".to_owned(), 5),
                ("Insight".to_owned(), 2),
                ("Perception".to_owned(), 2),
                ("Religion".to_owned(), 3)
            ]
        );
        assert!(brenna.conditions.is_empty());
        assert_eq!(brenna.ac, armor_class(&world.party.members[0], &data));
        assert!(brenna.ac >= 18, "chain mail and a shield: {}", brenna.ac);
        assert_eq!(
            brenna.gear.slots[0],
            ("Main hand".to_owned(), "Longsword".to_owned())
        );
        assert_eq!(brenna.gear.slots[3].1, "Chain mail");
        assert!(
            brenna
                .gear
                .carried
                .iter()
                .any(|(n, c)| n == "Crossbow bolt" && *c == 20)
        );
        assert_eq!(brenna.magic.casting, None);
        assert!(brenna.magic.spells.is_empty());
        assert_eq!(
            brenna.age_years,
            i64::from(world.party.members[0].age_years)
        );

        let wren = sheet_view(&world, &data, 1).expect("the wizard");
        assert_eq!(wren.magic.casting.as_deref(), Some("Intelligence"));
        assert_eq!(wren.magic.points, (1, 1), "Int 13: one point");
        assert_eq!(
            wren.magic.spells.len(),
            7,
            "three cantrips, four first-level spells"
        );
        assert_eq!(
            wren.magic.spells[0],
            ("Fire Bolt".to_owned(), "cantrip".to_owned())
        );
        assert_eq!(
            wren.magic.spells[3],
            ("Magic Missile".to_owned(), "1 pt".to_owned())
        );
        assert_eq!(wren.saves, [("INT", 3), ("WIS", 2)]);
        assert_eq!(
            brenna.hit_dice,
            (1, 1, 10),
            "a fighter's d10, one per level"
        );
        assert_eq!(wren.hit_dice, (1, 1, 6), "a wizard's d6");
        world.party.members[0].hit_dice_spent = 1;
        assert_eq!(
            sheet_view(&world, &data, 0).expect("the fighter").hit_dice,
            (0, 1, 10),
            "spent on a short rest"
        );
        let fresh = sheet_view(&world, &data, 0).unwrap();
        assert_eq!((fresh.ready, fresh.spell_picks), (false, 0));
        world.party.members[0].xp = 300;
        world.party.members[0].spell_picks = 2;
        let trained = sheet_view(&world, &data, 0).unwrap();
        assert_eq!(
            (trained.ready, trained.spell_picks),
            (true, 2),
            "300 XP: level 2 waits"
        );
        assert_eq!(sheet_view(&world, &data, 2), None, "no third member");
    }

    #[test]
    fn effects_show_their_time_left_and_age_follows_the_clock() {
        let data = data();
        let mut world = facing(&data, &["wizard"], &[("giant_rat", 1)]);
        world.mode = Mode::Explore;
        let now = world.party_clock().elapsed;
        let light = world.party.members[0].known_spells[1];
        let caster = world.party.members[0].id;
        world.party.members[0].effects.push(ActiveEffect {
            source: light,
            caster,
            concentration: false,
            kind: EffectKind::Light { depth: 8 },
            until: Expiry::Minute(now + 42),
        });
        let shield = world.party.members[0].known_spells[4];
        world.party.effects.push(ActiveEffect {
            source: shield,
            caster,
            concentration: false,
            kind: EffectKind::Buff {
                bonus: Dice::new(1, 4),
                on: vec![BuffOn::AttackRolls],
                consumed: false,
            },
            until: Expiry::NextTurn,
        });
        let view = sheet_view(&world, &data, 0).unwrap();
        assert_eq!(
            view.magic.effects,
            [
                ("Light".to_owned(), "42 min".to_owned()),
                ("Shield".to_owned(), "next turn".to_owned())
            ]
        );
        let born = view.age_years;
        let clock = world.clocks.get_mut(&omnis_sim::PARTY).unwrap();
        clock.elapsed += 1440 * 360 * 2 + 5;
        let older = sheet_view(&world, &data, 0).unwrap();
        assert_eq!(older.age_years, born + 2, "two subjective years passed");
        assert_eq!(older.magic.effects[0].1, "0 min", "never negative");
        assert_eq!(signed(3), "+3");
        assert_eq!(signed(-1), "-1");
        assert_eq!(signed(0), "+0");
    }

    #[test]
    fn keys_move_the_member_and_the_page_and_close() {
        let mut menu = SheetMenu::default();
        menu.open(Some(5), 3);
        assert_eq!(menu.member, 2, "clamped to the party");
        assert_eq!(menu.key(MenuKey::Right, 3), None);
        assert_eq!(menu.member, 0, "wraps");
        menu.key(MenuKey::Left, 3);
        assert_eq!(menu.member, 2);
        assert_eq!(menu.page, SheetPage::Stats);
        menu.key(MenuKey::Char('\t'), 3);
        assert_eq!(menu.page, SheetPage::Magic);
        menu.key(MenuKey::Down, 3);
        assert_eq!(menu.page, SheetPage::Gear);
        menu.key(MenuKey::Down, 3);
        assert_eq!(menu.page, SheetPage::Stats, "wraps");
        menu.key(MenuKey::Up, 3);
        assert_eq!(menu.page, SheetPage::Gear);
        menu.click_row(1);
        assert_eq!(menu.page, SheetPage::Magic);
        menu.click_row(3);
        assert_eq!(menu.page, SheetPage::Magic, "the member row is not a tab");
        assert_eq!(menu.key(MenuKey::Enter, 3), None);
        menu.click_row(ROW_TACTICS);
        assert_eq!(menu.page, SheetPage::Magic, "TACTICS is not a tab");
        assert_eq!(menu.key(MenuKey::Enter, 3), Some(SheetIntent::Tactics));
        assert_eq!(menu.key(MenuKey::Enter, 3), None, "one click, one opening");
        menu.click_row(ROW_TACTICS);
        menu.click_row(0);
        assert_eq!(menu.key(MenuKey::Enter, 3), None, "a tab click disarms it");
        assert_eq!(menu.key(MenuKey::Char('t'), 3), Some(SheetIntent::Tactics));
        assert_eq!(menu.key(MenuKey::Escape, 3), Some(SheetIntent::Close));
        assert_eq!(menu.key(MenuKey::Char('p'), 3), Some(SheetIntent::Close));
        menu.open(None, 0);
        assert_eq!(menu.member, 0);
        assert_eq!(menu.key(MenuKey::Right, 0), None, "an empty party is safe");
        assert_eq!(SheetPage::Gear.label(), "Gear");
        assert_eq!(slot_label(EquipSlot::OffHand), "Off hand");
    }
}
