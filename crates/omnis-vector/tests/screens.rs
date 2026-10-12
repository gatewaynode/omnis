//! The screens (presentation-ARCHITECTURE §6): descriptions, `apply`'s refusals, the overlays
//! following the world, the title, the new game's form, the pause with save and load, the fallen
//! party, and the renderer's text as agents read it.

use crate::common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use common::app::{app, app_at_title, app_in_game, key, met_with, set};
use common::fight::met;
use omnis_sim::omnis_core::rng::fnv1a64;
use omnis_sim::omnis_core::{Direction, Facing};
use omnis_sim::omnis_data::Skill;
use omnis_sim::{Command, EncounterChoice, SaveRule, Settings};
use omnis_vector::combat_menu::{Act, Action, Step};
use omnis_vector::party;
use omnis_vector::screen::create_party::CreateForm;
use omnis_vector::screen::nav::{Escape, Nav};
use omnis_vector::screen::new_game::NewGameForm;
use omnis_vector::screen::{
    self, ControlId, Ctx, Effect, Forms, HostOp, Payload, Refused, ScreenId, Slot,
};
use omnis_vector::shell::controls::{Action as Button_, Corner};
use omnis_vector::shell::screen::{Arrow, Pressable, Screens, SliderOf, colour, ctx};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Session;
use omnis_vector::sim::{Views, game_settings};
use omnis_vector::style::Role;

fn tap(app: &mut App, code: KeyCode) {
    key(app, code, ButtonState::Pressed);
    app.update();
    key(app, code, ButtonState::Released);
    app.update();
}

