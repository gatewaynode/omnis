//! The Feathers skin of party creation, Bevy-free: which control is which (`PanelId`), what a
//! control reports (`Payload`), and `apply`, which turns a report into the same `CreationForm`
//! edits and `CreationAction`s as the canvas screen's keys. The widgets are not trusted:
//! everything is clamped and refused here, a value equal to the form's is dropped (a text or
//! number input reports again when it loses focus), and the rules are `CreationForm`'s own.

use crate::creation_menu::{Catalog, CreationAction, CreationForm};
use omnis_sim::omnis_core::fnv1a64;
use omnis_sim::omnis_data::{Ability, Alignment};

/// A list the panel chooses from with a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Choice {
    /// The catalog's races.
    Race,
    /// The catalog's classes.
    Class,
    /// The catalog's backgrounds.
    Background,
    /// `Alignment::ALL`.
    Alignment,
}

impl Choice {
    /// Every choice, in the panel's order.
    pub const ALL: [Choice; 4] = [
        Choice::Race,
        Choice::Class,
        Choice::Background,
        Choice::Alignment,
    ];

    /// The row's label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Choice::Race => "Race",
            Choice::Class => "Class",
            Choice::Background => "Background",
            Choice::Alignment => "Alignment",
        }
    }

    /// The display names to choose from.
    #[must_use]
    pub fn options(self, catalog: &Catalog) -> Vec<String> {
        let labels = |ids: &[String]| ids.iter().map(|id| catalog.label(id).to_owned()).collect();
        match self {
            Choice::Race => labels(&catalog.races),
            Choice::Class => labels(&catalog.classes),
            Choice::Background => labels(&catalog.backgrounds),
            Choice::Alignment => Alignment::ALL
                .iter()
                .map(|a| crate::menu::words(&format!("{a:?}")))
                .collect(),
        }
    }

    /// The form's current index into `options`.
    #[must_use]
    pub fn current(self, form: &CreationForm) -> usize {
        match self {
            Choice::Race => form.race,
            Choice::Class => form.class,
            Choice::Background => form.background,
            Choice::Alignment => form.alignment % Alignment::ALL.len(),
        }
    }
}

/// One control of the panel. Tests and the sync system find entities by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum PanelId {
    /// The name's text input.
    #[default]
    Name,
    /// A choice's menu button; its caption shows the current option.
    Menu(Choice),
    /// One option in a choice's menu.
    Pick(Choice, usize),
    /// An ability's number input, by index into `Ability::ALL`.
    Score(usize),
    /// The same ability's slider.
    ScoreSlider(usize),
    /// A skill's checkbox, by index into the class's list.
    Skill(usize),
    /// Add the draft to the party.
    Add,
    /// Start exploring.
    Begin,
    /// Abandon the new game.
    Back,
    /// The font menu's button.
    FontMenu,
    /// One font in the font menu.
    FontPick(usize),
    /// The interface scale slider.
    UiScale,
}

/// What a control reported.
#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    /// A button or a menu item was activated.
    Activate,
    /// A text input's content.
    Text(String),
    /// A number input's value.
    Number(i64),
    /// A slider's value.
    Slide(f32),
    /// A checkbox's new state.
    Flag(bool),
}

/// What the panel asks of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelAction {
    /// What the canvas screen would ask.
    Creation(CreationAction),
    /// Use this font, by index into the app's list.
    Font(usize),
    /// Scale the interface, in tenths (10 is Bevy's 1.0).
    UiScale(u8),
}

/// The smallest interface scale, in tenths.
pub const SCALE_MIN: u8 = 10;
/// The largest interface scale, in tenths.
pub const SCALE_MAX: u8 = 30;

/// A slider's value as the whole number it stands for; not-a-number is refused.
fn whole(value: f32) -> Option<i64> {
    // Saturating by definition of `as`; the callers clamp to a few dozen.
    #[allow(clippy::cast_possible_truncation)]
    value.is_finite().then(|| value.round() as i64)
}

fn set_score(form: &mut CreationForm, ability: usize, value: i64, catalog: &Catalog) {
    if form.scores.get(ability).map(|s| i64::from(*s)) != Some(value) {
        form.set_score(ability, value, catalog);
    }
}

