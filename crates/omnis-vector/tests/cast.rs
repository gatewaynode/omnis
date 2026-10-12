//! Spells on the road (parity P5): the cast view the shell reads and the HUD lines a cast
//! makes. Real packs, the fixed party, the town's start.

use crate::common;
use common::ctx;

use bevy::prelude::*;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Target};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::screen::roster::Tool;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, Screen, ScreenId};
use omnis_vector::shell::screen::{Pressable, Screens, Side};
use omnis_vector::shell::session::Session;
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;

#[test]
fn the_views_hold_each_member_s_road_spells_and_a_cast_reads_as_lines() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let views = common::views(&world, &data);
    assert_eq!(
        views.casts,
        world.casts(&data),
        "the views carry the cast view"
    );
    let ilvara = common::id(&world, 2);
    let light = views
        .casts
        .iter()
        .find(|r| r.caster == ilvara && r.spell == "base:spell:light")
        .expect("Ilvara lights the way");
    assert_eq!((light.cost, light.refusal.as_ref()), (0, None));

    let mut names = Names::default();
    names.observe(&views.party);
    let events = world
        .apply(
            &data,
            Command::Cast {
                caster: ilvara,
                spell: "base:spell:light".into(),
                target: Target::Member(ilvara),
            },
        )
        .expect("cast");
    let got: Vec<String> = events
        .iter()
        .filter_map(|e| text::describe(e, &names, &data))
        .collect();
    let light = labels::spell(&data, "base:spell:light");
    assert_eq!(
        got,
        [
            format!("Ilvara casts {light} (free)"),
            format!("{light} lights the party's way"),
        ]
    );
}

// The screen (P5b).

fn forms(owner: CharacterId, selected: Option<CharacterId>) -> Forms {
    let mut forms = Forms::default();
    forms.owner = Some(owner);
    forms.selected = selected;
    forms
}

fn shown(data: &Data, world: &SimWorld, forms: &Forms) -> Screen {
    let views = common::views(world, data);
    screen::content(ScreenId::Cast, &ctx(data, world, &views), forms)
}

fn press(
    data: &Data,
    world: &SimWorld,
    forms: &mut Forms,
    id: ControlId,
) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    screen::apply(
        ScreenId::Cast,
        &ctx(data, world, &views),
        forms,
        id,
        &Payload::Press,
        0,
    )
}

/// The caster's road spells, each with the view's cost and where it goes; every live Cast is
/// taken by the engine, every dim one says why.
#[test]
fn the_caster_s_road_spells_cost_what_the_view_says_and_cast_as_shown() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let (brenna, durin) = (common::id(&world, 0), common::id(&world, 1));
    let views = common::views(&world, &data);
    let mine: Vec<_> = views.casts.iter().filter(|c| c.caster == durin).collect();
    assert!(!mine.is_empty(), "the cleric casts on the road");
    let mut f = forms(durin, Some(brenna));
    let s = shown(&data, &world, &f);
    let rows: Vec<_> = s.panels[0]
        .rows
        .iter()
        .filter(|r| !r.controls.is_empty() && r.controls[0].label == "Cast")
        .collect();
    assert_eq!(rows.len(), mine.len());
    for (row, cast) in rows.iter().zip(&mine) {
        assert_eq!(row.spans[0].text, labels::spell(&data, &cast.spell));
        let mut note = format!("{} pt", cast.cost);
        if cast.targets_members {
            note.push_str("  on Brenna");
        }
        assert_eq!(row.after[0].text, note);
        let id = row.controls[0].id;
        let used = press(&data, &world, &mut f, id);
        match &row.controls[0].why_not {
            None => {
                let Ok(Effect::Command {
                    command,
                    close: false,
                }) = used
                else {
                    panic!("Cast sends and stays open: {used:?}");
                };
                assert!(world.trial(&data, &command).is_ok(), "{command:?}");
            }
            Some(why) => assert_eq!(used, Err(Refused::Dim(why.clone()))),
        }
    }
}

