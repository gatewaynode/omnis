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

use bevy::input_focus::InputFocus;
use common::feathers::{
    Fault, activate, change, click_node, control, creating, draft_fighter, focus, form, keys,
    layout_faults, name_text, pick_class_by_mouse, slider, tab, ultrawide,
};
use common::{play_state, world};
use omnis_app::creation_panel::PanelId;
use omnis_app::feathers_creation::{PanelRoot, ScaleChoice};
use omnis_app::menus::Screens;
use omnis_app::sim::PlayState;

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
    draft_fighter(&mut app);
    assert_eq!(form(&app).name, "Brenna");
    assert_eq!(form(&app).scores, [15, 14, 13, 12, 10, 8]);
    assert_eq!(form(&app).skills.len(), 2);
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
    assert_eq!(form(&app).name, "Kell");
    activate(&mut app, PanelId::Begin);
    assert_eq!(play_state(&app), PlayState::Explore);
    let mut roots = app.world_mut().query::<&PanelRoot>();
    assert_eq!(
        roots.iter(app.world()).count(),
        0,
        "the panel leaves with the screen"
    );
}

#[test]
fn the_name_input_takes_a_click_and_keys() {
    let mut app = creating("feathers-name-keys.ron");
    let name = control(&mut app, PanelId::Name);
    click_node(&mut app, name);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    keys(&mut app, "Bren");
    assert_eq!(name_text(&mut app), "Bren");
    assert_eq!(form(&app).name, "Bren");
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
    assert_eq!(form(&app).name, "Brenna");
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
    assert_eq!(form(&app).name, "Kell");
}

#[test]
fn the_interface_scale_follows_the_window_until_the_slider_is_moved() {
    let mut app = creating("feathers-scale.ron");
    // 1280×720: the canvas fits once, and the panel keeps the ultrawide's proportions.
    assert!((app.world().resource::<UiScale>().0 - 0.75).abs() < 1e-6);
    assert!((slider(&mut app, PanelId::UiScale) - 0.75).abs() < 1e-6);
    assert_eq!(layout_faults(&mut app), Vec::new());
    // The check bites: the ultrawide's 1.5 does not fit this window.
    change(&mut app, PanelId::UiScale, 1.5_f32);
    let faults = layout_faults(&mut app);
    assert!(faults.contains(&Fault::Outside(PanelId::Add)), "{faults:?}");
    app.world_mut().resource_mut::<ScaleChoice>().0 = None;
    // 5120×1440: the canvas is doubled, and the scale is the owner's 1.5.
    ultrawide(&mut app);
    assert!((app.world().resource::<UiScale>().0 - 1.5).abs() < 1e-6);
    assert!((slider(&mut app, PanelId::UiScale) - 1.5).abs() < 1e-6);
    assert_eq!(layout_faults(&mut app), Vec::new());
    // The slider's word stands, across a rebuilt panel too.
    change(&mut app, PanelId::UiScale, 1.25_f32);
    assert!((app.world().resource::<UiScale>().0 - 1.25).abs() < 1e-6);
    pick_class_by_mouse(&mut app, 1);
    assert!((app.world().resource::<UiScale>().0 - 1.25).abs() < 1e-6);
    assert!((slider(&mut app, PanelId::UiScale) - 1.25).abs() < 1e-6);
    assert_eq!(layout_faults(&mut app), Vec::new());
}