fn pick(form: &mut CreationForm, choice: Choice, index: usize, catalog: &Catalog) {
    match choice {
        Choice::Race => form.set_race(index, catalog),
        Choice::Class => form.set_class(index, catalog),
        Choice::Background => form.set_background(index, catalog),
        Choice::Alignment => form.set_alignment(index),
    }
}

/// Apply one control's report to the form. `members` is how many the party already has.
pub fn apply(
    id: PanelId,
    payload: &Payload,
    form: &mut CreationForm,
    catalog: &Catalog,
    members: usize,
) -> Option<PanelAction> {
    match (id, payload) {
        (PanelId::Name, Payload::Text(text)) if *text != form.name => form.set_name(text),
        (PanelId::Pick(choice, index), Payload::Activate) => pick(form, choice, index, catalog),
        (PanelId::Score(ability), Payload::Number(value)) => {
            set_score(form, ability, *value, catalog);
        }
        (PanelId::ScoreSlider(ability), Payload::Slide(value)) => {
            if let Some(value) = whole(*value) {
                set_score(form, ability, value, catalog);
            }
        }
        (PanelId::Skill(index), Payload::Flag(on)) => {
            let skill = form.skill_list(catalog).1.get(index).copied();
            if let Some(skill) = skill.filter(|s| form.skills.contains(s) != *on) {
                form.toggle_skill(skill, catalog);
            }
        }
        (PanelId::Add, Payload::Activate) => {
            return form.add(catalog, members).map(PanelAction::Creation);
        }
        (PanelId::Begin, Payload::Activate) => {
            return form.begin(members).map(PanelAction::Creation);
        }
        (PanelId::Back, Payload::Activate) => {
            return Some(PanelAction::Creation(CreationAction::Back));
        }
        (PanelId::FontPick(index), Payload::Activate) => return Some(PanelAction::Font(index)),
        (PanelId::UiScale, Payload::Slide(value)) => {
            let tenths = whole(*value * 10.0)?.clamp(i64::from(SCALE_MIN), i64::from(SCALE_MAX));
            return u8::try_from(tenths).ok().map(PanelAction::UiScale);
        }
        _ => {}
    }
    None
}

/// What the panel shows that is not a control's own value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PanelText {
    /// "2 of 6 members".
    pub heading: String,
    /// Per ability: "STR" and "cost 5".
    pub scores: Vec<(String, String)>,
    /// "Points left: 12 of 27".
    pub points: String,
    /// "Skills (pick 2)".
    pub skills: String,
    /// The last refusal, or empty.
    pub message: String,
}

/// The panel's labels for a form.
#[must_use]
pub fn text(form: &CreationForm, catalog: &Catalog, members: usize) -> PanelText {
    let cost = |score: u8| {
        catalog
            .costs
            .get(usize::from(score.saturating_sub(catalog.min)))
            .copied()
            .unwrap_or(0)
    };
    PanelText {
        heading: format!("{members} of {} members", catalog.slots),
        scores: Ability::ALL
            .iter()
            .zip(form.scores)
            .map(|(ability, score)| (ability.short().to_owned(), format!("cost {}", cost(score))))
            .collect(),
        points: format!(
            "Points left: {} of {}",
            catalog.budget - form.spent(catalog),
            catalog.budget
        ),
        skills: format!("Skills (pick {})", form.skill_list(catalog).0),
        message: form.message.clone(),
    }
}

