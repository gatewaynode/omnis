//! Party creation in Feathers (PRD D26, ARCHITECTURE.md A11): the scenes, and the systems
//! that keep them honest. The widgets hold no truth: every report goes through
//! `creation_panel::apply` into the `CreationForm`, and `sync`
//! writes the form back into the widgets. The entity tree is spawned again when its shape
//! changes (another class's skills, a longer roster); values alone are synced.

use crate::creation_panel::{self as panel, Choice, LabelId, PanelAction, PanelId};
use crate::cursor::WindowSize;
use crate::menus::{Active, CreationAsk, Screens, Where};
use crate::sim::SimWorld;
use crate::ui_kit::{
    Control, FontChoice, PanelRoot, ScaleChoice, Shown, UiId, UiLabel, UiReport, UiScreen, Width,
    button, column, dropdown, message_line, panel as panel_root, row, row_label, scale_for,
    set_text, title,
};
use crate::ui_model::{FONTS, SCALE_MAX, SCALE_MIN, scale_cap};
use bevy::feathers::containers::flex_spacer;
use bevy::feathers::controls::{
    ButtonVariant, FeathersCheckbox, FeathersListRow, FeathersListView, FeathersNumberInput,
    FeathersScrollbar, FeathersSlider, FeathersTextInput, FeathersTextInputContainer, NumberFormat,
    NumberInputValue, UpdateNumberInput,
};
use bevy::feathers::display::{label, label_dim};
use bevy::feathers::theme::ThemedText;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::Checked;
use bevy::ui_widgets::{
    ControlOrientation, ScrollArea, SliderPrecision, SliderRange, SliderStep, SliderValue,
};
use omnis_sim::omnis_data::Ability;

/// The form as the widgets last saw it; `None` makes the next `sync` write everything.
#[derive(Resource, Debug, Default)]
pub struct Synced(Option<(crate::menu::CreationForm, usize)>);

fn name_row() -> impl Scene {
    bsn! {
        row()
        Children [
            row_label("Name"),
            (
                @FeathersTextInputContainer
                Children [
                    (
                        @FeathersTextInput { @max_characters: {crate::creation_menu::NAME_LIMIT} }
                        Control({UiId::Creation(PanelId::Name)})
                    )
                ]
            ),
        ]
    }
}

/// A choice as a dropdown.
fn choice_row(choice: Choice, options: Vec<String>) -> impl Scene {
    let options = options
        .into_iter()
        .enumerate()
        .map(|(index, text)| (PanelId::Pick(choice, index).into(), text))
        .collect();
    bsn! {
        row()
        Children [
            row_label(choice.label()),
            dropdown(
                PanelId::Menu(choice).into(),
                LabelId::Caption(choice).into(),
                options,
                Width::Grow,
            ),
        ]
    }
}

/// An ability: a slider and a number input on the same score, and what it costs.
fn score_row(index: usize, name: &'static str, min: f32, max: f32) -> impl Scene {
    bsn! {
        row()
        Children [
            (Node { width: px(40) } Children [ label(name) ]),
            (
                @FeathersSlider { @min: {min}, @max: {max}, @value: {min} }
                Control({UiId::Creation(PanelId::ScoreSlider(index))})
                SliderStep(1.)
                SliderPrecision(0)
                Node { flex_grow: 1.0 }
            ),
            (
                @FeathersNumberInput { @number_format: {NumberFormat::I32} }
                Control({UiId::Creation(PanelId::Score(index))})
                Node { width: px(64), flex_grow: 0.0 }
            ),
            (
                Node { width: px(56) }
                Children [ (label_dim("") Shown({UiLabel::Creation(LabelId::Cost(index))})) ]
            ),
        ]
    }
}

fn skill_box(index: usize, text: String) -> impl Scene {
    bsn! {
        @FeathersCheckbox { @caption: bsn! { Text({text.clone()}) ThemedText } }
        Control({UiId::Creation(PanelId::Skill(index))})
    }
}

/// The class's skills, in a pane that scrolls when the list is long (the rogue's eleven).
fn skills_pane(skills: Vec<String>) -> impl Scene {
    let boxes: Vec<_> = skills
        .into_iter()
        .enumerate()
        .map(|(index, text)| skill_box(index, text))
        .collect();
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            padding: UiRect { right: px(10) },
            max_height: px(150),
        }
        Children [
            (
                #skills
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    overflow: Overflow::scroll_y(),
                }
                ScrollArea
                Children [ {boxes} ]
            ),
            (
                @FeathersScrollbar {
                    @target: #skills,
                    @orientation: {ControlOrientation::Vertical}
                }
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
                    width: px(6),
                }
            )
        ]
    }
}

fn roster_row(name: String) -> impl Scene {
    bsn! { @FeathersListRow Children [ (Text({name.clone()}) ThemedText) ] }
}

