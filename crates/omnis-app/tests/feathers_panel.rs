//! The Feathers creation panel control by control (the Feathers experiment, step 4): every
//! control by its event, the pointer and the keys through real layout and picking, the layout
//! at both window sizes and four interface scales, the text tree that stands in for a screen
//! dump, the panel's fighter against the canvas screen's, and the refusals.
#![cfg(feature = "feathers")]

mod common;

use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::Checked;
use bevy::ui_widgets::{MenuPopup, ScrollArea, Scrollbar, ScrollbarThumb};
use common::feathers::{
    Fault, activate, change, click_at, click_node, control, controls, creating, draft_fighter,
    drag_node, form, keys, layout_faults, number_input, rect, resize, settle, shown, slider, tab,
    text_tree, type_name,
};
use common::{
    draft_fighter_by_mouse, play_state, start_new_game_by_mouse, ui_app_saving_to, world,
};
use omnis_app::creation_panel::{Choice, FONTS, PanelId};
use omnis_app::feathers_creation::{FontChoice, LabelId, PanelRoot, ScaleChoice};
use omnis_app::feathers_fonts::{Face, PanelFonts};
use omnis_app::menus::Screens;
use omnis_app::sim::{AppState, PlayState};

// ------------------------------------------------------------------ every control by event

/// What the panel must hold for the form and the catalog as they are.
fn expected_controls(app: &App) -> Vec<PanelId> {
    let screens = app.world().resource::<Screens>();
    let (form, catalog) = (&screens.creation, &screens.catalog);
    let mut ids = vec![
        PanelId::Name,
        PanelId::Add,
        PanelId::Begin,
        PanelId::Back,
        PanelId::FontMenu,
        PanelId::UiScale,
    ];
    ids.extend((0..FONTS.len()).map(PanelId::FontPick));
    for choice in Choice::ALL {
        ids.push(PanelId::Menu(choice));
        ids.extend((0..choice.options(catalog).len()).map(|i| PanelId::Pick(choice, i)));
    }
    ids.extend((0..6).flat_map(|i| [PanelId::Score(i), PanelId::ScoreSlider(i)]));
    ids.extend((0..form.skill_list(catalog).1.len()).map(PanelId::Skill));
    ids.sort();
    ids
}

fn options(app: &App, choice: Choice) -> Vec<String> {
    choice.options(&app.world().resource::<Screens>().catalog)
}

fn popup_open(app: &mut App, choice: Choice) -> bool {
    let button = control(app, PanelId::Menu(choice));
    let world = app.world();
    let menu = world.get::<ChildOf>(button).expect("a menu").parent();
    let children = world.get::<Children>(menu).expect("children");
    children.iter().any(|child| {
        world.get::<MenuPopup>(child).is_some()
            && world.get::<Visibility>(child) == Some(&Visibility::Visible)
    })
}

/// Every option of every menu, last to first, so each choice ends on its first option.
fn choices_by_event(app: &mut App) {
    for choice in Choice::ALL {
        assert!(!popup_open(app, choice));
        activate(app, PanelId::Menu(choice));
        assert!(popup_open(app, choice), "{choice:?} opens");
        activate(app, PanelId::Menu(choice));
        assert!(!popup_open(app, choice), "{choice:?} closes");
        let names = options(app, choice);
        for (index, name) in names.iter().enumerate().rev() {
            activate(app, PanelId::Pick(choice, index));
            assert_eq!(choice.current(form(app)), index, "{choice:?}");
            assert_eq!(&shown(app, LabelId::Caption(choice)), name);
            // Another class has other skills: the panel is built again for them.
            assert_eq!(controls(app), expected_controls(app));
        }
    }
}

fn number_text(app: &mut App, ability: usize) -> String {
    let input = number_input(app, ability);
    let text = app.world().get::<EditableText>(input).expect("an input");
    text.value().to_string()
}

