//! `UiPlugin`: hover and clicks over the composed frame, the member selection, the message
//! line, and the frame itself, composed every frame from the models and uploaded into a
//! canvas sprite above the viewport. Headless-capable: without a render stack the frame is
//! still composed as a resource and nothing is uploaded.

use crate::band::MemberRow;
use crate::canvas::Layout;
use crate::combat_menu::{FightView, fight_view};
use crate::combat_text::{Names, batch_lines};
use crate::cursor::{self, Pointer, UiSet};
use crate::debug_menu::{DebugView, debug_view};
use crate::layout::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::menus::{Active, Screens, Where};
use crate::panels::{Hud, Message};
use crate::pixel::PIXEL_LAYER;
use crate::screen::{self, Menu, View};
use crate::sim::{AppState, CommandRefused, Notice, PackData, SimEvent, SimWorld};
use crate::viewport::canvas_to_world;
use crate::widget::{self, Frame, Hit, PadState, WidgetId};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Event, World};

/// The canvas sprite the frame is uploaded into.
#[derive(Resource, Debug, Clone)]
pub struct UiImage(pub Handle<Image>);

/// Z of the UI sprite: above the viewport, the minimap, and the overlay automap.
const UI_Z: f32 = 50.0;

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

/// The names events refer to, kept fresh by the combat plugin.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct EventNames(pub Names);

/// The event log, oldest first, with the roll math; the band shows its tail.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct RollLog(pub Vec<String>);

impl RollLog {
    /// How many lines are kept.
    pub const CAPACITY: usize = 32;

    /// Append a line, dropping the oldest past the capacity.
    pub fn push(&mut self, line: String) {
        self.0.push(line);
        if self.0.len() > Self::CAPACITY {
            self.0.remove(0);
        }
    }

    /// Forget every line.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

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
/// Help before a fight.
pub const HELP_ENCOUNTER: &str = "Left/Right choose  Enter ok  Esc menu";
/// Help in a fight.
pub const HELP_COMBAT: &str = "Up/Down act  Left/Right target  C cast  Enter ok  Esc menu";
/// Help after a wipe.
pub const HELP_DEFEAT: &str = "Up/Down select  Enter ok";
/// Help on the debug menu.
pub const HELP_DEBUG: &str = "Arrows edit  Tab field  Enter act  Esc close";

/// The UI plugin.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<UiClick>()
            .init_resource::<UiFrame>()
            .init_resource::<Selected>()
            .init_resource::<MessageLine>()
            .init_resource::<EventNames>()
            .init_resource::<RollLog>()
            .add_systems(
                Update,
                hit.in_set(UiSet::Cursor).after(cursor::track_pointer),
            )
            .add_systems(Update, select_member.in_set(UiSet::Dispatch))
            .add_systems(Update, message_line.in_set(UiSet::Model))
            .add_systems(
                Update,
                (build_frame, upload.run_if(resource_changed::<UiFrame>))
                    .chain()
                    .in_set(UiSet::Draw),
            )
            .add_systems(Startup, make_sprite)
            .add_systems(
                OnExit(AppState::Playing),
                |mut selected: ResMut<Selected>| {
                    selected.0 = None;
                },
            );
    }
}

/// A one-line description of an exploration event, or `None` for events the message line
/// skips; combat events read through `combat_text::batch_lines`.
#[must_use]
pub fn event_text(event: &Event) -> Option<String> {
    Some(match event {
        Event::Blocked { reason } => format!("Blocked: {reason:?}"),
        Event::Door { open: true, .. } => "The door opens.".into(),
        Event::Door { open: false, .. } => "The door closes.".into(),
        Event::Message { key } => key.text_key().to_owned(),
        Event::Moved { from, to } if from.map != to.map => "You pass through.".into(),
        Event::TimeAdvanced {
            day_rolled: true, ..
        } => "A new day.".into(),
        Event::Dev { command } => crate::debug_menu::describe(command),
        _ => return None,
    })
}

/// The sprite the UI frame is uploaded into.
#[derive(Component)]
struct UiSprite;

