//! The interface kit: what every `bevy_ui` screen is built from (ARCHITECTURE.md §8.4). One id
//! for every control and every rewritten text (`UiId`, `UiLabel`), one root (`PanelRoot`), the
//! scenes screens share, the two systems that keep a panel over the canvas's viewport at the
//! interface scale, and one observer per payload type. The observers know no screen: they
//! publish a `UiReport`, and each screen reads the ones that carry its ids and answers them
//! with its own Bevy-free `apply`.

use crate::canvas::Layout;
use crate::confirm_panel::ConfirmId;
use crate::creation_panel::{LabelId, PanelId};
use crate::cursor::WindowSize;
use crate::layout::{VIEWPORT_SIZE, canvas_rect_to_window};
use crate::service_panel::{ServiceLabelId, ServicePanelId};
use crate::ui_model::{self as model, Payload};
use bevy::feathers::constants::{fonts, size};
use bevy::feathers::controls::{
    ButtonVariant, FeathersButton, FeathersMenu, FeathersMenuButton, FeathersMenuItem,
    FeathersMenuPopup,
};
use bevy::feathers::display::label;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeTextColor, ThemedText};
use bevy::feathers::tokens;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::text::{EditableText, FontSourceTemplate, TextEditChange};
use bevy::ui_widgets::{Activate, ValueChange};

/// A screen built on the kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiScreen {
    /// Party creation (`feathers_creation.rs`).
    Creation,
    /// The question before a step into or out of a service (`feathers_confirm.rs`).
    Confirm,
    /// Inside a town service (`feathers_service.rs`).
    Service,
}

/// One control, on whichever screen. Tests and the sync systems find entities by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiId {
    /// A control of the party creation panel.
    Creation(PanelId),
    /// A button of the confirmation.
    Confirm(ConfirmId),
    /// A control of the service panel.
    Service(ServicePanelId),
}

impl Default for UiId {
    fn default() -> Self {
        UiId::Creation(PanelId::default())
    }
}

impl UiId {
    /// The control's name on its own screen, as the text tree prints it.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            UiId::Creation(id) => format!("{id:?}"),
            UiId::Confirm(id) => format!("{id:?}"),
            UiId::Service(id) => format!("{id:?}"),
        }
    }
}

impl From<PanelId> for UiId {
    fn from(id: PanelId) -> Self {
        UiId::Creation(id)
    }
}

impl From<ConfirmId> for UiId {
    fn from(id: ConfirmId) -> Self {
        UiId::Confirm(id)
    }
}

impl From<ServicePanelId> for UiId {
    fn from(id: ServicePanelId) -> Self {
        UiId::Service(id)
    }
}

/// One text a screen rewrites from its model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLabel {
    /// A text of the party creation panel.
    Creation(LabelId),
    /// A text of the service panel.
    Service(ServiceLabelId),
}

impl Default for UiLabel {
    fn default() -> Self {
        UiLabel::Creation(LabelId::default())
    }
}

impl UiLabel {
    /// The text's name on its own screen, as the text tree prints it.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            UiLabel::Creation(id) => format!("{id:?}"),
            UiLabel::Service(id) => format!("{id:?}"),
        }
    }
}

impl From<LabelId> for UiLabel {
    fn from(id: LabelId) -> Self {
        UiLabel::Creation(id)
    }
}

impl From<ServiceLabelId> for UiLabel {
    fn from(id: ServiceLabelId) -> Self {
        UiLabel::Service(id)
    }
}

/// Which control an entity is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Control(pub UiId);

/// Which rewritten text an entity is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shown(pub UiLabel);

/// A panel's root: whose it is, and the shape it was spawned for.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelRoot {
    /// The screen that spawned it, and the only one that may despawn it.
    pub screen: UiScreen,
    /// The shape hash the entity tree was built for.
    pub shape: u64,
}

/// What a control reported. Widgets are not trusted: a screen's `apply` clamps and refuses.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct UiReport {
    /// The control.
    pub id: UiId,
    /// What it reported.
    pub payload: Payload,
}

/// The typeface the font menu chose, by index into `ui_model::FONTS`. It starts on Inter, the
/// owner's pick at look 2 (2026-09-20).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontChoice(pub usize);

impl Default for FontChoice {
    fn default() -> Self {
        FontChoice(1)
    }
}

/// The interface scale the slider chose, in hundredths; none follows the window.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScaleChoice(pub Option<u16>);

/// A refusal line's colour.
pub const ALERT: Color = Color::srgb(1.0, 0.47, 0.42);

// ------------------------------------------------------------------ scenes

/// A panel's root node: over the viewport (`place` sizes it), a padded column, clipped, one
/// tab group, on the theme's window colour. The screen adds its `PanelRoot` after the spawn.
pub fn panel() -> impl Scene {
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
    }
}

/// A heading in the bold face.
pub fn title(text: &'static str) -> impl Scene {
    bsn! {
        Text(text)
        TextFont {
            font: FontSourceTemplate::Handle(fonts::BOLD),
            font_size: size::MEDIUM_FONT,
        }
        ThemeTextColor(tokens::TEXT_MAIN)
    }
}

