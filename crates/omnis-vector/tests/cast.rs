//! Spells on the road (parity P5): the cast view the shell reads and the HUD lines a cast
//! makes. Real packs, the fixed party, the town's start.

use crate::common;
use crate::common::app::Pointing;
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

/// The caster's road spells, each with the view's cost. Cast sends nothing by itself: it opens
/// the spell's target step, where every live target is taken by the engine and every dim one
/// says why (owner, 2026-11-14: option, then target, then validate, pay and execute).
#[test]
fn the_caster_s_road_spells_cost_what_the_view_says_and_cast_through_the_target_step() {
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
    let mut stepped = 0;
    for (row, cast) in rows.iter().zip(&mine) {
        assert_eq!(row.spans[0].text, labels::spell(&data, &cast.spell));
        assert_eq!(
            row.after[0].text,
            format!("{} pt", cast.cost),
            "no target yet"
        );
        let id = row.controls[0].id;
        let used = press(&data, &world, &mut f, id);
        if let Some(why) = &row.controls[0].why_not {
            assert_eq!(used, Err(Refused::Dim(why.clone())));
            continue;
        }
        assert_eq!(used, Ok(Effect::None), "Cast sends nothing by itself");
        let step = shown(&data, &world, &f);
        assert_eq!(step.title.text, "Choose a target");
        let targets: Vec<_> = step.controls().filter(|c| c.label == "Choose").collect();
        assert!(!targets.is_empty(), "{}", cast.spell);
        for target in targets {
            let mut again = f.clone();
            let used = press(&data, &world, &mut again, target.id);
            match &target.why_not {
                None => {
                    let Ok(Effect::Command {
                        command,
                        close: false,
                    }) = used
                    else {
                        panic!("a target sends the cast and the screen stays: {used:?}");
                    };
                    assert!(world.trial(&data, &command).is_ok(), "{command:?}");
                    assert_eq!(again.aim, None, "the step is over");
                }
                Some(why) => assert_eq!(used, Err(Refused::Dim(why.clone()))),
            }
        }
        // Back returns to the list with nothing sent.
        assert_eq!(
            press(&data, &world, &mut f, ControlId::Back),
            Ok(Effect::None)
        );
        assert_eq!(f.aim, None);
        assert_eq!(shown(&data, &world, &f).title.text, "Spells");
        stepped += 1;
    }
    assert!(stepped > 0, "a spell went through the step");
}

