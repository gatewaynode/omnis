//! Things of the map in the road's target step (owner, 2026-11-14; `tasks/plans/targeting.md`
//! step 6): a hand's targets are the doors and objects the engine lists, each named with the way
//! it lies and how far, dim with the engine's reason; the one chosen is what the cast names; and
//! the cell of the row under the pointer is marked in the 3D view.

use crate::common;
use crate::common::app::Pointing;
use common::ctx;

use bevy::prelude::*;
use omnis_sim::omnis_core::{Facing, Level, Position};
use omnis_sim::{Command, Target};
use omnis_vector::labels;
use omnis_vector::pose::Pose;
use omnis_vector::screen::roster::Tool;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, ScreenId};
use omnis_vector::shell::aim::Marked;
use omnis_vector::shell::render::{RenderPlugin, ViewAim};
use omnis_vector::shell::screen::{Pressable, Screens, Side};
use omnis_vector::shell::session::Session;
use omnis_vector::targeting::Aim;

const HAND: &str = "base:spell:mage_hand";

/// South of the yard's hedge, at the gate, facing east: the gate a tile north (shut fast), the
/// chest two east, the fountain two west, and the door behind which the torch stands.
#[test]
fn a_hand_s_targets_are_named_with_the_way_they_lie_and_how_far() {
    let data = common::objects_data();
    let world = crate::loot::yard(&data, 3, 3, Facing::East);
    let ilvara = common::id(&world, 2);
    let views = common::views(&world, &data);
    let context = ctx(&data, &world, &views);
    let game = context.game.unwrap();
    let mut forms = Forms::default();
    forms.owner = Some(ilvara);
    forms.aim = Some(Aim::spell(ilvara, HAND));
    let step = screen::content(ScreenId::Cast, &context, &forms);
    assert_eq!(step.title.text, "Choose a target");
    let rows: Vec<_> = step.panels[0]
        .rows
        .iter()
        .filter(|r| r.controls.first().is_some_and(|c| c.label == "Choose"))
        .collect();
    let said: Vec<(String, String)> = rows
        .iter()
        .map(|r| (r.spans[0].text.clone(), r.after[0].text.clone()))
        .collect();
    let name = |key: &str| labels::text(&data, key).to_owned();
    assert_eq!(
        said,
        [
            (
                format!("{}, north", name("objects:text:gate.name")),
                "1 tile".to_owned()
            ),
            (
                format!("{}, east", name("objects:text:chest.name")),
                "2 tiles".to_owned()
            ),
            (
                format!("{}, west", name("objects:text:fountain.name")),
                "2 tiles".to_owned()
            ),
            ("Door, east".to_owned(), "3 tiles".to_owned()),
        ]
    );
    // The gate is listed dim, with the engine's reason, and pressing it sends nothing.
    let why = rows[0].controls[0].why_not.clone().expect("shut fast");
    assert!(!why.is_empty());
    let press = |forms: &mut Forms, at: u8| {
        let id = ControlId::Target(at);
        screen::apply(ScreenId::Cast, &context, forms, id, &Payload::Press, 0)
    };
    assert_eq!(press(&mut forms.clone(), 0), Err(Refused::Dim(why)));
    // The door is free: the cast names it, and the engine takes it.
    let mut chosen = forms.clone();
    let Ok(Effect::Command { command, .. }) = press(&mut chosen, 3) else {
        panic!("the door's row sends the cast");
    };
    let door = Target::Door {
        x: 6,
        y: 3,
        facing: Facing::South,
    };
    assert_eq!(
        command,
        Command::Cast {
            caster: ilvara,
            spell: HAND.to_owned(),
            target: door,
        }
    );
    assert!(world.trial(&data, &command).is_ok());
    assert_eq!(chosen.aim, None, "the step is over");
    // What the 3D view marks for each row: the thing's cell; a member has none.
    let aim = forms.aim.clone().unwrap();
    let marks: Vec<_> = (0..5).map(|at| aim.marked(&data, game, at)).collect();
    assert_eq!(
        marks,
        [Some((3, 2)), Some((5, 3)), Some((1, 3)), Some((6, 3)), None]
    );
    let heal = Aim::spell(common::id(&world, 1), "base:spell:cure_wounds");
    assert_eq!(heal.marked(&data, game, 0), None);
}

