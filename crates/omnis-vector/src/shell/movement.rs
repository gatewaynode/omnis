//! Input to intent, and intent through the binder (presentation-ARCHITECTURE.md §6).

use super::screen::Showing;
use super::session::Session;
use super::{VectorSet, ViewState};
use crate::pose::{Motion, Pose, integrate};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use core::f32::consts::FRAC_PI_2;

/// Cells a second on terrain that costs one minute a step; slower terrain is slower to cross.
pub const BASE_SPEED: f32 = 3.0;
/// Radians a second for the keyboard and button turns.
pub const TURN_SPEED: f32 = 2.5;
/// Radians per pixel of mouse motion.
pub const MOUSE_SENSITIVITY: f32 = 0.003;
/// The longest frame integrated, so a stall cannot carry the pose across several cells.
pub const MAX_DT: f32 = 0.1;
/// The most yaw one frame of mouse motion may add.
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
    if intent.looking
        && let Some(mouse) = mouse
    {
        intent.mouse_yaw =
            (-mouse.delta.x * MOUSE_SENSITIVITY).clamp(-MAX_MOUSE_YAW, MAX_MOUSE_YAW);
    }
}

/// The frame's yaw change: mouse, held turn, and a slice of any pending 90° turn.
fn yaw_delta(intent: &mut Intent, dt: f32) -> f32 {
    let slice = (TURN_SPEED * dt)
        .min(intent.pending_turn.abs())
        .copysign(intent.pending_turn);
    intent.pending_turn -= slice;
    if intent.pending_turn.abs() < 1e-4 {
        intent.pending_turn = 0.0;
    }
    intent.mouse_yaw + intent.turn * TURN_SPEED * dt + slice
}

fn advance(time: Res<Time>, mut session: ResMut<Session>, mut intent: ResMut<Intent>) {
    let dt = time.delta_secs().min(MAX_DT);
    let session = &mut *session;
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
    let yaw = yaw_delta(&mut intent, dt);
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
        );
    }
    session.pose = pose;
    session.note(&out.events);
    *intent = Intent {
        pending_turn: intent.pending_turn,
        looking: intent.looking,
        ..Intent::default()
    };
}

/// Move the pose toward the settled pose at walking pace, so a stop near a boundary eases in.
fn ease(pose: Pose, target: Pose, dt: f32) -> Pose {
    let (dx, dz) = (target.x - pose.x, target.z - pose.z);
    let len = dx.hypot(dz);
    let step = BASE_SPEED * 0.5 * dt;
    if len <= step {
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
    use super::{LookInput, look};

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