/// Both controls of every score, each followed by the other and by the labels.
fn scores_by_event(app: &mut App) {
    for ability in 0..6 {
        change(app, PanelId::Score(ability), 15_i32);
        assert_eq!(form(app).scores[ability], 15);
        assert!((slider(app, PanelId::ScoreSlider(ability)) - 15.0).abs() < 1e-6);
        assert_eq!(shown(app, LabelId::Cost(ability)), "cost 9");
        change(app, PanelId::ScoreSlider(ability), 10.4_f32);
        assert_eq!(form(app).scores[ability], 10);
        assert_eq!(number_text(app, ability), "10");
        assert_eq!(shown(app, LabelId::Cost(ability)), "cost 2");
    }
    assert_eq!(shown(app, LabelId::Points), "Points left: 15 of 27");
}

fn checked(app: &mut App, index: usize) -> bool {
    let entity = control(app, PanelId::Skill(index));
    app.world().get::<Checked>(entity).is_some()
}

fn skills_by_event(app: &mut App) {
    let screens = app.world().resource::<Screens>();
    let (_, skills) = screens.creation.skill_list(&screens.catalog);
    let skills: Vec<_> = skills.to_vec();
    for (index, skill) in skills.into_iter().enumerate() {
        change(app, PanelId::Skill(index), true);
        assert_eq!(form(app).skills, vec![skill]);
        assert!(checked(app, index), "the box follows the form");
        change(app, PanelId::Skill(index), false);
        assert!(form(app).skills.is_empty());
        assert!(!checked(app, index));
    }
}

#[test]
fn every_control_answers_its_event() {
    let mut app = creating("feathers-census.ron");
    assert_eq!(controls(&mut app), expected_controls(&app));
    type_name(&mut app, "Durin");
    assert_eq!(form(&app).name, "Durin");
    choices_by_event(&mut app);
    scores_by_event(&mut app);
    skills_by_event(&mut app);
    change(&mut app, PanelId::UiScale, 1.0_f32);
    assert!((app.world().resource::<UiScale>().0 - 1.0).abs() < 1e-6);
    // Add and Begin are heard in `a_fighter_is_drafted_through_the_panel`; Back abandons.
    activate(&mut app, PanelId::Back);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
    assert_eq!(controls(&mut app), Vec::new(), "the panel leaves");
}

// ------------------------------------------------------------------ the pointer

#[test]
fn a_button_a_checkbox_a_slider_and_a_number_input_take_the_pointer() {
    let mut app = creating("feathers-pointer.ron");
    // A button: Add refuses a nameless draft, and the panel says so.
    let add = control(&mut app, PanelId::Add);
    click_node(&mut app, add);
    assert_eq!(
        shown(&mut app, LabelId::Message),
        "Give the character a name"
    );
    // A checkbox, on and off again.
    let skill = control(&mut app, PanelId::Skill(0));
    click_node(&mut app, skill);
    assert_eq!(form(&app).skills.len(), 1);
    assert!(checked(&mut app, 0));
    click_node(&mut app, skill);
    assert!(form(&app).skills.is_empty());
    assert!(!checked(&mut app, 0));
    // A slider is dragged (Feathers' sliders ignore a click on the track): three sevenths
    // of its width is three points of the eight to fifteen it spans.
    let strength = control(&mut app, PanelId::ScoreSlider(0));
    assert_eq!(form(&app).scores[0], 8);
    let width = rect(&app, strength).width();
    drag_node(&mut app, strength, Vec2::new(width * 3.0 / 7.0, 0.0));
    assert_eq!(form(&app).scores[0], 11);
    assert_eq!(number_text(&mut app, 0), "11");
    // A number input: a click selects what it holds, and the keys replace it.
    let dexterity = number_input(&mut app, 1);
    click_node(&mut app, dexterity);
    keys(&mut app, "14");
    assert_eq!(form(&app).scores[1], 14);
    assert!((slider(&mut app, PanelId::ScoreSlider(1)) - 14.0).abs() < 1e-6);
}

/// The skills' scroll pane (the roster's list view is another), its scrollbar and its thumb.
fn skills_pane(app: &mut App) -> (Entity, Entity, Entity) {
    let skill = control(app, PanelId::Skill(0));
    let pane = app.world().get::<ChildOf>(skill).expect("a pane").parent();
    assert!(app.world().get::<ScrollArea>(pane).is_some());
    let mut bars = app.world_mut().query::<(Entity, &Scrollbar)>();
    let bar = bars
        .iter(app.world())
        .find_map(|(entity, bar)| (bar.target == pane).then_some(entity))
        .expect("the pane's scrollbar");
    let mut thumbs = app
        .world_mut()
        .query_filtered::<(Entity, &ChildOf), With<ScrollbarThumb>>();
    let thumb = thumbs
        .iter(app.world())
        .find_map(|(entity, parent)| (parent.parent() == bar).then_some(entity))
        .expect("the scrollbar's thumb");
    (pane, bar, thumb)
}

