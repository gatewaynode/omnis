//! `FeathersUiPlugin`: the interface toolkit of every build (PRD D26, ARCHITECTURE.md A11). Bevy's
//! own widgets and TrueType text, drawn in window space above the canvas sprite; party creation
//! is the first screen, and the canvas screens move over one at a time.
//!
//! `DefaultPlugins` already brings `bevy_ui`, its widgets, input focus and picking; this plugin adds Feathers itself, its dark theme, and the two things the
//! canvas pipeline would otherwise get wrong: which camera the interface belongs to, and the
//! global nearest sampler on Feathers' icons.

use crate::cursor::UiSet;
use crate::feathers_confirm as confirm;
use crate::feathers_creation::{self as creation, Synced};
use crate::feathers_fonts::{self as typefaces, PanelFonts};
use crate::feathers_service::{self as service, ServiceShown};
use crate::menu::CreationAction;
use crate::menus::{Active, CreationAsk, CreationFlow, Where};
use crate::pixel::OuterCamera;
use crate::ui::UiPointerCapture;
use crate::ui_kit::{self as kit, PanelRoot};
use bevy::feathers::FeathersPlugins;
use bevy::feathers::constants::icons;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::picking::PickingSystems;
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui_widgets::{MenuItem, MenuPopup};

/// The Feathers interface plugin.
pub struct FeathersUiPlugin;

impl Plugin for FeathersUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<SmoothIcons>()
            .init_resource::<Synced>()
            .init_resource::<ServiceShown>()
            .init_resource::<kit::ScaleChoice>()
            .init_resource::<kit::FontChoice>()
            .add_message::<kit::UiReport>()
            .init_resource::<PanelFonts>()
            .init_resource::<UiPointerCapture>()
            .add_observer(kit::on_activate)
            .add_observer(kit::on_slide)
            .add_observer(kit::on_number)
            .add_observer(kit::on_flag)
            .add_observer(kit::on_text)
            .add_systems(Startup, (smooth_icons, typefaces::register))
            .add_systems(PreUpdate, capture_pointer.after(PickingSystems::Hover))
            .add_systems(Update, claim_camera)
            .add_systems(Update, escape_abandons.in_set(UiSet::Dispatch))
            // A report is a click or a key: it is answered where those are, ahead of the
            // creation flow, so what a button asks is done in the frame it was pressed.
            .add_systems(
                Update,
                creation::reports
                    .in_set(UiSet::Dispatch)
                    .before(CreationFlow),
            )
            .add_systems(Update, confirm::reports.in_set(UiSet::Dispatch))
            .add_systems(
                Update,
                (service::reports, service::escape_leaves).in_set(UiSet::Dispatch),
            )
            .add_systems(
                Update,
                (
                    creation::reconcile,
                    confirm::reconcile,
                    service::look,
                    service::refusals,
                    service::reconcile,
                    kit::scale,
                    kit::place,
                    creation::show_scale,
                    creation::sync,
                    service::sync,
                    typefaces::wear,
                )
                    .chain()
                    .in_set(UiSet::Model),
            );
    }
}

fn capture_pointer(
    hover: Res<HoverMap>,
    nodes: Query<(), With<Node>>,
    mut capture: ResMut<UiPointerCapture>,
) {
    let over = hover
        .values()
        .any(|hits| hits.keys().any(|entity| nodes.contains(*entity)));
    if capture.0 != over {
        capture.0 = over;
    }
}

/// What keeps Escape for itself while it has the focus: a text input, an open menu.
pub(crate) type HoldsKeyboard = Or<(With<EditableText>, With<MenuItem>, With<MenuPopup>)>;

/// Escape abandons the new game, unless a text input or an open menu
/// holds the keyboard (there it means "leave this field").
fn escape_abandons(
    mut keys: MessageReader<KeyboardInput>,
    at: Where,
    focus: Res<InputFocus>,
    holders: Query<(), HoldsKeyboard>,
    roots: Query<(), With<PanelRoot>>,
    mut asks: MessageWriter<CreationAsk>,
) {
    let escaped = keys
        .read()
        .any(|k| k.state == ButtonState::Pressed && k.logical_key == Key::Escape);
    let panel_up = at.screen() == Active::CreateParty && !roots.is_empty();
    let held = focus.get().is_some_and(|entity| holders.contains(entity));
    if escaped && panel_up && !held {
        asks.write(CreationAsk(CreationAction::Back));
    }
}

/// Feathers' icons, held so the first load (whose sampler wins) is never dropped.
#[derive(Resource, Default)]
pub struct SmoothIcons(pub Vec<Handle<Image>>);

/// Feathers draws its 24-pixel icons at 14: under the app's global nearest sampler they would
/// be jagged, so they are loaded first with a linear one.
fn smooth_icons(server: Res<AssetServer>, mut held: ResMut<SmoothIcons>) {
    for path in [icons::CHEVRON_DOWN, icons::CHEVRON_RIGHT, icons::X] {
        held.0.push(
            server
                .load_builder()
                .with_settings(|s: &mut ImageLoaderSettings| s.sampler = ImageSampler::linear())
                .load(path),
        );
    }
}

/// The interface belongs to the window's camera, never to the canvas's. `bevy_ui` already
/// prefers a window camera; the marker makes it explicit. A headless app has no outer camera.
fn claim_camera(
    mut commands: Commands,
    cameras: Query<Entity, (With<OuterCamera>, Without<IsDefaultUiCamera>)>,
) {
    for camera in &cameras {
        commands.entity(camera).insert(IsDefaultUiCamera);
    }
}
