//! The tactics panel as a `bevy_ui` panel over the map (M7c step 7), opened from the sheet's
//! TACTICS button: a member menu and the reactions switch, then the member's declared
//! reactions (each with Edit and Remove), then the reaction being written: the action and the
//! trigger it answers, all or any of a list of conditions (each a kind with its menus and a
//! number), Add, Save, New, the last refusal and Close. Everything shown comes from
//! `omnis_sim::party_view` through `tactics_panel.rs`, which also answers every report. The
//! entity tree is spawned again when its shape changes (a row declared, a kind or the action
//! changed); captions, numbers, the switch and the refusal are rewritten in place.

use crate::feathers_ui::HoldsKeyboard;
use crate::menus::{Active, Screens, Where};
use crate::sim::{CommandRefused, PackData, PlayState, PlayerCommand, SimEvent, Views};
use crate::tactics_panel::{
    self as model, Choices, Field, Kind, TacticsAsk, TacticsForm, TacticsLabelId, TacticsPanelId,
};
use crate::ui_kit::{
    Control, PanelRoot, Shown, UiId, UiLabel, UiReport, UiScreen, Width, button, dropdown,
    message_line, panel as panel_root, row, row_label, scroll_column, set_text, title,
};
use bevy::feathers::containers::flex_spacer;
use bevy::feathers::controls::{
    ButtonVariant, FeathersCheckbox, FeathersNumberInput, NumberFormat, NumberInputValue,
    UpdateNumberInput,
};
use bevy::feathers::display::{label, label_dim};
use bevy::feathers::theme::ThemedText;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_rules::Trigger;
use omnis_sim::{Command, Event, PartyCommand, PartyView, TacticsView};

/// The party's tactics as the panel last read them, and what the panel keeps.
#[derive(Resource, Debug, Default)]
pub struct TacticsShown {
    /// The party, while the panel is up.
    pub party: Option<PartyView>,
    /// The monsters and conditions a condition may name.
    pub choices: Choices,
    /// The reaction being written and the last refusal.
    pub form: TacticsForm,
    /// Whether the widgets show all of the above.
    synced: bool,
}

impl TacticsShown {
    /// The shown member's tactics.
    #[must_use]
    pub fn view(&self) -> Option<&TacticsView> {
        Some(&self.party.as_ref()?.members.get(self.form.member)?.tactics)
    }
}

fn id(id: TacticsPanelId) -> UiId {
    UiId::Tactics(id)
}

fn shown(label: TacticsLabelId) -> UiLabel {
    UiLabel::Tactics(label)
}

// ------------------------------------------------------------------ scenes

/// A declared reaction: its line, Edit and Remove.
fn entry_row(at: usize) -> impl Scene {
    bsn! {
        row()
        Children [
            (
                Node { flex_grow: 1.0, flex_basis: px(0) }
                Children [ (label("") Shown({shown(TacticsLabelId::Entry(at))})) ]
            ),
            button(id(TacticsPanelId::Edit(at)), "Edit", ButtonVariant::Normal),
            button(id(TacticsPanelId::Remove(at)), "Remove", ButtonVariant::Normal),
        ]
    }
}

/// A field's menu in a condition row.
fn field_menu(at: usize, field: Field, options: Vec<String>) -> impl Scene {
    let options = options
        .into_iter()
        .enumerate()
        .map(|(i, text)| (id(TacticsPanelId::FieldPick(at, field, i)), text))
        .collect();
    dropdown(
        id(TacticsPanelId::Field(at, field)),
        shown(TacticsLabelId::Field(at, field)),
        options,
        Width::Px(150.0),
    )
}

