//! The menus as pure state machines: title, new game (seed and difficulty), character
//! creation by point buy, and the pause overlay. Each screen is a value that takes a key and
//! answers with an action, and renders itself as lines of text. Bevy-free, so every
//! transition is unit-tested; `menus.rs` only spawns text and feeds keys.

use omnis_sim::omnis_core::fnv1a64;
use omnis_sim::omnis_data::{Ability, Alignment, Data, Skill};
use omnis_sim::omnis_rules::Draft;
use omnis_sim::{SaveRule, Settings};
use std::collections::BTreeMap;

/// A key as the menus see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Enter or Space on a button.
    Enter,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// A printable character.
    Char(char),
}

/// The help line every screen ends with.
const HELP: &str = "Up/Down select   Left/Right change   Enter confirm   Esc back";

/// Camel-case variant names as words: `LawfulGood` to `Lawful Good`.
#[must_use]
pub fn words(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

pub(crate) fn cycle(index: usize, len: usize, key: MenuKey) -> usize {
    if len == 0 {
        return 0;
    }
    match key {
        MenuKey::Left | MenuKey::Up => (index + len - 1) % len,
        MenuKey::Right | MenuKey::Down => (index + 1) % len,
        _ => index,
    }
}

fn mark(cursor: usize, row: usize, text: String) -> String {
    if cursor == row {
        format!("> {text}")
    } else {
        format!("  {text}")
    }
}

// ---------------------------------------------------------------- title

/// What the title screen asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleAction {
    /// Go to the new game screen.
    NewGame,
    /// Load the quick save.
    Load,
    /// Exit.
    Quit,
}

/// The title screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Title {
    /// Selected item.
    pub cursor: usize,
}

impl Title {
    /// The items, in cursor order.
    pub const ITEMS: [&'static str; 3] = ["New game", "Load quick save", "Quit"];

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<TitleAction> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, 3, key),
            MenuKey::Enter => {
                return Some(match self.cursor {
                    0 => TitleAction::NewGame,
                    1 => TitleAction::Load,
                    _ => TitleAction::Quit,
                });
            }
            _ => {}
        }
        None
    }

    /// The screen as lines.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec!["OMNIS".to_owned(), String::new()];
        lines.extend(
            Self::ITEMS
                .iter()
                .enumerate()
                .map(|(i, item)| mark(self.cursor, i, (*item).to_owned())),
        );
        lines.push(String::new());
        lines.push(HELP.to_owned());
        lines
    }
}

/// "on" or "off".
#[must_use]
pub const fn on_off(flag: bool) -> &'static str {
    if flag { "on" } else { "off" }
}

// ---------------------------------------------------------------- new game

/// What the new game screen asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewGameAction {
    /// Start with the form's seed and settings.
    Start,
    /// Back to the title.
    Back,
}

/// Seed text plus the difficulty controls (PRD D17).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewGameForm {
    /// What was typed; blank means a random seed.
    pub seed_text: String,
    /// The difficulty.
    pub settings: Settings,
    /// Selected row: seed, save rule, permadeath, start, back.
    pub cursor: usize,
}

/// The save rule as the screens name it.
#[must_use]
pub fn rule_label(rule: SaveRule) -> &'static str {
    match rule {
        SaveRule::Anywhere => "Anywhere",
        SaveRule::Relief => "Inns, items, and spells",
        SaveRule::InnOnly => "Inns only",
    }
}

impl NewGameForm {
    const RULES: [SaveRule; 3] = [SaveRule::Anywhere, SaveRule::Relief, SaveRule::InnOnly];