/// Who the member is: name, the four choices, the skills.
fn identity(form: &crate::menu::CreationForm, catalog: &crate::menu::Catalog) -> impl Scene {
    let [race, class, background, alignment] = Choice::ALL.map(|c| c.options(catalog));
    let skills = form
        .skill_list(catalog)
        .1
        .iter()
        .map(|s| crate::menu::words(&format!("{s:?}")))
        .collect();
    bsn! {
        column()
        Children [
            name_row(),
            choice_row(Choice::Race, race),
            choice_row(Choice::Class, class),
            choice_row(Choice::Background, background),
            choice_row(Choice::Alignment, alignment),
            (label("") Shown({UiLabel::Creation(LabelId::Skills)})),
            skills_pane(skills),
        ]
    }
}

/// What the member can do: the six scores, the points, and who is already in the party.
fn abilities(catalog: &crate::menu::Catalog, roster: &[String]) -> impl Scene {
    let (min, max) = (f32::from(catalog.min), f32::from(catalog.max));
    let scores: Vec<_> = Ability::ALL
        .iter()
        .enumerate()
        .map(|(index, ability)| score_row(index, ability.short(), min, max))
        .collect();
    let members: Vec<_> = roster.iter().cloned().map(roster_row).collect();
    bsn! {
        column()
        Children [
            {scores},
            (label("") Shown({UiLabel::Creation(LabelId::Points)})),
            title("Party"),
            (
                @FeathersListView { @rows: {Box::new(members) as Box<dyn SceneList>} }
                Node { max_height: px(110) }
            ),
        ]
    }
}

/// The typefaces, as a dropdown like the choices'.
fn font_menu() -> impl Scene {
    let options = FONTS
        .iter()
        .enumerate()
        .map(|(index, name)| (PanelId::FontPick(index).into(), (*name).to_owned()))
        .collect();
    dropdown(
        PanelId::FontMenu.into(),
        LabelId::Font.into(),
        options,
        Width::Px(130.0),
    )
}

fn footer() -> impl Scene {
    bsn! {
        row()
        Children [
            button(PanelId::Add.into(), "Add member", ButtonVariant::Primary),
            button(PanelId::Begin.into(), "Begin", ButtonVariant::Normal),
            button(PanelId::Back.into(), "Back", ButtonVariant::Normal),
            flex_spacer(),
            label_dim("Font"),
            font_menu(),
            label_dim("Interface scale"),
            (
                @FeathersSlider {
                    @min: {f32::from(SCALE_MIN) / 100.0},
                    @max: {f32::from(SCALE_MAX) / 100.0},
                    @value: 1.0,
                }
                Control({UiId::Creation(PanelId::UiScale)})
                SliderStep(0.05)
                SliderPrecision(2)
                Node { width: px(140), flex_grow: 0.0 }
            ),
        ]
    }
}

/// The whole panel for this form, catalog and roster.
fn creation_panel(
    form: &crate::menu::CreationForm,
    catalog: &crate::menu::Catalog,
    roster: &[String],
) -> impl Scene {
    bsn! {
        panel_root()
        Children [
            (
                row()
                Children [
                    title("CREATE YOUR PARTY"),
                    flex_spacer(),
                    (label("") Shown({UiLabel::Creation(LabelId::Heading)})),
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(32),
                    flex_grow: 1.0,
                }
                Children [
                    identity(form, catalog),
                    abilities(catalog, roster),
                ]
            ),
            message_line(LabelId::Message.into()),
            footer(),
        ]
    }
}

fn roster(world: Option<&SimWorld>) -> Vec<String> {
    world.map_or_else(Vec::new, |w| {
        w.0.party.members.iter().map(|m| m.name.clone()).collect()
    })
}

