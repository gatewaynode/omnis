//! The player's settings (parity P8, presentation-ARCHITECTURE §10.5): the file read leniently and
//! written on Apply, the screen's draft dropped by Close and Esc, the screen opened from the
//! title and the pause, and what each setting changes: the floor grid, the quality preset on the
//! cameras. Reduced motion's turn and drift are unit tests in `shell/movement.rs`.

use crate::common;

use bevy::camera::Hdr;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use common::app::{config, press_ui, shell, tap};
use omnis_vector::geometry::{SegKind, extract};
use omnis_vector::look::LevelOnStep;
use omnis_vector::screen::settings::Field;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, ScreenId};
use omnis_vector::settings::{self as prefs, Face, Prefs, Quality};
use omnis_vector::shell::screen::{Arrow, Pressable, Screens};
use omnis_vector::shell::session::{Config, Session};
use omnis_vector::shell::settings::Current;
use std::path::Path;

/// A settings path for one test, under the crate's `.omnis/`, removed when the test ends.
struct Scratch(String);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let s = Scratch(format!(
            ".omnis/test-saves/settings-{name}-{}.ron",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&s.0);
        s
    }

    fn write(&self, text: &str) {
        std::fs::create_dir_all(".omnis/test-saves").expect("the test directory");
        std::fs::write(&self.0, text).expect("the test file");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn read(s: &Scratch) -> (Prefs, Option<String>) {
    prefs::read(Path::new("."), &s.0)
}

#[test]
fn a_missing_file_is_the_first_start_the_defaults_with_no_note() {
    let s = Scratch::new("missing");
    assert_eq!(read(&s), (Prefs::default(), None));
    let d = Prefs::default();
    assert!(!d.reduced_motion && d.grid);
    assert_eq!(
        (d.quality, d.font, d.scale),
        (Quality::High, Face::BuiltIn, 100)
    );
}

#[test]
fn the_settings_written_read_back() {
    let s = Scratch::new("round");
    let p = Prefs {
        reduced_motion: true,
        grid: false,
        quality: Quality::Low,
        font: Face::Alegreya,
        scale: 150,
        level_on_step: LevelOnStep::Never,
    };
    prefs::write(Path::new("."), &s.0, p).expect("written");
    assert_eq!(read(&s), (p, None));
}

#[test]
fn level_view_on_step_steps_through_its_three_choices_and_an_older_file_reads_as_the_default() {
    let s = Scratch::new("level");
    s.write("(reduced_motion: true)");
    assert_eq!(read(&s).0.level_on_step, LevelOnStep::Unlocked, "no key");
    s.write("(level_on_step: \"sideways\")");
    assert_eq!(
        read(&s).0.level_on_step,
        LevelOnStep::Unlocked,
        "an unknown word"
    );
    s.write("(level_on_step: \"always\")");
    assert_eq!(read(&s).0.level_on_step, LevelOnStep::Always);

    let data = common::data();
    let ctx = title_ctx(&data, Prefs::default());
    let mut forms = Forms::default();
    let level = ControlId::Setting(Field::Level);
    let mut seen = Vec::new();
    for _ in 0..3 {
        screen::apply(
            ScreenId::Settings,
            &ctx,
            &mut forms,
            level,
            &Payload::Right,
            0,
        )
        .expect("the choice steps");
        seen.push(forms.settings.expect("a draft").level_on_step);
    }
    assert_eq!(
        seen,
        [
            LevelOnStep::Always,
            LevelOnStep::Never,
            LevelOnStep::Unlocked
        ]
    );
    let shown = screen::content(ScreenId::Settings, &ctx, &forms);
    assert_eq!(
        shown.control(level).map(|c| c.label.clone()),
        Some("Level view on step".to_owned())
    );
}

#[test]
fn an_edited_file_keeps_what_it_can_unknown_keys_ignored_numbers_clamped() {
    let s = Scratch::new("edited");
    s.write("(scale: 333, colour: \"red\", font: \"inter\", quality: \"ultra\")");
    let (p, note) = read(&s);
    assert_eq!(note, None);
    assert_eq!(
        p,
        Prefs {
            scale: 200,
            font: Face::Inter,
            ..Prefs::default()
        }
    );
    s.write("(scale: 7)");
    assert_eq!(read(&s).0.scale, 50);
    s.write("(scale: 105)");
    assert_eq!(read(&s).0.scale, 110);
}

#[test]
fn a_file_that_cannot_be_read_gives_the_defaults_and_a_note() {
    let s = Scratch::new("malformed");
    s.write("((( not settings");
    let (p, note) = read(&s);
    assert_eq!(p, Prefs::default());
    assert!(
        note.is_some_and(|n| n.contains("not used")),
        "a note for the log"
    );
    let refused = prefs::read(Path::new("."), "../outside.ron");
    assert_eq!(refused.0, Prefs::default());
    assert!(refused.1.is_some(), "a path outside the tree is refused");
}

#[cfg(unix)]
#[test]
fn a_linked_settings_file_is_refused() {
    let target = Scratch::new("target");
    target.write("(scale: 150)");
    let link = Scratch::new("link");
    std::os::unix::fs::symlink(
        std::fs::canonicalize(&target.0).expect("the target"),
        &link.0,
    )
    .expect("a link");
    let (p, note) = read(&link);
    assert_eq!(p, Prefs::default());
    assert!(note.is_some(), "the link is refused with a note");
}

/// The settings screen's model at the title.
fn title_ctx(data: &omnis_sim::omnis_data::Data, prefs: Prefs) -> screen::Ctx<'_> {
    screen::Ctx {
        data,
        game: None,
        save: screen::Slot {
            path: "saves/none.ron",
            exists: false,
        },
        prefs,
        fits: omnis_vector::settings::SCALE_MAX,
    }
}