    /// The seed: a number as typed, any other text hashed, blank as `entropy`.
    #[must_use]
    pub fn seed(&self, entropy: u64) -> u64 {
        let text = self.seed_text.trim();
        if text.is_empty() {
            entropy
        } else {
            text.parse().unwrap_or_else(|_| fnv1a64(text.as_bytes()))
        }
    }

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<NewGameAction> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, 5, key),
            MenuKey::Left | MenuKey::Right => match self.cursor {
                1 => {
                    let at = Self::RULES
                        .iter()
                        .position(|r| *r == self.settings.save_rule)
                        .unwrap_or(0);
                    self.settings.save_rule = Self::RULES[cycle(at, 3, key)];
                }
                2 => self.settings.permadeath = !self.settings.permadeath,
                _ => {}
            },
            MenuKey::Char(c) if self.cursor == 0 && self.seed_text.len() < 32 => {
                self.seed_text.push(c);
            }
            MenuKey::Backspace if self.cursor == 0 => {
                self.seed_text.pop();
            }
            MenuKey::Enter => match self.cursor {
                3 => return Some(NewGameAction::Start),
                4 => return Some(NewGameAction::Back),
                _ => {}
            },
            MenuKey::Escape => return Some(NewGameAction::Back),
            _ => {}
        }
        None
    }

    /// The screen as lines.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let rule = rule_label(self.settings.save_rule);
        vec![
            "NEW GAME".to_owned(),
            String::new(),
            mark(
                self.cursor,
                0,
                format!(
                    "Seed: {}_   (blank for random; words are hashed)",
                    self.seed_text
                ),
            ),
            mark(self.cursor, 1, format!("Saving: < {rule} >")),
            mark(
                self.cursor,
                2,
                format!(
                    "Permadeath: < {} >",
                    if self.settings.permadeath {
                        "On"
                    } else {
                        "Off"
                    }
                ),
            ),
            mark(self.cursor, 3, "Start".to_owned()),
            mark(self.cursor, 4, "Back".to_owned()),
            String::new(),
            HELP.to_owned(),
        ]
    }
}

// ---------------------------------------------------------------- creation

/// What creation can choose from, lifted out of the loaded data once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    /// Race ids.
    pub races: Vec<String>,
    /// Class ids.
    pub classes: Vec<String>,
    /// Background ids.
    pub backgrounds: Vec<String>,
    /// Per class id: how many skills to pick and from which.
    pub class_skills: BTreeMap<String, (u8, Vec<Skill>)>,
    /// Display names by id.
    pub labels: BTreeMap<String, String>,
    /// Point budget.
    pub budget: i64,
    /// Lowest bought score.
    pub min: u8,
    /// Highest bought score.
    pub max: u8,
    /// Cost by `score - min`.
    pub costs: Vec<i64>,
    /// Party slots.
    pub slots: usize,
}

impl Catalog {
    /// From the loaded packs.
    #[must_use]
    pub fn from_data(data: &Data) -> Catalog {
        let mut labels = BTreeMap::new();
        let mut races: Vec<String> = Vec::new();
        for (id, race) in &data.races {
            let name = data.registry.races.name(*id).unwrap_or("?").to_owned();
            labels.insert(name.clone(), data.label("en", &race.name).to_owned());
            races.push(name);
        }
        let mut classes = Vec::new();
        let mut class_skills = BTreeMap::new();
        for (id, class) in &data.classes {
            let name = data.registry.classes.name(*id).unwrap_or("?").to_owned();
            labels.insert(name.clone(), data.label("en", &class.name).to_owned());
            class_skills.insert(
                name.clone(),
                (class.skills.choose, class.skills.from.clone()),
            );
            classes.push(name);
        }
        let mut backgrounds = Vec::new();
        for (id, background) in &data.backgrounds {
            let name = data
                .registry
                .backgrounds
                .name(*id)
                .unwrap_or("?")
                .to_owned();
            labels.insert(name.clone(), data.label("en", &background.name).to_owned());
            backgrounds.push(name);
        }
        let value = |name: &str, default: i64| data.rules.value(name).unwrap_or(default);
        Catalog {
            races,
            classes,
            backgrounds,
            class_skills,
            labels,
            budget: value("point_budget", 27),
            min: u8::try_from(value("score_min", 8)).unwrap_or(8),
            max: u8::try_from(value("score_max", 15)).unwrap_or(15),
            costs: data
                .rules
                .table("point_cost")
                .map(<[i64]>::to_vec)
                .unwrap_or_else(|| vec![0, 1, 2, 3, 4, 5, 7, 9]),
            slots: usize::try_from(value("party_slots", 6)).unwrap_or(6),
        }
    }

    /// The display name of a race, class, or background id.
    #[must_use]
    pub fn label<'a>(&'a self, id: &'a str) -> &'a str {
        self.labels.get(id).map_or(id, String::as_str)
    }
}

/// What the creation screen asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreationAction {
    /// Add this draft to the party.
    Add(Draft),
    /// Start exploring.
    Begin,
    /// Abandon the new game.
    Back,
}

