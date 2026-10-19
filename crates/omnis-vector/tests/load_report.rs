//! The window's load examines the save first (M8.5 7h4): a save that loads, loads at once, by
//! name on other packs, each place the loaded maps no longer allow a line in the log; a save that
//! does not opens the Load screen, which lists why and closes back where Load was pressed.
//! The Debug page's Reload packs (M8.5 7h5) fits the running world to the packs on disk by
//! name, and opens the same screen when they lack a name the world holds.

use crate::common::app::{config, set, shell, tap};
use crate::common::{map, objects_data, place, town_session};
use bevy::prelude::*;
use omnis_sim::api::{Missing, NameKind, SaveCheck, Verdict};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::Data;
use omnis_vector::game_io;
use omnis_vector::screen::debug::{DebugAct, Page};
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
    let loaded = opened.load(&other, false).unwrap();
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
    let loaded = game_io::load(&other, &root, "yard.ron", false, false).unwrap();
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
    assert!(game_io::load(&data, &root, "v7.ron", false, false).is_err());
    let loaded =
        game_io::load(&data, &root, "v7.ron", true, false).unwrap_or_else(|e| panic!("{e}"));
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
    forms.load_report = Some(load_report::Refusal {
        check: SaveCheck {
            verdict: Verdict::Unresolved,
            missing,
            ..SaveCheck::default()
        },
        reload: false,
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

/// In a game on `ahead` or not, the Debug page open on its World tab, the packs on disk now
/// `then`'s: what Reload packs reads.
fn on_the_world_page(ahead: bool, then: bool) -> App {
    let mut app = app_on(ahead, ".omnis/test-saves/none.ron", true);
    app.update();
    app.update();
    app.world_mut().resource_mut::<Screens>().forms.debug.page = Page::World;
    tap(&mut app, KeyCode::Backquote);
    assert_eq!(top(&app), Some(ScreenId::Debug));
    app.world_mut().resource_mut::<Session>().config.packs = packs(then);
    app
}

#[test]
fn reload_packs_fits_the_running_world_by_name() {
    let mut app = on_the_world_page(false, true);
    let party = app.world().resource::<Session>().views.party.clone();
    press(&mut app, ControlId::Debug(DebugAct::Reload));
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Debug), "the page stays");
    let session = app.world().resource::<Session>();
    assert!(
        session.data.fingerprints.iter().any(|p| p.id == "ahead"),
        "the game runs on the fresh packs"
    );
    assert_eq!(session.views.party, party, "every name the same");
    assert!(
        session
            .lines
            .iter()
            .any(|l| l == "Packs reloaded: loaded by name. The world has moved on"),
        "{:?}",
        session.lines
    );
    assert!(session.unreplayable.is_some());
}

#[test]
fn reload_packs_lacking_a_carried_name_opens_the_load_screen_and_changes_nothing() {
    let mut app = on_the_world_page(true, false);
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let id = session
            .data
            .registry
            .items
            .get("ahead:item:dagger")
            .unwrap();
        session.world.party.inventory.push((id, 1));
        session.refresh();
    }
    let packs = app.world().resource::<Session>().data.fingerprints.clone();
    let party = app.world().resource::<Session>().views.party.clone();
    press(&mut app, ControlId::Debug(DebugAct::Reload));
    app.update();
    assert_eq!(top(&app), Some(ScreenId::LoadReport));
    let text = screen_text(app.world());
    for line in [
        "These packs do not load",
        "The world names what the reloaded packs no longer have. Nothing changed.",
        "ahead: running 0.1.1, reloaded none",
        "item 'ahead:item:dagger' in party.inventory",
    ] {
        assert!(text.contains(line), "{line}: {text}");
    }
    assert!(text.is_ascii(), "{text}");
    let session = app.world().resource::<Session>();
    assert_eq!(session.data.fingerprints, packs, "nothing changed");
    assert_eq!(session.views.party, party);
    press(&mut app, ControlId::Close);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Debug), "back to the page");
}

/// What a reload says in the log, for each verdict that takes it.
#[test]
fn a_reload_says_how_it_fitted_and_each_place() {
    use omnis_sim::api::{PlaceFix, PlaceWarning, Spot};
    let at = |x, y| Spot {
        level: Level::GROUND,
        x,
        y,
    };
    let said = |verdict, places: Vec<PlaceWarning>| {
        game_io::reloaded(&SaveCheck {
            verdict,
            places,
            ..SaveCheck::default()
        })
    };
    assert_eq!(
        said(Verdict::Same, Vec::new()),
        ["Packs reloaded: the same packs"]
    );
    assert_eq!(
        said(Verdict::Renamed, Vec::new()),
        ["Packs reloaded: loaded by name. The world has moved on"]
    );
    let moved = PlaceWarning {
        map: "objects:map:yard".into(),
        fix: PlaceFix::PartyMoved {
            from: at(2, 2),
            to: at(2, 1),
        },
    };
    assert_eq!(
        said(Verdict::Content, vec![moved.clone()]),
        [
            "Packs reloaded: other packs, every name the same; 1 place changed, see the log",
            "The party could not stand at objects:map:yard (2, 2); moved to (2, 1)."
        ]
    );
    assert_eq!(
        said(Verdict::Content, vec![moved.clone(), moved])[0],
        "Packs reloaded: other packs, every name the same; 2 places changed, see the log"
    );
}

