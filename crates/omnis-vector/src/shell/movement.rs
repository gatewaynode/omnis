//! Input to intent, and intent through the binder (presentation-ARCHITECTURE.md §6).

use super::screen::Showing;
use super::session::Session;
use super::settings::Current;
use super::{VectorSet, ViewState};
use crate::look::{level, wrap_pitch, yaw_sign};
use crate::pose::{Motion, Pose, integrate};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use core::f32::consts::FRAC_PI_2;
use omnis_sim::omnis_core::MapId;
use omnis_sim::{BlockReason, Event};

/// Cells a second on terrain that costs one minute a step; slower terrain is slower to cross.
pub const BASE_SPEED: f32 = 3.0;
/// Radians a second for the keyboard and button turns.
pub const TURN_SPEED: f32 = 2.5;
/// Radians per pixel of mouse motion.
pub const MOUSE_SENSITIVITY: f32 = 0.003;
/// The longest frame integrated, so a stall cannot carry the pose across several cells.
pub const MAX_DT: f32 = 0.1;
/// The most yaw, or pitch, one frame of mouse motion may add.
pub const MAX_MOUSE_YAW: f32 = 0.5;

/// What the player asks for this frame, from keys, the mouse and the HUD buttons.
#[derive(Resource, Debug, Default)]
pub struct Intent {
    /// Held movement.
    pub motion: Motion,
    /// Held turning, `-1.0..=1.0` (positive turns left).
    pub turn: f32,
    /// Mouse yaw this frame, radians.
    pub mouse_yaw: f32,
    /// Yaw still to turn for a 90° button turn, radians.
    pub pending_turn: f32,
    /// Whether the mouse looks (the cursor is grabbed).
    pub looking: bool,
    /// Mouse pitch this frame, radians (positive looks up).
    pub mouse_pitch: f32,
    /// Held tilting, `-1.0..=1.0` (positive looks up).
    pub tilt: f32,
    /// Return the view to level now (Home, the Level button).
    pub level: bool,
}

/// Where the head looks, up or down, beside the pose and never in it (`look`, M8.5 step 4b):
/// the binder and the simulation never see it.
#[derive(Resource, Debug, Default)]
pub struct View {
    /// Radians, `(-π, π]`, positive up; past ±π/2 the view has turned over.
    pub pitch: f32,
    /// Easing back to level after a step.
    pub levelling: bool,
    /// The map and pose the last frame left, so a jump made elsewhere (a load, a new game, a
    /// teleport from the debug panel) starts the view level.
    seen: Option<(MapId, Pose)>,
}

/// What the mouse-look decision sees in a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LookInput {
    /// The left button went down.
    pub left_click: bool,
    /// The pointer is over a button (a click there is the button's, not the view's).
    pub over_button: bool,
    /// Space, a right click or Esc.
    pub release: bool,
    /// The centre panel is up and needs the pointer.
    pub panel: bool,
}

/// Whether the mouse looks after this frame. A left click in the view toggles it; the release
/// keys and the panel turn it off. Turning it off leaves the yaw where it is.
#[must_use]
pub fn look(looking: bool, input: LookInput) -> bool {
    if input.panel || input.release {
        return false;
    }
    if input.left_click {
        // While looking the pointer is hidden, so a click turns looking off wherever it is; a
        // click on a button never turns it on.
        return !looking && !input.over_button;
    }
    looking
}

/// Input, mouse look and movement, headless-safe: needs only `Time`, the input resources and a
/// `Session` (the cursor is grabbed only when there is a window).
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Intent>()
            .init_resource::<View>()
            .init_resource::<Current>()
            .add_systems(Update, grab_cursor.in_set(VectorSet::Grab))
            .add_systems(Update, read_input.in_set(VectorSet::Input))
            .add_systems(Update, advance.in_set(VectorSet::Move));
    }
}

/// A left click in the view turns mouse look on and another turns it off, leaving the view
/// where it looks. Space, a right click (the action keys) and Esc turn it off too, and so do
/// the panel opening and the fight screen (`movement::look`).
fn grab_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut intent: ResMut<Intent>,
    showing: Option<Res<Showing>>,
    view: Option<Res<State<ViewState>>>,
) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };
    let next = look(
        intent.looking,
        LookInput {
            left_click: mouse.just_pressed(MouseButton::Left),
            over_button: interactions.iter().any(|i| *i != Interaction::None),
            release: keys.any_just_pressed([KeyCode::Escape, KeyCode::Space])
                || mouse.just_pressed(MouseButton::Right),
            panel: showing.is_some_and(|s| s.0)
                || view.is_some_and(|v| *v.get() == ViewState::Fight),
        },
    );
    if next != intent.looking {
        intent.looking = next;
        cursor.grab_mode = if next {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        cursor.visible = !next;
    }
}