/// One member being drafted, and the cursor over the form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreationForm {
    /// Typed name.
    pub name: String,
    /// Index into the catalog's races.
    pub race: usize,
    /// Index into the catalog's classes.
    pub class: usize,
    /// Index into the catalog's backgrounds.
    pub background: usize,
    /// Index into `Alignment::ALL`.
    pub alignment: usize,
    /// Bought scores.
    pub scores: [u8; 6],
    /// Picked skills.
    pub skills: Vec<Skill>,
    /// Which skill of the class list Left/Right stands on.
    pub skill_cursor: usize,
    /// Selected row.
    pub cursor: usize,
    /// Feedback from the last attempt.
    pub message: String,
}

/// Row index of the name field.
pub const ROW_NAME: usize = 0;
/// Row index of the race choice.
pub const ROW_RACE: usize = 1;
/// Row index of the class choice.
pub const ROW_CLASS: usize = 2;
/// Row index of the background choice.
pub const ROW_BACKGROUND: usize = 3;
/// Row index of the alignment choice.
pub const ROW_ALIGNMENT: usize = 4;
/// Row index of the first score; the six scores follow.
pub const ROW_SCORES: usize = 5;
/// Row index of the skill picks.
pub const ROW_SKILLS: usize = 11;
/// Row index of the Add member button.
pub const ROW_ADD: usize = 12;
/// Row index of the Begin button.
pub const ROW_BEGIN: usize = 13;
/// Rows the cursor cycles through.
pub const ROWS: usize = 14;

impl CreationForm {
    /// A blank form at the catalog's minimum scores.
    #[must_use]
    pub fn new(catalog: &Catalog) -> CreationForm {
        CreationForm {
            scores: [catalog.min; 6],
            ..CreationForm::default()
        }
    }

