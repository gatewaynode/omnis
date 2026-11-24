//! A doorway's own lines inside the wall's outline (owner, 2026-11-24: "the door remains don't
//! really indicate debris ... better line art indicating a damaged entrance where a door once
//! was is needed (as well as the structural cave in marker)"). Shut, it is a frame with a
//! cross; open, the frame; broken, the posts snapped at unequal heights, a piece of the head
//! still hanging and planks lying across the sill on both sides; buried, a mound of rubble to
//! two thirds of the opening with boulders in it and the lintel fallen across. Every piece is
//! placed by the edge's own numbers, so a doorway looks the same each time it is drawn.

use crate::geometry::{DOOR, Named, SegKind, Segment, door_head};

/// How a door stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// Closed: the frame, crossed.
    Shut,
    /// Open: the frame.
    Open,
    /// Broken and passable: what is left of the frame, and debris.
    Broken,
    /// Broken, and the doorway fell in: rubble.
    Buried,
}

impl Leaf {
    /// From what the world says of the door.
    pub(crate) fn of(open: bool, broken: bool, collapsed: bool) -> Leaf {
        match (collapsed, broken, open) {
            (true, ..) => Leaf::Buried,
            (false, true, _) => Leaf::Broken,
            (false, false, true) => Leaf::Open,
            (false, false, false) => Leaf::Shut,
        }
    }
}

/// Where a doorway stands: the edge's ends on the plan, the ground's height at each, the wall's
/// top, the terrain's role, and the edge's own number.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Place {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub feet: [f32; 2],
    pub top: f32,
    pub role: Named,
    pub seed: u32,
}

/// The mound's height at its peak, as a share of the door's head.
pub(crate) const MOUND: f32 = 0.66;
/// The snapped posts' heights as shares of the head: the short one and the tall one.
pub(crate) const SNAPPED: [(f32, f32); 2] = [(0.22, 0.12), (0.42, 0.16)];
/// How far from the edge debris may lie, in cells.
pub(crate) const SPILL: f32 = 0.26;
/// Debris lies this far above the ground, clear of the floor's own lines.
const LIFT: f32 = 0.004;

impl Place {
    /// A share of one in `[0, 1)` for this doorway's `n`th choice: the same every time.
    fn chance(&self, n: u32) -> f32 {
        let mut x = self.seed.wrapping_mul(0x9E37_79B9) ^ n.wrapping_mul(0x85EB_CA6B);
        x ^= x >> 15;
        x = x.wrapping_mul(0x2C1B_3C6D);
        x ^= x >> 12;
        x = x.wrapping_mul(0x297A_2D39);
        x ^= x >> 15;
        #[allow(clippy::cast_precision_loss)]
        let share = (x >> 8) as f32 / (1u32 << 24) as f32;
        share
    }

    /// The ground's height `t` along the edge.
    fn foot(&self, t: f32) -> f32 {
        self.feet[0] + (self.feet[1] - self.feet[0]) * t
    }

    /// The door's head.
    fn head(&self) -> f32 {
        door_head(self.feet, self.top)
    }

    /// The height `share` of the way from the ground at `t` to the head.
    fn up(&self, t: f32, share: f32) -> f32 {
        self.foot(t) + (self.head() - self.foot(t)) * share
    }

    /// The point `t` along the edge at height `y`, `off` cells out of the wall's plane.
    fn at(&self, t: f32, y: f32, off: f32) -> [f32; 3] {
        let (dx, dz) = (self.b[0] - self.a[0], self.b[1] - self.a[1]);
        [
            self.a[0] + dx * t - dz * off,
            y,
            self.a[1] + dz * t + dx * off,
        ]
    }

    fn line(&self, out: &mut Vec<Segment>, a: [f32; 3], b: [f32; 3]) {
        out.push(Segment {
            a,
            b,
            kind: SegKind::Door,
            role: self.role,
        });
    }

    /// The two posts snapped off, one short and one tall; which is which is the doorway's own.
    /// Returns the heights' shares, left then right.
    fn stumps(&self, out: &mut Vec<Segment>) -> [f32; 2] {
        let [l, r] = DOOR;
        let swap = self.chance(0) < 0.5;
        let share = |i: usize, n: u32| SNAPPED[i].0 + SNAPPED[i].1 * self.chance(n);
        let (left, right) = if swap {
            (share(1, 1), share(0, 2))
        } else {
            (share(0, 1), share(1, 2))
        };
        for (t, s) in [(l, left), (r, right)] {
            self.line(
                out,
                self.at(t, self.foot(t), 0.0),
                self.at(t, self.up(t, s), 0.0),
            );
        }
        [left, right]
    }