/// A harmful spell on the road names the group in sight, how many, which way and how far; the
/// row sends the cast at the group and closes the screen, since a fight follows (the opening
/// strike); the group's cell is what the view marks.
#[test]
fn a_harmful_spell_s_target_is_the_group_in_sight_and_the_screen_gives_way() {
    let data = common::objects_data();
    let world = crate::loot::yard(&data, 3, 1, Facing::East);
    let ilvara = common::id(&world, 2);
    let views = common::views(&world, &data);
    let context = ctx(&data, &world, &views);
    let mut forms = Forms::default();
    forms.owner = Some(ilvara);
    let bolt = "base:spell:fire_bolt";
    // The spell is on the road's list now, free while a group is in sight.
    let at = views
        .casts
        .iter()
        .position(|c| c.caster == ilvara && c.spell == bolt);
    let cast = ControlId::Cast(u16::try_from(at.expect("fire bolt, on the road")).unwrap());
    let list = screen::content(ScreenId::Cast, &context, &forms);
    assert_eq!(list.control(cast).unwrap().why_not, None);
    let press = |forms: &mut Forms, id| {
        screen::apply(ScreenId::Cast, &context, forms, id, &Payload::Press, 0)
    };
    assert_eq!(press(&mut forms, cast), Ok(Effect::None));
    let step = screen::content(ScreenId::Cast, &context, &forms);
    let rows: Vec<_> = step.panels[0]
        .rows
        .iter()
        .filter(|r| r.controls.first().is_some_and(|c| c.label == "Choose"))
        .collect();
    // The group first, then the thing in sight that can be damaged (step 8 of combat in feet).
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].spans[0].text, "Crate, west");
    let rat = labels::monster(&data, "objects:monster:pack_rat");
    assert_eq!(
        (
            rows[0].spans[0].text.clone(),
            rows[0].after[0].text.as_str()
        ),
        (format!("{}, east", labels::count_label(rat, 2)), "2 tiles")
    );
    let aim = forms.aim.clone().unwrap();
    assert_eq!(aim.marked(&data, context.game.unwrap(), 0), Some((5, 1)));
    let sent = press(&mut forms, ControlId::Target(0));
    let Ok(Effect::Command {
        command,
        close: true,
    }) = sent
    else {
        panic!("the row sends the strike and closes the screen: {sent:?}");
    };
    let mut after = world.clone();
    after.apply(&data, command).expect("the strike");
    assert!(after.combat(&data).is_some(), "a fight follows");
}

/// The ends of the view's outline, when one is drawn.
fn outline(app: &mut App) -> Option<Vec<[f32; 3]>> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Option<&Mesh3d>, With<ViewAim>>();
    let handle = q.single(world).expect("one aim entity")?.0.clone();
    let mesh = world.resource::<Assets<Mesh>>().get(&handle).expect("mesh");
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::mesh::VertexAttributeValues::Float32x3(ends)) => Some(ends.clone()),
        _ => panic!("positions"),
    }
}

/// In the dungeon: the pointer on a door's row marks the door's cell in the 3D view, off it the
/// mark goes, and it goes when the step ends.
#[test]
fn the_row_under_the_pointer_is_marked_in_the_view() {
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(RenderPlugin);
    app.update();
    let ilvara = {
        let mut session = app.world_mut().resource_mut::<Session>();
        let map = common::map(&session.data, "test:map:dungeon");
        session.world.position = Position {
            map,
            level: Level::GROUND,
            x: 9,
            y: 2,
            facing: Facing::South,
        };
        session.pose = Pose::at(session.world.position);
        session.refresh();
        session.views.party.members[2].member
    };
    app.update();
    common::app::press_ui(&mut app, Side(ControlId::Member(ilvara)));
    common::app::press_ui(&mut app, Side(ControlId::Open(Tool::Spells)));
    let hand = {
        let casts = &app.world().resource::<Session>().views.casts;
        let at = casts
            .iter()
            .position(|c| c.caster == ilvara && c.spell == HAND);
        ControlId::Cast(u16::try_from(at.expect("Ilvara's mage hand")).unwrap())
    };
    common::app::press_ui(&mut app, Pressable(hand));
    let marked = |app: &App| app.world().resource::<Marked>().0;
    assert!(app.world().resource::<Screens>().forms.aim.is_some());
    assert_eq!(marked(&app), None, "the pointer is on no row");
    let expected = {
        let session = app.world().resource::<Session>();
        let rows = session.world.targets(
            &session.data,
            ilvara,
            &app.world()
                .resource::<Screens>()
                .forms
                .aim
                .clone()
                .unwrap()
                .option,
        );
        let Target::Door { x, y, .. } = rows[1].target else {
            panic!("two doors are in sight: {rows:?}");
        };
        (x, y)
    };
    let row = Pressable(ControlId::Target(1));
    common::app::set(&mut app, &row, Pointing::Hover);
    app.update();
    assert_eq!(marked(&app), Some(expected));
    app.update();
    let drawn = outline(&mut app).expect("the cell's floor is outlined");
    let (west, north) = (f32::from(expected.0), f32::from(expected.1));
    assert!(
        drawn
            .iter()
            .all(|[x, _, z]| (west..=west + 1.0).contains(x) && (north..=north + 1.0).contains(z)),
        "{drawn:?} lies in {expected:?}"
    );
    common::app::let_go(&mut app, &row);
    app.update();
    assert_eq!(marked(&app), None);
    app.update();
    assert_eq!(outline(&mut app), None, "and the outline goes with it");
    common::app::set(&mut app, &row, Pointing::Hover);
    app.update();
    assert!(marked(&app).is_some());
    common::app::press_ui(&mut app, Pressable(ControlId::Back));
    app.update();
    assert_eq!(marked(&app), None, "the step is over");
}

