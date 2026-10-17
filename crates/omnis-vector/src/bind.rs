//! The binder: continuous pose changes become simulation commands, applied and reconciled
//! (presentation-ARCHITECTURE.md §5.1–5.6). The simulation's position is authoritative throughout.

use crate::collide::{blocks, clamp};
use crate::grid::{crossings, facing_of, relative, rotation};
use crate::pose::Pose;
use crate::sim::SimWorld;
use omnis_sim::omnis_core::{Facing, Position};
use omnis_sim::omnis_data::Data;
use omnis_sim::{BlockReason, Command, Event, Rejection, Replay, ReplayError, Settings};
use std::time::Instant;

/// The binder's distances, in cells, and angles, in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// How far past a boundary the pose must be before the cell counts as entered.
    pub margin: f32,
    /// How close the pose may come to a closed side.
    pub radius: f32,
    /// How far past a diagonal the yaw must be before the facing changes.
    pub yaw_margin: f32,
    /// The longest movement integrated in one piece.
    pub max_substep: f32,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning {
            margin: 0.15,
            radius: 0.2,
            yaw_margin: 5f32.to_radians(),
            max_substep: 0.25,
        }
    }
}

/// What one call to [`Binder::advance`] did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outcome {
    /// Where the pose ends up.
    pub pose: Option<Pose>,
    /// Every event the applied commands produced, in order.
    pub events: Vec<Event>,
    /// The pose jumped to the simulation's position (a portal, a disagreement).
    pub snapped: bool,
    /// The world is no longer exploring; motion stops.
    pub frozen: bool,
    /// Why a closed side holds the pose, when this call first pressed on it. A held step never
    /// reaches the simulation, so this stands in for its `Blocked`.
    pub bumped: Option<BlockReason>,
    /// Why a closed side holds the pose during this call, first or not.
    pub pressed: Option<BlockReason>,
}

/// What the accepted commands cost this session on this machine (M8.5 7a): a load by replay
/// (PRD D28) does the same applies again, so this prices one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Timing {
    /// Commands the simulation accepted, across loads.
    pub applied: u64,
    /// Microseconds their applies took together.
    pub micros: u64,
    /// When the first was applied.
    pub since: Option<Instant>,
}

impl Timing {
    /// Microseconds an accepted command took on average; `None` before the first.
    #[must_use]
    pub fn per_command(&self) -> Option<u64> {
        self.micros.checked_div(self.applied)
    }

    /// Milliseconds a replay of `commands` would take at that rate.
    #[must_use]
    pub fn replay_millis(&self, commands: usize) -> Option<u64> {
        let per = self.per_command()?;
        Some((commands as u64).saturating_mul(per) / 1000)
    }

    /// Commands an hour since the first, menus and pauses counted; `None` before a minute has
    /// passed.
    #[must_use]
    pub fn per_hour(&self, now: Instant) -> Option<u64> {
        let seconds = now.duration_since(self.since?).as_secs();
        (seconds >= 60).then(|| self.applied.saturating_mul(3600) / seconds)
    }

    fn add(&mut self, started: Instant) {
        self.since.get_or_insert(started);
        self.applied += 1;
        let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        self.micros = self.micros.saturating_add(took);
    }
}

/// Turns pose changes into commands, applies them, and keeps the accepted ones.
#[derive(Debug, Clone, Default)]
pub struct Binder {
    /// Distances and angles.
    pub tuning: Tuning,
    /// Every command the simulation accepted, in order: a replay of the session.
    pub log: Vec<Command>,
    /// Commands the simulation rejected (never expected while exploring).
    pub refusals: u32,
    /// Steps the mirror allowed but the simulation blocked (a bug in the mirror).
    pub disagreements: u32,
    /// What held the pose on the last call, so a contact is named once.
    pressing: Option<BlockReason>,
    /// What the accepted commands cost.
    pub timing: Timing,
}

impl Binder {
    /// Apply one command, keeping it in the log when the simulation accepts it.
    ///
    /// # Errors
    /// The simulation's rejection; nothing is logged and the world is unchanged.
    pub fn apply(
        &mut self,
        world: &mut SimWorld,
        data: &Data,
        command: Command,
    ) -> Result<Vec<Event>, Rejection> {
        let started = Instant::now();
        match world.apply(data, command.clone()) {
            Ok(events) => {
                self.timing.add(started);
                self.log.push(command);
                Ok(events)
            }
            Err(rejection) => {
                self.refusals += 1;
                Err(rejection)
            }
        }
    }

    /// Move the pose from `from` to `to`, emitting a `Turn` when the yaw reaches a new facing
    /// and a `Step` for every cell boundary crossed, in the order they are met.
    pub fn advance(&mut self, world: &mut SimWorld, data: &Data, from: Pose, to: Pose) -> Outcome {
        let mut out = self.motion(world, data, from, to);
        out.bumped = out.pressed.filter(|&r| self.pressing != Some(r));
        self.pressing = out.pressed;
        out
    }