fn axis(keys: &ButtonInput<KeyCode>, plus: &[KeyCode], minus: &[KeyCode]) -> f32 {
    let held = |list: &[KeyCode]| list.iter().any(|k| keys.pressed(*k));
    f32::from(u8::from(held(plus))) - f32::from(u8::from(held(minus)))
}

/// Keys and mouse to `Intent`. Buttons add to it in the HUD's systems, which run first.
fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<AccumulatedMouseMotion>>,
    mut intent: ResMut<Intent>,
) {
    let forward = axis(
        &keys,
        &[KeyCode::KeyW, KeyCode::ArrowUp],
        &[KeyCode::KeyS, KeyCode::ArrowDown],
    );
    let strafe = axis(&keys, &[KeyCode::KeyD], &[KeyCode::KeyA]);
    if forward != 0.0 || strafe != 0.0 {
        intent.motion = Motion { forward, strafe };
    }
    let turn = axis(&keys, &[KeyCode::ArrowLeft], &[KeyCode::ArrowRight]);
    if turn != 0.0 {
        intent.turn = turn;
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        intent.pending_turn += FRAC_PI_2;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        intent.pending_turn -= FRAC_PI_2;
    }
    let tilt = axis(&keys, &[KeyCode::PageUp], &[KeyCode::PageDown]);
    if tilt != 0.0 {
        intent.tilt = tilt;
    }
    if keys.just_pressed(KeyCode::Home) {
        intent.level = true;
    }
    if intent.looking
        && let Some(mouse) = mouse
    {
        let most = |v: f32| (v * MOUSE_SENSITIVITY).clamp(-MAX_MOUSE_YAW, MAX_MOUSE_YAW);
        intent.mouse_yaw = most(-mouse.delta.x);
        intent.mouse_pitch = most(-mouse.delta.y);
    }
}

/// The frame's yaw change: mouse, held turn, and a slice of any pending 90° turn, or all of it at
/// once under reduced motion (`snap`).
fn yaw_delta(intent: &mut Intent, dt: f32, snap: bool) -> f32 {
    let most = if snap { f32::INFINITY } else { TURN_SPEED * dt };
    let slice = most
        .min(intent.pending_turn.abs())
        .copysign(intent.pending_turn);
    intent.pending_turn -= slice;
    if intent.pending_turn.abs() < 1e-4 {
        intent.pending_turn = 0.0;
    }
    intent.mouse_yaw + intent.turn * TURN_SPEED * dt + slice
}

/// The frame's pitch: the player's tilt first (it stops a return to level), then Home or the
/// Level button, then the return to level a step began, eased, or at once under reduced motion.
fn pitch(view: &mut View, intent: &Intent, dt: f32, snap: bool) {
    let tilt = intent.mouse_pitch + intent.tilt * TURN_SPEED * dt;
    if tilt != 0.0 {
        view.pitch = wrap_pitch(view.pitch + tilt);
        view.levelling = false;
    }
    if intent.level {
        view.pitch = 0.0;
        view.levelling = false;
    }
    if view.levelling {
        view.pitch = level(view.pitch, dt, snap);
        view.levelling = view.pitch != 0.0;
    }
}