/// On the road the square's actions offer "Attack..." while a group is in sight: it asks who
/// attacks and at what, names the group as a spell's step does, and the row sends the member's
/// attack and closes the screen, since a fight follows (the opening strike by a weapon).
#[test]
fn the_square_s_attack_asks_who_and_at_what_and_opens_the_fight() {
    let data = common::objects_data();
    let world = crate::loot::yard(&data, 3, 1, Facing::East);
    let (brenna, durin) = (common::id(&world, 0), common::id(&world, 1));
    let views = common::views(&world, &data);
    let context = ctx(&data, &world, &views);
    let mut forms = Forms::default();
    let list = screen::content(ScreenId::Actions, &context, &forms);
    let offered: Vec<_> = list.panels[0]
        .rows
        .iter()
        .flat_map(|r| &r.controls)
        .collect();
    let attack = offered
        .iter()
        .find(|c| c.label == "Attack...")
        .expect("a group is in sight");
    let press = |forms: &mut Forms, id, payload: &Payload| {
        screen::apply(ScreenId::Actions, &context, forms, id, payload, 0)
    };
    assert_eq!(
        press(&mut forms, attack.id, &Payload::Press),
        Ok(Effect::None)
    );
    assert_eq!(forms.aim, Some(Aim::attack(brenna)), "the first member");
    let step = screen::content(ScreenId::Actions, &context, &forms);
    assert_eq!(step.title.text, "Choose a target");
    let first = &step.panels[0].rows[0].controls;
    assert_eq!(
        first.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
        ["Back", "Attacker"]
    );
    // The attacker steps round the party.
    assert_eq!(
        press(&mut forms, ControlId::Owner, &Payload::Press),
        Ok(Effect::None)
    );
    assert_eq!(forms.aim, Some(Aim::attack(durin)));
    let step = screen::content(ScreenId::Actions, &context, &forms);
    let rows: Vec<_> = step.panels[0]
        .rows
        .iter()
        .filter(|r| r.controls.first().is_some_and(|c| c.label == "Choose"))
        .collect();
    // The group first, then the thing in sight that can be damaged (step 8 of combat in feet).
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].spans[0].text, "Crate, west");
    let rat = labels::monster(&data, "objects:monster:pack_rat");
    assert_eq!(
        rows[0].spans[0].text,
        format!("{}, east", labels::count_label(rat, 2))
    );
    assert_eq!(rows[0].controls[0].why_not, None, "the crossbow reaches");
    let sent = press(&mut forms, ControlId::Target(0), &Payload::Press);
    let Ok(Effect::Command {
        command,
        close: true,
    }) = sent
    else {
        panic!("the row sends the attack and closes the screen: {sent:?}");
    };
    assert_eq!(
        command,
        Command::Attack {
            member: durin,
            target: Target::Being {
                being: "rat".into()
            }
        }
    );
    let mut after = world.clone();
    after.apply(&data, command).expect("the strike");
    assert!(after.combat(&data).is_some(), "a fight follows");
    // Back leaves the step for the square's actions; out of sight nothing is offered.
    forms.aim = Some(Aim::attack(brenna));
    assert_eq!(
        press(&mut forms, ControlId::Back, &Payload::Press),
        Ok(Effect::None)
    );
    assert_eq!(forms.aim, None);
    // Behind the hedge no group is seen, but the chest in reach can be struck: the entry stays.
    // With nothing there that can be damaged it is not offered.
    let offered = |data: &omnis_sim::omnis_data::Data| {
        let hidden = crate::loot::yard(data, 5, 4, Facing::North);
        let views = common::views(&hidden, data);
        let context = ctx(data, &hidden, &views);
        let list = screen::content(ScreenId::Actions, &context, &Forms::default());
        let mut any = list.panels[0].rows.iter().flat_map(|r| &r.controls);
        any.any(|c| c.label == "Attack...")
    };
    assert!(offered(&data));
    let mut bare = common::objects_data();
    for kind in bare.objects.values_mut() {
        kind.attributes.durability = None;
    }
    assert!(!offered(&bare));
}