fn scroll_y(app: &App, pane: Entity) -> f32 {
    app.world().get::<ScrollPosition>(pane).expect("a pane").y
}

/// The class with the longest skill list (the rogue's eleven), chosen by event.
fn longest_skill_list(app: &mut App) -> usize {
    let classes = options(app, Choice::Class).len();
    let mut best = (0, 0);
    for index in 0..classes {
        activate(app, PanelId::Pick(Choice::Class, index));
        let screens = app.world().resource::<Screens>();
        let skills = screens.creation.skill_list(&screens.catalog).1.len();
        if skills > best.1 {
            best = (index, skills);
        }
    }
    activate(app, PanelId::Pick(Choice::Class, best.0));
    best.1
}

#[test]
fn the_skills_pane_scrolls_by_its_scrollbar() {
    let mut app = creating("feathers-scroll.ron");
    let last = longest_skill_list(&mut app) - 1;
    assert_eq!(last, 10, "the rogue's eleven");
    let (pane, bar, thumb) = skills_pane(&mut app);
    let visible = |app: &mut App, index: usize| {
        let skill = control(app, PanelId::Skill(index));
        !rect(app, skill).intersect(rect(app, pane)).is_empty()
    };
    assert!(visible(&mut app, 0) && !visible(&mut app, last));
    // A click on the track under the thumb pages down.
    let bar = rect(&app, bar);
    click_at(&mut app, Vec2::new(bar.center().x, bar.max.y - 2.0));
    assert!(scroll_y(&app, pane) > 0.0);
    assert!(
        visible(&mut app, last),
        "the last skill scrolled into sight"
    );
    assert_eq!(layout_faults(&mut app), Vec::new());
    // What scrolled into sight takes a click.
    let skill = control(&mut app, PanelId::Skill(last));
    click_node(&mut app, skill);
    assert_eq!(form(&app).skills.len(), 1);
    assert!(checked(&mut app, last));
    // The thumb is dragged back to the top.
    drag_node(&mut app, thumb, Vec2::new(0.0, -bar.height()));
    assert!(scroll_y(&app, pane).abs() < 1e-6);
    assert!(visible(&mut app, 0) && !visible(&mut app, last));
}

// ------------------------------------------------------------------ the layout

/// The faults at one interface scale (`None` follows the window), for every class's panel.
fn faults_at(app: &mut App, scale: Option<u16>) -> Vec<(usize, Fault)> {
    app.world_mut().resource_mut::<ScaleChoice>().0 = scale;
    let mut faults = Vec::new();
    for class in 0..options(app, Choice::Class).len() {
        activate(app, PanelId::Pick(Choice::Class, class));
        faults.extend(layout_faults(app).into_iter().map(|fault| (class, fault)));
    }
    faults
}

/// Measured 2026-09-20: nothing overlaps at any size, and everything is inside the panel
/// except where the slider forces a scale the window cannot hold. The viewport is 960 by 540
/// physical pixels in a 1280 by 720 window, so 1.5 and 2 push the footer out of it there; on
/// the owner's 5120 by 1440 it is 1920 by 1080 and all four fit.
#[test]
fn the_panel_is_laid_out_at_both_window_sizes() {
    let mut app = creating("feathers-layout.ron");
    for (width, height, too_big) in [
        (1280.0, 720.0, &[150_u16, 200][..]),
        (5120.0, 1440.0, &[][..]),
    ] {
        resize(&mut app, width, height);
        for scale in [None, Some(100), Some(150), Some(200)] {
            let faults = faults_at(&mut app, scale);
            if scale.is_some_and(|s| too_big.contains(&s)) {
                let footer = (0, Fault::Outside(PanelId::Add));
                assert!(faults.contains(&footer), "{width} at {scale:?}: {faults:?}");
                let pushed_out =
                    |f: &(usize, Fault)| matches!(f.1, Fault::Outside(_) | Fault::PaneOutside);
                assert!(faults.iter().all(pushed_out), "{scale:?}: {faults:?}");
            } else {
                assert_eq!(faults, Vec::new(), "{width} by {height} at {scale:?}");
            }
        }
    }
    // The overlap check bites: Begin pulled left lies over Add.
    app.world_mut().resource_mut::<ScaleChoice>().0 = None;
    let begin = control(&mut app, PanelId::Begin);
    let mut node = app.world_mut().get_mut::<Node>(begin).expect("a node");
    node.margin.left = px(-40);
    settle(&mut app);
    let faults = layout_faults(&mut app);
    assert_eq!(faults, vec![Fault::Overlap(PanelId::Add, PanelId::Begin)]);
}

