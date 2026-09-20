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
    // Feathers' own row height, so the theme and its sizes are in force.
    let button = probe(&mut app, Probe::Button);
    let height = app.world().get::<ComputedNode>(button).unwrap().size().y;
    assert!((height - 24.0).abs() < 0.5, "button height {height}");
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
    activate(&mut app, PanelId::Begin);
    assert_eq!(play_state(&app), PlayState::Explore);
    let mut roots = app.world_mut().query::<&PanelRoot>();
    assert_eq!(
        roots.iter(app.world()).count(),
        0,
        "the panel leaves with the screen"
    );
}