/// A spell aimed at a member goes to the column's selection, or to the caster with none; with no
/// points left the costed spells are dim and a cantrip is not.
#[test]
fn a_heal_goes_to_the_selection_or_the_caster_and_points_dim_the_costed() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let (brenna, durin) = (common::id(&world, 0), common::id(&world, 1));
    let views = common::views(&world, &data);
    let (at, heal) = views
        .casts
        .iter()
        .enumerate()
        .find(|(_, c)| c.caster == durin && c.targets_members)
        .expect("Durin has a spell for a member");
    let id = ControlId::Cast(u16::try_from(at).unwrap());
    for (selected, to) in [(None, durin), (Some(brenna), brenna)] {
        let mut f = forms(durin, selected);
        assert_eq!(
            press(&data, &world, &mut f, id),
            Ok(Effect::Command {
                command: Command::Cast {
                    caster: durin,
                    spell: heal.spell.clone(),
                    target: Target::Member(to),
                },
                close: false
            })
        );
    }
    world.party.members[1].spell_points = 0;
    let views = common::views(&world, &data);
    let s = shown(&data, &world, &forms(durin, None));
    for (i, c) in views
        .casts
        .iter()
        .enumerate()
        .filter(|(_, c)| c.caster == durin)
    {
        let control = s
            .control(ControlId::Cast(u16::try_from(i).unwrap()))
            .unwrap();
        if c.cost > 0 {
            assert_eq!(control.why_not, Some(format!("need {} pt", c.cost)));
        } else {
            assert_eq!(control.why_not, None, "a cantrip costs nothing");
        }
    }
}

#[test]
fn a_member_with_no_road_spell_says_so_and_so_does_a_party_with_none() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let brenna = common::id(&world, 0);
    let text = |s: &Screen| s.panels[0].rows.last().unwrap().spans[0].text.clone();
    assert_eq!(
        text(&shown(&data, &world, &forms(brenna, None))),
        "Brenna knows no spell for the road"
    );
    for m in &mut world.party.members {
        m.known_spells.clear();
    }
    assert_eq!(
        text(&shown(&data, &world, &forms(brenna, None))),
        "Nobody knows a spell for the road."
    );
}

/// The column's Spells opens the screen; a cast keeps it open and reports the line.
#[test]
fn a_cast_from_the_column_keeps_the_screen_open_and_reports() {
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    let ilvara = app.world().resource::<Session>().views.party.members[2].member;
    let press_ui = |app: &mut App, side: Option<Side>, pressable: Option<Pressable>| {
        if let Some(m) = side {
            common::app::set(app, &m, Interaction::Pressed);
        }
        if let Some(m) = pressable {
            common::app::set(app, &m, Interaction::Pressed);
        }
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&mut Interaction>();
        for mut i in q.iter_mut(world) {
            *i = Interaction::None;
        }
        app.update();
    };
    press_ui(&mut app, Some(Side(ControlId::Member(ilvara))), None);
    press_ui(&mut app, Some(Side(ControlId::Open(Tool::Spells))), None);
    let top = |app: &App| {
        let session = app.world().resource::<Session>();
        app.world()
            .resource::<Screens>()
            .nav
            .top(Some(&session.views))
    };
    assert_eq!(top(&app), Some(ScreenId::Cast));
    let light = {
        let session = app.world().resource::<Session>();
        let at = session
            .views
            .casts
            .iter()
            .position(|c| c.caster == ilvara && c.spell == "base:spell:light")
            .unwrap();
        ControlId::Cast(u16::try_from(at).unwrap())
    };
    press_ui(&mut app, None, Some(Pressable(light)));
    assert_eq!(top(&app), Some(ScreenId::Cast), "the screen stays open");
    let text = app
        .world()
        .resource::<Screens>()
        .forms
        .message
        .clone()
        .map(|m| m.text)
        .unwrap_or_default();
    assert!(text.ends_with("lights the party's way"), "{text}");
}
