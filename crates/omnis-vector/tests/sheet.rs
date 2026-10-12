//! The character sheet (parity P6; presentation-ARCHITECTURE §6.7): three pages of one member's
//! numbers, each the member view's, word for word; the member and the page step; Tactics opens
//! over the sheet outside a fight. In the app: the party column's Sheet opens it beside the
//! column, and Esc steps back from Tactics to the sheet, then to the street.

use crate::common;

use bevy::prelude::*;
use common::app::{app_with, press_ui, tap, top};
use common::ctx;
use omnis_sim::api::MemberView;
use omnis_sim::omnis_data::{Ability, Data, EquipSlot};
use omnis_sim::{Command, Target};
use omnis_vector::labels;
use omnis_vector::screen::roster::Tool;
use omnis_vector::screen::sheet::Page;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, Screen, ScreenId};
use omnis_vector::shell::screen::{Beside, Pressable, Side};
use omnis_vector::shell::session::Session;
use omnis_vector::sim::SimWorld;

fn forms(world: &SimWorld, slot: u8, page: Page) -> Forms {
    let mut forms = Forms::default();
    forms.owner = Some(common::id(world, slot));
    forms.page = page;
    forms
}

fn shown(data: &Data, world: &SimWorld, forms: &Forms) -> Screen {
    let views = common::views(world, data);
    screen::content(ScreenId::Sheet, &ctx(data, world, &views), forms)
}

fn press(
    data: &Data,
    world: &SimWorld,
    forms: &mut Forms,
    id: ControlId,
    payload: &Payload,
) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    screen::apply(
        ScreenId::Sheet,
        &ctx(data, world, &views),
        forms,
        id,
        payload,
        0,
    )
}