fn press(app: &mut App, id: ControlId) {
    set(app, &Pressable(id), Interaction::Pressed);
    app.update();
    // Let go, so the next press is a change; a screen that closed has no button left.
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

fn session(app: &App) -> &Session {
    app.world().resource::<Session>()
}

/// A save path for one test, under the crate's `.omnis/`, removed when the test ends.
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

/// A session at the dungeon door: the square offers "Open the door".
fn at_the_door() -> Session {
    let mut app = app();
    app.update();
    let mut session = app
        .world_mut()
        .remove_resource::<Session>()
        .expect("a session");
    let dungeon = common::map(&session.data, "test:map:dungeon");
    session.pose = common::place(&mut session.world, dungeon, 3, 5, Facing::South);
    session.refresh();
    session
}

fn views(session: &Session) -> Option<&Views> {
    Some(&session.views)
}

#[test]
fn apply_refuses_a_control_the_screen_does_not_have() {
    let session = at_the_door();
    let ctx = ctx(&session);
    let mut forms = Forms::default();
    for id in [ControlId::Leave, ControlId::Action(7)] {
        assert_eq!(
            screen::apply(ScreenId::Actions, &ctx, &mut forms, id, &Payload::Press, 0),
            Err(Refused::NotHere),
            "{id:?}"
        );
    }
}

#[test]
fn the_shape_ignores_words_and_follows_controls() {
    let mut session = at_the_door();
    let forms = Forms::default();
    let closed = screen::content(ScreenId::Actions, &ctx(&session), &forms);
    session.order(Command::Interact);
    let open = screen::content(ScreenId::Actions, &ctx(&session), &forms);
    assert_ne!(closed, open, "Open the door became Close the door");
    assert_eq!(closed.shape(), open.shape(), "the same controls");
    // Facing a wall, the square offers nothing: one control fewer.
    session.order(Command::Turn(omnis_sim::omnis_core::Rotation::Left));
    let none = screen::content(ScreenId::Actions, &ctx(&session), &forms);
    assert_ne!(none.shape(), open.shape());
}

#[test]
fn overlays_follow_the_world() {
    let mut session = at_the_door();
    let mut nav = Nav::default();
    nav.open(ScreenId::Actions, views(&session));
    assert_eq!(nav.top(views(&session)), Some(ScreenId::Actions));
    nav.settle(views(&session), false);
    assert!(nav.is_open(ScreenId::Actions), "no move, still open");
    nav.settle(views(&session), true);
    assert!(!nav.is_open(ScreenId::Actions), "a move closes it");
    // The form for a new game opens only at the title, and the game's overlays only in a game.
    nav.open(ScreenId::NewGame, views(&session));
    assert!(!nav.is_open(ScreenId::NewGame));
    nav.open(ScreenId::Pause, None);
    assert!(!nav.is_open(ScreenId::Pause));
    assert_eq!(nav.base(None), Some(ScreenId::Title));
    // A fallen party: its screen is the base, and nothing opens over it.
    for member in &mut session.world.party.members {
        member.hp = 0;
    }
    session.refresh();
    assert_eq!(nav.base(views(&session)), Some(ScreenId::Defeat));
    for id in [ScreenId::Actions, ScreenId::Pause] {
        nav.open(id, views(&session));
        assert!(!nav.is_open(id), "{id:?}");
    }
    assert!(nav.paused(views(&session)), "the fallen party waits");
}

#[test]
fn esc_closes_an_overlay_steps_a_fight_back_then_pauses() {
    let fight = met();
    let mut nav = Nav::default();
    assert_eq!(
        nav.base(views(&fight)),
        None,
        "the fight screen draws itself"
    );
    nav.open(ScreenId::Actions, views(&fight));
    assert!(!nav.is_open(ScreenId::Actions), "not over a fight");
    assert_eq!(nav.escape(views(&fight), false), Escape::FightBack);
    assert_eq!(nav.escape(views(&fight), true), Escape::Paused);
    assert_eq!(nav.escape(views(&fight), true), Escape::Closed);
    let calm = at_the_door();
    nav.open(ScreenId::Actions, views(&calm));
    assert_eq!(nav.escape(views(&calm), true), Escape::Closed);
    assert_eq!(nav.escape(views(&calm), true), Escape::Paused);
    assert!(nav.paused(views(&calm)));
    assert_eq!(nav.escape(None, true), Escape::Closed);
    assert_eq!(
        nav.escape(None, true),
        Escape::Nothing,
        "the title has nowhere to go"
    );
}

#[test]
fn a_seed_is_drawn_typed_or_hashed() {
    let form = |text: &str| NewGameForm {
        seed_text: text.to_owned(),
        ..NewGameForm::default()
    };
    assert_eq!(form("").seed(99), 99, "blank: the entropy");
    assert_eq!(form("   ").seed(99), 99);
    assert_eq!(form("42").seed(99), 42, "a number as typed");
    assert_eq!(form(" 42 ").seed(99), 42);
    assert_eq!(form("dragon").seed(99), fnv1a64(b"dragon"), "a word hashed");
}

fn title_ctx(data: &omnis_sim::omnis_data::Data, exists: bool) -> Ctx<'_> {
    Ctx {
        data,
        game: None,
        save: Slot {
            path: "saves/q.ron",
            exists,
        },
        prefs: omnis_vector::settings::Prefs::default(),
        fits: omnis_vector::settings::SCALE_MAX,
    }
}

#[test]
fn the_new_game_form_fills_and_starts_a_game() {
    let data = common::data();
    let ctx = title_ctx(&data, false);
    let mut forms = Forms::default();
    let mut use_ =
        |id, payload: Payload| screen::apply(ScreenId::NewGame, &ctx, &mut forms, id, &payload, 99);
    assert_eq!(
        use_(ControlId::Seed, Payload::Text("x".repeat(33))),
        Err(Refused::TooLong(32))
    );
    assert_eq!(
        use_(ControlId::Seed, Payload::Press),
        Err(Refused::WrongPayload)
    );
    assert_eq!(
        use_(ControlId::Seed, Payload::Text("7".into())),
        Ok(Effect::None)
    );
    // The save rule steps both ways and wraps: three right is back to Anywhere, one left InnOnly.
    for _ in 0..3 {
        assert_eq!(use_(ControlId::Rule, Payload::Right), Ok(Effect::None));
    }
    assert_eq!(use_(ControlId::Rule, Payload::Left), Ok(Effect::None));
    assert_eq!(
        use_(ControlId::Permadeath, Payload::Press),
        Ok(Effect::None)
    );
    let start = use_(ControlId::Start, Payload::Press);
    assert_eq!(
        start,
        Ok(Effect::Host(HostOp::NewGame {
            seed: 7,
            settings: game_settings(Settings {
                save_rule: SaveRule::InnOnly,
                permadeath: true,
                devtools: false,
            }),
        }))
    );
    assert_eq!(use_(ControlId::Back, Payload::Press), Ok(Effect::Back));
    // The form keeps what was filled in.
    assert_eq!(forms.new_game.seed_text, "7");
}