/// A condition: its kind, the menus the kind needs, its number, and Drop.
fn condition_row(at: usize, kind: Kind, choices: &Choices) -> impl Scene {
    let kinds = Kind::ALL
        .iter()
        .enumerate()
        .map(|(k, kind)| (id(TacticsPanelId::KindPick(at, k)), kind.label().to_owned()))
        .collect();
    let fields: Vec<_> = kind
        .fields()
        .iter()
        .map(|field| field_menu(at, *field, choices.options(*field)))
        .collect();
    let number: Vec<_> = kind
        .number_max()
        .map(|_| {
            bsn! {
                @FeathersNumberInput { @number_format: {NumberFormat::I32} }
                Control({id(TacticsPanelId::Number(at))})
                Node { width: px(90), flex_grow: 0.0 }
            }
        })
        .into_iter()
        .collect();
    bsn! {
        row()
        Children [
            dropdown(
                id(TacticsPanelId::Kind(at)),
                shown(TacticsLabelId::Kind(at)),
                kinds,
                Width::Px(190.0),
            ),
            {fields},
            {number},
            flex_spacer(),
            button(id(TacticsPanelId::Drop(at)), "Drop", ButtonVariant::Normal),
        ]
    }
}

/// The whole panel.
fn tactics_panel(
    party: &PartyView,
    view: &TacticsView,
    form: &TacticsForm,
    choices: &Choices,
) -> impl Scene {
    let members = party
        .members
        .iter()
        .enumerate()
        .map(|(slot, m)| (id(TacticsPanelId::MemberPick(slot)), m.name.clone()))
        .collect();
    let actions = view
        .answers
        .iter()
        .enumerate()
        .map(|(at, a)| {
            (
                id(TacticsPanelId::ActionPick(at)),
                choices.action_name(&a.action, &a.name),
            )
        })
        .collect();
    let triggers = view
        .answers
        .get(form.action)
        .map_or(&[][..], |a| a.triggers.as_slice())
        .iter()
        .enumerate()
        .map(|(at, t): (usize, &Trigger)| {
            (
                id(TacticsPanelId::TriggerPick(at)),
                model::trigger_name(*t).to_owned(),
            )
        })
        .collect();
    let combine = ["all of these", "any of these"]
        .iter()
        .enumerate()
        .map(|(at, text)| (id(TacticsPanelId::CombinePick(at)), (*text).to_owned()))
        .collect();
    let entries: Vec<_> = (0..view.reactions.len()).map(entry_row).collect();
    let conditions: Vec<_> = form
        .conditions
        .iter()
        .enumerate()
        .map(|(at, draft)| condition_row(at, draft.kind, choices))
        .collect();
    let body = bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
        }
        Children [
            label_dim("Declared reactions, in the order they are considered"),
            {entries},
            (Text("") ThemedText Shown({shown(TacticsLabelId::Editing)})),
            (
                row()
                Children [
                    row_label("Action"),
                    dropdown(
                        id(TacticsPanelId::Action),
                        shown(TacticsLabelId::Action),
                        actions,
                        Width::Px(220.0),
                    ),
                    label("on"),
                    dropdown(
                        id(TacticsPanelId::Trigger),
                        shown(TacticsLabelId::Trigger),
                        triggers,
                        Width::Px(220.0),
                    ),
                ]
            ),
            (
                row()
                Children [
                    row_label("When"),
                    dropdown(
                        id(TacticsPanelId::Combine),
                        shown(TacticsLabelId::Combine),
                        combine,
                        Width::Px(160.0),
                    ),
                    label_dim("hold (none: always)"),
                    flex_spacer(),
                    button(id(TacticsPanelId::Add), "Add condition", ButtonVariant::Normal),
                ]
            ),
            {conditions},
        ]
    };
    bsn! {
        panel_root()
        Children [
            (
                row()
                Children [
                    title("Tactics"),
                    dropdown(
                        id(TacticsPanelId::Member),
                        shown(TacticsLabelId::Member),
                        members,
                        Width::Px(220.0),
                    ),
                    flex_spacer(),
                    (
                        @FeathersCheckbox {
                            @caption: bsn! { Text("Reactions on") ThemedText }
                        }
                        Control({id(TacticsPanelId::Reactions)})
                    ),
                ]
            ),
            scroll_column(body),
            message_line(shown(TacticsLabelId::Message)),
            (
                row()
                Children [
                    button(id(TacticsPanelId::New), "New", ButtonVariant::Normal),
                    button(id(TacticsPanelId::Save), "Save", ButtonVariant::Normal),
                    flex_spacer(),
                    button(id(TacticsPanelId::Close), "Close", ButtonVariant::Primary),
                ]
            ),
        ]
    }
}

