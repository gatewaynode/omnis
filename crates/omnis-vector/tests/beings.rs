//! Groups standing in view, in the window (M8.5 step 2c; presentation-ARCHITECTURE §8.7): each
//! drawn as its front monster's figure on its own kind of line; the view redrawn when a group is
//! met, bought off or let be; Use named for a lone group, the verbs listing it before the objects
//! with Approach; a search naming it within reach; a refused Approach worded short.

use crate::common;

use common::{map, objects_data, place, town_session};
use omnis_sim::api::BeingView;
use omnis_sim::omnis_core::{CharacterId, Facing, Level};
use omnis_sim::omnis_data::footprint::Footprint;
use omnis_sim::omnis_data::{Data, Look, Shape};
use omnis_sim::{ActorRef, CheckKind, Command, Event, Rejection};
use omnis_vector::geometry::{SegKind, Segment};
use omnis_vector::ground::Ground;
use omnis_vector::labels;
use omnis_vector::scene::Scene;
use omnis_vector::screen::actions::{Does, available};
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, ScreenId};
use omnis_vector::shapes::figures;
use omnis_vector::shell::session::reshapes;
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;

fn yard(data: &Data, x: u16, y: u16, facing: Facing) -> SimWorld {
    let (mut world, _) = town_session(data);
    place(&mut world, map(data, "objects:map:yard"), x, y, facing);
    world
}

/// The rats' figure's footprint at (5, 1), a step east of centre.
fn rats_footprint() -> Footprint {
    Footprint::of(5, 1, (1, 0), Facing::West, 3, 3)
}

/// The lines of `kind` with both ends inside a footprint, its edges included.
fn inside(lines: &[Segment], kind: SegKind, f: &Footprint) -> usize {
    let edge = |v: i32| f32::from(i16::try_from(v).expect("small")) / 16.0;
    let (x0, y0, x1, y1) = (edge(f.x0), edge(f.y0), edge(f.x1), edge(f.y1));
    let within = |p: [f32; 3]| {
        (x0 - 1e-5..=x1 + 1e-5).contains(&p[0]) && (y0 - 1e-5..=y1 + 1e-5).contains(&p[2])
    };
    lines
        .iter()
        .filter(|s| s.kind == kind && within(s.a) && within(s.b))
        .count()
}

#[test]
fn a_standing_group_is_drawn_as_its_figure_on_its_own_lines_until_met() {
    let data = objects_data();
    let being = BeingView {
        being: "rat".into(),
        monster: "objects:monster:pack_rat".into(),
        count: 2,
        x: 5,
        y: 1,
        pos: (1, 0),
        facing: Facing::West,
        look: Look {
            shape: Shape::Figure,
            width: 3,
            depth: 3,
            height: 3,
        },
    };
    let (lines, fills) = figures(&[being], &Ground::default());
    assert!(lines.iter().all(|s| s.kind == SegKind::Being));
    assert_eq!(
        inside(&lines, SegKind::Being, &rats_footprint()),
        24,
        "body and head"
    );
    assert!(!fills.is_empty());
    // In the scene, on the ground level only.
    let mut world = yard(&data, 3, 1, Facing::East);
    let yard_id = map(&data, "objects:map:yard");
    let scene = Scene::build(&data, yard_id, Level::GROUND, &world);
    assert_eq!(inside(&scene.own, SegKind::Being, &rats_footprint()), 24);
    let scene = Scene::build(&data, yard_id, Level(1), &world);
    assert_eq!(inside(&scene.own, SegKind::Being, &rats_footprint()), 0);
    // Met: not drawn.
    place(&mut world, yard_id, 5, 2, Facing::North);
    world.apply(&data, Command::Interact).expect("met");
    let scene = Scene::build(&data, yard_id, Level::GROUND, &world);
    assert_eq!(inside(&scene.own, SegKind::Being, &rats_footprint()), 0);
}

#[test]
fn meeting_a_group_buying_it_off_or_letting_it_be_redraws_the_view() {
    let data = objects_data();
    let world = yard(&data, 5, 2, Facing::North);
    let events = world.trial(&data, &Command::Interact).expect("met");
    assert!(events.iter().any(reshapes), "{events:?}");
    let check = |kind| Event::Check {
        actor: ActorRef::Member(CharacterId(0)),
        kind,
        roll: None,
        dc: 10,
        success: true,
    };
    assert!(reshapes(&Event::Bribed { cost: 0 }));
    assert!(
        reshapes(&check(CheckKind::Hide)),
        "hidden from, it stands again"
    );
    assert!(
        reshapes(&check(CheckKind::Run)),
        "run from, it stands again"
    );
    assert!(
        !reshapes(&check(CheckKind::Stealth)),
        "the group's own sneaking"
    );
}

#[test]
fn use_names_a_lone_group_and_the_verbs_list_it_before_the_objects() {
    let data = objects_data();
    // Only the rats in reach: Use goes up to them.
    let world = yard(&data, 5, 2, Facing::North);
    let offers = available(&world, &data);
    assert_eq!(offers[0].label, "Approach: Pack rat x2");
    assert_eq!(offers[0].does, Does::Command(Command::Interact));
    // The plaque in reach too: the verbs list the rats first.
    let world = yard(&data, 4, 1, Facing::East);
    let views = common::views(&world, &data);
    let ctx = common::ctx(&data, &world, &views);
    let shown = screen::content(ScreenId::Verbs, &ctx, &Forms::default());
    let words: Vec<String> = shown.panels[0]
        .rows
        .iter()
        .flat_map(|r| {
            r.spans
                .iter()
                .map(|s| s.text.clone())
                .chain(r.controls.iter().map(|c| format!("[{}]", c.label)))
                .chain(r.after.iter().map(|s| s.text.clone()))
        })
        .collect();
    assert_eq!(&words[1..4], ["Pack rat", "x2", "[Approach]"], "{words:?}");
    assert!(words.iter().any(|w| w == "Plaque"), "{words:?}");
    let effect = screen::apply(
        ScreenId::Verbs,
        &ctx,
        &mut Forms::default(),
        ControlId::Verb(0),
        &Payload::Press,
        0,
    );
    assert_eq!(
        effect,
        Ok(Effect::Command {
            command: Command::Approach {
                being: "rat".into()
            },
            close: true,
        }),
        "the encounter takes over, so the verbs close"
    );
}

#[test]
fn a_search_names_the_group_within_reach_and_a_refusal_is_short() {
    let data = objects_data();
    let mut world = yard(&data, 4, 1, Facing::East);
    let events = world.apply(&data, Command::Search).expect("search");
    let views = common::views(&world, &data);
    let lines = text::searched(&events, &views, &data, None);
    assert!(
        lines[0].starts_with("Within reach: Pack rat x2, "),
        "{lines:?}"
    );
    let refusal = Rejection::NoSuchBeing {
        being: "rat".into(),
    };
    assert_eq!(
        labels::refused(&data, &views.party, &refusal),
        "no one there to approach"
    );
}