#[test]
fn the_title_loads_only_when_there_is_a_save() {
    let data = common::data();
    let mut forms = Forms::default();
    let missing = title_ctx(&data, false);
    let mut use_ =
        |ctx: &Ctx, id| screen::apply(ScreenId::Title, ctx, &mut forms, id, &Payload::Press, 0);
    assert_eq!(
        use_(&missing, ControlId::Load),
        Err(Refused::Dim("no save at saves/q.ron".into()))
    );
    assert_eq!(
        use_(&title_ctx(&data, true), ControlId::Load),
        Ok(Effect::Host(HostOp::Load))
    );
    assert_eq!(
        use_(&missing, ControlId::NewGame),
        Ok(Effect::Open(ScreenId::NewGame))
    );
    assert_eq!(use_(&missing, ControlId::Quit), Ok(Effect::Quit));
}

#[test]
fn the_pause_tells_the_game_and_why_saving_is_refused() {
    let mut app = app_in_game("saves/never-written.ron");
    app.update();
    let packs = app
        .world()
        .resource::<omnis_vector::shell::session::Packs>()
        .clone();
    let inns_only = Settings {
        save_rule: SaveRule::InnOnly,
        ..Settings::default()
    };
    let session = Session::new_game(&packs, 7, game_settings(inns_only)).expect("a game");
    let shown = screen::content(ScreenId::Pause, &ctx(&session), &Forms::default());
    let lines: Vec<&str> = shown.panels[0]
        .rows
        .iter()
        .flat_map(|r| &r.spans)
        .map(|s| s.text.as_str())
        .collect();
    assert!(lines.contains(&"Seed 7"), "{lines:?}");
    assert!(lines.contains(&"Saving: Inns only"), "{lines:?}");
    let save = shown.control(ControlId::Save).expect("Save");
    assert_eq!(
        save.why_not.as_deref(),
        Some("The save rule forbids saving here")
    );
    let load = shown.control(ControlId::Load).expect("Load");
    assert_eq!(
        load.why_not.as_deref(),
        Some("no save at saves/never-written.ron")
    );
    let log = shown.control(ControlId::SaveLog).expect("Save log");
    assert!(log.why_not.is_none(), "a new game's log replays");
}

/// Type `text` into the seed field, as the field's editing would leave it.
fn type_seed(app: &mut App, text: &str) {
    type_into(app, ControlId::Seed, text);
}

/// Type `text` into field `id`, as the field's editing would leave it.
fn type_into(app: &mut App, id: ControlId, text: &str) {
    let world = app.world_mut();
    let mut fields = world.query::<(Entity, &Pressable, &bevy::text::EditableText)>();
    let field = fields
        .iter(world)
        .find(|(_, p, _)| p.0 == id)
        .map(|(e, _, _)| e)
        .unwrap_or_else(|| panic!("the {id:?} field"));
    world
        .entity_mut(field)
        .insert(bevy::text::EditableText::new(text));
    app.update();
}

