//! Party creation's model, Bevy-free: what the loaded packs offer (`Catalog`) and one member
//! being drafted (`CreationForm`). The rules live in the named methods (`set_score`,
//! `toggle_skill`, `add`, ...), which the Feathers panel calls through `creation_panel::apply`.

use omnis_sim::omnis_data::{Alignment, Data, Skill};
use omnis_sim::omnis_rules::{Draft, NAME_MAX_BYTES};
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

/// One member being drafted.
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
    /// Feedback from the last attempt.
    pub message: String,
}

/// The longest name, in characters: what the panel's input lets a player type. The rules
/// count bytes (`NAME_MAX_BYTES`), so `set_name` holds a name inside both.
pub const NAME_LIMIT: usize = 24;

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

    /// The name as typed, cut to whole characters: at most `NAME_LIMIT` of them, as the panel's
    /// input counts, and at most `NAME_MAX_BYTES` bytes, which is all the rules accept.
    pub fn set_name(&mut self, name: &str) {
        let mut end = 0;
        for (at, c) in name.char_indices().take(NAME_LIMIT) {
            if at + c.len_utf8() > NAME_MAX_BYTES {
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
    /// player can overspend on the way to a build.
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

    /// Reset the draft after a member was added.
    pub fn next_member(&mut self, catalog: &Catalog) {
        *self = CreationForm::new(catalog);
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
        form.set_name("Brenna");
        // Races: dwarf, elf, halfling, human. Classes: cleric, fighter, rogue, wizard.
        form.set_race(3, &catalog);
        form.set_race(4, &catalog);
        assert_eq!(form.race, 3, "an index past the list is ignored");
        form.set_class(1, &catalog);
        assert_eq!(catalog.classes[form.class], "base:class:fighter");
        for (i, target) in [15i64, 14, 13, 12, 10, 8].iter().enumerate() {
            form.set_score(i, *target, &catalog);
        }
        assert_eq!(form.spent(&catalog), 27);
        form.toggle_skill(Skill::Athletics, &catalog);
        assert_eq!(form.skills, [Skill::Athletics]);
        form.toggle_skill(Skill::Arcana, &catalog);
        assert_eq!(form.skills, [Skill::Athletics], "not on the fighter's list");
        form.toggle_skill(Skill::Perception, &catalog);
        assert_eq!(form.skills, [Skill::Athletics, Skill::Perception]);
        form.toggle_skill(Skill::Survival, &catalog);
        assert!(
            form.message.contains("picks 2"),
            "a third pick is refused: {}",
            form.message
        );
        form.toggle_skill(Skill::Perception, &catalog);
        assert_eq!(form.skills, [Skill::Athletics], "a second press lets it go");
        form.toggle_skill(Skill::Perception, &catalog);
        let Some(CreationAction::Add(draft)) = form.add(&catalog, 0) else {
            panic!("Add answers with the draft: {}", form.message);
        };
        assert_eq!(draft.name, "Brenna");
        assert_eq!(draft.race, "base:race:human");
        assert_eq!(draft.scores, [15, 14, 13, 12, 10, 8]);
        assert_eq!(draft.skills, [Skill::Athletics, Skill::Perception]);
        form.next_member(&catalog);
        assert_eq!(form, CreationForm::new(&catalog));
    }

    #[test]
    fn the_form_refuses_what_the_rules_would() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        assert_eq!(form.add(&catalog, 0), None);
        assert!(form.message.contains("name"));
        form.set_name("x");
        form.scores = [15; 6];
        assert_eq!(form.add(&catalog, 0), None);
        assert!(form.message.contains("54 points"), "{}", form.message);
        form.scores = [8; 6];
        assert_eq!(form.add(&catalog, 0), None);
        assert!(form.message.contains("Pick 2"), "{}", form.message);
        assert_eq!(form.add(&catalog, 6), None);
        assert!(form.message.contains("full"));
        form.set_score(0, 7, &catalog);
        assert_eq!(form.scores[0], 8, "never below the minimum");
        form.set_score(0, 16, &catalog);
        assert_eq!(form.scores[0], 15, "never above the maximum");
        assert_eq!(form.begin(0), None);
        assert_eq!(form.message, "Add at least one member");
        assert_eq!(form.begin(1), Some(CreationAction::Begin));
        form.skills = vec![Skill::Athletics];
        form.set_class(form.class, &catalog);
        assert_eq!(form.skills.len(), 1, "the same class keeps the picks");
        form.set_class(form.class + 1, &catalog);
        assert!(form.skills.is_empty(), "a class change drops the picks");
    }

    /// The panel's input stops at 24 characters, so the form counts characters too (it counted
    /// bytes, and kept 12 of 24 two-byte letters the input showed); the rules take 32 bytes,
    /// so a name of wider letters is cut there, on a whole character, and still drafts.
    #[test]
    fn a_name_is_held_to_24_characters_and_to_the_bytes_the_rules_accept() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        form.set_name(&"x".repeat(40));
        assert_eq!(form.name, "x".repeat(24), "the 25th character is cut");
        form.set_name(&"é".repeat(16));
        assert_eq!(
            form.name.chars().count(),
            16,
            "16 two-byte letters are kept"
        );
        assert_eq!(form.name.len(), NAME_MAX_BYTES);
        form.set_name(&"é".repeat(24));
        assert_eq!(form.name, "é".repeat(16), "cut at the rules' 32 bytes");
        form.set_name(&format!("{}界", "x".repeat(30)));
        assert_eq!(form.name, "x".repeat(24));
        form.set_name(&"界".repeat(11));
        assert_eq!(form.name, "界".repeat(10), "never inside a character");
    }
}