    /// `advance` without naming the contact.
    fn motion(&mut self, world: &mut SimWorld, data: &Data, from: Pose, to: Pose) -> Outcome {
        let mut out = Outcome::default();
        let Some(here) = world.exploring_at(data) else {
            out.frozen = true;
            out.pose = Some(self.settle(world, data, from));
            return out;
        };
        let facing = here.facing;
        let want = facing_of(to.yaw, facing, self.tuning.yaw_margin);
        if let Some(turn) = rotation(facing, want)
            && let Ok(events) = self.apply(world, data, Command::Turn(turn))
        {
            out.events.extend(events);
        }
        let (dx, dz) = (to.x - from.x, to.z - from.z);
        // A frame moves a few cells at most (dt is clamped by the shell).
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let pieces = (dx.hypot(dz) / self.tuning.max_substep).ceil().max(1.0) as u32;
        let mut point = from.ground();
        for i in 1..=pieces {
            #[allow(clippy::cast_precision_loss)]
            let f = i as f32 / pieces as f32;
            let target = (from.x + dx * f, from.z + dz * f);
            match self.substep(world, data, (point, target), to.yaw, &mut out) {
                Some(next) => point = next,
                None => return out,
            }
        }
        out.pose = Some(Pose {
            x: point.0,
            z: point.1,
            yaw: to.yaw,
        });
        out
    }

    /// One short movement: clamp against closed sides, then step across up to two open
    /// boundaries. `None` when the pose was snapped or frozen and `out.pose` is final.
    fn substep(
        &mut self,
        world: &mut SimWorld,
        data: &Data,
        (prev, target): ((f32, f32), (f32, f32)),
        yaw: f32,
        out: &mut Outcome,
    ) -> Option<(f32, f32)> {
        let (mut point, pressed) = self.clamp_here(world, data, target);
        out.pressed = out.pressed.or(pressed);
        for _ in 0..2 {
            let Some(here) = world.position(data) else {
                break;
            };
            let cell = (i32::from(here.x), i32::from(here.y));
            let Some(&heading) = crossings(prev, point, cell, self.tuning.margin).first() else {
                break;
            };
            let command = Command::Step(relative(heading, here.facing));
            let Ok(events) = self.apply(world, data, command) else {
                return Some(self.hold(here, point, heading));
            };
            let moves = events
                .iter()
                .filter(|e| matches!(e, Event::Moved { .. }))
                .count();
            let blocked = events.iter().any(|e| matches!(e, Event::Blocked { .. }));
            out.events.extend(events);
            if blocked {
                self.disagreements += 1;
                return Some(self.hold(here, point, heading));
            }
            let Some(now) = world.position(data) else {
                break;
            };
            if moves > 1 || now.map != here.map || here.neighbour(heading) != Some((now.x, now.y)) {
                out.snapped = true;
                out.pose = Some(Pose::at(now));
                return None;
            }
            if world.exploring_at(data).is_none() {
                out.frozen = true;
                out.pose = Some(self.settle(
                    world,
                    data,
                    Pose {
                        x: point.0,
                        z: point.1,
                        yaw,
                    },
                ));
                return None;
            }
            let (held, pressed) = self.clamp_here(world, data, point);
            point = held;
            out.pressed = out.pressed.or(pressed);
        }
        Some(point)
    }

    /// Clamp a point against the closed sides of the simulation's current cell, with why the
    /// first side that moved it is closed.
    fn clamp_here(
        &self,
        world: &SimWorld,
        data: &Data,
        point: (f32, f32),
    ) -> ((f32, f32), Option<BlockReason>) {
        let Some(p) = world.position(data) else {
            return (point, None);
        };
        let reason = |f| blocks(data, world, p.map, p.level, p.x, p.y, f);
        let cell = (p.x, p.y);
        let radius = self.tuning.radius;
        let pressed = Facing::ALL
            .into_iter()
            .filter(|&f| clamp(point, cell, radius, |g| g == f) != point)
            .find_map(reason);
        (clamp(point, cell, radius, |f| reason(f).is_some()), pressed)
    }

    /// Keep a point inside `here` on the side toward `heading` after a refused step.
    fn hold(&self, here: Position, point: (f32, f32), heading: Facing) -> (f32, f32) {
        clamp(point, (here.x, here.y), self.tuning.radius, |f| {
            f == heading
        })
    }

    /// The pose moved just inside the simulation's cell, so the camera's cell equals the
    /// simulation's position. Used when motion stops.
    #[must_use]
    pub fn settle(&self, world: &SimWorld, data: &Data, pose: Pose) -> Pose {
        let Some(p) = world.position(data) else {
            return pose;
        };
        let (x, z) = clamp(pose.ground(), (p.x, p.y), 0.02, |_| true);
        Pose {
            x,
            z,
            yaw: pose.yaw,
        }
    }

    /// Use whatever is on the simulation's faced edge.
    pub fn interact(&mut self, world: &mut SimWorld, data: &Data) -> Vec<Event> {
        self.apply(world, data, Command::Interact)
            .unwrap_or_default()
    }

    /// The session so far as a replay, re-run from a fresh world to fill its fingerprint.
    ///
    /// # Errors
    /// The replay did not reproduce (a command was refused on re-run, or packs differ).
    pub fn replay(
        &self,
        data: &Data,
        seed: u64,
        settings: Settings,
    ) -> Result<Replay, ReplayError> {
        Replay::record(data, seed, settings, self.log.clone())
    }
}