/// What the panel's entity tree depends on: the lists behind the menus, the class's skill
/// list and the roster. When it changes the panel is spawned again; values alone are synced.
#[must_use]
pub fn shape(form: &CreationForm, catalog: &Catalog, roster: &[String]) -> u64 {
    let mut bytes = Vec::new();
    let mut list = |items: &[String]| {
        for item in items {
            bytes.extend_from_slice(item.as_bytes());
            bytes.push(0);
        }
        bytes.push(1);
    };
    for choice in Choice::ALL {
        list(&choice.options(catalog));
    }
    let (_, skills) = form.skill_list(catalog);
    list(&skills.iter().map(|s| format!("{s:?}")).collect::<Vec<_>>());
    list(roster);
    fnv1a64(&bytes)
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

    fn act(id: PanelId, payload: Payload, form: &mut CreationForm, catalog: &Catalog) {
        assert_eq!(apply(id, &payload, form, catalog, 0), None, "{id:?}");
    }

    /// The fighter of `creation_menu`'s key-driven test, drafted through the controls.
    fn fighter(form: &mut CreationForm, catalog: &Catalog) {
        act(PanelId::Name, Payload::Text("Brenna".into()), form, catalog);
        act(
            PanelId::Pick(Choice::Race, 3),
            Payload::Activate,
            form,
            catalog,
        );
        let class = catalog
            .classes
            .iter()
            .position(|c| c == "base:class:fighter")
            .expect("a fighter");
        act(
            PanelId::Pick(Choice::Class, class),
            Payload::Activate,
            form,
            catalog,
        );
        for (ability, score) in [15, 13, 14, 8, 12, 10].into_iter().enumerate() {
            act(
                PanelId::Score(ability),
                Payload::Number(score),
                form,
                catalog,
            );
        }
        act(PanelId::Skill(0), Payload::Flag(true), form, catalog);
        act(PanelId::Skill(1), Payload::Flag(true), form, catalog);
    }

    #[test]
    fn the_controls_draft_what_the_keys_draft() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        fighter(&mut form, &catalog);
        assert_eq!(form.spent(&catalog), 27);
        let action = apply(PanelId::Add, &Payload::Activate, &mut form, &catalog, 0);
        let Some(PanelAction::Creation(CreationAction::Add(draft))) = action else {
            panic!("no draft: {action:?} ({})", form.message);
        };
        assert_eq!(draft.name, "Brenna");
        assert_eq!(draft.race, "base:race:human");
        assert_eq!(draft.class, "base:class:fighter");
        assert_eq!(draft.scores, [15, 13, 14, 8, 12, 10]);
        assert_eq!(draft.skills.len(), 2);
        assert_eq!(draft, form.draft(&catalog));
    }

    #[test]
    fn a_slider_and_a_number_input_buy_the_same_score_and_stay_inside_point_buy() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        act(
            PanelId::ScoreSlider(0),
            Payload::Slide(12.4),
            &mut form,
            &catalog,
        );
        assert_eq!(form.scores[0], 12);
        act(PanelId::Score(0), Payload::Number(99), &mut form, &catalog);
        assert_eq!(form.scores[0], catalog.max);
        act(PanelId::Score(0), Payload::Number(-4), &mut form, &catalog);
        assert_eq!(form.scores[0], catalog.min);
        act(
            PanelId::ScoreSlider(0),
            Payload::Slide(f32::NAN),
            &mut form,
            &catalog,
        );
        assert_eq!(form.scores[0], catalog.min, "not a number is refused");
        act(PanelId::Score(6), Payload::Number(12), &mut form, &catalog);
        assert_eq!(form.scores, [catalog.min; 6], "there is no seventh ability");
    }

    #[test]
    fn the_panel_refuses_what_the_canvas_screen_refuses() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        let add = |form: &mut CreationForm, members| {
            apply(PanelId::Add, &Payload::Activate, form, &catalog, members)
        };
        assert_eq!(add(&mut form, 0), None);
        assert_eq!(form.message, "Give the character a name");
        fighter(&mut form, &catalog);
        assert_eq!(add(&mut form, catalog.slots), None);
        assert_eq!(form.message, "The party is full");
        act(PanelId::Skill(2), Payload::Flag(true), &mut form, &catalog);
        assert_eq!(form.skills.len(), 2, "a third pick is refused");
        assert_eq!(form.message, "This class picks 2 skills");
        act(PanelId::Skill(0), Payload::Flag(false), &mut form, &catalog);
        assert_eq!(add(&mut form, 0), None);
        assert_eq!(form.message, "Pick 2 skills");
        act(PanelId::Skill(0), Payload::Flag(true), &mut form, &catalog);
        act(PanelId::Score(3), Payload::Number(15), &mut form, &catalog);
        assert_eq!(add(&mut form, 0), None);
        assert_eq!(form.message, "36 points spent of 27");
        let begin = apply(PanelId::Begin, &Payload::Activate, &mut form, &catalog, 0);
        assert_eq!(begin, None);
        assert_eq!(form.message, "Add at least one member");
        let long = "x".repeat(40);
        act(PanelId::Name, Payload::Text(long), &mut form, &catalog);
        assert_eq!(form.name.len(), 24, "the 25th character is cut");
    }

    #[test]
    fn a_report_that_changes_nothing_changes_nothing() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        fighter(&mut form, &catalog);
        "kept".clone_into(&mut form.message);
        let before = form.clone();
        // What a text or number input sends when it loses focus, and a checkbox's echo.
        act(
            PanelId::Name,
            Payload::Text("Brenna".into()),
            &mut form,
            &catalog,
        );
        act(PanelId::Score(0), Payload::Number(15), &mut form, &catalog);
        act(
            PanelId::ScoreSlider(0),
            Payload::Slide(15.0),
            &mut form,
            &catalog,
        );
        act(PanelId::Skill(0), Payload::Flag(true), &mut form, &catalog);
        act(PanelId::Skill(40), Payload::Flag(true), &mut form, &catalog);
        act(
            PanelId::Pick(Choice::Class, 99),
            Payload::Activate,
            &mut form,
            &catalog,
        );
        act(
            PanelId::Pick(Choice::Class, form.class),
            Payload::Activate,
            &mut form,
            &catalog,
        );
        act(
            PanelId::Menu(Choice::Race),
            Payload::Activate,
            &mut form,
            &catalog,
        );
        assert_eq!(form, before, "the skills survive choosing the same class");
    }

    #[test]
    fn the_shell_controls_answer_without_touching_the_form() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        let before = form.clone();
        let mut ask = |id, payload| apply(id, &payload, &mut form, &catalog, 1);
        assert_eq!(
            ask(PanelId::FontPick(2), Payload::Activate),
            Some(PanelAction::Font(2))
        );
        assert_eq!(
            ask(PanelId::UiScale, Payload::Slide(1.54)),
            Some(PanelAction::UiScale(15))
        );
        assert_eq!(
            ask(PanelId::UiScale, Payload::Slide(9.0)),
            Some(PanelAction::UiScale(30))
        );
        assert_eq!(ask(PanelId::UiScale, Payload::Slide(f32::NAN)), None);
        assert_eq!(
            ask(PanelId::Back, Payload::Activate),
            Some(PanelAction::Creation(CreationAction::Back))
        );
        assert_eq!(
            ask(PanelId::Begin, Payload::Activate),
            Some(PanelAction::Creation(CreationAction::Begin))
        );
        assert_eq!(form, before);
    }

    #[test]
    fn the_labels_and_the_shape_follow_the_form() {
        let catalog = catalog();
        let mut form = CreationForm::new(&catalog);
        let labels = text(&form, &catalog, 2);
        assert_eq!(labels.heading, "2 of 6 members");
        assert_eq!(labels.scores[0], ("STR".to_owned(), "cost 0".to_owned()));
        assert_eq!(labels.points, "Points left: 27 of 27");
        assert_eq!(Choice::Alignment.options(&catalog).len(), 9);
        assert_eq!(Choice::Race.options(&catalog)[0], "Dwarf (hill dwarf)");

        let roster = vec!["Brenna".to_owned()];
        let before = shape(&form, &catalog, &roster);
        act(PanelId::Score(0), Payload::Number(15), &mut form, &catalog);
        assert_eq!(
            shape(&form, &catalog, &roster),
            before,
            "a value is not a shape"
        );
        assert_eq!(text(&form, &catalog, 2).scores[0].1, "cost 9");
        let other = (form.class + 1) % catalog.classes.len();
        act(
            PanelId::Pick(Choice::Class, other),
            Payload::Activate,
            &mut form,
            &catalog,
        );
        assert_ne!(
            shape(&form, &catalog, &roster),
            before,
            "another skill list"
        );
        assert_ne!(shape(&form, &catalog, &[]), shape(&form, &catalog, &roster));
    }
}