// ------------------------------------------------------------------ systems

/// Read the party again whenever the world changed while the panel is up, starting on the
/// sheet's member; forget it all when the panel is down.
pub fn look(
    at: Where,
    views: Option<Res<Views>>,
    data: Option<Res<PackData>>,
    screens: Res<Screens>,
    mut shown: ResMut<TacticsShown>,
) {
    let (Some(views), Some(data)) = (views, data) else {
        return;
    };
    if at.screen() != Active::Tactics {
        if shown.party.is_some() {
            *shown = TacticsShown::default();
        }
        return;
    }
    if shown.party.is_none() {
        shown.form = TacticsForm::new(screens.sheet.member);
        shown.choices = Choices::from_data(&data.0);
    }
    if shown.party.is_none() || views.is_changed() {
        let party = views.party.clone();
        if shown.form.member >= party.members.len() {
            shown.form = TacticsForm::new(0);
        }
        shown.party = Some(party);
        shown.synced = false;
    }
}

/// A declared or removed reaction starts the form afresh; the switch needs only a redraw; a
/// refusal while the panel is up is the message.
pub fn refusals(
    at: Where,
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    mut shown: ResMut<TacticsShown>,
) {
    for SimEvent(event) in events.read() {
        if matches!(event, Event::TacticsChanged { .. }) {
            shown.form = TacticsForm::new(shown.form.member);
        }
        shown.synced = false;
    }
    for CommandRefused(rejection) in refused.read() {
        if at.screen() == Active::Tactics {
            shown.form.message = rejection.to_string();
            shown.synced = false;
        }
    }
}