#[test]
fn the_title_starts_a_game_from_the_form() {
    let save = Scratch::new("title");
    let mut app = app_at_title(&save.0);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Title));
    assert!(
        app.world().get_resource::<Session>().is_none(),
        "no game yet"
    );
    let text = screen_text(app.world());
    assert!(text.contains("\"OMNIS\""), "{text}");
    assert!(text.contains("\"Load quick save: no save at"), "{text}");
    press(&mut app, ControlId::NewGame);
    app.update();
    assert_eq!(top(&app), Some(ScreenId::NewGame));
    assert!(
        screen_text(app.world()).contains("[field] \"\""),
        "an empty seed field"
    );
    type_seed(&mut app, "42");
    assert_eq!(
        app.world().resource::<Screens>().forms.new_game.seed_text,
        "42"
    );
    set(
        &mut app,
        &Arrow(ControlId::Rule, true),
        Interaction::Pressed,
    );
    app.update();
    set(&mut app, &Arrow(ControlId::Rule, true), Interaction::None);
    app.update();
    assert!(screen_text(app.world()).contains("\"Inns, items, and spells\""));
    press(&mut app, ControlId::Start);
    app.update();
    let s = session(&app);
    assert_eq!(s.views.here.seed, 42);
    assert_eq!(s.views.here.settings.save_rule, SaveRule::Relief);
    assert!(s.views.party.members.is_empty(), "the party is made next");
    assert_eq!(top(&app), Some(ScreenId::CreateParty));
}

#[test]
fn the_pause_holds_the_game() {
    let mut app = app();
    app.update();
    let start = session(&app).views.position;
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause));
    set(&mut app, &Button_::Forward, Interaction::Pressed);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        session(&app).views.position,
        start,
        "nothing moves under the pause"
    );
    press(&mut app, ControlId::Resume);
    set(&mut app, &Button_::Forward, Interaction::Pressed);
    for _ in 0..30 {
        app.update();
    }
    set(&mut app, &Button_::Forward, Interaction::None);
    assert_ne!(session(&app).views.position, start, "it walks once resumed");
}

fn pause(app: &mut App) {
    set(app, &Button_::Menu, Interaction::Pressed);
    app.update();
    set(app, &Button_::Menu, Interaction::None);
    assert_eq!(top(app), Some(ScreenId::Pause));
    // The button opens it after this frame's screen was drawn; the next frame draws it.
    app.update();
}

fn fingerprint(app: &App) -> u64 {
    session(app).world.fingerprint().expect("a fingerprint")
}

fn message(app: &App) -> String {
    app.world()
        .resource::<Screens>()
        .forms
        .message
        .as_ref()
        .map(|m| m.text.clone())
        .unwrap_or_default()
}

#[test]
fn a_save_loads_back_and_the_log_stops_replaying() {
    let save = Scratch::new("pause");
    let mut app = app_in_game(&save.0);
    app.update();
    pause(&mut app);
    press(&mut app, ControlId::Save);
    assert_eq!(message(&app), format!("Saved to {}", save.0));
    let saved = fingerprint(&app);
    press(&mut app, ControlId::Resume);
    app.world_mut()
        .resource_mut::<Session>()
        .order(Command::Step(Direction::Forward));
    assert_ne!(fingerprint(&app), saved, "the step changed the world");
    pause(&mut app);
    press(&mut app, ControlId::Load);
    assert_eq!(fingerprint(&app), saved, "the save came back");
    assert_eq!(top(&app), None, "back in the game");
    assert!(session(&app).lines.iter().any(|l| l.starts_with("Loaded")));
    pause(&mut app);
    app.update();
    let text = screen_text(app.world());
    assert!(
        text.contains("\"Save log: the log no longer replays: the save"),
        "{text}"
    );
}

#[test]
fn quit_to_title_ends_the_game_and_hides_its_buttons() {
    let mut app = app();
    app.update();
    pause(&mut app);
    press(&mut app, ControlId::QuitToTitle);
    app.update();
    assert!(app.world().get_resource::<Session>().is_none());
    assert_eq!(top(&app), Some(ScreenId::Title));
    let world = app.world_mut();
    let mut corners = world.query_filtered::<&Visibility, With<Corner>>();
    assert!(corners.iter(world).all(|v| *v == Visibility::Hidden));
    assert!(screen_text(app.world()).contains("\"OMNIS\""));
}

fn fall(app: &mut App) {
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        for member in &mut session.world.party.members {
            member.hp = 0;
        }
        session.refresh();
    }
    app.update();
    assert_eq!(top(app), Some(ScreenId::Defeat));
}

