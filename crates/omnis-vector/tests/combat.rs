//! The fight screen, headless (alt-ARCHITECTURE.md §9): walking into the placed group switches
//! to it, with the encounter's choices as buttons and the 3D view's movement pad put away; a
//! fight runs to its end from its buttons and clicks on the figures alone, every one accepted;
//! and leaving puts the pose where the party is.

mod common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{click, key, met_with, set};
use omnis_sim::{Command, EncounterChoice, Mode};
use omnis_vector::arena::Rect;
use omnis_vector::cinema::Scene;
use omnis_vector::combat_menu::{Act, Action, Pick, Step};
use omnis_vector::grid::cell_of;
use omnis_vector::shell::ViewState;
use omnis_vector::shell::cinema::Screen;
use omnis_vector::shell::combat::{Choose, FightRoot, FightScreen};
use omnis_vector::shell::controls::Corner;
use omnis_vector::shell::panel::Showing;
use omnis_vector::shell::session::Session;

fn view(app: &App) -> ViewState {
    *app.world().resource::<State<ViewState>>().get()
}

fn session(app: &App) -> &Session {
    app.world().resource::<Session>()
}

fn screen(app: &App) -> &FightScreen {
    app.world().resource::<FightScreen>()
}

/// What the action column's buttons that can be pressed do, top to bottom.
fn enabled(app: &mut App) -> Vec<Act> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Choose, With<Button>>();
    query.iter(world).map(|c| c.0.clone()).collect()
}

fn press(app: &mut App, act: Act) {
    set(app, &Choose(act), Interaction::Pressed);
    app.update();
}

fn encounter(choice: EncounterChoice) -> Act {
    Act::Command(Command::Encounter(choice))
}

fn pad_display(app: &mut App) -> Display {
    let world = app.world_mut();
    let mut query = world.query::<(&Corner, &Node)>();
    query
        .iter(world)
        .find(|(c, _)| **c == Corner::Left)
        .map(|(_, n)| n.display)
        .expect("the movement pad")
}

