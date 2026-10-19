//! The window's load examines the save first (M8.5 7h4): a save that loads, loads at once, by
//! name on other packs, each place the loaded maps no longer allow a line in the log; a save that
//! does not opens the Load screen, which lists why and closes back where Load was pressed.

use crate::common::app::{config, set, shell};
use crate::common::{map, objects_data, place, town_session};
use bevy::prelude::*;
use omnis_sim::api::{Missing, NameKind, SaveCheck, Verdict};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::Data;
use omnis_vector::game_io;
use omnis_vector::screen::{ControlId, Forms, ScreenId, load_report};
use omnis_vector::shell::screen::{Pressable, Screens};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::{Config, Session};
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Base, test and the objects fixture, with `ahead` (whose every definition sorts first) when
/// asked.
fn packs(ahead: bool) -> Vec<PathBuf> {
    let root = repo();
    let mut packs = vec![
        root.join("packs/base"),
        root.join("packs/test"),
        root.join("crates/omnis-sim/tests/packs/objects"),
    ];
    if ahead {
        packs.push(root.join("crates/omnis-sim/tests/packs/ahead"));
    }
    packs
}

fn app_on(ahead: bool, save: &str, in_game: bool) -> App {
    shell(
        Config {
            packs: packs(ahead),
            save: save.to_owned(),
            ..config()
        },
        in_game,
    )
}

/// A save path for one test, removed when the test ends with its world folder left to tidy.
struct Scratch(String);

impl Scratch {
    fn new(name: &str) -> Scratch {
        Scratch(format!(
            ".omnis/test-saves/{name}-{}.ron",
            std::process::id()
        ))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn press(app: &mut App, id: ControlId) {
    set(app, &Pressable(id), Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut buttons = world.query::<(&Pressable, &mut Interaction)>();
    for (p, mut i) in buttons.iter_mut(world) {
        if p.0 == id {
            *i = Interaction::None;
        }
    }
}

fn top(app: &App) -> Option<ScreenId> {
    let game = app.world().get_resource::<Session>().map(|s| &s.views);
    app.world().resource::<Screens>().nav.top(game)
}

/// Save a game on `ahead` or not, holding `ahead`'s dagger when asked.
fn saved(ahead: bool, dagger: bool, save: &str) {
    let mut app = app_on(ahead, save, true);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        if dagger {
            let id = session
                .data
                .registry
                .items
                .get("ahead:item:dagger")
                .unwrap();
            session.world.party.inventory.push((id, 1));
            session.refresh();
        }
        let data = session.data.clone();
        game_io::save(
            &session.world,
            &data,
            &repo().join("crates/omnis-vector"),
            save,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn a_save_naming_what_the_packs_lack_opens_the_load_screen_and_closes_to_the_title() {
    let save = Scratch::new("load-refused");
    saved(true, true, &save.0);
    let mut app = app_on(false, &save.0, false);
    app.update();
    press(&mut app, ControlId::Load);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::LoadReport));
    assert!(
        app.world().get_resource::<Session>().is_none(),
        "nothing loaded"
    );
    let text = screen_text(app.world());
    for line in [
        "This save does not load",
        "It names what the loaded packs no longer have.",
        "ahead: saved 0.1.1, loaded none",
        "item 'ahead:item:dagger' in party.inventory",
        "[button]",
    ] {
        assert!(text.contains(line), "{line}: {text}");
    }
    press(&mut app, ControlId::Close);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Title));
    assert!(
        app.world()
            .resource::<Screens>()
            .forms
            .load_report
            .is_none()
    );
}

#[test]
fn a_save_on_other_packs_loads_by_name_at_once() {
    let save = Scratch::new("load-by-name");
    saved(false, false, &save.0);
    let mut app = app_on(true, &save.0, false);
    app.update();
    press(&mut app, ControlId::Load);
    app.update();
    assert_eq!(top(&app), None, "in the game");
    let session = app.world().resource::<Session>();
    assert!(
        session
            .lines
            .iter()
            .any(|l| l.contains("made with other packs, loaded by name. The world has moved on")),
        "{:?}",
        session.lines
    );
}

/// `data` with every pack's version raised and the yard's cell `(x, y)` without ground.
fn edited(data: &Data, x: u16, y: u16) -> Data {
    let mut data = data.clone();
    for pack in &mut data.fingerprints {
        pack.version.push_str("-edited");
    }
    let yard = map(&data, "objects:map:yard");
    let map = data.maps.get_mut(&yard).unwrap();
    let width = usize::from(map.def.width);
    let level = map
        .levels
        .iter_mut()
        .find(|l| l.level == Level::GROUND)
        .unwrap();
    level.cells[usize::from(y) * width + usize::from(x)] = None;
    data
}

#[test]
fn a_place_the_maps_no_longer_allow_loads_with_a_line_for_it() {
    let data = objects_data();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vector-load-places");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let (mut world, _) = town_session(&data);
    place(
        &mut world,
        map(&data, "objects:map:yard"),
        2,
        2,
        Facing::East,
    );
    game_io::save(&world, &data, &root, "yard.ron").unwrap();
    let other = edited(&data, 2, 2);
    let opened = game_io::examine(&other, &root, "yard.ron").unwrap();
    assert!(opened.report().verdict.loads());
    let loaded = opened.load(&other).unwrap();
    assert!(
        loaded
            .said
            .ends_with("1 place changed, see the log; made with other packs, every name the same"),
        "{}",
        loaded.said
    );
    assert_eq!(
        loaded.warnings,
        ["The party could not stand at objects:map:yard (2, 2); moved to (2, 1)."]
    );
    assert_eq!(
        loaded.lines("yard.ron")[1],
        "The party could not stand at objects:map:yard (2, 2); moved to (2, 1)."
    );
    let loaded = game_io::load(&other, &root, "yard.ron", false).unwrap();
    assert_eq!(loaded.warnings.len(), 1, "the socket's load says it too");
}

/// A whole-world save made on packs since resealed keeps no names: refused, and through the
/// socket's forced load read as written.
#[test]
fn a_save_without_names_loads_only_forced() {
    let data = crate::common::data();
    let root = repo().join("crates/omnis-sim/tests/saves");
    let opened = game_io::examine(&data, &root, "v7.ron").unwrap();
    assert_eq!(opened.report().verdict, Verdict::Unnamed);
    assert!(game_io::load(&data, &root, "v7.ron", false).is_err());
    let loaded = game_io::load(&data, &root, "v7.ron", true).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        loaded.said.ends_with("loaded as written"),
        "{}",
        loaded.said
    );
}

#[test]
fn the_load_screen_shows_twelve_of_a_list_and_counts_the_rest() {
    let missing = (0..15)
        .map(|i| Missing {
            kind: NameKind::Item,
            name: format!("x:item:{i}"),
            at: "party.inventory".into(),
        })
        .collect();
    let mut forms = Forms::default();
    forms.load_report = Some(SaveCheck {
        verdict: Verdict::Unresolved,
        missing,
        ..SaveCheck::default()
    });
    let screen = load_report::content(&forms);
    let texts: Vec<String> = screen.panels[0]
        .rows
        .iter()
        .filter_map(|r| r.span(0).map(|s| s.text.clone()))
        .collect();
    assert_eq!(
        texts
            .iter()
            .filter(|t| t.starts_with("item 'x:item:"))
            .count(),
        12
    );
    assert!(
        texts.contains(&"and 3 more; save_check lists them all".to_owned()),
        "{texts:?}"
    );
    assert!(
        texts.iter().all(|t| t.is_ascii()) && screen.title.text.is_ascii(),
        "the built-in face draws ASCII only: {texts:?}"
    );
}