#[test]
fn a_fallen_party_loads_its_save_or_starts_again() {
    let save = Scratch::new("defeat");
    let mut app = app_in_game(&save.0);
    app.update();
    let seed = session(&app).views.here.seed;
    fall(&mut app);
    let text = screen_text(app.world());
    assert!(text.contains("\"Load last save: no save at"), "{text}");
    assert!(text.contains("\"Every member is down.\""), "{text}");
    press(&mut app, ControlId::StartAgain);
    assert!(!session(&app).views.fallen());
    assert_eq!(session(&app).views.here.seed, seed, "the same game again");
    pause(&mut app);
    press(&mut app, ControlId::Save);
    press(&mut app, ControlId::Resume);
    let saved = fingerprint(&app);
    fall(&mut app);
    press(&mut app, ControlId::Load);
    assert_eq!(fingerprint(&app), saved);
    assert_eq!(top(&app), None);
}

#[test]
fn in_a_fight_esc_steps_back_first_and_the_pause_holds_the_fight() {
    let mut app = met_with(1);
    let step = common::fight::step;
    let choose = |app: &mut App, act: Act| common::fight::press(app, &act);
    let offered = |app: &mut App, act: &Act| common::fight::enabled(app).contains(act);
    choose(
        &mut app,
        Act::Command(Command::Encounter(EncounterChoice::Attack)),
    );
    let attack = Act::Open(Step::Target(Action::Attack));
    let dodge = Act::Command(Command::Combat(omnis_sim::CombatCommand::Dodge));
    for _ in 0..40 {
        if offered(&mut app, &attack) {
            break;
        }
        choose(&mut app, dodge.clone());
    }
    choose(&mut app, attack);
    assert_eq!(step(&app), Step::Target(Action::Attack));
    tap(&mut app, KeyCode::Escape);
    assert_eq!(step(&app), Step::Top, "Esc steps the menu back first");
    assert_eq!(top(&app), None);
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause), "then pauses");
    let log = session(&app).binder.log.len();
    choose(&mut app, dodge);
    assert_eq!(session(&app).binder.log.len(), log, "the fight waits");
}

#[test]
fn the_screen_text_lists_the_open_screens_controls_with_their_keys() {
    let mut app = app();
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = common::map(&session.data, "test:map:dungeon");
        let pose = common::place(&mut session.world, dungeon, 3, 5, Facing::South);
        session.pose = pose;
        session.refresh();
    }
    app.update();
    tap(&mut app, KeyCode::Space);
    let text = screen_text(app.world());
    for line in [
        "\"Here: (3, 5) facing south\"",
        "[button] \"1  Open the door\"",
        "[button] \"2  Close\"",
    ] {
        assert!(text.contains(line), "{line} in:\n{text}");
    }
}

/// A change of words under an open screen rewrites them in place: the same button, new label.
#[test]
fn new_words_are_written_in_place() {
    let mut app = app();
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let dungeon = common::map(&session.data, "test:map:dungeon");
        session.pose = common::place(&mut session.world, dungeon, 3, 5, Facing::South);
        session.refresh();
    }
    app.update();
    tap(&mut app, KeyCode::Space);
    let button = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Pressable)>();
        q.iter(world)
            .find(|(_, p)| p.0 == ControlId::Action(0))
            .map(|(e, _)| e)
    };
    let before = button(&mut app).expect("the door's button");
    app.world_mut()
        .resource_mut::<Session>()
        .order(Command::Interact);
    app.update();
    assert_eq!(button(&mut app), Some(before), "the same node");
    assert!(screen_text(app.world()).contains("\"1  Close the door\""));
}

fn skill(s: Skill) -> ControlId {
    let at = Skill::ALL.iter().position(|k| *k == s).unwrap();
    ControlId::Skill(u8::try_from(at).unwrap())
}

fn step_choice(app: &mut App, id: ControlId) {
    set(app, &Arrow(id, true), Interaction::Pressed);
    app.update();
    set(app, &Arrow(id, true), Interaction::None);
    app.update();
}

