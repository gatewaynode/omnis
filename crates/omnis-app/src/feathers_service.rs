//! A town service as a `bevy_ui` panel over the map (M7 step 7): the service's name, the
//! party's money and food, one row per thing on offer with its price and a button, the bank's
//! amount, the last refusal and Leave. Everything shown comes from `omnis_sim::service_view`
//! through `service_panel.rs`, which also answers every report; leaving goes through the same
//! question as a step out (`input::Gate`). The entity tree is spawned again when its shape
//! changes (a row added or emptied); prices, refusals and counts are rewritten in place.

use crate::feathers_ui::HoldsKeyboard;
use crate::input::Gate;
use crate::menus::{Active, Where};
use crate::service_panel::{
    self as model, OfferRow, ServiceAsk, ServiceForm, ServiceLabelId, ServicePanelId,
};
use crate::sim::{CommandRefused, PackData, SimEvent, SimWorld};
use crate::ui_kit::{
    Control, PanelRoot, Shown, UiId, UiLabel, UiReport, UiScreen, button, message_line,
    panel as panel_root, row, scroll_column, set_text,
};
use bevy::feathers::constants::{fonts, size};
use bevy::feathers::containers::flex_spacer;
use bevy::feathers::controls::{
    ButtonVariant, FeathersNumberInput, NumberFormat, NumberInputValue, UpdateNumberInput,
};
use bevy::feathers::display::{label, label_dim};
use bevy::feathers::theme::ThemeTextColor;
use bevy::feathers::tokens;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use bevy::ui::InteractionDisabled;
use omnis_sim::omnis_data::ServiceKind;
use omnis_sim::{Command, Event, ServiceCommand, ServiceView, service_view};

/// The service as the panel last read it, and what the panel keeps.
#[derive(Resource, Debug, Default)]
pub struct ServiceShown {
    /// The service the party is inside, while the panel is up.
    pub view: Option<ServiceView>,
    /// Its offers' rows, but leaving.
    pub rows: Vec<OfferRow>,
    /// The bank's amount and the last refusal.
    pub form: ServiceForm,
    /// Whether the widgets show all of the above.
    synced: bool,
}

// ------------------------------------------------------------------ scenes

fn heading(name: String) -> impl Scene {
    bsn! {
        Text(name)
        TextFont {
            font: FontSourceTemplate::Handle(fonts::BOLD),
            font_size: size::MEDIUM_FONT,
        }
        ThemeTextColor(tokens::TEXT_MAIN)
    }
}

/// One offer: what it is, its price or why not, and its button.
fn offer_row(index: usize, caption: &'static str) -> impl Scene {
    bsn! {
        row()
        Children [
            (
                Node { flex_grow: 1.0, flex_basis: px(0) }
                Children [ (label("") Shown({UiLabel::Service(ServiceLabelId::Row(index))})) ]
            ),
            (
                Node { width: px(160), flex_shrink: 0.0 }
                Children [ (label_dim("") Shown({UiLabel::Service(ServiceLabelId::Note(index))})) ]
            ),
            (
                Node { width: px(112), flex_shrink: 0.0 }
                Children [
                    button(ServicePanelId::Offer(index).into(), caption, ButtonVariant::Normal),
                ]
            ),
        ]
    }
}

/// A list of offers under a heading, scrolling when it is longer than the panel (the smith's
/// stock).
fn offer_list(title: &'static str, offers: Vec<(usize, &'static str)>) -> impl Scene {
    let rows: Vec<_> = offers
        .into_iter()
        .map(|(index, caption)| offer_row(index, caption))
        .collect();
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            flex_grow: 1.0,
            flex_basis: px(0),
            min_height: px(0),
        }
        Children [
            label_dim(title),
            scroll_column(bsn! {
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                }
                Children [ {rows} ]
            }),
        ]
    }
}

/// The bank: an amount in whole gold, and which way it goes.
fn bank_row() -> impl Scene {
    bsn! {
        row()
        Children [
            label("Gold pieces"),
            (
                @FeathersNumberInput { @number_format: {NumberFormat::I32} }
                Control({UiId::Service(ServicePanelId::Amount)})
                Node { width: px(120), flex_grow: 0.0 }
            ),
            button(ServicePanelId::Deposit.into(), "Deposit", ButtonVariant::Normal),
            button(ServicePanelId::Withdraw.into(), "Withdraw", ButtonVariant::Normal),
        ]
    }
}

