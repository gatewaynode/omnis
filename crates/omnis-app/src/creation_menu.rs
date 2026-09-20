//! Party creation's model, Bevy-free: what the loaded packs offer (`Catalog`) and one member
//! being drafted (`CreationForm`). The rules live in the named methods (`set_score`,
//! `toggle_skill`, `add`, ...), which both skins of the screen call: the canvas screen through
//! `key`, the Feathers panel through `creation_panel::apply`.

use crate::menu::{HELP, MenuKey, cycle, mark, words};
use omnis_sim::omnis_data::{Ability, Alignment, Data, Skill};
use omnis_sim::omnis_rules::Draft;
use std::collections::BTreeMap;

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

/// The longest name, in bytes.
pub const NAME_LIMIT: usize = 24;
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
            MenuKey::Char(c) if self.cursor == ROW_NAME => {
                let mut name = self.name.clone();
                name.push(c);
                self.set_name(&name);
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

    /// The name as typed, cut at `NAME_LIMIT` bytes on a character boundary.
    pub fn set_name(&mut self, name: &str) {
        let mut end = 0;
        for (at, c) in name.char_indices() {
            if at + c.len_utf8() > NAME_LIMIT {
                break;
            }
            end = at + c.len_utf8();
        }
        name[..end].clone_into(&mut self.name);
    }

    /// Choose a race by its index in the catalog; an index past the list is ignored.
    pub fn set_race(&mut self, index: usize, catalog: &Catalog) {
        if index < catalog.races.len() {
            self.race = index;
        }
    }

    /// Choose a class. A different class has a different skill list, so the picks go.
    pub fn set_class(&mut self, index: usize, catalog: &Catalog) {
        if index < catalog.classes.len() && index != self.class {
            self.class = index;
            self.skills.clear();
            self.skill_cursor = 0;
        }
    }

    /// Choose a background by its index in the catalog.
    pub fn set_background(&mut self, index: usize, catalog: &Catalog) {
        if index < catalog.backgrounds.len() {
            self.background = index;
        }
    }

    /// Choose an alignment by its index in `Alignment::ALL`.
    pub fn set_alignment(&mut self, index: usize) {
        if index < Alignment::ALL.len() {
            self.alignment = index;
        }
    }

    /// Buy a score, held inside what point buy sells. The budget is checked on `add`, so a
    /// player can overspend on the way to a build, as on the canvas screen.
    pub fn set_score(&mut self, ability: usize, value: i64, catalog: &Catalog) {
        if let Some(score) = self.scores.get_mut(ability) {
            let held = value.clamp(i64::from(catalog.min), i64::from(catalog.max));
            *score = u8::try_from(held).unwrap_or(catalog.min);
        }
    }

    /// Pick a skill of the class's list, or let it go; a pick past the class's count is
    /// refused with a message.
    pub fn toggle_skill(&mut self, skill: Skill, catalog: &Catalog) {
        let (choose, list) = self.skill_list(catalog);
        if !list.contains(&skill) {
            return;
        }
        if let Some(at) = self.skills.iter().position(|s| *s == skill) {
            self.skills.remove(at);
        } else if self.skills.len() < usize::from(choose) {
            self.skills.push(skill);
        } else {
            self.message = format!("This class picks {choose} skills");
        }
    }

    /// Ask to add the draft to the party; what is wrong with it becomes the message.
    pub fn add(&mut self, catalog: &Catalog, members: usize) -> Option<CreationAction> {
        let (choose, _) = self.skill_list(catalog);
        let spent = self.spent(catalog);
        if members >= catalog.slots {
            "The party is full".clone_into(&mut self.message);
        } else if self.name.trim().is_empty() {
            "Give the character a name".clone_into(&mut self.message);
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

    /// Ask to start exploring; an empty party is refused with a message.
    pub fn begin(&mut self, members: usize) -> Option<CreationAction> {
        if members == 0 {
            "Add at least one member".clone_into(&mut self.message);
            None
        } else {
            Some(CreationAction::Begin)
        }
    }

    fn adjust(&mut self, key: MenuKey, catalog: &Catalog) {
        match self.cursor {
            ROW_RACE => self.set_race(cycle(self.race, catalog.races.len(), key), catalog),
            ROW_CLASS => self.set_class(cycle(self.class, catalog.classes.len(), key), catalog),
            ROW_BACKGROUND => self.set_background(
                cycle(self.background, catalog.backgrounds.len(), key),
                catalog,
            ),
            ROW_ALIGNMENT => {
                self.set_alignment(cycle(self.alignment, Alignment::ALL.len(), key));
            }
            row if (ROW_SCORES..ROW_SKILLS).contains(&row) => {
                let ability = row - ROW_SCORES;
                let step = if key == MenuKey::Right { 1 } else { -1 };
                self.set_score(ability, i64::from(self.scores[ability]) + step, catalog);
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
                let (_, list) = self.skill_list(catalog);
                if let Some(skill) = list.get(self.skill_cursor).copied() {
                    self.toggle_skill(skill, catalog);
                }
                None
            }
            ROW_ADD => self.add(catalog, members),
            ROW_BEGIN => self.begin(members),
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
}
