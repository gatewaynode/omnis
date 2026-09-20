//! The Feathers experiment headless (feature `feathers`): Bevy's widgets build, lay out and
//! answer with no window and no GPU, beside the whole canvas app.
#![cfg(feature = "feathers")]

mod common;

use bevy::feathers::controls::{FeathersButton, FeathersTextInput, FeathersTextInputContainer};
use bevy::feathers::theme::ThemedText;
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit, TextEditChange};
use bevy::ui_widgets::Activate;
use common::feathers_app;

/// Which probe control an entity is.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, FromTemplate)]
enum Probe {
    #[default]
    Button,
    Name,
}

/// What the probes reported.
#[derive(Resource, Default)]
struct Heard {
    activated: usize,
    edits: usize,
}

fn probes() -> impl Scene {
    bsn! {
        Node {
            width: px(400),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
        }
        Children [
            (
                @FeathersButton { @caption: bsn! { Text("Add member") ThemedText } }
                Probe::Button
            ),
            (
                @FeathersTextInputContainer
                Children [
                    (
                        @FeathersTextInput { @max_characters: 24usize }
                        Probe::Name
                    )
                ]
            ),
        ]
    }
}

fn probe(app: &mut App, which: Probe) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &Probe)>();
    query
        .iter(app.world())
        .find_map(|(entity, probe)| (*probe == which).then_some(entity))
        .unwrap_or_else(|| panic!("{which:?} was not spawned"))
}

fn probe_app() -> App {
    let mut app = feathers_app("feathers-smoke.ron", false);
    app.init_resource::<Heard>()
        .add_observer(|_: On<Activate>, mut heard: ResMut<Heard>| heard.activated += 1)
        .add_observer(|_: On<TextEditChange>, mut heard: ResMut<Heard>| heard.edits += 1);
    app.world_mut()
        .spawn_scene(probes())
        .expect("the probe scene spawns");
    for _ in 0..3 {
        app.update();
    }
    app
}

#[test]
fn feathers_builds_and_lays_out_with_no_window_and_no_gpu() {
    let mut app = probe_app();
    for which in [Probe::Button, Probe::Name] {
        let entity = probe(&mut app, which);
        let node = app.world().get::<ComputedNode>(entity).expect("a node");
        assert!(
            node.size().x > 0.0 && node.size().y > 0.0,
            "{which:?} has no size: {:?}",
            node.size()
        );
        assert!(node.size().x <= 1280.0 && node.size().y <= 720.0);
    }
    // Feathers' own row height at the interface scale, so the theme and its sizes are in force.
    let button = probe(&mut app, Probe::Button);
    let height = app.world().get::<ComputedNode>(button).unwrap().size().y;
    let wanted = 24.0 * app.world().resource::<UiScale>().0;
    assert!((height - wanted).abs() < 0.5, "button height {height}");
}

#[test]
fn a_button_activates_and_a_text_input_reads_and_writes() {
    let mut app = probe_app();
    let button = probe(&mut app, Probe::Button);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert_eq!(app.world().resource::<Heard>().activated, 1);

    let name = probe(&mut app, Probe::Name);
    app.world_mut()
        .get_mut::<EditableText>(name)
        .expect("an editable text")
        .queue_edit(TextEdit::Insert("Durin".into()));
    for _ in 0..3 {
        app.update();
    }
    let text = app.world().get::<EditableText>(name).unwrap();
    assert_eq!(text.value().to_string(), "Durin");
    assert!(
        app.world().resource::<Heard>().edits >= 1,
        "no change event"
    );
}

// ------------------------------------------------------------------ the creation panel

use bevy::ui_widgets::ValueChange;
use common::{play_state, start_new_game_by_mouse, world};
use omnis_app::creation_panel::{Choice, PanelId};
use omnis_app::feathers_creation::{Control, PanelRoot};
use omnis_app::menus::Screens;
use omnis_app::sim::PlayState;