/// The transparent canvas-sized image and its sprite; skipped without a render stack.
fn make_sprite(mut commands: Commands, images: Option<ResMut<Assets<Image>>>) {
    let Some(mut images) = images else {
        return;
    };
    let image = Image::new(
        Extent3d {
            width: CANVAS_WIDTH,
            height: CANVAS_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (CANVAS_WIDTH * CANVAS_HEIGHT * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let handle = images.add(image);
    commands.spawn((
        Sprite::from_image(handle.clone()),
        Anchor::TOP_LEFT,
        Transform::from_translation(canvas_to_world(CANVAS_WIDTH, 0, 0, UI_Z)),
        PIXEL_LAYER,
        UiSprite,
    ));
    commands.insert_resource(UiImage(handle));
}

/// Copy the frame's pixels into the sprite's image; runs only when the frame changed. A
/// frame of a new width resizes the image first and moves the sprite to the new corner.
fn upload(
    ui: Res<UiFrame>,
    target: Option<Res<UiImage>>,
    images: Option<ResMut<Assets<Image>>>,
    sprite: Option<Single<&mut Transform, With<UiSprite>>>,
) {
    let (Some(target), Some(mut images)) = (target, images) else {
        return;
    };
    let Some(mut image) = images.get_mut(&target.0) else {
        return;
    };
    let (width, height) = (ui.frame.raster.width, ui.frame.raster.height);
    if (image.width(), image.height()) != (width, height) {
        image.resize(Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        });
        if let Some(mut sprite) = sprite {
            sprite.translation = canvas_to_world(width, 0, 0, UI_Z);
        }
    }
    image.data = Some(ui.frame.raster.rgba.clone());
}

fn hit(pointer: Res<Pointer>, mut ui: ResMut<UiFrame>, mut clicks: MessageWriter<UiClick>) {
    let found = pointer
        .canvas
        .and_then(|(x, y)| widget::hit(&ui.frame.widgets, x, y));
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

pub(crate) fn message_line(
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    notice: Res<Notice>,
    names: Res<EventNames>,
    at: Where,
    mut line: ResMut<MessageLine>,
) {
    let mut next = None;
    let mut moved = false;
    let batch: Vec<Event> = events.read().map(|e| e.0.clone()).collect();
    for event in &batch {
        moved |= matches!(event, Event::Moved { .. });
        if let Some(text) = event_text(event) {
            next = Some(Message { text, alert: false });
        }
    }
    if let Some(last) = batch_lines(&batch, &names.0).pop() {
        next = Some(Message {
            text: last.long,
            alert: false,
        });
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
            level: m.level,
            hp: m.hp,
            hp_max: m.hp_max,
            sp: m.spell_points,
            ac: omnis_sim::omnis_rules::armor_class(m, data),
            condition: m
                .conditions
                .first()
                .and_then(|c| data.conditions.get(c))
                .map(|c| data.label("en", &c.name).to_owned())
                .or_else(|| {
                    m.effects
                        .first()
                        .and_then(|e| data.spells.get(&e.source))
                        .map(|s| data.label("en", &s.name).to_owned())
                }),
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

/// The message a screen's model reports, when it has one.
fn model_message(active: Active, screens: &Screens) -> Option<Message> {
    let text = match active {
        Active::CreateParty => &screens.creation.message,
        Active::Encounter => &screens.encounter.message,
        Active::Combat => &screens.combat.message,
        Active::Debug => &screens.debug.message,
        _ => return None,
    };
    (!text.is_empty()).then(|| Message {
        text: text.clone(),
        alert: true,
    })
}

/// The menu and help line for the active screen.
fn menu_for<'a>(
    active: Active,
    screens: &'a Screens,
    world: Option<&World>,
    fight: Option<&'a FightView>,
    debug: Option<&'a DebugView>,
    members: usize,
    log: &'a [String],
) -> (Menu<'a>, &'static str) {
    if let (Active::Debug, Some(view)) = (active, debug) {
        return (
            Menu::Debug {
                menu: &screens.debug,
                view,
            },
            HELP_DEBUG,
        );
    }
    match (active, fight) {
        (Active::Title, _) => (Menu::Title(&screens.title), HELP_TITLE),
        (Active::NewGame, _) => (Menu::NewGame(&screens.new_game), HELP_NEW_GAME),
        (Active::CreateParty, _) => (
            Menu::Creation {
                form: &screens.creation,
                catalog: &screens.catalog,
                members,
            },
            HELP_CREATION,
        ),
        (Active::Paused, _) => (
            Menu::Pause {
                pause: &screens.pause,
                settings: world.map_or_else(Default::default, |w| w.settings),
                seed: world.map_or(0, |w| w.seed),
            },
            HELP_PAUSE,
        ),
        (Active::Encounter, Some(view)) => (
            Menu::Encounter {
                menu: &screens.encounter,
                view,
            },
            HELP_ENCOUNTER,
        ),
        (Active::Combat, Some(view)) => (
            Menu::Combat {
                menu: &screens.combat,
                view,
            },
            HELP_COMBAT,
        ),
        (Active::Defeat, _) => (
            Menu::Defeat {
                menu: &screens.defeat,
                log,
            },
            HELP_DEFEAT,
        ),
        (Active::None | Active::Encounter | Active::Combat | Active::Debug, _) => {
            (Menu::None, HELP_EXPLORE)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_frame(
    at: Where,
    screens: Res<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    selected: Res<Selected>,
    line: Res<MessageLine>,
    log: Res<RollLog>,
    layout: Res<Layout>,
    mut ui: ResMut<UiFrame>,
    mut scratch: Local<Frame>,
) {
    let active = at.screen();
    let loaded = world.as_ref().zip(data.as_ref());
    let members = loaded.map_or_else(Vec::new, |(w, d)| member_rows(&w.0, &d.0));
    let hud = loaded.map(|(w, d)| hud_text(&w.0, &d.0));
    let fight = loaded.and_then(|(w, d)| fight_view(&w.0, &d.0));
    let debug = (active == Active::Debug)
        .then(|| loaded.map(|(w, d)| debug_view(&w.0, &d.0)))
        .flatten();
    let front_row = data
        .as_ref()
        .map_or(3, |d| omnis_sim::party::front_row(&d.0));
    let model_message = model_message(active, &screens);
    let (menu, help) = menu_for(
        active,
        &screens,
        world.as_ref().map(|w| &w.0),
        fight.as_ref(),
        debug.as_ref(),
        members.len(),
        &log.0,
    );
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
        acting: fight
            .as_ref()
            .and_then(|f| f.own)
            .filter(|s| *s < members.len()),
        log: &log.0,
        pad,
        message: model_message.as_ref().unwrap_or(&line.0),
        help,
    };
    // Paint into the scratch buffer; the resource changes only when the pixels or widgets do,
    // so the upload and everything gated on the frame run only then.
    screen::compose_into(&mut scratch, &layout, &view, ui.hover, ui.pressed);
    if ui.frame != *scratch {
        ui.frame.clone_from(&scratch);
    }
}