/// Spawn, despawn or spawn again, so a panel exists exactly while party creation is
/// up, and always for the current shape.
pub fn reconcile(
    mut commands: Commands,
    at: Where,
    screens: Res<Screens>,
    world: Option<Res<SimWorld>>,
    roots: Query<(Entity, &PanelRoot)>,
    mut synced: ResMut<Synced>,
) {
    let wanted = at.screen() == Active::CreateParty;
    let names = roster(world.as_deref());
    let shape = panel::shape(&screens.creation, &screens.catalog, &names);
    let mut standing = false;
    for (entity, root) in &roots {
        if root.screen != UiScreen::Creation {
            continue;
        }
        if wanted && root.shape == shape && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if wanted && !standing {
        commands
            .spawn_scene(creation_panel(&screens.creation, &screens.catalog, &names))
            .insert(PanelRoot {
                screen: UiScreen::Creation,
                shape,
            });
        synced.0 = None;
    }
}

/// The scale slider shows the interface scale, whichever it is (its scene cannot know, and a
/// rebuilt panel starts over), and ends at what the window holds (`scale_cap`), so a scale
/// that pushes the footer out of the panel cannot be chosen.
pub fn show_scale(
    mut commands: Commands,
    size: Res<WindowSize>,
    choice: Res<ScaleChoice>,
    sliders: Query<(Entity, &Control, &SliderValue, &SliderRange)>,
) {
    let canvas_scale = size.fit().scale;
    let shown = f32::from(scale_for(*choice, canvas_scale)) / 100.0;
    let end = f32::from(scale_cap(canvas_scale)) / 100.0;
    for (entity, control, value, range) in &sliders {
        if control.0 != UiId::Creation(PanelId::UiScale) {
            continue;
        }
        if (range.end() - end).abs() > f32::EPSILON {
            commands.entity(entity).insert(range.with_end(end));
        }
        if (value.0 - shown).abs() > f32::EPSILON {
            commands.entity(entity).insert(SliderValue(shown));
        }
    }
}

fn label_text(
    id: LabelId,
    form: &crate::menu::CreationForm,
    catalog: &crate::menu::Catalog,
    shown: &panel::PanelText,
    font: usize,
) -> String {
    match id {
        LabelId::Font => FONTS.get(font).copied().unwrap_or_default().to_owned(),
        LabelId::Heading => shown.heading.clone(),
        LabelId::Caption(choice) => choice
            .options(catalog)
            .get(choice.current(form))
            .cloned()
            .unwrap_or_default(),
        LabelId::Cost(index) => shown
            .scores
            .get(index)
            .map(|s| s.1.clone())
            .unwrap_or_default(),
        LabelId::Points => shown.points.clone(),
        LabelId::Skills => shown.skills.clone(),
        LabelId::Message => shown.message.clone(),
    }
}

/// Write the form into the widgets: they hold external state, so nothing moves unless this
/// says so. Runs when the form or the party changed, and once after every spawn.
#[allow(clippy::too_many_arguments)]
pub fn sync(
    mut commands: Commands,
    screens: Res<Screens>,
    world: Option<Res<SimWorld>>,
    focus: Res<InputFocus>,
    mut synced: ResMut<Synced>,
    font: Res<FontChoice>,
    controls: Query<(Entity, &Control, Option<&SliderValue>, Has<Checked>)>,
    mut inputs: Query<&mut EditableText>,
    mut labels: Query<(&Shown, &mut Text)>,
) {
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    let (form, catalog) = (&screens.creation, &screens.catalog);
    let fresh = synced
        .0
        .as_ref()
        .is_some_and(|(f, m)| f == form && *m == members);
    if fresh || controls.is_empty() {
        return;
    }
    let shown = panel::text(form, catalog, members);
    for (shown_id, mut text) in &mut labels {
        let UiLabel::Creation(id) = shown_id.0;
        set_text(&mut text, &label_text(id, form, catalog, &shown, font.0));
    }
    for (entity, control, slider, checked) in &controls {
        let UiId::Creation(id) = control.0;
        match id {
            // A focused input is left to the player, except where the form kept less than
            // it shows: a name cut at the rules' limit, or the blank of the next member.
            PanelId::Name => {
                if let Ok(mut input) = inputs.get_mut(entity)
                    && input.value().to_string() != form.name
                    && (focus.get() != Some(entity)
                        || input.value().to_string().starts_with(&form.name))
                {
                    input.queue_edit(TextEdit::SelectAll);
                    input.queue_edit(TextEdit::Insert(form.name.as_str().into()));
                }
            }
            PanelId::Score(index) => commands.trigger(UpdateNumberInput {
                entity,
                value: NumberInputValue::I32(i32::from(form.scores[index % 6])),
            }),
            PanelId::ScoreSlider(index) => {
                let wanted = f32::from(form.scores[index % 6]);
                if slider.map(|s| s.0) != Some(wanted) {
                    commands.entity(entity).insert(SliderValue(wanted));
                }
            }
            PanelId::Skill(index) => {
                let picked = form
                    .skill_list(catalog)
                    .1
                    .get(index)
                    .is_some_and(|s| form.skills.contains(s));
                if picked && !checked {
                    commands.entity(entity).insert(Checked);
                } else if !picked && checked {
                    commands.entity(entity).remove::<Checked>();
                }
            }
            _ => {}
        }
    }
    synced.0 = Some((form.clone(), members));
}

/// Answer the panel's reports (`ui_kit::UiReport`): each goes through `creation_panel::apply`
/// into the form, and what comes back is asked of the creation flow or kept as a choice.
pub fn reports(
    mut reports: MessageReader<UiReport>,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    mut asks: MessageWriter<CreationAsk>,
    mut scale: ResMut<ScaleChoice>,
    mut font: ResMut<FontChoice>,
    mut synced: ResMut<Synced>,
) {
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    for report in reports.read() {
        let UiId::Creation(id) = report.id;
        let Screens {
            creation, catalog, ..
        } = &mut *screens;
        let action = panel::apply(id, &report.payload, creation, catalog, members);
        // What was reported may have been held or refused (a typed 99 is a 15), and then
        // the form did not change: the widgets are written again whatever happened.
        synced.0 = None;
        match action {
            Some(PanelAction::Creation(action)) => {
                asks.write(CreationAsk(action));
            }
            Some(PanelAction::UiScale(hundredths)) => scale.0 = Some(hundredths),
            Some(PanelAction::Font(index)) => font.0 = index,
            None => {}
        }
    }
}
