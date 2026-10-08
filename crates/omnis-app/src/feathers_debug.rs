//! The debug panel as a `bevy_ui` panel over the map (feature `devtools`; owner, 2026-10-04:
//! numbers are typed, not pushed): the member's numbers, scores and a condition; the party's
//! purse and food; an item list with a count and two give buttons; a flag; a teleport; the
//! stacks of a fight. Opened from the pause overlay's item or the backtick while exploring or
//! fighting; Escape (outside a field) or Close goes back to the world. Everything shown comes
//! from `debug_menu::debug_view` through `debug_panel.rs`, which also answers every report; a
//! number bound to the world is sent as a `Dev` command when Enter or leaving it commits it.

use crate::cursor::UiSet;
use crate::debug_menu::{DebugView, debug_view};
use crate::debug_panel::{self as model, DebugAsk, DebugForm, DebugLabelId, DebugPanelId, FACINGS};
use crate::feathers_ui::HoldsKeyboard;
use crate::menus::{Active, Where};
use crate::sim::{CommandRefused, PackData, PlayState, PlayerCommand, SimEvent, SimWorld, Views};
use crate::ui_kit::{
    Control, PanelRoot, Shown, UiId, UiLabel, UiReport, UiScreen, Width, button, column, dropdown,
    message_line, panel as panel_root, row, row_label, scroll_column, set_text, title,
};
use bevy::feathers::containers::flex_spacer;
use bevy::feathers::controls::{
    ButtonVariant, FeathersButton, FeathersNumberInput, FeathersScrollbar, NumberFormat,
    NumberInputValue, UpdateNumberInput,
};
use bevy::feathers::display::{label, label_dim};
use bevy::feathers::theme::ThemedText;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{ControlOrientation, ScrollArea};
use omnis_sim::Command;
use omnis_sim::omnis_data::Ability;

/// The debug panel: its systems and its state.
pub struct DebugPanelPlugin;

impl Plugin for DebugPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugShown>()
            .add_message::<KeyboardInput>()
            .add_systems(
                Update,
                (toggle, reports, escape_closes)
                    .chain()
                    .in_set(UiSet::Dispatch),
            )
            .add_systems(
                Update,
                (look, refusals, reconcile, sync)
                    .chain()
                    .in_set(UiSet::Model),
            );
    }
}

/// The world as the panel last read it, and what the panel keeps.
#[derive(Resource, Debug, Default)]
pub struct DebugShown {
    /// The view, while the panel is up.
    pub view: Option<DebugView>,
    /// The selectors, the staged values and the last refusal.
    pub form: Option<DebugForm>,
    /// Whether the widgets show all of the above.
    synced: bool,
}

fn id(id: DebugPanelId) -> UiId {
    UiId::Debug(id)
}

fn shown(label: DebugLabelId) -> UiLabel {
    UiLabel::Debug(label)
}

// ------------------------------------------------------------------ scenes

/// A number field bound to a control.
fn number(at: DebugPanelId, wide: f32) -> impl Scene {
    bsn! {
        @FeathersNumberInput { @number_format: {NumberFormat::I32} }
        Control({id(at)})
        Node { width: px(wide), flex_grow: 0.0 }
    }
}

/// A dropdown whose options are the given texts.
fn menu(
    button: DebugPanelId,
    caption: DebugLabelId,
    pick: fn(usize) -> DebugPanelId,
    texts: Vec<String>,
    wide: f32,
) -> impl Scene {
    let options = texts
        .into_iter()
        .enumerate()
        .map(|(at, text)| (id(pick(at)), text))
        .collect();
    dropdown(id(button), shown(caption), options, Width::Px(wide))
}

/// A section's heading.
fn heading(text: &'static str) -> impl Scene {
    bsn! { label_dim(text) }
}

/// One score's label and field.
fn score(at: usize) -> impl Scene {
    let short = Ability::ALL[at].short();
    bsn! {
        Node { display: Display::Flex, flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(4) }
        Children [ label(short), number(DebugPanelId::Score(at), 64.0) ]
    }
}

/// An item of the list, a button that chooses it.
fn item_button(at: usize, text: String) -> impl Scene {
    bsn! {
        @FeathersButton { @caption: bsn! { Text({text.clone()}) ThemedText } }
        Control({id(DebugPanelId::Item(at))})
    }
}

/// The item list, in a pane that scrolls (a menu's popup would not).
fn items_pane(items: Vec<String>) -> impl Scene {
    let buttons: Vec<_> = items
        .into_iter()
        .enumerate()
        .map(|(at, text)| item_button(at, text))
        .collect();
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            min_height: px(0),
        }
        Children [
            (
                #items
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    row_gap: px(4),
                    overflow: Overflow::scroll_y(),
                }
                ScrollArea
                Children [ {buttons} ]
            ),
            (
                @FeathersScrollbar {
                    @target: #items,
                    @orientation: {ControlOrientation::Vertical}
                }
            ),
        ]
    }
}