#[test]
fn apply_is_dim_until_a_setting_changes_and_then_sends_the_draft() {
    let data = common::data();
    let ctx = title_ctx(&data, Prefs::default());
    let mut forms = Forms::default();
    let shown = screen::content(ScreenId::Settings, &ctx, &forms);
    assert_eq!(
        shown
            .control(ControlId::Apply)
            .and_then(|c| c.why_not.clone()),
        Some("nothing changed".to_owned())
    );
    let quality = ControlId::Setting(Field::Quality);
    screen::apply(
        ScreenId::Settings,
        &ctx,
        &mut forms,
        quality,
        &Payload::Left,
        0,
    )
    .expect("the quality steps");
    let scale = ControlId::Setting(Field::Scale);
    screen::apply(
        ScreenId::Settings,
        &ctx,
        &mut forms,
        scale,
        &Payload::Right,
        0,
    )
    .expect("the scale steps");
    let shown = screen::content(ScreenId::Settings, &ctx, &forms);
    assert_eq!(
        shown
            .control(ControlId::Apply)
            .and_then(|c| c.why_not.clone()),
        None
    );
    let effect = screen::apply(
        ScreenId::Settings,
        &ctx,
        &mut forms,
        ControlId::Apply,
        &Payload::Press,
        0,
    );
    assert_eq!(
        effect,
        Ok(Effect::Settings(Prefs {
            quality: Quality::Medium,
            scale: 110,
            ..Prefs::default()
        }))
    );
    // Back to the applied settings: nothing changed again.
    screen::apply(
        ScreenId::Settings,
        &ctx,
        &mut forms,
        quality,
        &Payload::Right,
        0,
    )
    .expect("on");
    screen::apply(
        ScreenId::Settings,
        &ctx,
        &mut forms,
        scale,
        &Payload::Left,
        0,
    )
    .expect("back");
    let shown = screen::content(ScreenId::Settings, &ctx, &forms);
    assert!(
        shown
            .control(ControlId::Apply)
            .is_some_and(|c| c.why_not.is_some())
    );
}

