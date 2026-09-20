//! Party creation in Feathers (the Feathers experiment, PRD D26): the scenes, and the systems
//! that keep them honest. The widgets hold no truth: every report goes through
//! `creation_panel::apply` into the same `CreationForm` the canvas screen edits, and `sync`
//! writes the form back into the widgets. The entity tree is spawned again when its shape
//! changes (another class's skills, a longer roster); values alone are synced.

use crate::canvas::Layout;
use crate::creation_panel::{
    self as panel, Choice, PanelAction, PanelId, Payload, SCALE_MAX, SCALE_MIN,
};
use crate::cursor::WindowSize;
use crate::layout::{VIEWPORT_SIZE, canvas_rect_to_window};
use crate::menus::{Active, CreationAsk, Screens, Where};
use crate::sim::SimWorld;
use bevy::feathers::constants::{fonts, size};
use bevy::feathers::containers::flex_spacer;
use bevy::feathers::controls::{
    ButtonVariant, FeathersButton, FeathersCheckbox, FeathersListRow, FeathersListView,
    FeathersMenu, FeathersMenuButton, FeathersMenuItem, FeathersMenuPopup, FeathersNumberInput,
    FeathersScrollbar, FeathersSlider, FeathersTextInput, FeathersTextInputContainer, NumberFormat,
    NumberInputValue, UpdateNumberInput,
};
use bevy::feathers::display::{label, label_dim};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::text::{EditableText, FontSourceTemplate, TextEdit, TextEditChange};
use bevy::ui::Checked;
use bevy::ui_widgets::{
    Activate, ControlOrientation, ScrollArea, SliderPrecision, SliderStep, SliderValue, ValueChange,
};
use omnis_sim::omnis_data::Ability;

/// The panel's root, with the shape it was spawned for.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelRoot(pub u64);

/// Which control an entity is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Control(pub PanelId);

/// A text the panel rewrites from the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelId {
    /// "2 of 6 members".
    #[default]
    Heading,
    /// A choice's current option, on its menu button.
    Caption(Choice),
    /// An ability's point cost.
    Cost(usize),
    /// Points left.
    Points,
    /// "Skills (pick 2)".
    Skills,
    /// The last refusal.
    Message,
}

/// Which rewritten text an entity is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shown(pub LabelId);

/// The form as the widgets last saw it; `None` makes the next `sync` write everything.
#[derive(Resource, Debug, Default)]
pub struct Synced(Option<(crate::menu::CreationForm, usize)>);

/// The refusal line's colour.
const ALERT: Color = Color::srgb(1.0, 0.47, 0.42);

fn title(text: &'static str) -> impl Scene {
    bsn! {
        Text(text)
        TextFont {
            font: FontSourceTemplate::Handle(fonts::BOLD),
            font_size: size::MEDIUM_FONT,
        }
        bevy::feathers::theme::ThemeTextColor(tokens::TEXT_MAIN)
    }
}

/// A row: a fixed-width label, then the control.
fn row() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            min_height: size::ROW_HEIGHT,
        }
    }
}

fn row_label(text: &'static str) -> impl Scene {
    bsn! {
        Node { width: px(96) }
        Children [ label(text) ]
    }
}

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
                        Control({PanelId::Name})
                    )
                ]
            ),
        ]
    }
}

fn menu_item(id: PanelId, text: String) -> impl Scene {
    bsn! {
        @FeathersMenuItem { @caption: bsn! { Text({text.clone()}) ThemedText } }
        Control({id})
    }
}

/// A choice as a menu: Feathers has no dropdown, so the button's caption is ours to keep.
fn choice_row(choice: Choice, options: Vec<String>) -> impl Scene {
    let items: Vec<_> = options
        .into_iter()
        .enumerate()
        .map(|(index, text)| menu_item(PanelId::Pick(choice, index), text))
        .collect();
    bsn! {
        row()
        Children [
            row_label(choice.label()),
            (
                @FeathersMenu
                Node { flex_grow: 1.0 }
                Children [
                    (
                        @FeathersMenuButton {
                            @caption: bsn! { Text("") ThemedText Shown({LabelId::Caption(choice)}) }
                        }
                        Control({PanelId::Menu(choice)})
                        Node { flex_grow: 1.0 }
                    ),
                    (
                        @FeathersMenuPopup
                        Children [ {items} ]
                    )
                ]
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
                Control({PanelId::ScoreSlider(index)})
                SliderStep(1.)
                SliderPrecision(0)
                Node { flex_grow: 1.0 }
            ),
            (
                @FeathersNumberInput { @number_format: {NumberFormat::I32} }
                Control({PanelId::Score(index)})
                Node { width: px(64), flex_grow: 0.0 }
            ),
            (
                Node { width: px(56) }
                Children [ (label_dim("") Shown({LabelId::Cost(index)})) ]
            ),
        ]
    }
}

