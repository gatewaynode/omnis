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