/// Every row's text, its spans then its after-text, two spaces apart.
fn lines(screen: &Screen) -> Vec<String> {
    screen.panels[0]
        .rows
        .iter()
        .map(|r| {
            r.spans
                .iter()
                .chain(&r.after)
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect()
}

fn has(lines: &[String], line: &str) {
    assert!(lines.iter().any(|l| l == line), "{line:?} in {lines:#?}");
}

fn member(data: &Data, world: &SimWorld, slot: usize) -> MemberView {
    common::views(world, data).party.members[slot].clone()
}

#[test]
fn the_stats_page_is_the_member_view_word_for_word() {
    let data = common::data();
    let (world, _) = common::session(&data);
    let m = member(&data, &world, 0);
    let page = lines(&shown(&data, &world, &forms(&world, 0, Page::Stats)));
    has(
        &page,
        &format!(
            "{} {} {}  member 1 of 4",
            labels::race(&data, &m.race),
            labels::class(&data, &m.class),
            m.level
        ),
    );
    has(
        &page,
        &format!(
            "HP {}/{}  SP {}/{}  AC {}  Proficiency +{}",
            m.hp, m.hp_max, m.spell_points, m.spell_points_max, m.ac, m.proficiency
        ),
    );
    has(&page, &format!("XP {} / {}", m.xp, m.next_xp.unwrap()));
    has(
        &page,
        &format!("Hit dice {}/{} d{}", m.hit_dice_left, m.hit_dice, m.hit_die),
    );
    let score = |a: Ability| {
        format!(
            "{} {} {}",
            a.short(),
            m.scores[a.index()],
            labels::signed(m.modifiers[a.index()])
        )
    };
    has(
        &page,
        &[Ability::Strength, Ability::Dexterity, Ability::Constitution]
            .map(score)
            .join("  "),
    );
    let saves: Vec<String> = m
        .saves
        .iter()
        .map(|(a, b)| format!("{} {}", a.short(), labels::signed(*b)))
        .collect();
    has(&page, &format!("Saves  {}", saves.join("  ")));
    has(&page, "Skills");
    let first = &m.skills[0];
    assert!(
        page.iter().any(|l| l.starts_with(&format!(
            "{} {}",
            labels::skill(first.0),
            labels::signed(first.1)
        ))),
        "{page:#?}"
    );
    has(&page, "Conditions  none");
    has(
        &page,
        &format!(
            "{}  {}  {} years",
            labels::background(&data, &m.background),
            labels::alignment(m.alignment),
            m.age_years
        ),
    );
}

#[test]
fn the_stats_page_says_max_ready_and_picks_when_they_apply() {
    let data = common::data();
    let (mut world, _) = common::session(&data);
    world.party.members[0].level = 20;
    world.party.members[1].xp = 100_000;
    world.party.members[1].spell_picks = 2;
    let top = lines(&shown(&data, &world, &forms(&world, 0, Page::Stats)));
    let m = member(&data, &world, 0);
    assert_eq!(m.next_xp, None, "level 20 is the top");
    has(&top, &format!("XP {} / max", m.xp));
    let durin = lines(&shown(&data, &world, &forms(&world, 1, Page::Stats)));
    let d = member(&data, &world, 1);
    assert!(d.ready, "Durin has the experience for a level");
    has(
        &durin,
        &format!(
            "Hit dice {}/{} d{}  Ready to train  Spell picks 2",
            d.hit_dice_left, d.hit_dice, d.hit_die
        ),
    );
}

#[test]
fn the_magic_page_prices_each_known_spell_as_the_view_does_and_lists_effects() {
    let data = common::data();
    let (mut world, _) = common::session(&data);
    let durin = common::id(&world, 1);
    world
        .apply(
            &data,
            Command::Cast {
                caster: durin,
                spell: "base:spell:light".to_owned(),
                target: Target::Member(durin),
            },
        )
        .expect("Light");
    let d = member(&data, &world, 1);
    let page = lines(&shown(&data, &world, &forms(&world, 1, Page::Magic)));
    has(
        &page,
        &format!(
            "Casting Wisdom  Points {}/{}",
            d.spell_points, d.spell_points_max
        ),
    );
    assert!(!d.known_spells.is_empty());
    for k in &d.known_spells {
        let cost = if k.cost == 0 {
            "cantrip".to_owned()
        } else {
            format!("{} pt", k.cost)
        };
        has(
            &page,
            &format!(
                "{}  {cost}  {}",
                labels::spell(&data, &k.spell),
                labels::pay(k.pay)
            ),
        );
    }
    has(&page, "Cure Wounds  1 pt  action");
    let light = &common::views(&world, &data).party.effects[0];
    has(
        &page,
        &format!("Light  {} min", light.minutes_left.expect("Light lasts")),
    );

    let brenna = lines(&shown(&data, &world, &forms(&world, 0, Page::Magic)));
    has(&brenna, "No spellcasting");
    assert_eq!(
        brenna.iter().filter(|l| *l == "none").count(),
        1,
        "no spells; the party's light is on her too: {brenna:#?}"
    );
}

#[test]
fn the_gear_page_lists_the_slots_in_order_and_the_kit() {
    let data = common::data();
    let (world, _) = common::session(&data);
    let m = member(&data, &world, 0);
    let page = lines(&shown(&data, &world, &forms(&world, 0, Page::Gear)));
    has(&page, "Worn and wielded");
    for slot in EquipSlot::ALL {
        let item = m
            .worn
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or("-", |(_, id)| labels::item(&data, id));
        has(&page, &format!("{}  {item}", labels::slot(slot)));
    }
    has(&page, "Main hand  Longsword");
    has(&page, "Carried");
    let carried: Vec<String> = m
        .equipment
        .iter()
        .map(|i| labels::count_label(labels::item(&data, &i.item), i.count))
        .collect();
    for item in &carried {
        assert!(
            page.iter().any(|l| l.contains(item.as_str())),
            "{item} in {page:#?}"
        );
    }
}

#[test]
fn the_member_and_the_page_step_and_tactics_opens_outside_a_fight() {
    let data = common::data();
    let (world, _) = common::session(&data);
    let mut f = forms(&world, 0, Page::Stats);
    press(&data, &world, &mut f, ControlId::Owner, &Payload::Press).unwrap();
    assert_eq!(f.owner, Some(common::id(&world, 1)));
    press(&data, &world, &mut f, ControlId::Owner, &Payload::Left).unwrap();
    assert_eq!(f.owner, Some(common::id(&world, 0)));
    press(&data, &world, &mut f, ControlId::Page, &Payload::Left).unwrap();
    assert_eq!(f.page, Page::Gear, "left from the first page wraps round");
    press(&data, &world, &mut f, ControlId::Page, &Payload::Press).unwrap();
    assert_eq!(f.page, Page::Stats);
    assert_eq!(
        press(&data, &world, &mut f, ControlId::Tactics, &Payload::Press),
        Ok(Effect::Open(ScreenId::Tactics))
    );
    assert_eq!(
        press(&data, &world, &mut f, ControlId::Close, &Payload::Press),
        Ok(Effect::Back)
    );

    let session = common::fight::met();
    let views = session.views.clone();
    let mut f = Forms::default();
    let c = ctx(&session.data, &session.world, &views);
    let sheet = screen::content(ScreenId::Sheet, &c, &f);
    assert_eq!(
        sheet
            .control(ControlId::Tactics)
            .unwrap()
            .why_not
            .as_deref(),
        Some("Not in a fight")
    );
    assert_eq!(
        screen::apply(
            ScreenId::Sheet,
            &c,
            &mut f,
            ControlId::Tactics,
            &Payload::Press,
            0
        ),
        Err(Refused::Dim("Not in a fight".to_owned()))
    );
}

#[test]
fn the_column_opens_the_sheet_beside_it_and_esc_steps_back_from_tactics() {
    let mut app = app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    let (durin, here) = {
        let s = app.world().resource::<Session>();
        (s.views.party.members[1].member, s.views.position)
    };
    press_ui(&mut app, Side(ControlId::Member(durin)));
    press_ui(&mut app, Side(ControlId::Open(Tool::Sheet)));
    assert_eq!(top(&app), Some(ScreenId::Sheet));
    assert!(
        app.world().resource::<Beside>().0 > 0.0,
        "beside the column"
    );
    let screens = app
        .world()
        .resource::<omnis_vector::shell::screen::Screens>();
    assert_eq!(screens.forms.owner, Some(durin));

    common::app::key(&mut app, KeyCode::KeyW, bevy::input::ButtonState::Pressed);
    for _ in 0..30 {
        app.update();
    }
    common::app::key(&mut app, KeyCode::KeyW, bevy::input::ButtonState::Released);
    app.update();
    assert_eq!(
        app.world().resource::<Session>().views.position,
        here,
        "no walking"
    );

    press_ui(&mut app, Pressable(ControlId::Tactics));
    assert_eq!(top(&app), Some(ScreenId::Tactics));
    tap(&mut app, KeyCode::Escape);
    assert_eq!(
        top(&app),
        Some(ScreenId::Sheet),
        "Esc steps back to the sheet"
    );
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None);
}