/// The whole panel.
fn debug_panel(view: &DebugView) -> impl Scene {
    let members = view.members.iter().map(|m| m.name.clone()).collect();
    let conditions = view.conditions.iter().map(|c| c.1.clone()).collect();
    let flags = view.flags.iter().map(|f| f.0.clone()).collect();
    let maps = view.maps.iter().map(|m| m.0.clone()).collect();
    let facings = FACINGS.iter().map(ToString::to_string).collect();
    let stacks = view.stacks.iter().map(|s| s.name.clone()).collect();
    let items = view.items.iter().map(|i| i.1.clone()).collect();
    let scores: Vec<_> = (0..Ability::ALL.len()).map(score).collect();
    let left = bsn! {
        Node { display: Display::Flex, flex_direction: FlexDirection::Column, row_gap: px(6) }
        Children [
            heading("Member"),
            (
                row()
                Children [
                    row_label("Hit points"),
                    number(DebugPanelId::Hp, 90.0),
                    (label("") Shown({shown(DebugLabelId::HpMax)})),
                ]
            ),
            (
                row()
                Children [
                    row_label("Spell points"),
                    number(DebugPanelId::Sp, 90.0),
                    (label("") Shown({shown(DebugLabelId::SpMax)})),
                ]
            ),
            (
                row()
                Children [
                    row_label("Experience"),
                    number(DebugPanelId::Xp, 110.0),
                    (label("") Shown({shown(DebugLabelId::Level)})),
                ]
            ),
            (row() Children [ {scores} ]),
            (
                row()
                Children [
                    row_label("Condition"),
                    menu(DebugPanelId::Condition, DebugLabelId::Condition, DebugPanelId::ConditionPick, conditions, 180.0),
                    (label("") Shown({shown(DebugLabelId::ConditionState)})),
                    button(id(DebugPanelId::Toggle), "On / off", ButtonVariant::Normal),
                ]
            ),
            heading("Party"),
            (
                row()
                Children [
                    row_label("Copper"),
                    number(DebugPanelId::Gold, 110.0),
                    row_label("Food"),
                    number(DebugPanelId::Food, 90.0),
                ]
            ),
            heading("Flag"),
            (
                row()
                Children [
                    menu(DebugPanelId::Flag, DebugLabelId::Flag, DebugPanelId::FlagPick, flags, 220.0),
                    number(DebugPanelId::FlagValue, 90.0),
                    button(id(DebugPanelId::SetFlag), "Set", ButtonVariant::Normal),
                ]
            ),
            heading("Teleport"),
            (
                row()
                Children [
                    menu(DebugPanelId::Map, DebugLabelId::Map, DebugPanelId::MapPick, maps, 220.0),
                    label("x"),
                    number(DebugPanelId::X, 64.0),
                    label("y"),
                    number(DebugPanelId::Y, 64.0),
                    menu(DebugPanelId::Facing, DebugLabelId::Facing, DebugPanelId::FacingPick, facings, 90.0),
                    button(id(DebugPanelId::Go), "Go", ButtonVariant::Normal),
                ]
            ),
            heading("Fight"),
            (
                row()
                Children [
                    menu(DebugPanelId::Stack, DebugLabelId::Stack, DebugPanelId::StackPick, stacks, 220.0),
                    label("lead hit points"),
                    number(DebugPanelId::StackHp, 90.0),
                    button(id(DebugPanelId::Kill), "Kill", ButtonVariant::Normal),
                ]
            ),
        ]
    };
    bsn! {
        panel_root()
        Children [
            (
                row()
                Children [
                    title("Debug"),
                    menu(DebugPanelId::Member, DebugLabelId::Member, DebugPanelId::MemberPick, members, 220.0),
                    flex_spacer(),
                    label_dim("Type a number, then Enter or Tab"),
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(16),
                    flex_grow: 1.0,
                    min_height: px(0),
                }
                Children [
                    (column() Children [ scroll_column(left) ]),
                    (
                        column()
                        Children [
                            heading("Items"),
                            items_pane(items),
                            (label("") Shown({shown(DebugLabelId::Item)})),
                            (
                                row()
                                Children [
                                    label("Count"),
                                    number(DebugPanelId::Count, 64.0),
                                ]
                            ),
                            (
                                row()
                                Children [
                                    button(id(DebugPanelId::GiveMember), "To member", ButtonVariant::Normal),
                                    button(id(DebugPanelId::GiveStores), "To stores", ButtonVariant::Normal),
                                ]
                            ),
                        ]
                    ),
                ]
            ),
            message_line(shown(DebugLabelId::Message)),
            (
                row()
                Children [
                    flex_spacer(),
                    button(id(DebugPanelId::Close), "Close", ButtonVariant::Primary),
                ]
            ),
        ]
    }
}

// ------------------------------------------------------------------ systems

/// The backtick opens the panel while exploring or fighting, and closes it.
fn toggle(
    mut keys: MessageReader<KeyboardInput>,
    at: Where,
    views: Option<Res<Views>>,
    mut next: ResMut<NextState<PlayState>>,
) {
    let pressed = keys.read().any(|k| {
        k.state == ButtonState::Pressed
            && matches!(&k.logical_key, Key::Character(text) if text.as_str() == "`")
    });
    let Some(views) = views else {
        return;
    };
    if !pressed || !crate::menu::debug_available(views.here.settings) {
        return;
    }
    match at.screen() {
        Active::None | Active::Combat if at.playing() => next.set(PlayState::Debug),
        Active::Debug => next.set(PlayState::for_kind(views.here.mode)),
        _ => {}
    }
}

