//! The binder: continuous pose changes become simulation commands, applied and reconciled
//! (presentation-ARCHITECTURE.md §5.1–5.6). The simulation's position is authoritative throughout.

use crate::collide::{blocks, clamp};
use crate::grid::{crossings, facing_of, relative, rotation};
use crate::pose::Pose;
use omnis_sim::omnis_core::{Facing, Position};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Event, Mode, Rejection, Replay, ReplayError, Settings, World, apply};

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
}

impl Binder {
    /// Apply one command, keeping it in the log when the simulation accepts it.
    ///
    /// # Errors
    /// The simulation's rejection; nothing is logged and the world is unchanged.
    pub fn apply(
        &mut self,
        world: &mut World,
        data: &Data,
        command: Command,
    ) -> Result<Vec<Event>, Rejection> {
        match apply(world, data, command.clone()) {
            Ok(events) => {
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
    pub fn advance(&mut self, world: &mut World, data: &Data, from: Pose, to: Pose) -> Outcome {
        let mut out = Outcome::default();
        if !matches!(world.mode, Mode::Explore) {
            out.frozen = true;
            out.pose = Some(self.settle(world, from));
            return out;
        }
        let facing = world.position.facing;
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
        world: &mut World,
        data: &Data,
        (prev, target): ((f32, f32), (f32, f32)),
        yaw: f32,
        out: &mut Outcome,
    ) -> Option<(f32, f32)> {
        let mut point = self.clamp_here(world, data, target);
        for _ in 0..2 {
            let here = world.position;
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
            let now = world.position;
            if moves > 1 || now.map != here.map || here.neighbour(heading) != Some((now.x, now.y)) {
                out.snapped = true;
                out.pose = Some(Pose::at(now));
                return None;
            }
            if !matches!(world.mode, Mode::Explore) {
                out.frozen = true;
                out.pose = Some(self.settle(
                    world,
                    Pose {
                        x: point.0,
                        z: point.1,
                        yaw,
                    },
                ));
                return None;
            }
            point = self.clamp_here(world, data, point);
        }
        Some(point)
    }

    /// Clamp a point against the closed sides of the simulation's current cell.
    fn clamp_here(&self, world: &World, data: &Data, point: (f32, f32)) -> (f32, f32) {
        let p = world.position;
        clamp(point, (p.x, p.y), self.tuning.radius, |f| {
            blocks(data, world, p.map, p.x, p.y, f).is_some()
        })
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
    pub fn settle(&self, world: &World, pose: Pose) -> Pose {
        let p = world.position;
        let (x, z) = clamp(pose.ground(), (p.x, p.y), 0.02, |_| true);
        Pose {
            x,
            z,
            yaw: pose.yaw,
        }
    }

    /// Use whatever is on the simulation's faced edge.
    pub fn interact(&mut self, world: &mut World, data: &Data) -> Vec<Event> {
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