    fn class_id<'a>(&self, catalog: &'a Catalog) -> &'a str {
        catalog.classes.get(self.class).map_or("", String::as_str)
    }

    /// How many skills the drafted class picks, and from which.
    #[must_use]
    pub fn skill_list<'a>(&self, catalog: &'a Catalog) -> (u8, &'a [Skill]) {
        catalog
            .class_skills
            .get(self.class_id(catalog))
            .map_or((0, &[][..]), |(n, list)| (*n, list.as_slice()))
    }

    /// Points spent on the current scores.
    #[must_use]
    pub fn spent(&self, catalog: &Catalog) -> i64 {
        self.scores
            .iter()
            .map(|s| {
                catalog
                    .costs
                    .get(usize::from(s.saturating_sub(catalog.min)))
                    .copied()
                    .unwrap_or(0)
            })
            .sum()
    }

    /// The draft as it stands.
    #[must_use]
    pub fn draft(&self, catalog: &Catalog) -> Draft {
        let pick = |list: &[String], i: usize| list.get(i).cloned().unwrap_or_default();
        Draft {
            name: self.name.trim().to_owned(),
            race: pick(&catalog.races, self.race),
            class: pick(&catalog.classes, self.class),
            background: pick(&catalog.backgrounds, self.background),
            alignment: Alignment::ALL[self.alignment % Alignment::ALL.len()],
            scores: self.scores,
            skills: self.skills.clone(),
        }
    }

    /// Handle a key; `members` is how many the party already has.
    pub fn key(
        &mut self,
        key: MenuKey,
        catalog: &Catalog,
        members: usize,
    ) -> Option<CreationAction> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, ROWS, key),
            MenuKey::Escape => return Some(CreationAction::Back),
            MenuKey::Char(c) if self.cursor == ROW_NAME && self.name.len() < 24 => {
                self.name.push(c)
            }
            MenuKey::Backspace if self.cursor == ROW_NAME => {
                self.name.pop();
            }
            MenuKey::Left | MenuKey::Right => self.adjust(key, catalog),
            MenuKey::Enter => return self.confirm(catalog, members),
            _ => {}
        }
        None
    }

    fn adjust(&mut self, key: MenuKey, catalog: &Catalog) {
        match self.cursor {
            ROW_RACE => self.race = cycle(self.race, catalog.races.len(), key),
            ROW_CLASS => {
                self.class = cycle(self.class, catalog.classes.len(), key);
                self.skills.clear();
                self.skill_cursor = 0;
            }
            ROW_BACKGROUND => {
                self.background = cycle(self.background, catalog.backgrounds.len(), key);
            }
            ROW_ALIGNMENT => self.alignment = cycle(self.alignment, Alignment::ALL.len(), key),
            row if (ROW_SCORES..ROW_SKILLS).contains(&row) => {
                let score = &mut self.scores[row - ROW_SCORES];
                *score = match key {
                    MenuKey::Right => (*score + 1).min(catalog.max),
                    _ => score.saturating_sub(1).max(catalog.min),
                };
            }
            ROW_SKILLS => {
                let (_, list) = self.skill_list(catalog);
                self.skill_cursor = cycle(self.skill_cursor, list.len(), key);
            }
            _ => {}
        }
    }

    fn confirm(&mut self, catalog: &Catalog, members: usize) -> Option<CreationAction> {
        match self.cursor {
            ROW_SKILLS => {
                let (choose, list) = self.skill_list(catalog);
                if let Some(skill) = list.get(self.skill_cursor).copied() {
                    if let Some(at) = self.skills.iter().position(|s| *s == skill) {
                        self.skills.remove(at);
                    } else if self.skills.len() < usize::from(choose) {
                        self.skills.push(skill);
                    } else {
                        self.message = format!("This class picks {choose} skills");
                    }
                }
                None
            }
            ROW_ADD => {
                let (choose, _) = self.skill_list(catalog);
                let spent = self.spent(catalog);
                if members >= catalog.slots {
                    self.message = "The party is full".to_owned();
                } else if self.name.trim().is_empty() {
                    self.message = "Give the character a name".to_owned();
                } else if spent > catalog.budget {
                    self.message = format!("{spent} points spent of {}", catalog.budget);
                } else if self.skills.len() != usize::from(choose) {
                    self.message = format!("Pick {choose} skills");
                } else {
                    self.message.clear();
                    return Some(CreationAction::Add(self.draft(catalog)));
                }
                None
            }
            ROW_BEGIN => {
                if members == 0 {
                    self.message = "Add at least one member".to_owned();
                    None
                } else {
                    Some(CreationAction::Begin)
                }
            }
            _ => None,
        }
    }

    /// Reset the draft after a member was added, keeping the cursor.
    pub fn next_member(&mut self, catalog: &Catalog) {
        let cursor = self.cursor;
        *self = CreationForm::new(catalog);
        self.cursor = cursor;
    }

    /// The screen as lines. `members` are the party's current names.
    #[must_use]
    pub fn lines(&self, catalog: &Catalog, members: &[String]) -> Vec<String> {
        let mut lines = vec![
            format!(
                "CREATE YOUR PARTY   {} of {} members",
                members.len(),
                catalog.slots
            ),
            if members.is_empty() {
                "(no members yet)".to_owned()
            } else {
                members.join(", ")
            },
            String::new(),
            mark(self.cursor, ROW_NAME, format!("Name: {}_", self.name)),
        ];
        let choice = |row: usize, label: &str, list: &[String], i: usize| {
            let id = list.get(i).map_or("?", String::as_str);
            mark(
                self.cursor,
                row,
                format!("{label}: < {} >", catalog.label(id)),
            )
        };
        lines.push(choice(ROW_RACE, "Race", &catalog.races, self.race));
        lines.push(choice(ROW_CLASS, "Class", &catalog.classes, self.class));
        lines.push(choice(
            ROW_BACKGROUND,
            "Background",
            &catalog.backgrounds,
            self.background,
        ));
        let alignment = Alignment::ALL[self.alignment % Alignment::ALL.len()];
        lines.push(mark(
            self.cursor,
            ROW_ALIGNMENT,
            format!("Alignment: < {} >", words(&format!("{alignment:?}"))),
        ));
        for (i, ability) in Ability::ALL.iter().enumerate() {
            let score = self.scores[i];
            let cost = catalog
                .costs
                .get(usize::from(score.saturating_sub(catalog.min)))
                .copied()
                .unwrap_or(0);
            lines.push(mark(
                self.cursor,
                ROW_SCORES + i,
                format!("{}: < {score:>2} >   cost {cost}", ability.short()),
            ));
        }
        lines.push(format!(
            "    Points left: {} of {}",
            catalog.budget - self.spent(catalog),
            catalog.budget
        ));
        let (choose, list) = self.skill_list(catalog);
        let skills: Vec<String> = list
            .iter()
            .enumerate()
            .map(|(i, skill)| {
                let picked = if self.skills.contains(skill) {
                    "x"
                } else {
                    " "
                };
                let name = words(&format!("{skill:?}"));
                if i == self.skill_cursor && self.cursor == ROW_SKILLS {
                    format!("[{picked}] <{name}>")
                } else {
                    format!("[{picked}] {name}")
                }
            })
            .collect();
        lines.push(mark(
            self.cursor,
            ROW_SKILLS,
            format!("Skills (pick {choose}): {}", skills.join("  ")),
        ));
        lines.push(mark(self.cursor, ROW_ADD, "Add member".to_owned()));
        lines.push(mark(self.cursor, ROW_BEGIN, "Begin".to_owned()));
        lines.push(self.message.clone());
        lines.push(HELP.to_owned());
        lines
    }
}