/// Read the world again whenever it changed while the panel is up; forget it all when down.
fn look(
    at: Where,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut shown: ResMut<DebugShown>,
) {
    let (Some(world), Some(data)) = (world, data) else {
        return;
    };
    if at.screen() != Active::Debug {
        if shown.view.is_some() {
            *shown = DebugShown::default();
        }
        return;
    }
    if shown.view.is_none() || world.is_changed() {
        let view = debug_view(&world.0, &data.0);
        let form = shown.form.get_or_insert_with(|| DebugForm::open(&view));
        form.sync(&view);
        shown.view = Some(view);
        shown.synced = false;
    }
}

/// A refusal while the panel is up is the message; any event asks for a redraw.
fn refusals(
    at: Where,
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    mut shown: ResMut<DebugShown>,
) {
    if events.read().count() > 0 {
        shown.synced = false;
    }
    for CommandRefused(rejection) in refused.read() {
        if at.screen() == Active::Debug
            && let Some(form) = shown.form.as_mut()
        {
            form.message = rejection.to_string();
            shown.synced = false;
        }
    }
}

/// Spawn, despawn or spawn again, so a panel exists exactly while the debug panel is open, and
/// always for the current shape.
fn reconcile(
    mut commands: Commands,
    mut shown: ResMut<DebugShown>,
    roots: Query<(Entity, &PanelRoot)>,
) {
    let shape = shown.view.as_ref().map(model::shape);
    let mut standing = false;
    for (entity, root) in &roots {
        if root.screen != UiScreen::Debug {
            continue;
        }
        if Some(root.shape) == shape && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let (Some(view), Some(shape), false) = (&shown.view, shape, standing) {
        commands.spawn_scene(debug_panel(view)).insert(PanelRoot {
            screen: UiScreen::Debug,
            shape,
        });
        shown.synced = false;
    }
}

/// Write the view into the panel: the captions, the numbers (not while one is being typed in),
/// and the grey controls.
fn sync(
    mut commands: Commands,
    mut shown: ResMut<DebugShown>,
    focus: Res<InputFocus>,
    controls: Query<(Entity, &Control, Has<InteractionDisabled>)>,
    mut labels: Query<(&Shown, &mut Text)>,
) {
    let (Some(view), Some(form)) = (&shown.view, &shown.form) else {
        return;
    };
    let ours = controls
        .iter()
        .any(|(_, c, _)| matches!(c.0, UiId::Debug(_)));
    if shown.synced || !ours {
        return;
    }
    for (label, mut text) in &mut labels {
        if let UiLabel::Debug(label) = label.0 {
            set_text(&mut text, &model::label_text(label, view, form));
        }
    }
    for (entity, control, disabled) in &controls {
        let UiId::Debug(control) = control.0 else {
            continue;
        };
        if focus.get() != Some(entity)
            && let Some(value) = model::number(control, view, form)
        {
            let value = i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX });
            commands.trigger(UpdateNumberInput {
                entity,
                value: NumberInputValue::I32(value),
            });
        }
        let dim = model::dim(control, view);
        if dim && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !dim && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    shown.synced = true;
}

/// Answer the panel's reports: each goes through `debug_panel::apply`; a command goes to the
/// simulation, Close back to the world.
fn reports(
    mut reports: MessageReader<UiReport>,
    mut shown: ResMut<DebugShown>,
    views: Option<Res<Views>>,
    mut out: MessageWriter<PlayerCommand>,
    mut next: ResMut<NextState<PlayState>>,
) {
    for report in reports.read() {
        let UiId::Debug(control) = report.id else {
            continue;
        };
        let DebugShown { view, form, synced } = &mut *shown;
        let (Some(view), Some(form)) = (view.as_ref(), form.as_mut()) else {
            continue;
        };
        *synced = false;
        form.message.clear();
        match model::apply(control, &report.payload, view, form) {
            Some(DebugAsk::Send(command)) => {
                out.write(PlayerCommand(Command::Dev(command)));
            }
            Some(DebugAsk::Close) => {
                if let Some(views) = views.as_ref() {
                    next.set(PlayState::for_kind(views.here.mode));
                }
            }
            None => {}
        }
    }
}

/// Escape goes back to the world, unless a control holds the keyboard.
fn escape_closes(
    mut keys: MessageReader<KeyboardInput>,
    at: Where,
    focus: Res<InputFocus>,
    holders: Query<(), HoldsKeyboard>,
    views: Option<Res<Views>>,
    mut next: ResMut<NextState<PlayState>>,
) {
    let escaped = keys
        .read()
        .any(|k| k.state == ButtonState::Pressed && k.logical_key == Key::Escape);
    let held = focus.get().is_some_and(|entity| holders.contains(entity));
    if escaped
        && at.screen() == Active::Debug
        && !held
        && let Some(views) = views
    {
        next.set(PlayState::for_kind(views.here.mode));
    }
}