/// Spawn, despawn or spawn again, so a panel exists exactly while tactics are open, and always
/// for the current shape.
pub fn reconcile(
    mut commands: Commands,
    mut shown: ResMut<TacticsShown>,
    roots: Query<(Entity, &PanelRoot)>,
) {
    let members = shown.party.as_ref().map_or(0, |p| p.members.len());
    let shape = shown
        .view()
        .map(|view| model::shape(view, &shown.form, members));
    let mut standing = false;
    for (entity, root) in &roots {
        if root.screen != UiScreen::Tactics {
            continue;
        }
        if Some(root.shape) == shape && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let (Some(party), Some(view), Some(shape), false) =
        (&shown.party, shown.view(), shape, standing)
    {
        commands
            .spawn_scene(tactics_panel(party, view, &shown.form, &shown.choices))
            .insert(PanelRoot {
                screen: UiScreen::Tactics,
                shape,
            });
        shown.synced = false;
    }
}

/// The text a label shows.
fn label_text(label: TacticsLabelId, shown: &TacticsShown, view: &TacticsView) -> String {
    let form = &shown.form;
    let draft = |at: usize| form.conditions.get(at);
    match label {
        TacticsLabelId::Member => shown
            .party
            .as_ref()
            .and_then(|p| p.members.get(form.member))
            .map_or_else(String::new, |m| m.name.clone()),
        TacticsLabelId::Entry(at) => model::entry_line(view, at, &shown.choices),
        TacticsLabelId::Editing => model::editing_line(form),
        TacticsLabelId::Action => model::action_caption(view, form, &shown.choices),
        TacticsLabelId::Trigger => model::trigger_caption(view, form),
        TacticsLabelId::Combine => model::combine_caption(form).to_owned(),
        TacticsLabelId::Kind(at) => draft(at).map_or("", |d| d.kind.label()).to_owned(),
        TacticsLabelId::Field(at, field) => {
            draft(at).map_or_else(String::new, |d| d.caption(field, &shown.choices))
        }
        TacticsLabelId::Message => form.message.clone(),
    }
}

/// Write the view into the panel: the captions and lines, the switch, the numbers (not while
/// one is being typed in), and Save and Add dimmed when they would do nothing.
pub fn sync(
    mut commands: Commands,
    mut shown: ResMut<TacticsShown>,
    focus: Res<InputFocus>,
    controls: Query<(Entity, &Control, Has<InteractionDisabled>, Has<Checked>)>,
    mut labels: Query<(&Shown, &mut Text)>,
) {
    let Some(view) = shown.view() else {
        return;
    };
    let ours = controls
        .iter()
        .any(|(_, c, _, _)| matches!(c.0, UiId::Tactics(_)));
    if shown.synced || !ours {
        return;
    }
    for (label, mut text) in &mut labels {
        if let UiLabel::Tactics(label) = label.0 {
            set_text(&mut text, &label_text(label, &shown, view));
        }
    }
    for (entity, control, disabled, checked) in &controls {
        let UiId::Tactics(control) = control.0 else {
            continue;
        };
        let dim = match control {
            TacticsPanelId::Reactions => {
                if view.reactions_on && !checked {
                    commands.entity(entity).insert(Checked);
                } else if !view.reactions_on && checked {
                    commands.entity(entity).remove::<Checked>();
                }
                continue;
            }
            TacticsPanelId::Number(at) => {
                if focus.get() != Some(entity)
                    && let Some(draft) = shown.form.conditions.get(at)
                {
                    commands.trigger(UpdateNumberInput {
                        entity,
                        value: NumberInputValue::I32(i32::from(draft.n)),
                    });
                }
                continue;
            }
            TacticsPanelId::Save => !model::can_save(view, &shown.form, &shown.choices),
            TacticsPanelId::Add => !model::can_add(&shown.form),
            _ => continue,
        };
        if dim && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !dim && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    shown.synced = true;
}

/// Answer the panel's reports: each goes through `tactics_panel::apply`; a command goes to the
/// simulation, Close back to the sheet.
pub(crate) fn reports(
    mut reports: MessageReader<UiReport>,
    mut shown: ResMut<TacticsShown>,
    mut out: MessageWriter<PlayerCommand>,
    mut next: ResMut<NextState<PlayState>>,
) {
    for report in reports.read() {
        let UiId::Tactics(control) = report.id else {
            continue;
        };
        let ids: Vec<CharacterId> = shown
            .party
            .as_ref()
            .map_or_else(Vec::new, |p| p.members.iter().map(|m| m.id).collect());
        let Some(view) = shown.view().cloned() else {
            continue;
        };
        let TacticsShown {
            choices,
            form,
            synced,
            ..
        } = &mut *shown;
        *synced = false;
        form.message.clear();
        match model::apply(control, &report.payload, &view, &ids, choices, form) {
            Some(TacticsAsk::Send(command)) => {
                out.write(PlayerCommand(Command::Party(PartyCommand::Tactics(
                    command,
                ))));
            }
            Some(TacticsAsk::Close) => next.set(PlayState::Sheet),
            None => {}
        }
    }
}

/// Escape goes back to the sheet, unless a control holds the keyboard.
pub(crate) fn escape_closes(
    mut keys: MessageReader<KeyboardInput>,
    at: Where,
    focus: Res<InputFocus>,
    holders: Query<(), HoldsKeyboard>,
    mut next: ResMut<NextState<PlayState>>,
) {
    let escaped = keys
        .read()
        .any(|k| k.state == ButtonState::Pressed && k.logical_key == Key::Escape);
    let held = focus.get().is_some_and(|entity| holders.contains(entity));
    if escaped && at.screen() == Active::Tactics && !held {
        next.set(PlayState::Sheet);
    }
}