// ---------------------------------------------------------------- pause

/// What the pause overlay asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    /// Back to the world.
    Resume,
    /// Write the quick save.
    Save,
    /// Read the quick save.
    Load,
    /// Open the character sheet.
    Sheet,
    /// Drop the game and return to the title.
    QuitToTitle,
    /// Exit.
    Quit,
}

/// The pause overlay: the settings, read-only, and the items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pause {
    /// Selected item.
    pub cursor: usize,
}

impl Pause {
    /// The items, in cursor order.
    pub const ITEMS: [&'static str; 6] = [
        "Resume",
        "Save",
        "Load",
        "Character sheet",
        "Quit to title",
        "Quit",
    ];
    /// The action of each item, in the same order.
    const ACTIONS: [PauseAction; 6] = [
        PauseAction::Resume,
        PauseAction::Save,
        PauseAction::Load,
        PauseAction::Sheet,
        PauseAction::QuitToTitle,
        PauseAction::Quit,
    ];

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<PauseAction> {
        match key {
            MenuKey::Up | MenuKey::Down => {
                self.cursor = cycle(self.cursor, Self::ITEMS.len(), key);
            }
            MenuKey::Escape => return Some(PauseAction::Resume),
            MenuKey::Enter => return Self::ACTIONS.get(self.cursor).copied(),
            _ => {}
        }
        None
    }