/// The owner's bug (2026-11-14): a heal on the road could go to no one but its caster. Its
/// target step offers every member, the caster marked, whoever the column has selected; the
/// one chosen is who the cast names. With no points left the costed spells are dim and a
/// cantrip is not.
#[test]
fn a_heal_offers_every_member_and_goes_to_the_one_chosen() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let (brenna, durin) = (common::id(&world, 0), common::id(&world, 1));
    let views = common::views(&world, &data);
    let (at, heal) = views
        .casts
        .iter()
        .enumerate()
        .find(|(_, c)| c.caster == durin && c.spell == "base:spell:cure_wounds")
        .expect("Durin cures wounds");
    let id = ControlId::Cast(u16::try_from(at).unwrap());
    let names: Vec<String> = views.party.members.iter().map(|m| m.name.clone()).collect();
    for selected in [None, Some(brenna), Some(durin)] {
        let mut f = forms(durin, selected);
        assert_eq!(press(&data, &world, &mut f, id), Ok(Effect::None));
        let step = shown(&data, &world, &f);
        let rows: Vec<_> = step.panels[0]
            .rows
            .iter()
            .filter(|r| r.controls.iter().any(|c| c.label == "Choose"))
            .collect();
        let listed: Vec<String> = rows.iter().map(|r| r.spans[0].text.clone()).collect();
        assert_eq!(listed, names, "every member, in marching order");
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.controls[0].why_not, None);
            let own = row.after.first().map(|s| s.text.as_str());
            assert_eq!(own, (i == 1).then_some("self"), "only Durin is himself");
        }
        // Each row is that member, whatever the column has selected.
        for (i, member) in views.party.members.iter().enumerate() {
            let mut one = f.clone();
            let target = ControlId::Target(u8::try_from(i).unwrap());
            assert_eq!(
                press(&data, &world, &mut one, target),
                Ok(Effect::Command {
                    command: Command::Cast {
                        caster: durin,
                        spell: heal.spell.clone(),
                        target: Target::Member(member.member),
                    },
                    close: false
                })
            );
        }
    }
    // The step belongs to the screen it was opened on: it ends when another screen is shown.
    let mut f = forms(durin, None);
    f.shown(Some(ScreenId::Cast), None);
    assert_eq!(press(&data, &world, &mut f, id), Ok(Effect::None));
    f.shown(Some(ScreenId::Cast), None);
    assert!(f.aim.is_some(), "the same screen keeps it");
    f.shown(Some(ScreenId::Inventory), None);
    assert_eq!(f.aim, None, "another screen ends it");

    // A spell of the caster's own: one row, the caster.
    let ilvara = common::id(&world, 2);
    let light = views
        .casts
        .iter()
        .position(|c| c.caster == ilvara && c.spell == "base:spell:light")
        .unwrap();
    let mut f = forms(ilvara, Some(brenna));
    let light = ControlId::Cast(u16::try_from(light).unwrap());
    assert_eq!(press(&data, &world, &mut f, light), Ok(Effect::None));
    let step = shown(&data, &world, &f);
    let choose: Vec<_> = step.controls().filter(|c| c.label == "Choose").collect();
    assert_eq!(choose.len(), 1, "self only shows self");

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
        } else if c.spell == "base:spell:sacred_flame" {
            // A harmful cantrip is listed on the road and waits for a group in sight.
            assert_eq!(control.why_not.as_deref(), Some("nothing in reach"));
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

/// The column's Spells opens the screen; Cast opens the target step, a target casts, and the
/// screen stays open and reports the line. During the step a member's entry in the column is
/// that member as the target.
#[test]
fn a_cast_from_the_column_keeps_the_screen_open_and_reports() {
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    let ilvara = app.world().resource::<Session>().views.party.members[2].member;
    let press_ui = |app: &mut App, side: Option<Side>, pressable: Option<Pressable>| {
        if let Some(m) = side {
            common::app::set(app, &m, Pointing::Press);
        }
        if let Some(m) = pressable {
            common::app::set(app, &m, Pointing::Press);
        }
        app.update();
        common::app::rest(app);
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
    {
        let forms = &app.world().resource::<Screens>().forms;
        assert!(forms.aim.is_some(), "Cast opens the target step");
        assert_eq!(forms.message, None, "nothing is cast yet");
    }
    press_ui(&mut app, None, Some(Pressable(ControlId::Target(0))));
    assert_eq!(
        top(&app),
        Some(ScreenId::Cast),
        "and stays open after the cast"
    );
    assert_eq!(app.world().resource::<Screens>().forms.aim, None);
    let text = app
        .world()
        .resource::<Screens>()
        .forms
        .message
        .clone()
        .map(|m| m.text)
        .unwrap_or_default();
    assert!(text.ends_with("lights the party's way"), "{text}");

    // Durin's cure wounds, aimed by a click on Brenna's entry in the column.
    let (brenna, durin) = {
        let members = &app.world().resource::<Session>().views.party.members;
        (members[0].member, members[1].member)
    };
    press_ui(&mut app, Some(Side(ControlId::Member(durin))), None);
    press_ui(&mut app, Some(Side(ControlId::Open(Tool::Spells))), None);
    let (cure, points) = {
        let session = app.world().resource::<Session>();
        let at = session
            .views
            .casts
            .iter()
            .position(|c| c.caster == durin && c.spell == "base:spell:cure_wounds")
            .unwrap();
        (
            ControlId::Cast(u16::try_from(at).unwrap()),
            session.views.party.members[1].spell_points,
        )
    };
    press_ui(&mut app, None, Some(Pressable(cure)));
    assert!(app.world().resource::<Screens>().forms.aim.is_some());
    press_ui(&mut app, Some(Side(ControlId::Member(brenna))), None);
    let screens = app.world().resource::<Screens>();
    assert_eq!(screens.forms.aim, None, "the click chose the target");
    let text = screens
        .forms
        .message
        .clone()
        .map(|m| m.text)
        .unwrap_or_default();
    assert!(text.contains("Brenna"), "{text}");
    let session = app.world().resource::<Session>();
    assert!(session.views.party.members[1].spell_points < points, "paid");
}

/// The fog is drawn at the depth the engine sees to, so a light in the dungeon lifts it as it
/// lifts what the automap records (a bug: the fog stayed at the floor's depth).
#[test]
fn a_light_lifts_the_fog_with_the_engine_s_sight() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let dungeon = common::map(&data, "test:map:dungeon");
    common::place(
        &mut world,
        dungeon,
        1,
        1,
        omnis_sim::omnis_core::Facing::South,
    );
    let before = common::views(&world, &data);
    assert_eq!(before.sight, 6, "the dungeon floor's depth");
    let ilvara = common::id(&world, 2);
    world
        .apply(
            &data,
            Command::Cast {
                caster: ilvara,
                spell: "base:spell:light".into(),
                target: Target::Member(ilvara),
            },
        )
        .expect("cast");
    let lit = common::views(&world, &data);
    let engine = omnis_sim::api::viewport(&world, &data)
        .expect("viewport")
        .visibility_depth;
    assert_eq!(engine, 8, "Light reaches 8");
    assert_eq!(lit.sight, engine, "the fog follows the engine");
}
