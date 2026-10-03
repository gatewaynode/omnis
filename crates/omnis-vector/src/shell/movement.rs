//! Input to intent, and intent through the binder (alt-ARCHITECTURE.md §6).

use super::VectorSet;
use super::fight::fallen;
use super::session::Session;
use crate::pose::{Motion, Pose, integrate};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use core::f32::consts::FRAC_PI_2;
use omnis_sim::Mode;

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
    /// Use the faced edge this frame.
    pub interact: bool,
    /// Whether the mouse looks (the cursor is grabbed).
    pub looking: bool,
}

/// Input and movement, headless-safe: needs only `Time`, the input resources and a `Session`.
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Intent>()
            .add_systems(Update, read_input.in_set(VectorSet::Input))
            .add_systems(Update, advance.in_set(VectorSet::Move));
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
    if keys.just_pressed(KeyCode::KeyR) {
        intent.pending_turn -= FRAC_PI_2;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        intent.interact = true;
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
        .get(&session.world.position.map)
        .and_then(|m| {
            m.cell(session.world.position.x, session.world.position.y)
                .map(|c| m.terrain(c).step_minutes)
        })
        .unwrap_or(1)
        .max(1);
    #[allow(clippy::cast_precision_loss)]
    let speed = BASE_SPEED / terrain_minutes as f32;
    let yaw = yaw_delta(&mut intent, dt);
    if fallen(&session.world, &session.data) {
        // The notice offers a restart; a fallen party does not walk on.
        intent.motion = Motion::default();
    }
    let from = session.pose;
    let to = integrate(from, intent.motion, yaw, speed, dt);
    let out = session
        .binder
        .advance(&mut session.world, &session.data, from, to);
    let mut pose = out.pose.unwrap_or(to);
    if intent.motion.is_idle() && matches!(session.world.mode, Mode::Explore) {
        pose = ease(pose, session.binder.settle(&session.world, pose), dt);
    }
    session.pose = pose;
    let mut events = out.events;
    if intent.interact && matches!(session.world.mode, Mode::Explore) {
        events.extend(session.binder.interact(&mut session.world, &session.data));
    }
    session.note(&events);
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