    /// The overlay as lines.
    #[must_use]
    pub fn lines(&self, settings: Settings, seed: u64) -> Vec<String> {
        let mut lines = vec![
            "PAUSED".to_owned(),
            format!("Seed {seed}"),
            format!(
                "Saving: {:?}   Permadeath: {}   Devtools: {}",
                settings.save_rule,
                on_off(settings.permadeath),
                on_off(settings.devtools)
            ),
            String::new(),
        ];
        lines.extend(
            Self::ITEMS
                .iter()
                .enumerate()
                .map(|(i, item)| mark(self.cursor, i, (*item).to_owned())),
        );
        lines.push(String::new());
        lines.push("Up/Down select   Enter confirm   Esc resume".to_owned());
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_data::load_packs;
    use std::path::PathBuf;

    fn catalog() -> Catalog {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base")]).unwrap_or_else(|r| panic!("{r}"));
        Catalog::from_data(&data)
    }

    fn press(
        form: &mut CreationForm,
        catalog: &Catalog,
        keys: &[MenuKey],
    ) -> Option<CreationAction> {
        let mut action = None;
        for key in keys {
            action = form.key(*key, catalog, 0);
        }
        action
    }

    fn type_text(form: &mut CreationForm, catalog: &Catalog, text: &str) {
        for c in text.chars() {
            form.key(MenuKey::Char(c), catalog, 0);
        }
    }

    #[test]
    fn the_title_cycles_and_chooses() {
        let mut title = Title::default();
        assert_eq!(title.key(MenuKey::Up), None);
        assert_eq!(title.cursor, 2, "wraps");
        assert_eq!(title.key(MenuKey::Enter), Some(TitleAction::Quit));
        title.key(MenuKey::Down);
        assert_eq!(title.key(MenuKey::Enter), Some(TitleAction::NewGame));
        assert!(title.lines()[2].starts_with("> New game"));
        assert!(title.lines()[3].starts_with("  Load"));
    }

    #[test]
    fn the_new_game_form_makes_a_seed_and_settings() {
        let mut form = NewGameForm::default();
        assert_eq!(form.seed(99), 99, "blank is entropy");
        for c in "42".chars() {
            form.key(MenuKey::Char(c));
        }
        assert_eq!(form.seed(99), 42);
        form.key(MenuKey::Backspace);
        form.key(MenuKey::Backspace);
        for c in "toel".chars() {
            form.key(MenuKey::Char(c));
        }
        assert_eq!(form.seed(99), fnv1a64(b"toel"), "words are hashed");
        form.key(MenuKey::Down);
        form.key(MenuKey::Right);
        assert_eq!(form.settings.save_rule, SaveRule::Relief);
        form.key(MenuKey::Right);
        form.key(MenuKey::Right);
        assert_eq!(form.settings.save_rule, SaveRule::Anywhere, "cycles");
        form.key(MenuKey::Left);
        assert_eq!(form.settings.save_rule, SaveRule::InnOnly);
        form.key(MenuKey::Down);
        form.key(MenuKey::Left);
        assert!(form.settings.permadeath);
        form.key(MenuKey::Down);
        assert_eq!(form.key(MenuKey::Enter), Some(NewGameAction::Start));
        form.key(MenuKey::Down);
        assert_eq!(form.key(MenuKey::Enter), Some(NewGameAction::Back));
        assert_eq!(form.key(MenuKey::Escape), Some(NewGameAction::Back));
        assert!(form.lines()[3].contains("Inns only"), "{:?}", form.lines());
        assert!(form.lines()[4].contains("On"));
    }

    #[test]
    fn the_catalog_lifts_the_base_pack() {
        let catalog = catalog();
        assert_eq!(
            catalog.races,
            [
                "base:race:dwarf",
                "base:race:elf",
                "base:race:halfling",
                "base:race:human"
            ]
        );
        assert_eq!(catalog.classes.len(), 4);
        assert_eq!(catalog.label("base:class:wizard"), "Wizard");
        assert_eq!(
            (catalog.budget, catalog.min, catalog.max, catalog.slots),
            (27, 8, 15, 6)
        );
        assert_eq!(catalog.class_skills["base:class:rogue"].0, 4);
    }

    #[test]
    fn a_fighter_is_drafted_by_point_buy() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        assert_eq!(form.spent(&catalog), 0);
        type_text(&mut form, &catalog, "Brenna");
        // Race row: dwarf, elf, halfling, human; Left from dwarf wraps to human.
        press(&mut form, &catalog, &[MenuKey::Down, MenuKey::Left]);
        assert_eq!(form.race, 3);
        // Class row: cleric, fighter, rogue, wizard.
        press(&mut form, &catalog, &[MenuKey::Down, MenuKey::Right]);
        assert_eq!(catalog.classes[form.class], "base:class:fighter");
        // Scores: STR 15, DEX 14, CON 13, INT 12, WIS 10, CHA 8.
        press(
            &mut form,
            &catalog,
            &[MenuKey::Down, MenuKey::Down, MenuKey::Down],
        );
        assert_eq!(form.cursor, ROW_SCORES);
        for (i, target) in [15u8, 14, 13, 12, 10, 8].iter().enumerate() {
            for _ in catalog.min..*target {
                form.key(MenuKey::Right, &catalog, 0);
            }
            assert_eq!(form.scores[i], *target);
            form.key(MenuKey::Down, &catalog, 0);
        }
        assert_eq!(form.spent(&catalog), 27);
        assert_eq!(form.cursor, ROW_SKILLS);
        // Fighter list: Acrobatics, AnimalHandling, Athletics, ...; pick Athletics and Perception.
        press(
            &mut form,
            &catalog,
            &[MenuKey::Right, MenuKey::Right, MenuKey::Enter],
        );
        assert_eq!(form.skills, [Skill::Athletics]);
        for _ in 0..4 {
            form.key(MenuKey::Right, &catalog, 0);
        }
        form.key(MenuKey::Enter, &catalog, 0);
        assert_eq!(form.skills, [Skill::Athletics, Skill::Perception]);
        form.key(MenuKey::Left, &catalog, 0);
        form.key(MenuKey::Enter, &catalog, 0);
        assert!(
            form.message.contains("picks 2"),
            "a third pick is refused: {}",
            form.message
        );
        form.key(MenuKey::Down, &catalog, 0);
        let Some(CreationAction::Add(draft)) = form.key(MenuKey::Enter, &catalog, 0) else {
            panic!("Add answers with the draft: {}", form.message);
        };
        assert_eq!(draft.name, "Brenna");
        assert_eq!(draft.race, "base:race:human");
        assert_eq!(draft.scores, [15, 14, 13, 12, 10, 8]);
        assert_eq!(draft.skills, [Skill::Athletics, Skill::Perception]);
        form.next_member(&catalog);
        assert_eq!(form.cursor, ROW_ADD);
        assert!(form.name.is_empty() && form.skills.is_empty());
    }