/// The party column's Attack on a member's entry (the owner's QA note, 2026-11-14: "no
/// functionality to shoot a mundane weapon like Pip's shortbow from a range"): it opens the
/// square's actions on the attack's target step with that member the attacker, and the row
/// starts the fight. Dim with the reason when no group is in sight or nothing of theirs reaches.
#[test]
fn a_member_s_attack_in_the_column_opens_the_target_step_and_the_fight() {
    let mut app = common::app::in_the_yard((3, 1, Facing::East));
    let (pip, ilvara) = {
        let members = &app.world().resource::<Session>().views.party.members;
        (members[3].member, members[2].member)
    };
    let press = |app: &mut App, side: Option<Side>, pressable: Option<Pressable>| {
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
    press(&mut app, Some(Side(ControlId::Member(pip))), None);
    press(&mut app, Some(Side(ControlId::Open(Tool::Attack))), None);
    {
        let session = app.world().resource::<Session>();
        let screens = app.world().resource::<Screens>();
        assert_eq!(
            screens.nav.top(Some(&session.views)),
            Some(ScreenId::Actions)
        );
        assert_eq!(screens.forms.aim, Some(Aim::attack(pip)), "Pip attacks");
    }
    press(&mut app, None, Some(Pressable(ControlId::Target(0))));
    let session = app.world().resource::<Session>();
    assert!(
        session.views.combat.is_some(),
        "the shortbow opened the fight"
    );
    assert!(
        session.fight_log.iter().any(|l| l.starts_with("Pip ")),
        "{:?}",
        session.fight_log
    );

    // The reasons, as the column words them.
    let data = common::objects_data();
    let why = |x, y, facing, member| {
        let world = crate::loot::yard(&data, x, y, facing);
        let views = common::views(&world, &data);
        let context = ctx(&data, &world, &views);
        Tool::Attack.why_not(&data, context.game.unwrap(), member)
    };
    assert_eq!(why(3, 1, Facing::East, pip), None);
    // Behind the hedge no group is seen, but the chest in reach can be struck.
    assert_eq!(why(5, 4, Facing::North, pip), None);
    let mut bare = common::objects_data();
    for kind in bare.objects.values_mut() {
        kind.attributes.durability = None;
    }
    let world = crate::loot::yard(&bare, 5, 4, Facing::North);
    let views = common::views(&world, &bare);
    let context = ctx(&bare, &world, &views);
    assert_eq!(
        Tool::Attack
            .why_not(&bare, context.game.unwrap(), pip)
            .as_deref(),
        Some("No one in sight")
    );
    assert!(
        why(3, 1, Facing::East, ilvara).is_some(),
        "no bow, two tiles off"
    );
    // In a fight the turn's menu attacks, not the column.
    let mut world = crate::loot::yard(&data, 3, 1, Facing::East);
    let strike = Command::Attack {
        member: pip,
        target: Target::Being {
            being: "rat".into(),
        },
    };
    world.apply(&data, strike).expect("the strike");
    if world.combat(&data).is_some() {
        let views = common::views(&world, &data);
        let context = ctx(&data, &world, &views);
        assert_eq!(
            Tool::Attack
                .why_not(&data, context.game.unwrap(), pip)
                .as_deref(),
            Some("In a fight the turn's menu attacks")
        );
    }
    // The attacker handed over by the column is for the square's actions alone, and once.
    let mut forms = Forms::default();
    forms.attacker = Some(pip);
    forms.shown(Some(ScreenId::Cast), None);
    assert_eq!((forms.aim.clone(), forms.attacker), (None, None));
    forms.shown(Some(ScreenId::Actions), None);
    assert_eq!(forms.aim, None, "taken already");
    forms.shown(Some(ScreenId::Cast), None);
    forms.attacker = Some(pip);
    forms.shown(Some(ScreenId::Actions), None);
    assert_eq!(forms.aim, Some(Aim::attack(pip)));
}

/// The owner's report (2026-11-15): "the options menu brings up the attack option and seems to
/// list the objects available but I can't seem to target the objects with spells or weapons."
/// The attack's Choose on an object's row sent no command, and a blow at an object had no
/// line, so a bolt that landed showed nothing. Both rows send their command now, and every
/// blow is told.
#[test]
fn choosing_an_object_s_row_strikes_it_with_a_weapon_or_a_spell_and_the_blow_is_told() {
    use omnis_sim::Event;
    use omnis_vector::rolllog::Names;
    use omnis_vector::shell::text;
    let data = common::objects_data();
    let world = crate::loot::yard(&data, 2, 1, Facing::West);
    let views = common::views(&world, &data);
    let context = ctx(&data, &world, &views);
    let game = context.game.unwrap();
    let (brenna, ilvara) = (views.party.members[0].member, views.party.members[2].member);
    let crate_1 = Target::Object {
        object: "crate_1".into(),
    };
    let mut names = Names::default();
    names.observe(&views.party);
    names.locate(views.position.map);
    let bolt = Aim {
        member: ilvara,
        option: omnis_sim::Chosen::Spell {
            spell: "base:spell:fire_bolt".into(),
            pay: omnis_sim::Pay::default(),
        },
    };
    for (aim, who) in [(Aim::attack(brenna), "Brenna"), (bolt, "Ilvara")] {
        let rows = aim.targets(&data, game);
        let at = rows
            .iter()
            .position(|r| r.target == crate_1)
            .expect("listed");
        let command = aim
            .chosen(&data, game, u8::try_from(at).unwrap())
            .unwrap_or_else(|| panic!("{who}: Choose sends a command"));
        // Through the screen too: the square's actions for the weapon.
        if who == "Brenna" {
            let mut forms = Forms::default();
            forms.aim = Some(aim.clone());
            let id = ControlId::Target(u8::try_from(at).unwrap());
            let pressed = screen::apply(
                ScreenId::Actions,
                &context,
                &mut forms,
                id,
                &Payload::Press,
                0,
            );
            assert_eq!(
                pressed,
                Ok(Effect::Command {
                    command: command.clone(),
                    close: true
                })
            );
        }
        // Until a blow lands, so both the miss's and the hit's lines are read.
        let mut world = world.clone();
        let (mut hit, mut lines) = (false, Vec::new());
        for _ in 0..40 {
            let events = world.apply(&data, command.clone()).expect("a blow");
            for event in &events {
                if let Event::ObjectDamaged { amount, .. } = event {
                    let line = text::describe(event, &names, &data).expect("told");
                    assert!(line.starts_with(&format!("{who} ")), "{line}");
                    assert!(
                        line.contains("Crate") && line.contains("against AC 15"),
                        "{line}"
                    );
                    assert_eq!(line.contains("HP left"), *amount > 0, "{line}");
                    assert_eq!(line.contains(" misses "), *amount == 0, "{line}");
                    hit |= *amount > 0;
                    lines.push(line);
                }
                if matches!(event, Event::ObjectBroken { .. }) {
                    assert_eq!(
                        text::describe(event, &names, &data).as_deref(),
                        Some("Crate breaks")
                    );
                }
            }
            if hit {
                break;
            }
        }
        assert!(hit, "{who}: {lines:?}");
    }
    // A blow with no roll to hit, shrugged off and landed; and the breaking.
    let blow = |amount, left| Event::ObjectDamaged {
        object: "crate_1".into(),
        by: ilvara,
        attack: None,
        kind: omnis_sim::omnis_data::DamageType::Thunder,
        rolls: Vec::new(),
        raw: 7,
        amount,
        left,
    };
    let told = |event: &Event| text::describe(event, &names, &data);
    assert_eq!(
        told(&blow(0, 18)).as_deref(),
        Some("Ilvara strikes Crate: no damage")
    );
    assert_eq!(
        told(&blow(7, 11)).as_deref(),
        Some("Ilvara strikes Crate: 7 thunder, 11 HP left")
    );
    let broken = Event::ObjectBroken {
        object: "crate_1".into(),
    };
    assert_eq!(told(&broken).as_deref(), Some("Crate breaks"));
}