/// A new game by the canvas menus, arriving on party creation in the Feathers skin.
fn creating(save: &str) -> App {
    let mut app = feathers_app(save, false);
    start_new_game_by_mouse(&mut app);
    for _ in 0..3 {
        app.update();
    }
    app
}

fn control(app: &mut App, id: PanelId) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &Control)>();
    query
        .iter(app.world())
        .find_map(|(entity, control)| (control.0 == id).then_some(entity))
        .unwrap_or_else(|| panic!("{id:?} is not on the panel"))
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

fn activate(app: &mut App, id: PanelId) {
    let entity = control(app, id);
    app.world_mut().trigger(Activate { entity });
    settle(app);
}

fn change<T: Send + Sync + 'static + Clone>(app: &mut App, id: PanelId, value: T) {
    let source = control(app, id);
    app.world_mut().trigger(ValueChange {
        source,
        value,
        is_final: true,
    });
    settle(app);
}

fn type_name(app: &mut App, name: &str) {
    let entity = control(app, PanelId::Name);
    app.world_mut()
        .get_mut::<EditableText>(entity)
        .expect("the name input")
        .queue_edit(TextEdit::Insert(name.into()));
    settle(app);
}

#[test]
fn a_new_game_opens_party_creation_in_the_feathers_skin() {
    let mut app = creating("feathers-open.ron");
    assert_eq!(play_state(&app), PlayState::CreateParty);
    assert!(app.world().resource::<Screens>().skin.feathers);
    let mut roots = app.world_mut().query::<&PanelRoot>();
    assert_eq!(roots.iter(app.world()).count(), 1);
    // The canvas paints no creation widgets under the panel.
    assert!(
        common::frame(&app)
            .frame
            .widget(omnis_app::widget::WidgetId::Row(0))
            .is_none()
    );
}

#[test]
fn a_fighter_is_drafted_through_the_panel() {
    let mut app = creating("feathers-draft.ron");
    type_name(&mut app, "Brenna");
    activate(&mut app, PanelId::Pick(Choice::Race, 3));
    activate(&mut app, PanelId::Pick(Choice::Class, 1));
    for (ability, score) in [15, 14, 13, 12, 10, 8].into_iter().enumerate() {
        if ability % 2 == 0 {
            change(&mut app, PanelId::Score(ability), score);
        } else {
            #[allow(clippy::cast_precision_loss)]
            change(&mut app, PanelId::ScoreSlider(ability), score as f32);
        }
    }
    change(&mut app, PanelId::Skill(2), true);
    change(&mut app, PanelId::Skill(6), true);
    {
        let form = &app.world().resource::<Screens>().creation;
        assert_eq!(form.name, "Brenna");
        assert_eq!(form.scores, [15, 14, 13, 12, 10, 8]);
        assert_eq!(form.skills.len(), 2);
    }
    activate(&mut app, PanelId::Add);
    let party = &world(&app).party;
    assert_eq!(
        party.members.len(),
        1,
        "{}",
        app.world().resource::<Screens>().creation.message
    );
    assert_eq!(party.members[0].name, "Brenna");
    // The form starts over and the panel follows it.
    let name = control(&mut app, PanelId::Name);
    let text = app.world().get::<EditableText>(name).unwrap();
    assert_eq!(text.value().to_string(), "");
    click_node(&mut app, name);
    keys(&mut app, "Kell");
    assert_eq!(app.world().resource::<Screens>().creation.name, "Kell");
    activate(&mut app, PanelId::Begin);
    assert_eq!(play_state(&app), PlayState::Explore);
    let mut roots = app.world_mut().query::<&PanelRoot>();
    assert_eq!(
        roots.iter(app.world()).count(),
        0,
        "the panel leaves with the screen"
    );
}

// ------------------------------------------------------------------ pointer and keys

use bevy::camera::{NormalizedRenderTarget, RenderTarget};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef};