    /// `count` short pieces lying on the ground by the sill, turn about on either side of the
    /// wall, each up to `long` cells long.
    fn litter(&self, out: &mut Vec<Segment>, count: u32, long: f32, from: u32) {
        let [l, r] = DOOR;
        for i in 0..count {
            let n = from + i * 4;
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let t = l + (r - l) * (0.1 + 0.8 * self.chance(n));
            let off = side * (0.07 + (SPILL - 0.07 - long / 2.0) * self.chance(n + 1));
            // Lying any way round: along the wall, across it, or between.
            let turn = self.chance(n + 2) * core::f32::consts::PI;
            let half = long * (0.6 + 0.4 * self.chance(n + 3)) / 2.0;
            let (along, across) = (turn.cos() * half, turn.sin() * half);
            let end = |t: f32, off: f32| self.at(t, self.foot(t) + LIFT, off);
            self.line(
                out,
                end(t - along, off - across),
                end(t + along, off + across),
            );
        }
    }

    /// Broken and passable: splintered stumps, a piece of the frame's head hanging from one
    /// jamb, and planks across the sill.
    fn broken(&self, out: &mut Vec<Segment>) {
        let [l, r] = DOOR;
        let shares = self.stumps(out);
        // Each stump ends in a splinter leaning into the opening.
        for (t, s, lean) in [(l, shares[0], 0.035), (r, shares[1], -0.035)] {
            let tip = (s + 0.09).min(1.0);
            self.line(
                out,
                self.at(t, self.up(t, s), 0.0),
                self.at(t + lean, self.up(t + lean, tip), 0.0),
            );
        }
        // What still hangs: the top of one jamb and a drooping length of the head.
        let (t, droop) = if self.chance(3) < 0.5 {
            (l, 0.17)
        } else {
            (r, -0.17)
        };
        let head = self.head();
        self.line(
            out,
            self.at(t, head, 0.0),
            self.at(t, self.up(t, 0.78), 0.0),
        );
        self.line(
            out,
            self.at(t, head, 0.0),
            self.at(t + droop, self.up(t + droop, 0.92), 0.0),
        );
        self.litter(out, 4, 0.2, 10);
    }

    /// Buried: the mound's outline from sill to sill, boulders within it, the lintel fallen
    /// across it at a slant, and stones spilled on the ground either side.
    fn buried(&self, out: &mut Vec<Segment>) {
        let [l, r] = DOOR;
        let w = r - l;
        self.stumps(out);
        // The mound: up from one sill, over a peak off its middle, down to the other.
        let peak = 0.35 + 0.3 * self.chance(4);
        let ridge = [
            (0.0, 0.0),
            (peak * 0.35, MOUND * (0.5 + 0.1 * self.chance(5))),
            (peak * 0.75, MOUND * (0.82 + 0.1 * self.chance(6))),
            (peak, MOUND),
            (
                peak + (1.0 - peak) * 0.4,
                MOUND * (0.7 + 0.12 * self.chance(7)),
            ),
            (
                peak + (1.0 - peak) * 0.75,
                MOUND * (0.36 + 0.1 * self.chance(8)),
            ),
            (1.0, 0.0),
        ];
        let on = |(u, s): (f32, f32)| {
            let t = l + w * u;
            self.at(t, self.up(t, s), 0.0)
        };
        for pair in ridge.windows(2) {
            self.line(out, on(pair[0]), on(pair[1]));
        }
        // Boulders: lopsided four-sided stones low in the mound.
        for (i, u) in [(0u32, 0.24), (1, 0.52), (2, 0.78)] {
            let n = 20 + i * 4;
            let u = u + 0.06 * (self.chance(n) - 0.5);
            let s = MOUND * (0.12 + 0.2 * self.chance(n + 1));
            let (half, tall) = (
                0.05 + 0.03 * self.chance(n + 2),
                MOUND * (0.1 + 0.06 * self.chance(n + 3)),
            );
            let stone = [
                (u - half, s),
                (u - half * 0.3, s + tall),
                (u + half, s + tall * 0.55),
                (u + half * 0.4, s - tall * 0.5),
            ];
            for k in 0..4 {
                self.line(out, on(stone[k]), on(stone[(k + 1) % 4]));
            }
        }
        // The lintel, come down across the mound: a beam with one end high.
        let (high, low) = if self.chance(9) < 0.5 {
            (0.04, 0.96)
        } else {
            (0.96, 0.04)
        };
        let (thick, top, bottom) = (0.07, 0.6, 0.2);
        let beam = [
            (high, top),
            (low, bottom),
            (low, bottom + thick),
            (high, top + thick),
        ];
        for k in 0..4 {
            self.line(out, on(beam[k]), on(beam[(k + 1) % 4]));
        }
        self.litter(out, 4, 0.1, 40);
    }