    #[test]
    fn the_form_refuses_what_the_rules_would() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        form.cursor = ROW_ADD;
        assert_eq!(form.key(MenuKey::Enter, &catalog, 0), None);
        assert!(form.message.contains("name"));
        form.name = "x".to_owned();
        form.scores = [15; 6];
        assert_eq!(form.key(MenuKey::Enter, &catalog, 0), None);
        assert!(form.message.contains("54 points"), "{}", form.message);
        form.scores = [8; 6];
        assert_eq!(form.key(MenuKey::Enter, &catalog, 0), None);
        assert!(form.message.contains("Pick 2"), "{}", form.message);
        assert_eq!(form.key(MenuKey::Enter, &catalog, 6), None);
        assert!(form.message.contains("full"));
        form.cursor = ROW_SCORES;
        form.key(MenuKey::Left, &catalog, 0);
        assert_eq!(form.scores[0], 8, "never below the minimum");
        form.scores[0] = 15;
        form.key(MenuKey::Right, &catalog, 0);
        assert_eq!(form.scores[0], 15, "never above the maximum");
        form.cursor = ROW_BEGIN;
        assert_eq!(form.key(MenuKey::Enter, &catalog, 0), None);
        assert_eq!(
            form.key(MenuKey::Enter, &catalog, 1),
            Some(CreationAction::Begin)
        );
        assert_eq!(
            form.key(MenuKey::Escape, &catalog, 1),
            Some(CreationAction::Back)
        );
        form.cursor = ROW_CLASS;
        form.skills = vec![Skill::Athletics];
        form.key(MenuKey::Right, &catalog, 0);
        assert!(form.skills.is_empty(), "a class change drops the picks");
        let lines = form.lines(&catalog, &["Brenna".to_owned()]);
        assert!(lines[0].contains("1 of 6"));
        assert_eq!(lines[1], "Brenna");
        assert!(
            lines.iter().any(|l| l.contains("Points left: 18 of 27")),
            "{lines:?}"
        );
    }

    #[test]
    fn the_pause_overlay_shows_the_settings_and_every_item_answers() {
        let mut pause = Pause::default();
        let settings = Settings {
            save_rule: SaveRule::InnOnly,
            permadeath: true,
            devtools: true,
        };
        let lines = pause.lines(settings, 7);
        assert_eq!(lines[1], "Seed 7");
        assert_eq!(lines[2], "Saving: InnOnly   Permadeath: on   Devtools: on");
        assert_eq!(
            &lines[4..10],
            [
                "> Resume",
                "  Save",
                "  Load",
                "  Character sheet",
                "  Quit to title",
                "  Quit"
            ]
        );
        assert_eq!(pause.key(MenuKey::Escape), Some(PauseAction::Resume));
        let expected = [
            PauseAction::Resume,
            PauseAction::Save,
            PauseAction::Load,
            PauseAction::Sheet,
            PauseAction::QuitToTitle,
            PauseAction::Quit,
        ];
        for (i, action) in expected.iter().enumerate() {
            assert_eq!(pause.cursor, i);
            assert_eq!(pause.key(MenuKey::Enter), Some(*action), "item {i}");
            assert!(pause.lines(settings, 7)[4 + i].starts_with("> "));
            pause.key(MenuKey::Down);
        }
        assert_eq!(pause.cursor, 0, "wraps after the last item");
        assert_eq!(words("LawfulGood"), "Lawful Good");
        assert_eq!(words("SleightOfHand"), "Sleight Of Hand");
    }
}