fn window(app: &mut App) -> Entity {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>();
    query.single(app.world()).expect("a primary window")
}

fn target(app: &mut App) -> NormalizedRenderTarget {
    let window = window(app);
    RenderTarget::Window(WindowRef::Entity(window))
        .normalize(Some(window))
        .expect("a window target")
}

/// The centre of a node in logical window pixels.
fn centre(app: &App, entity: Entity) -> Vec2 {
    let transform = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("a laid-out node");
    transform.translation
}

/// A real click: the pointer moves there and presses in one frame, and lets go in the next.
fn click_node(app: &mut App, entity: Entity) {
    let location = Location {
        target: target(app),
        position: centre(app, entity),
    };
    let mut inputs = app.world_mut().resource_mut::<Messages<PointerInput>>();
    inputs.write(PointerInput::new(
        PointerId::Mouse,
        location.clone(),
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    app.update();
    let mut inputs = app.world_mut().resource_mut::<Messages<PointerInput>>();
    inputs.write(PointerInput::new(
        PointerId::Mouse,
        location.clone(),
        PointerAction::Press(PointerButton::Primary),
    ));
    app.update();
    let mut inputs = app.world_mut().resource_mut::<Messages<PointerInput>>();
    inputs.write(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Release(PointerButton::Primary),
    ));
    settle(app);
}

/// Keys as a keyboard sends them: a logical key with its text.
fn keys(app: &mut App, text: &str) {
    let window = window(app);
    for c in text.chars() {
        let s: bevy::platform::prelude::String = c.to_string();
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::F24,
                logical_key: Key::Character(s.as_str().into()),
                state: ButtonState::Pressed,
                text: Some(s.as_str().into()),
                repeat: false,
                window,
            });
        app.update();
    }
    settle(app);
}

fn name_text(app: &mut App) -> String {
    let name = control(app, PanelId::Name);
    app.world()
        .get::<EditableText>(name)
        .unwrap()
        .value()
        .to_string()
}

#[test]
fn the_name_input_takes_a_click_and_keys() {
    let mut app = creating("feathers-name-keys.ron");
    let name = control(&mut app, PanelId::Name);
    click_node(&mut app, name);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    keys(&mut app, "Bren");
    assert_eq!(name_text(&mut app), "Bren");
    assert_eq!(app.world().resource::<Screens>().creation.name, "Bren");
}

fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn tab(app: &mut App) {
    let window = window(app);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code: KeyCode::Tab,
            logical_key: Key::Tab,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
    settle(app);
}

/// The owner's display. The camera's target follows the resize message,
/// and so does the app's `WindowSize`.
fn ultrawide(app: &mut App) {
    let window = window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .resolution
        .set(5120.0, 1440.0);
    app.world_mut()
        .resource_mut::<Messages<bevy::window::WindowResized>>()
        .write(bevy::window::WindowResized {
            window,
            width: 5120.0,
            height: 1440.0,
        });
    settle(app);
}

/// Choose a class as a person does: open the menu, click the item. The panel is rebuilt
/// under the pointer, with the keyboard focus left on an entity that is gone.
fn pick_class_by_mouse(app: &mut App, index: usize) {
    let before = control(app, PanelId::Name);
    let menu = control(app, PanelId::Menu(Choice::Class));
    click_node(app, menu);
    let item = control(app, PanelId::Pick(Choice::Class, index));
    click_node(app, item);
    assert_eq!(app.world().resource::<Screens>().creation.class, index);
    assert_ne!(control(app, PanelId::Name), before, "the panel was rebuilt");
}