/// Base, test, objects and the `chain` fixture at `version` (`v1` is 0.1.0, `v3` 0.3.0 with
/// its two migrations, M8.5 7h6).
fn chain(version: &str) -> Vec<PathBuf> {
    let mut packs = packs(false);
    packs.push(repo().join(format!("crates/omnis-sim/tests/packs/chain/{version}")));
    packs
}

/// The old sword in the party's stores, in the running game.
fn with_the_old_sword(app: &mut App) {
    let mut session = app.world_mut().resource_mut::<Session>();
    let id = session
        .data
        .registry
        .items
        .get("chain:item:old_sword")
        .unwrap();
    session.world.party.inventory.push((id, 1));
    session.refresh();
}

fn holds_the_sword(app: &App) -> bool {
    let session = app.world().resource::<Session>();
    session
        .views
        .party
        .inventory
        .iter()
        .any(|i| i.item == "chain:item:sword")
}

/// A save the packs' migrations change opens the Load screen to ask: the steps listed, Close
/// loading nothing; Apply and load loads it through them.
#[test]
fn a_save_the_migrations_change_asks_then_applies() {
    let save = Scratch::new("load-migrate");
    {
        let mut app = shell(
            Config {
                packs: chain("v1"),
                save: save.0.clone(),
                ..config()
            },
            true,
        );
        app.update();
        with_the_old_sword(&mut app);
        let session = app.world().resource::<Session>();
        let data = session.data.clone();
        game_io::save(
            &session.world,
            &data,
            &repo().join("crates/omnis-vector"),
            &save.0,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    }
    let mut app = shell(
        Config {
            packs: chain("v3"),
            save: save.0.clone(),
            ..config()
        },
        false,
    );
    app.update();
    press(&mut app, ControlId::Load);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::LoadReport));
    let text = screen_text(app.world());
    for line in [
        "These packs moved on",
        "The packs changed since this save, and their migrations change it.",
        "chain: saved 0.1.0, loaded 0.3.0",
        "Migrations:",
        "chain 0001.ron: item 'chain:item:old_sword' is now 'chain:item:mid_sword'",
        "chain 0002.ron: map 'chain:map:hall': the cell (0, 2, 1) moved to (0, 3, 1)",
        "Apply and load",
    ] {
        assert!(text.contains(line), "{line}: {text}");
    }
    assert!(text.is_ascii(), "{text}");
    press(&mut app, ControlId::Close);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Title));
    assert!(
        app.world().get_resource::<Session>().is_none(),
        "Close loads nothing"
    );
    press(&mut app, ControlId::Load);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::LoadReport));
    press(&mut app, ControlId::Apply);
    app.update();
    assert_ne!(top(&app), Some(ScreenId::LoadReport));
    assert!(holds_the_sword(&app), "loaded through the migrations");
    let session = app.world().resource::<Session>();
    assert!(
        session
            .lines
            .iter()
            .any(|l| l.contains("made with older packs, their migrations applied")),
        "{:?}",
        session.lines
    );
}

/// Reload packs whose migrations change the running world asks the same way; Apply and reload
/// reloads through them and goes back to the Debug page.
#[test]
fn reload_packs_asks_then_applies_the_migrations() {
    let mut app = shell(
        Config {
            packs: chain("v1"),
            save: ".omnis/test-saves/none.ron".into(),
            ..config()
        },
        true,
    );
    app.update();
    app.update();
    app.world_mut().resource_mut::<Screens>().forms.debug.page = Page::World;
    tap(&mut app, KeyCode::Backquote);
    app.world_mut().resource_mut::<Session>().config.packs = chain("v3");
    with_the_old_sword(&mut app);
    press(&mut app, ControlId::Debug(DebugAct::Reload));
    app.update();
    assert_eq!(top(&app), Some(ScreenId::LoadReport));
    let text = screen_text(app.world());
    for line in [
        "These packs moved on",
        "The reloaded packs' migrations change the running world.",
        "chain: running 0.1.0, reloaded 0.3.0",
        "Apply and reload",
    ] {
        assert!(text.contains(line), "{line}: {text}");
    }
    assert!(!holds_the_sword(&app), "nothing changed yet");
    press(&mut app, ControlId::Apply);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Debug), "back to the page");
    assert!(holds_the_sword(&app));
    let session = app.world().resource::<Session>();
    assert!(
        session
            .lines
            .iter()
            .any(|l| l == "Packs reloaded: their migrations applied. The world has moved on"),
        "{:?}",
        session.lines
    );
}