fn centre(r: &Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

/// Where to click to choose `pick`: the middle of its first figure.
fn spot(app: &App, pick: Pick) -> (f32, f32) {
    let arena = screen(app).arena.as_ref().expect("a fight laid out");
    let group = arena
        .groups()
        .find(|g| g.pick == pick)
        .unwrap_or_else(|| panic!("a group for {pick:?}"));
    centre(&group.figures[0].rect)
}

#[test]
fn meeting_the_group_switches_to_the_fight_screen() {
    let mut app = met_with(1);
    assert_eq!(view(&app), ViewState::Fight);
    let acts = enabled(&mut app);
    assert_eq!(
        acts[..],
        [
            encounter(EncounterChoice::Attack),
            encounter(EncounterChoice::Bribe),
            encounter(EncounterChoice::Hide),
            encounter(EncounterChoice::Run),
        ]
    );
    assert_eq!(pad_display(&mut app), Display::None, "the pad is put away");
    assert!(!app.world().resource::<Showing>().0, "the panel stays down");
    // The picture window shows the monsters met, inside the screen.
    let monster = match &session(&app).world.mode {
        Mode::Encounter(state) => state.stacks[0].monster,
        _ => unreachable!(),
    };
    let world = app.world_mut();
    let mut pictures = world.query::<(&Screen, &ChildOf)>();
    let (picture, parent) = pictures.single(world).expect("one picture window");
    assert_eq!(*picture, Screen(Scene::Enemy(monster)));
    assert!(world.get::<FightRoot>(parent.parent()).is_some());
    // A name label for the rats and each member.
    let mut texts = world.query::<&Text>();
    let texts: Vec<&str> = texts.iter(world).map(|t| t.0.as_str()).collect();
    for name in ["Giant Rat", "Brenna", "Durin", "Ilvara", "Pip"] {
        assert!(texts.contains(&name), "{name} labelled in {texts:?}");
    }
}

#[test]
fn running_puts_the_pose_on_the_retreat_and_the_3d_view_back() {
    // Seed 2: the run check succeeds.
    let mut app = met_with(2);
    let retreat = match &session(&app).world.mode {
        Mode::Encounter(state) => state.retreat,
        _ => unreachable!(),
    };
    press(&mut app, encounter(EncounterChoice::Run));
    app.update();
    let s = session(&app);
    assert!(matches!(s.world.mode, Mode::Explore), "the party got away");
    assert_eq!(s.world.position, retreat);
    let here = cell_of(s.pose.x, s.pose.z);
    assert_eq!(here, (i32::from(retreat.x), i32::from(retreat.y)));
    assert_eq!(s.binder.refusals, 0);
    assert_eq!(view(&app), ViewState::Explore);
    let world = app.world_mut();
    assert_eq!(world.query::<&FightRoot>().iter(world).count(), 0);
    assert_eq!(pad_display(&mut app), Display::Flex, "the pad is back");
}

#[test]
fn a_fight_runs_to_its_end_from_the_screen_alone() {
    let mut app = met_with(1);
    let accepted = session(&app).binder.log.len();
    // The number key picks the first button: Fight.
    key(&mut app, KeyCode::Digit1, ButtonState::Pressed);
    app.update();
    key(&mut app, KeyCode::Digit1, ButtonState::Released);
    app.update();
    assert!(matches!(session(&app).world.mode, Mode::Combat(_)));
    assert_eq!(session(&app).binder.log.len(), accepted + 1);
    let mut clicks = 0;
    for _ in 0..300 {
        if view(&app) == ViewState::Explore {
            break;
        }
        // A target to click when the step has one; otherwise the first button.
        if let Some(&target) = screen(&app).targets.first() {
            let point = spot(&app, target);
            click(&mut app, point);
            clicks += 1;
        } else if let Some(act) = enabled(&mut app).into_iter().next() {
            press(&mut app, act);
        } else {
            app.update();
        }
    }
    let s = session(&app);
    assert!(matches!(s.world.mode, Mode::Explore), "the fight ended");
    assert_eq!(view(&app), ViewState::Explore);
    assert_eq!(s.binder.refusals, 0, "every offered choice was accepted");
    assert!(clicks > 0, "targets were chosen by clicking");
    let here = cell_of(s.pose.x, s.pose.z);
    assert_eq!(
        here,
        (i32::from(s.world.position.x), i32::from(s.world.position.y)),
        "the pose stands on the party's cell"
    );
    let ending = ["Victory", "The party got away", "The party has fallen"];
    assert!(
        s.lines
            .iter()
            .any(|l| ending.iter().any(|e| l.starts_with(e))),
        "the HUD says how it ended: {:?}",
        s.lines
    );
}

/// The app at the start of the fight, on the first member's turn with the rats in reach.
fn fighting() -> App {
    let mut app = met_with(1);
    press(&mut app, encounter(EncounterChoice::Attack));
    for _ in 0..40 {
        if !screen(&app).targets.is_empty() {
            return app;
        }
        press(
            &mut app,
            Act::Command(Command::Combat(omnis_sim::CombatCommand::Dodge)),
        );
    }
    panic!("no turn with a foe in reach");
}

fn point_at(app: &mut App, (x, y): (f32, f32)) {
    app.world_mut()
        .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
        .single_mut(app.world_mut())
        .expect("the window")
        .set_cursor_position(Some(Vec2::new(x, y)));
    app.update();
}

#[test]
fn the_pointer_frames_a_target_and_a_click_attacks_it() {
    let mut app = fighting();
    assert_eq!(screen(&app).targets, [Pick::Stack(0)]);
    let rats = spot(&app, Pick::Stack(0));
    // A click on empty field chooses nothing.
    let field = screen(&app).arena.as_ref().expect("laid out").layout.field;
    let accepted = session(&app).binder.log.len();
    click(&mut app, (field.x + 2.0, field.y + 2.0));
    assert_eq!(session(&app).binder.log.len(), accepted);
    assert_eq!(screen(&app).hover, None);
    // A member is no target for an attack: resting on one frames nothing, clicking does nothing.
    let pip = spot(&app, Pick::Member(3));
    point_at(&mut app, pip);
    assert_eq!(screen(&app).hover, None);
    click(&mut app, pip);
    assert_eq!(session(&app).binder.log.len(), accepted);
    // Resting on the rats frames them; clicking them attacks.
    point_at(&mut app, rats);
    assert_eq!(screen(&app).hover, Some(Pick::Stack(0)));
    let marks = &screen(&app).arena.as_ref().expect("laid out").marks;
    assert_eq!(marks.hover, Some(Pick::Stack(0)));
    click(&mut app, rats);
    let s = session(&app);
    assert!(s.binder.log.len() > accepted, "the attack was accepted");
    assert_eq!(s.binder.refusals, 0);
}

#[test]
fn a_step_opens_from_its_button_and_esc_steps_back() {
    let mut app = fighting();
    press(&mut app, Act::Open(Step::Target(Action::Attack)));
    assert_eq!(screen(&app).menu.step, Step::Target(Action::Attack));
    let acts = enabled(&mut app);
    assert_eq!(acts.last(), Some(&Act::Back), "a way back: {acts:?}");
    let world = app.world_mut();
    let mut texts = world.query::<&Text>();
    assert!(
        texts.iter(world).any(|t| t.0.ends_with("attacks whom?")),
        "the step asks its question"
    );
    key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    app.update();
    assert_eq!(screen(&app).menu.step, Step::Top);
    assert_eq!(session(&app).binder.refusals, 0);
}