// ------------------------------------------------------------------ the typefaces

/// The texts under the panel that do not wear `family`, with the face they were given.
fn not_wearing(app: &mut App, family: usize) -> Vec<(Option<Face>, String)> {
    let faces = app.world().resource::<PanelFonts>().0[family].clone();
    let mut roots = app.world_mut().query_filtered::<Entity, With<PanelRoot>>();
    let root = roots.single(app.world()).expect("one panel");
    let mut texts = app
        .world_mut()
        .query::<(Entity, &TextFont, Option<&Face>, Option<&Text>)>();
    let world = app.world();
    let under = |entity: Entity| {
        let mut at = entity;
        while let Some(parent) = world.get::<ChildOf>(at) {
            at = parent.parent();
        }
        at == root
    };
    let mut seen = 0;
    let odd = texts
        .iter(world)
        .filter(|(entity, ..)| under(*entity))
        .inspect(|_| seen += 1)
        .filter(|(_, font, face, _)| {
            face.is_none_or(|face| {
                font.font != bevy::text::FontSource::Handle(faces[*face as usize].clone())
            })
        })
        .map(|(_, _, face, text)| (face.copied(), text.map(|t| t.0.clone()).unwrap_or_default()))
        .collect();
    assert!(seen > 40, "only {seen} texts under the panel");
    odd
}

#[test]
fn the_font_menu_dresses_the_whole_panel() {
    let mut app = creating("feathers-fonts.ron");
    assert_eq!(shown(&mut app, LabelId::Font), "Fira Sans");
    assert_eq!(not_wearing(&mut app, 0), Vec::new());
    // By mouse: the menu opens over the footer and an item takes the click.
    let menu = control(&mut app, PanelId::FontMenu);
    click_node(&mut app, menu);
    let inter = control(&mut app, PanelId::FontPick(1));
    click_node(&mut app, inter);
    assert_eq!(app.world().resource::<FontChoice>().0, 1);
    assert_eq!(shown(&mut app, LabelId::Font), "Inter");
    assert_eq!(not_wearing(&mut app, 1), Vec::new());
    // The choice outlives a rebuilt panel, and every family keeps the layout whole.
    for family in [2, 1, 0] {
        activate(&mut app, PanelId::FontPick(family));
        assert_eq!(shown(&mut app, LabelId::Font), FONTS[family]);
        for (width, height) in [(1280.0, 720.0), (5120.0, 1440.0)] {
            resize(&mut app, width, height);
            assert_eq!(faults_at(&mut app, None), Vec::new(), "{}", FONTS[family]);
            assert_eq!(not_wearing(&mut app, family), Vec::new());
        }
    }
    // The three are three: the title's bold face differs from family to family.
    let fonts = &app.world().resource::<PanelFonts>().0;
    assert!(fonts[0][1] != fonts[1][1] && fonts[1][1] != fonts[2][1]);
}

// ------------------------------------------------------------------ the text tree