/// Drag a number's slider to `value`, as the widget reports it.
fn slide(app: &mut App, id: ControlId, value: f32) {
    let world = app.world_mut();
    let mut sliders = world.query::<(Entity, &SliderOf)>();
    let slider = sliders
        .iter(world)
        .find(|(_, s)| s.0 == id)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("the {id:?} slider"));
    world.trigger(ValueChange {
        source: slider,
        value,
        is_final: true,
    });
    app.update();
    app.update();
}

/// Whether the game's corner buttons show (the test shell has no HUD; one system shows both).
fn hud_shown(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut corners = world.query_filtered::<&Visibility, With<Corner>>();
    let seen: Vec<bool> = corners
        .iter(world)
        .map(|v| *v != Visibility::Hidden)
        .collect();
    assert!(!seen.is_empty(), "the corner buttons exist");
    seen.iter().all(|s| *s)
}

/// From the title through the form to party creation, with `seed` typed.
fn to_creation(app: &mut App, seed: &str) {
    app.update();
    press(app, ControlId::NewGame);
    app.update();
    type_seed(app, seed);
    press(app, ControlId::Start);
    app.update();
    assert_eq!(top(app), Some(ScreenId::CreateParty));
}

#[test]
fn a_party_is_made_through_the_screen_then_play_begins() {
    let save = Scratch::new("create");
    let mut app = app_at_title(&save.0);
    to_creation(&mut app, "42");
    assert!(!hud_shown(&mut app), "the game's HUD waits for the party");
    let text = screen_text(app.world());
    assert!(
        text.contains("\"CREATE YOUR PARTY   0 of 6 members\""),
        "{text}"
    );
    assert!(text.contains("[slider] 8 (8..15)"), "{text}");
    type_into(&mut app, ControlId::Name, "Ash");
    let form = |app: &App| app.world().resource::<Screens>().forms.create.clone();
    assert_eq!(form(&app).name, "Ash");
    step_choice(&mut app, ControlId::Class);
    assert!(screen_text(app.world()).contains("\"Fighter\""));
    // The slider asks for 14.6; the score is a whole 15, and the slider shows it.
    slide(&mut app, ControlId::Score(0), 14.6);
    assert_eq!(form(&app).scores[0], 15);
    let text = screen_text(app.world());
    assert!(text.contains("[slider] 15 (8..15)"), "{text}");
    assert!(text.contains("\"cost 9\""), "{text}");
    press(&mut app, skill(Skill::Athletics));
    app.update();
    press(&mut app, skill(Skill::Perception));
    app.update();
    press(&mut app, ControlId::AddMember);
    app.update();
    let members = &session(&app).views.party.members;
    assert_eq!(members.len(), 1);
    assert_eq!(
        (members[0].name.as_str(), members[0].scores[0]),
        ("Ash", 15)
    );
    assert_eq!(
        form(&app),
        CreateForm::default(),
        "the next member starts blank"
    );
    assert!(screen_text(app.world()).contains("\"CREATE YOUR PARTY   1 of 6 members\""));
    assert_eq!(top(&app), Some(ScreenId::CreateParty), "still creating");
    press(&mut app, ControlId::Begin);
    app.update();
    assert_eq!(top(&app), None, "play begins");
    assert!(hud_shown(&mut app));
    // Esc now pauses, as in any game.
    tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), Some(ScreenId::Pause));
}

#[test]
fn esc_on_party_creation_returns_to_the_filled_form() {
    let save = Scratch::new("create-esc");
    let mut app = app_at_title(&save.0);
    to_creation(&mut app, "dragon");
    tap(&mut app, KeyCode::Escape);
    app.update();
    assert!(
        app.world().get_resource::<Session>().is_none(),
        "the world is dropped"
    );
    assert_eq!(top(&app), Some(ScreenId::NewGame));
    assert_eq!(
        app.world().resource::<Screens>().forms.new_game.seed_text,
        "dragon"
    );
}