fn at_title(settings: &Scratch) -> App {
    shell(
        Config {
            settings: settings.0.clone(),
            ..config()
        },
        false,
    )
}

fn top(app: &App) -> Option<ScreenId> {
    let game = app.world().get_resource::<Session>().map(|s| &s.views);
    app.world().resource::<Screens>().nav.top(game)
}

fn current(app: &App) -> Prefs {
    app.world().resource::<Current>().0
}

#[test]
fn apply_on_the_title_s_settings_keeps_them_and_writes_the_file() {
    let s = Scratch::new("apply");
    let mut app = at_title(&s);
    app.update();
    press_ui(&mut app, Pressable(ControlId::Settings));
    assert_eq!(top(&app), Some(ScreenId::Settings));
    press_ui(&mut app, Arrow(ControlId::Setting(Field::Grid), false));
    press_ui(&mut app, Arrow(ControlId::Setting(Field::Motion), true));
    assert_eq!(current(&app), Prefs::default(), "nothing until Apply");
    press_ui(&mut app, Pressable(ControlId::Apply));
    let want = Prefs {
        grid: false,
        reduced_motion: true,
        ..Prefs::default()
    };
    assert_eq!(current(&app), want);
    assert_eq!(read(&s), (want, None), "the file holds them");
    assert_eq!(top(&app), Some(ScreenId::Settings), "the screen stays open");
    let message = app.world().resource::<Screens>().forms.message.clone();
    assert_eq!(
        message.map(|m| m.text),
        Some(format!("Settings saved to {}", s.0))
    );
}

#[test]
fn close_and_esc_drop_the_draft() {
    let s = Scratch::new("close");
    let mut app = at_title(&s);
    app.update();
    press_ui(&mut app, Pressable(ControlId::Settings));
    press_ui(&mut app, Arrow(ControlId::Setting(Field::Font), true));
    press_ui(&mut app, Pressable(ControlId::Close));
    assert_eq!(top(&app), Some(ScreenId::Title));
    assert_eq!(current(&app), Prefs::default());
    assert!(!Path::new(&s.0).exists(), "nothing written");
    press_ui(&mut app, Pressable(ControlId::Settings));
    assert_eq!(
        app.world().resource::<Screens>().forms.settings,
        None,
        "it reopens on the applied settings"
    );
    press_ui(&mut app, Arrow(ControlId::Setting(Field::Font), true));
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Title), "Esc closes it");
    assert_eq!(current(&app), Prefs::default());
    assert_eq!(app.world().resource::<Screens>().forms.settings, None);
}

#[test]
fn the_pause_opens_the_settings_and_esc_returns_to_the_pause() {
    let s = Scratch::new("pause");
    let mut app = shell(
        Config {
            settings: s.0.clone(),
            ..config()
        },
        true,
    );
    app.update();
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause));
    press_ui(&mut app, Pressable(ControlId::Settings));
    assert_eq!(top(&app), Some(ScreenId::Settings));
    let session = app.world().resource::<Session>();
    let nav = &app.world().resource::<Screens>().nav;
    assert!(nav.paused(Some(&session.views)), "the game waits");
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause));
}