    /// The doorway's lines for a door that stands as `leaf`.
    pub(crate) fn draw(&self, out: &mut Vec<Segment>, leaf: Leaf) {
        let [l, r] = DOOR;
        let head = self.head();
        match leaf {
            Leaf::Broken => self.broken(out),
            Leaf::Buried => self.buried(out),
            Leaf::Shut | Leaf::Open => {
                let post = |t: f32| (self.at(t, self.foot(t), 0.0), self.at(t, head, 0.0));
                let ((lf, lh), (rf, rh)) = (post(l), post(r));
                self.line(out, lf, lh);
                self.line(out, rf, rh);
                self.line(out, lh, rh);
                if leaf == Leaf::Shut {
                    self.line(out, lf, rh);
                    self.line(out, rf, lh);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(seed: u32) -> Place {
        Place {
            a: [3.0, 5.0],
            b: [4.0, 5.0],
            feet: [0.0; 2],
            top: 1.0,
            role: None,
            seed,
        }
    }

    fn drawn(seed: u32, leaf: Leaf) -> Vec<Segment> {
        let mut out = Vec::new();
        place(seed).draw(&mut out, leaf);
        out
    }

    /// Whether a point lies in the wall's plane.
    fn in_plane(p: [f32; 3]) -> bool {
        (p[2] - 5.0).abs() < 1e-5
    }

    fn upright(s: &Segment) -> bool {
        in_plane(s.a) && in_plane(s.b) && (s.a[0] - s.b[0]).abs() < 1e-6
    }

    #[test]
    fn the_world_s_word_picks_the_picture() {
        assert_eq!(Leaf::of(false, false, false), Leaf::Shut);
        assert_eq!(Leaf::of(true, false, false), Leaf::Open);
        assert_eq!(Leaf::of(true, true, false), Leaf::Broken);
        assert_eq!(Leaf::of(false, true, true), Leaf::Buried);
    }

    #[test]
    fn a_shut_door_is_a_crossed_frame_and_an_open_one_the_frame() {
        assert_eq!(drawn(1, Leaf::Shut).len(), 5);
        assert_eq!(drawn(1, Leaf::Open).len(), 3);
        assert_eq!(drawn(1, Leaf::Shut)[..3], drawn(1, Leaf::Open)[..]);
    }

    #[test]
    fn a_broken_door_is_snapped_posts_a_hanging_piece_and_planks_on_both_sides() {
        let head = door_head([0.0; 2], 1.0);
        for seed in 0..200 {
            let lines = drawn(seed, Leaf::Broken);
            assert!(lines.iter().all(|s| s.kind == SegKind::Door));
            // The two posts rise from the sill and stop short of the head, unequally.
            let posts: Vec<&Segment> = lines
                .iter()
                .filter(|s| upright(s) && s.a[1].abs() < 1e-6)
                .collect();
            assert_eq!(posts.len(), 2, "seed {seed}");
            let (one, other) = (posts[0].b[1], posts[1].b[1]);
            assert!(one < head * 0.6 && other < head * 0.6, "seed {seed}");
            assert!((one - other).abs() > head * 0.05, "seed {seed}: unequal");
            // No head spans the opening, and something still hangs from where it was.
            let spans = |s: &&Segment| (s.a[0] - s.b[0]).abs() > 0.5 && in_plane(s.a);
            assert_eq!(lines.iter().filter(spans).count(), 0, "seed {seed}");
            let hangs = lines
                .iter()
                .filter(|s| (s.a[1] - head).abs() < 1e-6 && s.b[1] < head)
                .count();
            assert_eq!(
                hangs, 2,
                "seed {seed}: a jamb's top and a length of the head"
            );
            // Planks lie on the ground out of the wall's plane, on both sides of it.
            let lying: Vec<&Segment> = lines
                .iter()
                .filter(|s| s.a[1] < 0.01 && s.b[1] < 0.01)
                .collect();
            assert_eq!(lying.len(), 4, "seed {seed}");
            let side = |s: &&Segment| (s.a[2] + s.b[2]) / 2.0 - 5.0;
            assert!(lying.iter().any(|s| side(s) > 0.03), "seed {seed}");
            assert!(lying.iter().any(|s| side(s) < -0.03), "seed {seed}");
            for s in &lines {
                for p in [s.a, s.b] {
                    assert!((p[2] - 5.0).abs() <= SPILL + 1e-5, "seed {seed}: {p:?}");
                    assert!((3.0..=4.0).contains(&p[0]), "seed {seed}: {p:?}");
                    assert!((0.0..=head + 1e-6).contains(&p[1]), "seed {seed}: {p:?}");
                }
            }
            assert_eq!(lines.len(), 10);
        }
    }

    #[test]
    fn a_buried_doorway_is_a_mound_with_boulders_and_the_fallen_lintel() {
        let head = door_head([0.0; 2], 1.0);
        let [l, r] = DOOR;
        for seed in 0..200 {
            let lines = drawn(seed, Leaf::Buried);
            let flat: Vec<&Segment> = lines
                .iter()
                .filter(|s| in_plane(s.a) && in_plane(s.b))
                .collect();
            // The mound: a chain from sill to sill whose peak is two thirds of the head.
            let ridge = &flat[2..8];
            assert!((ridge[0].a[0] - (3.0 + l)).abs() < 1e-5 && ridge[0].a[1].abs() < 1e-6);
            assert!((ridge[5].b[0] - (3.0 + r)).abs() < 1e-5 && ridge[5].b[1].abs() < 1e-6);
            for pair in ridge.windows(2) {
                assert_eq!(pair[0].b, pair[1].a, "seed {seed}: one outline");
                assert!(pair[0].b[0] < pair[1].b[0], "seed {seed}: left to right");
            }
            let peak = ridge.iter().map(|s| s.b[1]).fold(0.0, f32::max);
            assert!((peak - head * MOUND).abs() < 1e-5, "seed {seed}: {peak}");
            // Nothing stands above the head, and the opening is filled: lines cross its middle.
            let high = lines.iter().flat_map(|s| [s.a[1], s.b[1]]);
            assert!(high.fold(0.0, f32::max) <= head, "seed {seed}");
            let middle = flat
                .iter()
                .filter(|s| (s.a[0] - 3.5) * (s.b[0] - 3.5) <= 0.0)
                .count();
            assert!(middle >= 2, "seed {seed}: the mound and the lintel");
            // The lintel: a slanted beam the width of the opening, one end high.
            let beam: Vec<&&Segment> = flat
                .iter()
                .filter(|s| (s.a[0] - s.b[0]).abs() > (r - l) * 0.8)
                .collect();
            assert_eq!(beam.len(), 2, "seed {seed}: the beam's two long sides");
            assert!((beam[0].a[1] - beam[0].b[1]).abs() > head * 0.3, "a slant");
            // Stones spilled on the ground on both sides.
            let spilled: Vec<&Segment> = lines.iter().filter(|s| !in_plane(s.a)).collect();
            assert_eq!(spilled.len(), 4, "seed {seed}");
            assert_eq!(lines.len(), 2 + 6 + 12 + 4 + 4);
        }
    }

    #[test]
    fn a_doorway_is_drawn_the_same_each_time_and_not_as_its_neighbour() {
        for leaf in [Leaf::Broken, Leaf::Buried] {
            assert_eq!(drawn(7, leaf), drawn(7, leaf));
            assert_ne!(drawn(7, leaf), drawn(8, leaf));
        }
        // Over many doorways both posts are the short one sometimes.
        let short_left = (0..64)
            .filter(|seed| {
                let lines = drawn(*seed, Leaf::Broken);
                lines[0].b[1] < lines[1].b[1]
            })
            .count();
        assert!((12..=52).contains(&short_left), "{short_left}");
    }

    #[test]
    fn a_doorway_on_a_slope_stands_on_its_ground() {
        let mut out = Vec::new();
        let sloped = Place {
            feet: [0.0, 0.5],
            top: 1.5,
            ..place(3)
        };
        sloped.draw(&mut out, Leaf::Buried);
        for s in &out {
            for p in [s.a, s.b] {
                let ground = (p[0] - 3.0) * 0.5;
                assert!(p[1] >= ground - 1e-5, "{p:?} under the ground");
            }
        }
    }
}