#[test]
fn the_text_tree_shows_the_panel() {
    let mut app = creating("feathers-tree.ron");
    draft_fighter(&mut app);
    let tree = text_tree(&mut app);
    for wanted in [
        "[Name] \"Brenna\"",
        "[Caption(Class)] \"Fighter\"",
        "[Points] \"Points left: 0 of 27\"",
        "[Skills] \"Skills (pick 2)\"",
        "\"Add member\"",
        "[Pick(Class, 1)] (hidden)",
        "[Heading] \"0 of 6 members\"",
    ] {
        assert!(tree.contains(wanted), "no {wanted} in\n{tree}");
    }
    assert!(tree.contains("\"CREATE YOUR PARTY\""));
    // Every control is a line.
    for id in controls(&mut app) {
        assert!(tree.contains(&format!("[{id:?}]")), "no {id:?} in\n{tree}");
    }
    if let Ok(dir) = std::env::var("OMNIS_DUMP_SCREENS") {
        let path = std::path::Path::new(&dir).join("creation_feathers.txt");
        std::fs::write(&path, &tree).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

// ------------------------------------------------------------------ one fighter, two paths

#[test]
fn the_panel_and_the_canvas_screen_draft_the_same_fighter() {
    let mut canvas = ui_app_saving_to("feathers-same-canvas.ron", false);
    start_new_game_by_mouse(&mut canvas);
    draft_fighter_by_mouse(&mut canvas);
    let mut panel = creating("feathers-same-panel.ron");
    draft_fighter(&mut panel);
    activate(&mut panel, PanelId::Add);
    assert_eq!(world(&canvas).party.members.len(), 1);
    assert_eq!(world(&panel).party.members, world(&canvas).party.members);
    assert_eq!(
        world(&panel).fingerprint().expect("a fingerprint"),
        world(&canvas).fingerprint().expect("a fingerprint"),
        "the same world, to the byte"
    );
}

// ------------------------------------------------------------------ the refusals

fn refused(app: &mut App, message: &str) {
    activate(app, PanelId::Add);
    assert_eq!(shown(app, LabelId::Message), message);
    assert!(world(app).party.members.is_empty());
}

#[test]
fn the_panel_refuses_what_the_canvas_screen_refuses() {
    let mut app = creating("feathers-refusals.ron");
    activate(&mut app, PanelId::Begin);
    assert_eq!(shown(&mut app, LabelId::Message), "Add at least one member");
    assert_eq!(play_state(&app), PlayState::CreateParty);
    refused(&mut app, "Give the character a name");
    // A twenty-fifth character is not taken, by the input or by the form.
    let name = control(&mut app, PanelId::Name);
    click_node(&mut app, name);
    keys(&mut app, "Abcdefghijklmnopqrstuvwxyz");
    assert_eq!(form(&app).name, "Abcdefghijklmnopqrstuvwx");
    refused(&mut app, "Pick 2 skills");
    // A third skill is refused and its box stays empty.
    for index in 0..3 {
        let skill = control(&mut app, PanelId::Skill(index));
        click_node(&mut app, skill);
    }
    assert_eq!(form(&app).skills.len(), 2);
    assert!(!checked(&mut app, 2));
    assert_eq!(
        shown(&mut app, LabelId::Message),
        "This class picks 2 skills"
    );
    // Point buy sells eight to fifteen: a typed 99 is held at fifteen, and the input shows
    // it once the keyboard leaves.
    let strength = number_input(&mut app, 0);
    click_node(&mut app, strength);
    keys(&mut app, "99");
    assert_eq!(form(&app).scores[0], 15);
    tab(&mut app);
    assert_eq!(number_text(&mut app, 0), "15");
    // Six fifteens cost more than the budget.
    for ability in 1..6 {
        change(&mut app, PanelId::Score(ability), 15_i32);
    }
    refused(&mut app, "54 points spent of 27");
}

#[test]
fn a_full_party_is_refused() {
    let mut app = creating("feathers-full.ron");
    let slots = app.world().resource::<Screens>().catalog.slots;
    for member in 0..slots {
        draft_fighter(&mut app);
        activate(&mut app, PanelId::Add);
        assert_eq!(world(&app).party.members.len(), member + 1);
    }
    assert_eq!(
        shown(&mut app, LabelId::Heading),
        format!("{slots} of {slots} members")
    );
    draft_fighter(&mut app);
    activate(&mut app, PanelId::Add);
    assert_eq!(shown(&mut app, LabelId::Message), "The party is full");
    assert_eq!(world(&app).party.members.len(), slots);
}