#[test]
fn quality_sets_the_bloom_on_every_camera() {
    let s = Scratch::new("quality");
    let mut app = at_title(&s);
    let camera = app
        .world_mut()
        .spawn((
            Hdr,
            Bloom {
                intensity: 0.25,
                ..Bloom::NATURAL
            },
        ))
        .id();
    app.update();
    let bloom = |app: &App| app.world().get::<Bloom>(camera).map(|b| b.intensity);
    let set = |app: &mut App, q: Quality| {
        app.world_mut().resource_mut::<Current>().0.quality = q;
        app.update();
    };
    assert_eq!(bloom(&app), Some(0.25), "high: today's look");
    set(&mut app, Quality::Medium);
    assert_eq!(bloom(&app), Some(0.15), "medium softens the bloom in place");
    set(&mut app, Quality::Low);
    assert_eq!(bloom(&app), None, "low: no bloom at all");
    set(&mut app, Quality::High);
    assert_eq!(bloom(&app), Some(0.25), "high puts it back");
    // A camera that appears later takes the preset too.
    let late = app
        .world_mut()
        .spawn((
            Hdr,
            Bloom {
                intensity: 0.9,
                ..Bloom::NATURAL
            },
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Bloom>(late).map(|b| b.intensity),
        Some(0.25)
    );
}

#[test]
fn the_font_applied_dresses_every_text_and_text_that_comes_later() {
    use bevy::text::FontSource;
    use omnis_vector::shell::fonts::{INTER, handle};
    let s = Scratch::new("font");
    let mut app = at_title(&s);
    app.update();
    let faces = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<&TextFont>();
        q.iter(world).map(|f| f.font.clone()).collect::<Vec<_>>()
    };
    assert!(!faces(&mut app).is_empty(), "the title has text");
    assert!(
        faces(&mut app)
            .iter()
            .all(|f| *f == FontSource::Handle(handle(Face::BuiltIn)))
    );
    press_ui(&mut app, Pressable(ControlId::Settings));
    press_ui(&mut app, Arrow(ControlId::Setting(Field::Font), true));
    press_ui(&mut app, Pressable(ControlId::Apply));
    assert_eq!(current(&app).font, Face::Inter);
    // Close: the title's text is built again, after the change.
    press_ui(&mut app, Pressable(ControlId::Close));
    app.update();
    let all = faces(&mut app);
    assert!(all.len() > 3);
    assert!(
        all.iter().all(|f| *f == FontSource::Handle(INTER)),
        "every text in Inter: {all:?}"
    );
}

#[test]
fn the_hidden_grid_drops_the_floor_and_ceiling_and_keeps_the_rest() {
    let session = Session::start(&config()).expect("a session");
    let map = session.views.position.map;
    let data = &session.data.maps[&map];
    let ground = omnis_vector::ground::Ground::from_map(data, omnis_sim::omnis_core::Level::GROUND);
    let lines = extract(
        data,
        map,
        omnis_sim::omnis_core::Level::GROUND,
        &session.world,
        &ground,
    );
    let grid = |k: SegKind| matches!(k, SegKind::Floor | SegKind::Ceiling);
    assert!(
        lines.iter().any(|l| grid(l.kind)),
        "the town has a floor grid"
    );
    let hidden: Vec<_> = lines.iter().filter(|l| l.kind.drawn(false)).collect();
    assert!(hidden.iter().all(|l| !grid(l.kind)));
    assert_eq!(
        hidden.len(),
        lines.iter().filter(|l| !grid(l.kind)).count(),
        "every wall, door and block stays"
    );
    assert!(
        lines.iter().all(|l| l.kind.drawn(true)),
        "shown: everything"
    );
}

#[test]
fn a_scale_larger_than_the_window_holds_says_how_far_it_fits() {
    let data = common::data();
    let mut ctx = title_ctx(&data, Prefs::default());
    ctx.fits = 120;
    let mut forms = Forms::default();
    forms.settings = Some(Prefs {
        scale: 150,
        ..Prefs::default()
    });
    let note = |forms: &Forms| {
        let shown = screen::content(ScreenId::Settings, &ctx, forms);
        let row = shown.panels[0]
            .rows
            .iter()
            .find(|r| {
                r.controls
                    .iter()
                    .any(|c| c.id == ControlId::Setting(Field::Scale))
            })
            .expect("the scale's row")
            .clone();
        row.after.first().map(|s| s.text.clone())
    };
    assert_eq!(note(&forms), Some("120% fits this window".to_owned()));
    forms.settings = Some(Prefs {
        scale: 120,
        ..Prefs::default()
    });
    assert_eq!(note(&forms), None);
}
