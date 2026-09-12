//! `UiPlugin`: hover and clicks over the composed frame, the member selection, the message
//! line, and the frame itself, composed every frame from the models. Headless-capable: the
//! frame is a resource that `pixel` shows as a sprite when there is a render stack.

use crate::cursor::{self, Pointer, UiSet};
use crate::hud::event_text;
use crate::menus::{Active, Screens, Where};
use crate::panels::{Hud, MemberRow, Message};
use crate::screen::{self, Frame, Hit, Menu, PadState, View, WidgetId};
use crate::sim::{AppState, CommandRefused, Notice, PackData, SimEvent, SimWorld};
use bevy::prelude::*;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Event, World};

/// The frame the last `Update` composed, with what the pointer is over.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiFrame {
    /// The pixels and widgets.
    pub frame: Frame,
    /// The widget under the pointer.
    pub hover: Option<WidgetId>,
    /// The pad button held down.
    pub pressed: Option<WidgetId>,
}

/// A click on a widget of the previous frame.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiClick(pub Hit);

/// The party slot the mouse selected.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Selected(pub Option<usize>);

/// The band's message line.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageLine(pub Message);

/// Help while exploring.
pub const HELP_EXPLORE: &str = "Arrows/pad move  M map  F5 save  F9 load  Esc menu";
/// Help on the title.
pub const HELP_TITLE: &str = "Arrows or click  Enter ok";
/// Help on the new game form.
pub const HELP_NEW_GAME: &str = "Arrows or click  Enter ok  Esc back";
/// Help while creating.
pub const HELP_CREATION: &str = "Arrows or click  Enter ok  Esc abandon";
/// Help while paused.
pub const HELP_PAUSE: &str = "Arrows or click  Enter ok  Esc resume";

/// The UI plugin.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<UiClick>()
            .init_resource::<UiFrame>()
            .init_resource::<Selected>()
            .init_resource::<MessageLine>()
            .add_systems(
                Update,
                hit.in_set(UiSet::Cursor).after(cursor::track_pointer),
            )
            .add_systems(Update, select_member.in_set(UiSet::Dispatch))
            .add_systems(Update, message_line.in_set(UiSet::Model))
            .add_systems(Update, build_frame.in_set(UiSet::Draw))
            .add_systems(
                OnExit(AppState::Playing),
                |mut selected: ResMut<Selected>| {
                    selected.0 = None;
                },
            );
    }
}

fn hit(pointer: Res<Pointer>, mut ui: ResMut<UiFrame>, mut clicks: MessageWriter<UiClick>) {
    let found = pointer
        .canvas
        .and_then(|(x, y)| screen::hit(&ui.frame.widgets, x, y));
    let hover = found.map(|h| h.id);
    let pressed = if pointer.held {
        hover.filter(|id| matches!(id, WidgetId::Pad(_)))
    } else {
        None
    };
    if ui.hover != hover || ui.pressed != pressed {
        ui.hover = hover;
        ui.pressed = pressed;
    }
    if pointer.clicked
        && let Some(found) = found
    {
        clicks.write(UiClick(found));
    }
}

fn select_member(mut clicks: MessageReader<UiClick>, mut selected: ResMut<Selected>) {
    for UiClick(hit) in clicks.read() {
        if let WidgetId::Member(slot) = hit.id {
            selected.0 = if selected.0 == Some(slot) {
                None
            } else {
                Some(slot)
            };
        }
    }
}

fn message_line(
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    notice: Res<Notice>,
    at: Where,
    mut line: ResMut<MessageLine>,
) {
    let mut next = None;
    let mut moved = false;
    for SimEvent(event) in events.read() {
        moved |= matches!(event, Event::Moved { .. });
        if let Some(text) = event_text(event) {
            next = Some(Message { text, alert: false });
        }
    }
    if moved && next.is_none() {
        next = Some(Message::default());
    }
    for CommandRefused(rejection) in refused.read() {
        // While creating, the form reports its own rejections.
        if at.screen() != Active::CreateParty {
            next = Some(Message {
                text: rejection.to_string(),
                alert: true,
            });
        }
    }
    if notice.is_changed() && !notice.0.is_empty() {
        let text = notice.0.clone();
        let alert = text.contains("fail") || text.contains("forbid");
        next = Some(Message { text, alert });
    }
    if let Some(next) = next
        && line.0 != next
    {
        line.0 = next;
    }
}

/// The party as band rows.
#[must_use]
pub fn member_rows(world: &World, data: &Data) -> Vec<MemberRow> {
    world
        .party
        .members
        .iter()
        .map(|m| MemberRow {
            name: m.name.clone(),
            class: data
                .classes
                .get(&m.class)
                .map_or("?", |c| data.label("en", &c.name))
                .to_owned(),
            hp: m.hp,
            hp_max: m.hp_max,
            sp: m.spell_points,
            condition: m
                .conditions
                .first()
                .and_then(|c| data.conditions.get(c))
                .and_then(|c| data.label("en", &c.name).chars().next()),
        })
        .collect()
}

/// The location lines for the world.
#[must_use]
pub fn hud_text(world: &World, data: &Data) -> Hud {
    let p = world.position;
    let map = data
        .maps
        .get(&p.map)
        .map_or("?", |m| data.text("en", m.name));
    Hud::new(
        map,
        i32::from(p.x),
        i32::from(p.y),
        &p.facing.to_string(),
        world.party_clock().elapsed,
    )
}

fn build_frame(
    at: Where,
    screens: Res<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    selected: Res<Selected>,
    line: Res<MessageLine>,
    mut ui: ResMut<UiFrame>,
) {
    let active = at.screen();
    let loaded = world.as_ref().zip(data.as_ref());
    let members = loaded.map_or_else(Vec::new, |(w, d)| member_rows(&w.0, &d.0));
    let hud = loaded.map(|(w, d)| hud_text(&w.0, &d.0));
    let front_row = data
        .as_ref()
        .map_or(3, |d| omnis_sim::party::front_row(&d.0));
    let creation_message = (active == Active::CreateParty && !screens.creation.message.is_empty())
        .then(|| Message {
            text: screens.creation.message.clone(),
            alert: true,
        });
    let (menu, help) = match active {
        Active::Title => (Menu::Title(&screens.title), HELP_TITLE),
        Active::NewGame => (Menu::NewGame(&screens.new_game), HELP_NEW_GAME),
        Active::CreateParty => (
            Menu::Creation {
                form: &screens.creation,
                catalog: &screens.catalog,
                members: members.len(),
            },
            HELP_CREATION,
        ),
        Active::Paused => (
            Menu::Pause {
                pause: &screens.pause,
                settings: world
                    .as_ref()
                    .map_or_else(Default::default, |w| w.0.settings),
                seed: world.as_ref().map_or(0, |w| w.0.seed),
            },
            HELP_PAUSE,
        ),
        Active::None => (Menu::None, HELP_EXPLORE),
    };
    let pad = if world.is_none() {
        PadState::Hidden
    } else if at.exploring() {
        PadState::Enabled
    } else {
        PadState::Disabled
    };
    let view = View {
        menu,
        hud: hud.as_ref(),
        members: &members,
        front_row,
        selected: selected.0.filter(|s| *s < members.len()),
        creating: active == Active::CreateParty,
        pad,
        message: creation_message.as_ref().unwrap_or(&line.0),
        help,
    };
    ui.frame = screen::compose(&view, ui.hover, ui.pressed);
}