#[test]
fn the_name_input_survives_a_rebuilt_panel() {
    let mut app = creating("feathers-name-rebuilt.ron");
    let name = control(&mut app, PanelId::Name);
    click_node(&mut app, name);
    keys(&mut app, "Bren");
    pick_class_by_mouse(&mut app, 1);
    assert_eq!(name_text(&mut app), "Bren", "the new input shows the form");
    let name = control(&mut app, PanelId::Name);
    click_node(&mut app, name);
    assert_eq!(focus(&app), Some(name));
    keys(&mut app, "na");
    assert_eq!(name_text(&mut app), "Brenna");
    assert_eq!(app.world().resource::<Screens>().creation.name, "Brenna");
}

#[test]
fn the_name_input_is_reached_by_tab_on_the_ultrawide_after_a_rebuild() {
    let mut app = creating("feathers-name-tab.ron");
    ultrawide(&mut app);
    pick_class_by_mouse(&mut app, 2);
    tab(&mut app);
    let name = control(&mut app, PanelId::Name);
    assert_eq!(focus(&app), Some(name), "the first stop is the name");
    keys(&mut app, "Kell");
    assert_eq!(app.world().resource::<Screens>().creation.name, "Kell");
}

/// The controls that lie outside the panel, with their rectangles (menu items are laid out
/// only while their menu is open).
fn outside_the_panel(app: &mut App) -> Vec<(PanelId, Rect)> {
    let mut roots = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<PanelRoot>>();
    let (node, at) = roots.single(app.world()).expect("one panel");
    let root = Rect::from_center_size(at.translation, node.size());
    let mut controls = app
        .world_mut()
        .query::<(&Control, &ComputedNode, &UiGlobalTransform)>();
    controls
        .iter(app.world())
        .filter(|(control, ..)| !matches!(control.0, PanelId::Pick(..) | PanelId::FontPick(_)))
        .map(|(control, node, at)| {
            (
                control.0,
                Rect::from_center_size(at.translation, node.size()),
            )
        })
        .filter(|(_, rect)| rect.is_empty() || !root.contains(rect.min) || !root.contains(rect.max))
        .collect()
}

fn scale_slider(app: &mut App) -> f32 {
    let slider = control(app, PanelId::UiScale);
    app.world()
        .get::<bevy::ui_widgets::SliderValue>(slider)
        .expect("a slider")
        .0
}

#[test]
fn the_interface_scale_follows_the_window_until_the_slider_is_moved() {
    let mut app = creating("feathers-scale.ron");
    // 1280×720: the canvas fits once, and the panel keeps the ultrawide's proportions.
    assert!((app.world().resource::<UiScale>().0 - 0.75).abs() < 1e-6);
    assert!((scale_slider(&mut app) - 0.75).abs() < 1e-6);
    assert_eq!(outside_the_panel(&mut app), Vec::new());
    // The check bites: the ultrawide's 1.5 does not fit this window.
    change(&mut app, PanelId::UiScale, 1.5_f32);
    let outside = outside_the_panel(&mut app);
    assert!(
        outside.iter().any(|(id, _)| *id == PanelId::Add),
        "{outside:?}"
    );
    app.world_mut()
        .resource_mut::<omnis_app::feathers_creation::ScaleChoice>()
        .0 = None;
    // 5120×1440: the canvas is doubled, and the scale is the owner's 1.5.
    ultrawide(&mut app);
    assert!((app.world().resource::<UiScale>().0 - 1.5).abs() < 1e-6);
    assert!((scale_slider(&mut app) - 1.5).abs() < 1e-6);
    assert_eq!(outside_the_panel(&mut app), Vec::new());
    // The slider's word stands, across a rebuilt panel too.
    change(&mut app, PanelId::UiScale, 1.25_f32);
    assert!((app.world().resource::<UiScale>().0 - 1.25).abs() < 1e-6);
    pick_class_by_mouse(&mut app, 1);
    assert!((app.world().resource::<UiScale>().0 - 1.25).abs() < 1e-6);
    assert!((scale_slider(&mut app) - 1.25).abs() < 1e-6);
    assert_eq!(outside_the_panel(&mut app), Vec::new());
}
