//! The party column (presentation-ARCHITECTURE §6.6): each member in marching order under the
//! front and back words, their numbers from the views, the entry yellow under half its hit
//! points and red under a quarter, the acting member marked in a fight, a member selected by their entry, and the row
//! of screens under the selected member, each dim with its reason until it is built. Model
//! tests on the real packs, no window; the app's column is in `screens.rs`.

use crate::common;

use common::app::config;
use common::fight::turn_of;
use omnis_vector::screen::roster::{self, RosterEffect, Tool};
use omnis_vector::screen::{ControlId, Game, Panel, Refused};
use omnis_vector::shell::session::Session;
use omnis_vector::sim::Views;
use omnis_vector::style::Role;

fn start() -> Session {
    Session::start(&config()).expect("the shipped packs start a session")
}

fn game<'a>(session: &'a Session, views: &'a Views) -> Game<'a> {
    Game {
        world: &session.world,
        views,
        log: &[],
        unreplayable: None,
        history: &session.history,
        timing: session.binder.timing,
    }
}

/// Every line's words, spans then control labels.
fn lines(panel: &Panel) -> Vec<String> {
    panel
        .rows
        .iter()
        .map(|r| {
            r.spans
                .iter()
                .map(|s| s.text.clone())
                .chain(r.controls.iter().map(|c| c.label.clone()))
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect()
}

#[test]
fn each_member_shows_in_marching_order_with_their_numbers() {
    let session = start();
    let views = &session.views;
    let panel = roster::content(&session.data, game(&session, views), None);
    let text = lines(&panel);
    let members = &views.party.members;
    let mut at = 0;
    for m in members {
        let entry = text[at..]
            .iter()
            .position(|l| l.starts_with(&m.name))
            .unwrap_or_else(|| panic!("{} after line {at}: {text:?}", m.name))
            + at;
        let numbers = &text[entry + 1];
        assert!(
            numbers.starts_with(&format!("HP {}/{}", m.hp, m.hp_max)),
            "{numbers}"
        );
        assert!(
            numbers.contains(&format!(
                "SP {}/{}  AC {}",
                m.spell_points, m.spell_points_max, m.ac
            )),
            "{numbers}"
        );
        at = entry + 1;
    }
    // The fixed party stands three in front and one behind.
    let front = text.iter().position(|l| l == "front").expect("front");
    let back = text.iter().position(|l| l == "back").expect("back");
    let last = members.last().unwrap();
    assert!(front < back);
    assert!(text[back + 1].starts_with(&last.name), "{text:?}");
    assert!(text[1..].iter().filter(|l| *l == "front").count() == 1);
    // A class's name, not its id.
    assert!(text.iter().any(|l| l.contains("Fighter 1")), "{text:?}");
}

#[test]
fn an_entry_is_yellow_under_half_its_hit_points_and_red_under_a_quarter() {
    let session = start();
    let mut views = session.views.clone();
    let name = views.party.members[0].name.clone();
    // Brenna: 12 hit points, so under half is 5 or fewer and under a quarter 2 or fewer.
    assert_eq!(views.party.members[0].hp_max, 12);
    // The entry's name box and every span of its numbers line, but the condition word.
    let entry = |views: &Views| {
        let panel = roster::content(&session.data, game(&session, views), None);
        let at = lines(&panel)
            .iter()
            .position(|l| l.starts_with(&name))
            .unwrap();
        (panel.rows[at].tint, panel.rows[at + 1].spans.clone())
    };
    for (hp, role) in [
        (12, Role::Interface),
        (6, Role::Interface),
        (5, Role::Caution),
        (3, Role::Caution),
        (2, Role::Alert),
        (1, Role::Alert),
    ] {
        views.party.members[0].hp = hp;
        let (tint, spans) = entry(&views);
        assert_eq!(tint, Some(role), "the name box at {hp} hp");
        assert_eq!(spans[0].role, role, "HP at {hp} hp");
        assert_eq!(spans[1].role, role, "SP and AC at {hp} hp");
    }
    let m = &mut views.party.members[0];
    m.hp = 0;
    m.down = true;
    let (tint, spans) = entry(&views);
    assert_eq!(tint, Some(Role::Alert), "a down member is red");
    assert_eq!(spans[0].role, Role::Alert);
    assert_eq!(spans.last().unwrap().text, "down");
    let m = &mut views.party.members[0];
    m.down = false;
    m.dead = true;
    let (tint, spans) = entry(&views);
    assert_eq!(tint, Some(Role::Dim), "a dead member is dim");
    assert_eq!(spans[0].role, Role::Dim);
    assert_eq!(spans.last().unwrap().text, "dead");
}

#[test]
fn the_acting_member_is_marked_in_a_fight() {
    let session = turn_of("Durin");
    let panel = roster::content(&session.data, game(&session, &session.views), None);
    let text = lines(&panel);
    assert!(text.iter().any(|l| l.starts_with("> Durin")), "{text:?}");
    assert_eq!(text.iter().filter(|l| l.starts_with("> ")).count(), 1);
}

#[test]
fn an_entry_selects_its_member_and_the_screens_open_under_them() {
    let session = start();
    let g = game(&session, &session.views);
    let first = session.views.party.members[0].member;
    let second = session.views.party.members[1].member;
    let mut selected = None;
    assert_eq!(
        roster::apply(&session.data, g, &mut selected, ControlId::Member(first)),
        Ok(RosterEffect::None)
    );
    assert_eq!(selected, Some(first));
    let panel = roster::content(&session.data, g, selected);
    let text = lines(&panel);
    let at = text
        .iter()
        .position(|l| l == "Sheet | Inventory | Spells | Journal");
    let entry = text
        .iter()
        .position(|l| l.starts_with(&session.views.party.members[0].name));
    assert_eq!(
        at,
        entry.map(|e| e + 2),
        "the row under the selected member: {text:?}"
    );
    assert_eq!(text.iter().filter(|l| l.starts_with("Sheet")).count(), 1);
    // Another member moves the selection; the same one again clears it.
    roster::apply(&session.data, g, &mut selected, ControlId::Member(second)).unwrap();
    assert_eq!(selected, Some(second));
    roster::apply(&session.data, g, &mut selected, ControlId::Member(second)).unwrap();
    assert_eq!(selected, None);
    assert!(
        !lines(&roster::content(&session.data, g, None))
            .iter()
            .any(|l| l.starts_with("Sheet"))
    );
}

#[test]
fn each_screen_is_dim_with_its_reason_until_it_can_open() {
    let session = start();
    let g = game(&session, &session.views);
    let first = session.views.party.members[0].member;
    let mut selected = Some(first);
    let panel = roster::content(&session.data, g, selected);
    let reasons: Vec<_> = panel
        .rows
        .iter()
        .flat_map(|r| &r.controls)
        .filter(|c| matches!(c.id, ControlId::Open(_)))
        .map(|c| (c.label.clone(), c.why_not.clone()))
        .collect();
    assert_eq!(
        reasons,
        [
            ("Sheet".into(), None),
            ("Inventory".into(), None),
            ("Spells".into(), Some("No spells for the road".into())),
            ("Journal".into(), Some("No quests yet".into())),
        ]
    );
    assert_eq!(
        roster::apply(
            &session.data,
            g,
            &mut selected,
            ControlId::Open(Tool::Journal)
        ),
        Err(Refused::Dim("No quests yet".into()))
    );
    assert_eq!(
        roster::apply(
            &session.data,
            g,
            &mut selected,
            ControlId::Open(Tool::Inventory)
        ),
        Ok(RosterEffect::Open {
            tool: Tool::Inventory,
            member: first,
            read_only: false
        })
    );
    // The cleric has spells for the road.
    let durin = session.views.party.members[1].member;
    let mut cleric = Some(durin);
    assert_eq!(
        roster::apply(&session.data, g, &mut cleric, ControlId::Open(Tool::Spells)),
        Ok(RosterEffect::Open {
            tool: Tool::Spells,
            member: durin,
            read_only: false
        })
    );
    // Nothing selected: no open row, so nothing to press.
    let mut none = None;
    assert_eq!(
        roster::apply(&session.data, g, &mut none, ControlId::Open(Tool::Sheet)),
        Err(Refused::NotHere)
    );
}

#[test]
fn a_selection_is_dropped_when_its_member_is_not_in_the_party() {
    let session = start();
    let mut views = session.views.clone();
    let gone = views.party.members.remove(0).member;
    let kept = views.party.members[0].member;
    let g = game(&session, &views);
    assert_eq!(roster::settle(g, Some(gone)), None);
    assert_eq!(roster::settle(g, Some(kept)), Some(kept));
}