fn skill_box(index: usize, text: String) -> impl Scene {
    bsn! {
        @FeathersCheckbox { @caption: bsn! { Text({text.clone()}) ThemedText } }
        Control({PanelId::Skill(index)})
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

fn column() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            flex_grow: 1.0,
            flex_basis: px(0),
        }
    }
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
            (label("") Shown({LabelId::Skills})),
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
            (label("") Shown({LabelId::Points})),
            title("Party"),
            (
                @FeathersListView { @rows: {Box::new(members) as Box<dyn SceneList>} }
                Node { max_height: px(110) }
            ),
        ]
    }
}

fn button(id: PanelId, text: &'static str, variant: ButtonVariant) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { Text(text) ThemedText },
            @variant: {variant},
        }
        Control({id})
    }
}

fn footer() -> impl Scene {
    bsn! {
        row()
        Children [
            button(PanelId::Add, "Add member", ButtonVariant::Primary),
            button(PanelId::Begin, "Begin", ButtonVariant::Normal),
            button(PanelId::Back, "Back", ButtonVariant::Normal),
            flex_spacer(),
            label_dim("Interface scale"),
            (
                @FeathersSlider {
                    @min: {f32::from(SCALE_MIN) / 100.0},
                    @max: {f32::from(SCALE_MAX) / 100.0},
                    @value: 1.0,
                }
                Control({PanelId::UiScale})
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
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: {UiRect::all(px(16))},
            overflow: {Overflow::clip()},
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            (
                row()
                Children [
                    title("CREATE YOUR PARTY"),
                    flex_spacer(),
                    (label("") Shown({LabelId::Heading})),
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
            (
                Text("")
                TextFont {
                    font: FontSourceTemplate::Handle(fonts::REGULAR),
                    font_size: size::MEDIUM_FONT,
                }
                TextColor({ALERT})
                Shown({LabelId::Message})
            ),
            footer(),
        ]
    }
}

fn roster(world: Option<&SimWorld>) -> Vec<String> {
    world.map_or_else(Vec::new, |w| {
        w.0.party.members.iter().map(|m| m.name.clone()).collect()
    })
}

/// Spawn, despawn or spawn again, so a panel exists exactly while the Feathers skin of party
/// creation is up, and always for the current shape.
pub fn reconcile(
    mut commands: Commands,
    at: Where,
    screens: Res<Screens>,
    world: Option<Res<SimWorld>>,
    roots: Query<(Entity, &PanelRoot)>,
    mut synced: ResMut<Synced>,
) {
    let wanted = at.screen() == Active::CreateParty && screens.skin.feathers;
    let names = roster(world.as_deref());
    let shape = panel::shape(&screens.creation, &screens.catalog, &names);
    let mut standing = false;
    for (entity, root) in &roots {
        if wanted && root.0 == shape && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if wanted && !standing {
        commands
            .spawn_scene(creation_panel(&screens.creation, &screens.catalog, &names))
            .insert(PanelRoot(shape));
        synced.0 = None;
    }
}

/// The interface scale the slider chose, in hundredths; none follows the window.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScaleChoice(pub Option<u16>);

/// The interface scale is the slider's, or follows the canvas's whole-number scale; the
/// slider shows whichever it is (its scene cannot know, and a rebuilt panel starts over).
pub fn scale(
    mut commands: Commands,
    size: Res<WindowSize>,
    choice: Res<ScaleChoice>,
    mut scale: ResMut<UiScale>,
    sliders: Query<(Entity, &Control, &SliderValue)>,
) {
    let hundredths = choice
        .0
        .unwrap_or_else(|| panel::fitted_scale(size.fit().scale));
    // A logical pixel is already `scale_factor` physical ones.
    let wanted = f32::from(hundredths) / 100.0 / size.scale_factor.max(0.1);
    if (scale.0 - wanted).abs() > f32::EPSILON {
        scale.0 = wanted;
    }
    let shown = f32::from(hundredths) / 100.0;
    for (entity, control, value) in &sliders {
        if control.0 == PanelId::UiScale && (value.0 - shown).abs() > f32::EPSILON {
            commands.entity(entity).insert(SliderValue(shown));
        }
    }
}

/// Keep the panel over the canvas's viewport, whatever the window and the interface scale.
pub fn place(
    size: Res<WindowSize>,
    layout: Res<Layout>,
    scale: Res<UiScale>,
    mut roots: Query<&mut Node, With<PanelRoot>>,
) {
    let rect = (
        layout.core.0,
        layout.core.1,
        u32::from(VIEWPORT_SIZE.0),
        u32::from(VIEWPORT_SIZE.1),
    );
    let (x, y, w, h) = canvas_rect_to_window(rect, (size.width, size.height), size.scale_factor);
    let s = scale.0.max(0.1);
    let wanted = [x / s, y / s, w / s, h / s].map(px);
    for mut node in &mut roots {
        let now = [node.left, node.top, node.width, node.height];
        if now != wanted {
            [node.left, node.top, node.width, node.height] = wanted;
        }
    }
}

fn set_text(text: &mut Text, wanted: &str) {
    if text.0 != wanted {
        wanted.clone_into(&mut text.0);
    }
}

fn label_text(
    id: LabelId,
    form: &crate::menu::CreationForm,
    catalog: &crate::menu::Catalog,
    shown: &panel::PanelText,
) -> String {
    match id {
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
    for (id, mut text) in &mut labels {
        set_text(&mut text, &label_text(id.0, form, catalog, &shown));
    }
    for (entity, control, slider, checked) in &controls {
        match control.0 {
            PanelId::Name => {
                if let Ok(mut input) = inputs.get_mut(entity)
                    && input.value().to_string() != form.name
                    && (focus.get() != Some(entity) || form.name.is_empty())
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

/// What the app does with a control's report.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Reports<'w, 's> {
    controls: Query<'w, 's, &'static Control>,
    screens: ResMut<'w, Screens>,
    world: Option<Res<'w, SimWorld>>,
    asks: MessageWriter<'w, CreationAsk>,
    scale: ResMut<'w, ScaleChoice>,
}

impl Reports<'_, '_> {
    /// Apply a report from `entity`, if it is one of the panel's controls.
    pub fn report(&mut self, entity: Entity, payload: &Payload) {
        let Ok(control) = self.controls.get(entity) else {
            return;
        };
        let members = self.world.as_ref().map_or(0, |w| w.0.party.members.len());
        let Screens {
            creation, catalog, ..
        } = &mut *self.screens;
        match panel::apply(control.0, payload, creation, catalog, members) {
            Some(PanelAction::Creation(action)) => {
                self.asks.write(CreationAsk(action));
            }
            Some(PanelAction::UiScale(hundredths)) => self.scale.0 = Some(hundredths),
            // The font menu arrives with the fonts.
            Some(PanelAction::Font(_)) | None => {}
        }
    }
}

pub(crate) fn on_activate(event: On<Activate>, mut reports: Reports) {
    reports.report(event.entity, &Payload::Activate);
}

pub(crate) fn on_slide(event: On<ValueChange<f32>>, mut reports: Reports) {
    reports.report(event.source, &Payload::Slide(event.value));
}

pub(crate) fn on_number(event: On<ValueChange<i32>>, mut reports: Reports) {
    reports.report(event.source, &Payload::Number(i64::from(event.value)));
}

pub(crate) fn on_flag(event: On<ValueChange<bool>>, mut reports: Reports) {
    reports.report(event.source, &Payload::Flag(event.value));
}

pub(crate) fn on_text(
    event: On<TextEditChange>,
    inputs: Query<&EditableText>,
    mut reports: Reports,
) {
    let entity = event.event_target();
    if let Ok(input) = inputs.get(entity) {
        reports.report(entity, &Payload::Text(input.value().to_string()));
    }
}