fn advance(
    time: Res<Time>,
    current: Res<Current>,
    mut session: ResMut<Session>,
    mut intent: ResMut<Intent>,
    mut view: ResMut<View>,
) {
    let dt = time.delta_secs().min(MAX_DT);
    let snap = current.0.reduced_motion;
    let session = &mut *session;
    if view.seen != Some((session.views.position.map, session.pose)) {
        view.pitch = 0.0;
        view.levelling = false;
    }
    // Upside down, the mouse turns the other way, so the picture turns the way it moves.
    intent.mouse_yaw *= yaw_sign(view.pitch);
    let terrain_minutes = session
        .data
        .maps
        .get(&session.views.position.map)
        .and_then(|m| {
            m.cell(session.views.position.x, session.views.position.y)
                .map(|c| m.terrain(c).step_minutes)
        })
        .unwrap_or(1)
        .max(1);
    #[allow(clippy::cast_precision_loss)]
    let speed = BASE_SPEED / terrain_minutes as f32;
    let yaw = yaw_delta(&mut intent, dt, snap);
    if session.views.fallen() {
        // The notice offers a restart; a fallen party does not walk on.
        intent.motion = Motion::default();
    }
    let from = session.pose;
    let to = integrate(from, intent.motion, yaw, speed, dt);
    let out = session
        .binder
        .advance(&mut session.world, &session.data, from, to);
    let mut pose = out.pose.unwrap_or(to);
    if intent.motion.is_idle() && session.world.exploring_at(&session.data).is_some() {
        pose = ease(
            pose,
            session.binder.settle(&session.world, &session.data, pose),
            dt,
            snap,
        );
    }
    session.pose = pose;
    session.note(&out.events);
    let stepped = out.events.iter().any(|e| matches!(e, Event::Moved { .. }));
    if stepped && current.0.level_on_step.levels(intent.looking) {
        view.levelling = true;
    }
    pitch(&mut view, &intent, dt, snap);
    view.seen = Some((session.views.position.map, session.pose));
    // The mirror holds the pose at a cliff before the engine hears of a step, so the hold is
    // named with the line a step into it would give.
    if let Some(reason) = out.bumped.filter(|&r| r == BlockReason::Cliff) {
        session.note(&[Event::Blocked { reason }]);
    }
    *intent = Intent {
        pending_turn: intent.pending_turn,
        looking: intent.looking,
        ..Intent::default()
    };
}

/// Move the pose toward the settled pose at walking pace, so a stop near a boundary eases in; at
/// once under reduced motion (`snap`).
fn ease(pose: Pose, target: Pose, dt: f32, snap: bool) -> Pose {
    let (dx, dz) = (target.x - pose.x, target.z - pose.z);
    let len = dx.hypot(dz);
    let step = BASE_SPEED * 0.5 * dt;
    if snap || len <= step {
        return target;
    }
    Pose {
        x: pose.x + dx / len * step,
        z: pose.z + dz / len * step,
        yaw: pose.yaw,
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAC_PI_2, Intent, LookInput, Pose, ease, look, yaw_delta};

    #[test]
    fn reduced_motion_turns_a_quarter_in_one_frame_and_the_eased_turn_does_not() {
        let mut eased = Intent {
            pending_turn: FRAC_PI_2,
            ..Intent::default()
        };
        let first = yaw_delta(&mut eased, 1.0 / 60.0, false);
        assert!(first < FRAC_PI_2 * 0.1, "a slice: {first}");
        assert!(eased.pending_turn > 0.0, "the rest still to turn");
        let mut snapped = Intent {
            pending_turn: -FRAC_PI_2,
            ..Intent::default()
        };
        let all = yaw_delta(&mut snapped, 1.0 / 60.0, true);
        assert!((all + FRAC_PI_2).abs() < 1e-6, "the whole quarter: {all}");
        assert_eq!(snapped.pending_turn, 0.0);
    }

    #[test]
    fn reduced_motion_settles_onto_the_cell_at_once() {
        let at = Pose {
            x: 0.0,
            z: 0.0,
            yaw: 0.3,
        };
        let cell = Pose {
            x: 0.4,
            z: 0.0,
            yaw: 0.3,
        };
        assert_eq!(ease(at, cell, 1.0 / 60.0, true), cell);
        let eased = ease(at, cell, 1.0 / 60.0, false);
        assert!(
            eased.x > 0.0 && eased.x < 0.4,
            "part of the way: {}",
            eased.x
        );
    }

    fn click(over_button: bool) -> LookInput {
        LookInput {
            left_click: true,
            over_button,
            ..LookInput::default()
        }
    }

    #[test]
    fn a_left_click_in_the_view_toggles_mouse_look() {
        assert!(look(false, click(false)), "on");
        assert!(!look(true, click(false)), "and off again");
        assert!(
            !look(false, click(true)),
            "a click on a button is the button's"
        );
        assert!(
            !look(true, click(true)),
            "off wherever the hidden pointer is"
        );
        assert!(look(true, LookInput::default()), "no click: unchanged");
    }

    #[test]
    fn the_action_keys_esc_and_the_panel_give_the_pointer_back() {
        let release = LookInput {
            release: true,
            ..LookInput::default()
        };
        let panel = LookInput {
            panel: true,
            ..LookInput::default()
        };
        assert!(!look(true, release));
        assert!(!look(true, panel));
        // A click with the panel up does not turn looking on.
        assert!(!look(
            false,
            LookInput {
                panel: true,
                ..click(false)
            }
        ));
    }
}