/// The lists a kind of service shows, each a title and its offers by index.
fn lists(view: &ServiceView, rows: &[OfferRow]) -> Vec<(&'static str, Vec<(usize, &'static str)>)> {
    let pick = |wanted: fn(ServiceCommand) -> bool| -> Vec<(usize, &'static str)> {
        view.offers
            .iter()
            .zip(rows)
            .enumerate()
            .filter(|(_, (offer, _))| wanted(offer.command))
            .map(|(index, (_, row))| (index, row.caption))
            .collect()
    };
    match view.kind {
        ServiceKind::Smith => vec![
            (
                "For sale",
                pick(|c| matches!(c, ServiceCommand::Buy { .. })),
            ),
            (
                "Your stores",
                pick(|c| matches!(c, ServiceCommand::Sell { .. })),
            ),
        ],
        ServiceKind::Inn | ServiceKind::Tavern => vec![("On offer", pick(|_| true))],
        ServiceKind::Temple => vec![
            (
                "On offer",
                pick(|c| !matches!(c, ServiceCommand::Learn { .. })),
            ),
            (
                "Spells",
                pick(|c| matches!(c, ServiceCommand::Learn { .. })),
            ),
        ],
        ServiceKind::Trainer => vec![
            (
                "Levels",
                pick(|c| matches!(c, ServiceCommand::Train { .. })),
            ),
            (
                "Spell picks",
                pick(|c| matches!(c, ServiceCommand::Choose { .. })),
            ),
        ],
        ServiceKind::Guild => vec![(
            "Spells",
            pick(|c| matches!(c, ServiceCommand::Learn { .. })),
        )],
        ServiceKind::Bank => Vec::new(),
    }
}

/// The whole panel for a service.
fn service_panel(view: &ServiceView, rows: &[OfferRow], name: String) -> impl Scene {
    let lists: Vec<_> = lists(view, rows)
        .into_iter()
        .map(|(title, offers)| offer_list(title, offers))
        .collect();
    let bank: Vec<_> = (view.kind == ServiceKind::Bank)
        .then(bank_row)
        .into_iter()
        .collect();
    bsn! {
        panel_root()
        Children [
            (
                row()
                Children [
                    heading(name),
                    flex_spacer(),
                    (label("") Shown({UiLabel::Service(ServiceLabelId::Money)})),
                ]
            ),
            (
                Node { display: Display::Flex, flex_direction: FlexDirection::Column }
                Children [ {bank} ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(24),
                    flex_grow: 1.0,
                    min_height: px(0),
                }
                Children [ {lists} ]
            ),
            message_line(ServiceLabelId::Message.into()),
            (
                row()
                Children [
                    flex_spacer(),
                    button(ServicePanelId::Leave.into(), "Leave", ButtonVariant::Primary),
                ]
            ),
        ]
    }
}

// ------------------------------------------------------------------ systems

/// Read the service again whenever the world changed while the panel is up; forget it when
/// the panel is down.
pub fn look(
    at: Where,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut shown: ResMut<ServiceShown>,
) {
    let (Some(world), Some(data)) = (world, data) else {
        return;
    };
    if at.screen() != Active::Service {
        if shown.view.is_some() {
            shown.view = None;
            shown.rows.clear();
        }
        return;
    }
    if shown.view.is_none() || world.is_changed() {
        let view = service_view(&world.0, &data.0);
        shown.rows = view
            .as_ref()
            .map_or_else(Vec::new, |v| model::offer_rows(v, &world.0, &data.0));
        shown.view = view;
        shown.synced = false;
    }
}

/// A new service starts with a blank form; anything the rules did clears the message; a
/// refusal while the panel is up is the message.
pub fn refusals(
    at: Where,
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    mut shown: ResMut<ServiceShown>,
) {
    for SimEvent(event) in events.read() {
        if matches!(event, Event::ServiceEntered { .. }) {
            shown.form = ServiceForm::default();
        } else if !shown.form.message.is_empty() {
            shown.form.message.clear();
        }
        shown.synced = false;
    }
    for CommandRefused(rejection) in refused.read() {
        if at.screen() == Active::Service {
            shown.form.message = rejection.to_string();
            shown.synced = false;
        }
    }
}