#[test]
fn start_again_makes_the_same_party_and_after_a_load_the_save_s_game_anew() {
    let save = Scratch::new("again");
    let mut app = app_in_game(&save.0);
    app.update();
    let names = |app: &App| -> Vec<String> {
        session(app)
            .views
            .party
            .members
            .iter()
            .map(|m| m.name.clone())
            .collect()
    };
    let party = names(&app);
    assert_eq!(party.len(), 4);
    fall(&mut app);
    press(&mut app, ControlId::StartAgain);
    app.update();
    assert_eq!(names(&app), party, "the same party again");
    assert_eq!(top(&app), None);

    // A save from another game (seed 99), loaded into this one: Start again starts the save's
    // game, whose party this session never made, so it is made anew.
    let packs = app
        .world()
        .resource::<omnis_vector::shell::session::Packs>()
        .clone();
    let other = Session::with_party(
        &packs,
        99,
        game_settings(Settings::default()),
        &party::fixed(),
    )
    .expect("a game");
    omnis_vector::game_io::save(&other.world, std::path::Path::new("."), &save.0).expect("saved");
    pause(&mut app);
    press(&mut app, ControlId::Load);
    app.update();
    assert_eq!(session(&app).views.here.seed, 99);
    fall(&mut app);
    press(&mut app, ControlId::StartAgain);
    app.update();
    assert_eq!(session(&app).views.here.seed, 99, "the save's game");
    assert!(session(&app).views.party.members.is_empty());
    assert_eq!(top(&app), Some(ScreenId::CreateParty));
}

/// The shell with the window's keyboard path to a text field: focus, its dispatch, the text
/// input observers and the edits applied, so a test types as a player does.
fn keyboard_shell(save: &str) -> (App, Entity) {
    use bevy::input_focus::{InputDispatchPlugin, InputFocusPlugin};
    let mut app = app_at_title(save);
    app.add_plugins((
        bevy::asset::AssetPlugin::default(),
        InputFocusPlugin,
        InputDispatchPlugin,
        bevy::text::TextPlugin,
        bevy::ui_widgets::EditableTextInputPlugin,
    ))
    .add_message::<bevy::window::Ime>()
    .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Release>>()
    .init_resource::<bevy::ui::UiScale>();
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    (app, window)
}

/// Type `text` on the keyboard, one key at a time, to whatever has focus.
fn keyboard(app: &mut App, window: Entity, text: &str) {
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput};
    for c in text.chars() {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::KeyA,
                logical_key: Key::Character(c.to_string().into()),
                state,
                text: Some(c.to_string().into()),
                repeat: false,
                window,
            });
            app.update();
        }
    }
}

#[test]
fn the_name_field_draws_a_cursor_in_the_interface_colour() {
    let save = Scratch::new("create-cursor");
    let mut app = app_at_title(&save.0);
    to_creation(&mut app, "42");
    let world = app.world_mut();
    let mut fields = world.query::<(&Pressable, Option<&bevy::text::TextCursorStyle>)>();
    let style = fields
        .iter(world)
        .find(|(p, _)| p.0 == ControlId::Name)
        .and_then(|(_, s)| s.copied());
    // Without a style Bevy draws no cursor at all.
    let style = style.expect("the name field has a cursor style");
    assert_eq!(style.color, colour(Role::Interface));
}

#[test]
fn the_second_member_is_named_on_the_keyboard_after_the_first_is_added() {
    let save = Scratch::new("create-keys");
    let (mut app, window) = keyboard_shell(&save.0);
    to_creation(&mut app, "42");
    let name = |app: &App| app.world().resource::<Screens>().forms.create.name.clone();
    keyboard(&mut app, window, "Ash");
    assert_eq!(name(&app), "Ash");
    step_choice(&mut app, ControlId::Class);
    press(&mut app, skill(Skill::Athletics));
    app.update();
    press(&mut app, skill(Skill::Perception));
    app.update();
    press(&mut app, ControlId::AddMember);
    app.update();
    assert_eq!(session(&app).views.party.members.len(), 1);
    // The screen was drawn anew for the next member; its field has the keyboard.
    keyboard(&mut app, window, "Bree");
    assert_eq!(name(&app), "Bree");
}