/// A row: its children side by side, centred, a row high.
pub fn row() -> impl Scene {
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

/// A row's fixed-width label.
pub fn row_label(text: &'static str) -> impl Scene {
    bsn! {
        Node { width: px(96) }
        Children [ label(text) ]
    }
}

/// A column that shares its parent's width with its siblings.
pub fn column() -> impl Scene {
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

/// A button.
pub fn button(id: UiId, text: &'static str, variant: ButtonVariant) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { Text(text) ThemedText },
            @variant: {variant},
        }
        Control({id})
    }
}

/// A refusal line: a text the screen rewrites, in the alert colour.
pub fn message_line(shown: UiLabel) -> impl Scene {
    bsn! {
        Text("")
        TextFont {
            font: FontSourceTemplate::Handle(fonts::REGULAR),
            font_size: size::MEDIUM_FONT,
        }
        TextColor({ALERT})
        Shown({shown})
    }
}

/// How wide a dropdown's button is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Width {
    /// It takes what the row has left.
    Grow,
    /// So many logical pixels.
    Px(f32),
}

fn menu_item(id: UiId, text: String) -> impl Scene {
    bsn! {
        @FeathersMenuItem { @caption: bsn! { Text({text.clone()}) ThemedText } }
        Control({id})
    }
}

/// A dropdown from a menu: Feathers has none, so the button's caption is a text the screen
/// keeps (`caption`), and every option is a control that reports `Payload::Activate`.
pub fn dropdown(
    button: UiId,
    caption: UiLabel,
    options: Vec<(UiId, String)>,
    width: Width,
) -> impl Scene {
    let items: Vec<_> = options
        .into_iter()
        .map(|(id, text)| menu_item(id, text))
        .collect();
    let (wide, grow) = match width {
        Width::Grow => (Val::Auto, 1.0_f32),
        Width::Px(pixels) => (Val::Px(pixels), 0.0_f32),
    };
    bsn! {
        @FeathersMenu
        Node { flex_grow: {grow} }
        Children [
            (
                @FeathersMenuButton {
                    @caption: bsn! { Text("") ThemedText Shown({caption}) }
                }
                Control({button})
                Node { width: {wide}, flex_grow: {grow} }
            ),
            (
                @FeathersMenuPopup
                Children [ {items} ]
            )
        ]
    }
}

// ------------------------------------------------------------------ systems

/// The interface scale is the slider's, or follows the canvas's whole-number scale; neither
/// is ever more than the window holds (`ui_model::scale_cap`).
#[must_use]
pub fn scale_for(choice: ScaleChoice, canvas_scale: u32) -> u16 {
    let cap = model::scale_cap(canvas_scale);
    choice.0.map_or_else(
        || model::fitted_scale(canvas_scale),
        |chosen| chosen.min(cap),
    )
}

/// Keep `UiScale` at the interface scale.
pub fn scale(size: Res<WindowSize>, choice: Res<ScaleChoice>, mut scale: ResMut<UiScale>) {
    let hundredths = scale_for(*choice, size.fit().scale);
    // A logical pixel is already `scale_factor` physical ones.
    let wanted = f32::from(hundredths) / 100.0 / size.scale_factor.max(0.1);
    if (scale.0 - wanted).abs() > f32::EPSILON {
        scale.0 = wanted;
    }
}

/// Keep every panel over the canvas's viewport, whatever the window and the interface scale.
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

/// Write a text only when it differs, so change detection stays quiet.
pub fn set_text(text: &mut Text, wanted: &str) {
    if text.0 != wanted {
        wanted.clone_into(&mut text.0);
    }
}

// ------------------------------------------------------------------ reports

/// What the observers need: which control an entity is, and where reports go.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Reporter<'w, 's> {
    controls: Query<'w, 's, &'static Control>,
    reports: MessageWriter<'w, UiReport>,
}

impl Reporter<'_, '_> {
    fn report(&mut self, entity: Entity, payload: Payload) {
        if let Ok(control) = self.controls.get(entity) {
            self.reports.write(UiReport {
                id: control.0,
                payload,
            });
        }
    }
}

pub(crate) fn on_activate(event: On<Activate>, mut reporter: Reporter) {
    reporter.report(event.entity, Payload::Activate);
}

pub(crate) fn on_slide(event: On<ValueChange<f32>>, mut reporter: Reporter) {
    reporter.report(event.source, Payload::Slide(event.value));
}

pub(crate) fn on_number(event: On<ValueChange<i32>>, mut reporter: Reporter) {
    reporter.report(event.source, Payload::Number(i64::from(event.value)));
}

pub(crate) fn on_flag(event: On<ValueChange<bool>>, mut reporter: Reporter) {
    reporter.report(event.source, Payload::Flag(event.value));
}

pub(crate) fn on_text(
    event: On<TextEditChange>,
    inputs: Query<&EditableText>,
    mut reporter: Reporter,
) {
    let entity = event.event_target();
    if let Ok(input) = inputs.get(entity) {
        reporter.report(entity, Payload::Text(input.value().to_string()));
    }
}