/// Spawn, despawn or spawn again, so a panel exists exactly while the party is inside a
/// service, and always for the current shape.
pub fn reconcile(
    mut commands: Commands,
    data: Option<Res<PackData>>,
    mut shown: ResMut<ServiceShown>,
    roots: Query<(Entity, &PanelRoot)>,
) {
    let shape = shown.view.as_ref().map(|v| model::shape(v, &shown.rows));
    let mut standing = false;
    for (entity, root) in &roots {
        if root.screen != UiScreen::Service {
            continue;
        }
        if Some(root.shape) == shape && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let (Some(view), Some(shape), false) = (&shown.view, shape, standing) {
        let name = data
            .as_ref()
            .map_or_else(String::new, |d| d.0.label("en", &view.name).to_owned());
        commands
            .spawn_scene(service_panel(view, &shown.rows, name))
            .insert(PanelRoot {
                screen: UiScreen::Service,
                shape,
            });
        shown.synced = false;
    }
}

/// Write the view into the panel: the money line, each row's label and note, the message, the
/// refused offers' buttons dimmed, and the amount unless the player is typing it.
pub fn sync(
    mut commands: Commands,
    focus: Res<InputFocus>,
    mut shown: ResMut<ServiceShown>,
    controls: Query<(Entity, &Control, Has<InteractionDisabled>)>,
    mut labels: Query<(&Shown, &mut Text)>,
) {
    let Some(view) = shown.view.as_ref() else {
        return;
    };
    let ours = controls
        .iter()
        .any(|(_, c, _)| matches!(c.0, UiId::Service(_)));
    if shown.synced || !ours {
        return;
    }
    for (shown_id, mut text) in &mut labels {
        let UiLabel::Service(id) = shown_id.0 else {
            continue;
        };
        let wanted = match id {
            ServiceLabelId::Money => model::money_line(view),
            ServiceLabelId::Row(index) => shown
                .rows
                .get(index)
                .map(|r| r.label.clone())
                .unwrap_or_default(),
            ServiceLabelId::Note(index) => shown
                .rows
                .get(index)
                .map(|r| r.note.clone())
                .unwrap_or_default(),
            ServiceLabelId::Message => shown.form.message.clone(),
        };
        set_text(&mut text, &wanted);
    }
    for (entity, control, disabled) in &controls {
        match control.0 {
            UiId::Service(ServicePanelId::Offer(index)) => {
                let refused = shown.rows.get(index).is_none_or(|r| r.refused);
                if refused && !disabled {
                    commands.entity(entity).insert(InteractionDisabled);
                } else if !refused && disabled {
                    commands.entity(entity).remove::<InteractionDisabled>();
                }
            }
            UiId::Service(ServicePanelId::Amount) if focus.get() != Some(entity) => {
                commands.trigger(UpdateNumberInput {
                    entity,
                    value: NumberInputValue::I32(
                        i32::try_from(shown.form.amount_gp).unwrap_or(i32::MAX),
                    ),
                });
            }
            _ => {}
        }
    }
    shown.synced = true;
}

/// Answer the panel's reports: each goes through `service_panel::apply`, and what it asks goes
/// through the gate, so leaving asks first.
pub(crate) fn reports(
    mut reports: MessageReader<UiReport>,
    mut shown: ResMut<ServiceShown>,
    mut gate: Gate,
) {
    for report in reports.read() {
        let UiId::Service(id) = report.id else {
            continue;
        };
        let ServiceShown { view, form, .. } = &mut *shown;
        let Some(view) = view.as_ref() else {
            continue;
        };
        let asked = model::apply(id, &report.payload, view, form);
        match asked {
            Some(ServiceAsk::Send(command)) => gate.send(Command::Service(command)),
            Some(ServiceAsk::Leave) => gate.send(Command::Service(ServiceCommand::Leave)),
            None => {}
        }
    }
}

/// Escape leaves (asking first), unless a text input or an open menu holds the keyboard.
pub(crate) fn escape_leaves(
    mut keys: MessageReader<KeyboardInput>,
    at: Where,
    focus: Res<InputFocus>,
    holders: Query<(), HoldsKeyboard>,
    roots: Query<&PanelRoot>,
    mut gate: Gate,
) {
    let escaped = keys
        .read()
        .any(|k| k.state == ButtonState::Pressed && k.logical_key == Key::Escape);
    let panel_up =
        at.screen() == Active::Service && roots.iter().any(|r| r.screen == UiScreen::Service);
    let held = focus.get().is_some_and(|entity| holders.contains(entity));
    if escaped && panel_up && !held {
        gate.send(Command::Service(ServiceCommand::Leave));
    }
}
