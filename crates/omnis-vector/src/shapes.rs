//! Objects and piles in the view (presentation-ARCHITECTURE.md §8.7, M8.5 step 2b): each
//! object's look (box, post, slab, arch, sign, heap, and a being's figure) as lines over dark fills, in the same world
//! space as [`crate::geometry`]'s segments and [`crate::fill`]'s triangles.
//!
//! A look's width runs across its facing and its depth along it, both in eighths of a cell, so
//! a shape's plan is the engine's footprint (`omnis_data::footprint`), turned the same way. It
//! stands on its home cell's floor, as the engine's sight test stands it, and rises `height`
//! eighths (a wall is 8). A shut arch (one that blocks a step) is drawn with its opening
//! crossed and filled, as a shut door is. A pile is a low heap of a fixed size. Bevy-free.

use crate::fill::Tri;
use crate::geometry::{SegKind, Segment};
use crate::ground::Ground;
use omnis_sim::api::{BeingView, PileView, SeenObject};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::{Look, Shape};

/// How a pile is drawn: a heap a quarter of a cell across and an eighth high.
pub const PILE_LOOK: Look = Look {
    shape: Shape::Heap,
    width: 2,
    depth: 2,
    height: 1,
};

/// The colour carried on an object's segments; the renderer colours them by kind.
const RGB: (u8, u8, u8) = (255, 255, 255);

/// An arch's legs, as a share of its half width from each side.
const LEG: f32 = 0.5;

/// Where an arch's lintel starts, as a share of its height.
const LINTEL: f32 = 0.75;

/// A sign's board starts at this share of its height, on a post this share of its half width.
const BOARD: f32 = 0.5;
const SIGN_POST: f32 = 0.25;

/// A heap's top, as a share of its base.
const HEAP_TOP: f32 = 0.5;

/// A figure (M8.5 step 2c): a body narrowing from its whole footprint to this share of it at the
/// shoulders, which stand at `SHOULDER` of its height, and a head this share of its width above.
const BODY_TOP: f32 = 0.6;
const SHOULDER: f32 = 0.75;
const HEAD: f32 = 0.4;

/// Where a shape stands: its centre on the ground, the unit vectors across and along its
/// facing, its half width and half depth, its floor and its height, all in cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// The centre, `(X, Z)`.
    pub centre: (f32, f32),
    /// Across the facing (its right), `(dX, dZ)`.
    pub across: (f32, f32),
    /// Along the facing (ahead), `(dX, dZ)`.
    pub along: (f32, f32),
    /// Half the width, across.
    pub half_width: f32,
    /// Half the depth, along.
    pub half_depth: f32,
    /// The floor it stands on.
    pub floor: f32,
    /// How tall it is.
    pub height: f32,
}

impl Body {
    /// A look at `(x, y)`, `pos` eighths east and south of the cell's centre, turned to
    /// `facing`, on that cell's floor in `ground`.
    #[must_use]
    pub fn of(x: u16, y: u16, pos: (i8, i8), facing: Facing, look: &Look, ground: &Ground) -> Body {
        let eighths = |v: i8| f32::from(v) / 8.0;
        let (along, across) = match facing {
            Facing::North => ((0.0, -1.0), (1.0, 0.0)),
            Facing::East => ((1.0, 0.0), (0.0, 1.0)),
            Facing::South => ((0.0, 1.0), (-1.0, 0.0)),
            Facing::West => ((-1.0, 0.0), (0.0, -1.0)),
        };
        Body {
            centre: (
                f32::from(x) + 0.5 + eighths(pos.0),
                f32::from(y) + 0.5 + eighths(pos.1),
            ),
            across,
            along,
            half_width: f32::from(look.width) / 16.0,
            half_depth: f32::from(look.depth) / 16.0,
            floor: ground.patch(x, y).map_or(0.0, |p| p.centre),
            height: f32::from(look.height) / 8.0,
        }
    }

    /// The point `u` of the half width across, `v` of the half depth along and `h` of the
    /// height up.
    fn at(&self, u: f32, v: f32, h: f32) -> [f32; 3] {
        let (a, f) = (u * self.half_width, v * self.half_depth);
        [
            self.centre.0 + self.across.0 * a + self.along.0 * f,
            self.floor + h * self.height,
            self.centre.1 + self.across.1 * a + self.along.1 * f,
        ]
    }
}

/// The lines and fills of every object and pile given, on `ground`.
#[must_use]
pub fn shapes(
    objects: &[SeenObject],
    piles: &[PileView],
    ground: &Ground,
) -> (Vec<Segment>, Vec<Tri>) {
    let mut out = Out {
        kind: SegKind::Object,
        lines: Vec::new(),
        fills: Vec::new(),
    };
    for o in objects {
        let body = Body::of(o.x, o.y, o.pos, o.facing, &o.look, ground);
        out.kind = SegKind::Object;
        out.look(&body, o.look.shape, o.blocks);
    }
    for p in piles {
        let body = Body::of(p.x, p.y, p.pos, Facing::North, &PILE_LOOK, ground);
        out.kind = SegKind::Pile;
        out.look(&body, PILE_LOOK.shape, false);
    }
    (out.lines, out.fills)
}

/// The lines and fills of every standing group given, each its front monster's figure, on
/// `ground` (M8.5 step 2c).
#[must_use]
pub fn figures(beings: &[BeingView], ground: &Ground) -> (Vec<Segment>, Vec<Tri>) {
    let mut out = Out {
        kind: SegKind::Being,
        lines: Vec::new(),
        fills: Vec::new(),
    };
    for b in beings {
        let body = Body::of(b.x, b.y, b.pos, b.facing, &b.look, ground);
        out.look(&body, b.look.shape, false);
    }
    (out.lines, out.fills)
}

struct Out {
    kind: SegKind,
    lines: Vec<Segment>,
    fills: Vec<Tri>,
}

/// A frustum's corners: the base `(u0, u1, v0, v1)` at `h0` and the top, shrunk by `top`
/// toward the middle, at `h1`.
struct Frustum {
    u: (f32, f32),
    v: (f32, f32),
    h: (f32, f32),
    top: f32,
}

impl Out {
    fn look(&mut self, body: &Body, shape: Shape, shut: bool) {
        let block = |u, v, h| Frustum { u, v, h, top: 1.0 };
        match shape {
            Shape::Box | Shape::Post | Shape::Slab => {
                self.frustum(body, &block((-1.0, 1.0), (-1.0, 1.0), (0.0, 1.0)));
            }
            Shape::Arch => {
                let inner = 1.0 - LEG;
                self.frustum(body, &block((-1.0, -inner), (-1.0, 1.0), (0.0, LINTEL)));
                self.frustum(body, &block((inner, 1.0), (-1.0, 1.0), (0.0, LINTEL)));
                self.frustum(body, &block((-1.0, 1.0), (-1.0, 1.0), (LINTEL, 1.0)));
                if shut {
                    let corner = |u, h| body.at(u, 0.0, h);
                    let (a, b) = (corner(-inner, 0.0), corner(inner, 0.0));
                    let (c, d) = (corner(inner, LINTEL), corner(-inner, LINTEL));
                    self.line(a, c);
                    self.line(b, d);
                    self.fills.extend([[a, b, c], [a, c, d]]);
                }
            }
            Shape::Sign => {
                let post = (-SIGN_POST, SIGN_POST);
                self.frustum(body, &block(post, (-1.0, 1.0), (0.0, BOARD)));
                self.frustum(body, &block((-1.0, 1.0), (-1.0, 1.0), (BOARD, 1.0)));
            }
            Shape::Heap => self.frustum(
                body,
                &Frustum {
                    top: HEAP_TOP,
                    ..block((-1.0, 1.0), (-1.0, 1.0), (0.0, 1.0))
                },
            ),
            Shape::Figure => {
                self.frustum(
                    body,
                    &Frustum {
                        top: BODY_TOP,
                        ..block((-1.0, 1.0), (-1.0, 1.0), (0.0, SHOULDER))
                    },
                );
                let head = (-HEAD, HEAD);
                self.frustum(body, &block(head, head, (SHOULDER, 1.0)));
            }
        }
    }

    /// Twelve edges and six faces.
    fn frustum(&mut self, body: &Body, f: &Frustum) {
        let ring = |h: f32, k: f32| {
            let (mu, mv) = ((f.u.0 + f.u.1) / 2.0, (f.v.0 + f.v.1) / 2.0);
            let s = |e: f32, m: f32| m + (e - m) * k;
            [
                body.at(s(f.u.0, mu), s(f.v.0, mv), h),
                body.at(s(f.u.1, mu), s(f.v.0, mv), h),
                body.at(s(f.u.1, mu), s(f.v.1, mv), h),
                body.at(s(f.u.0, mu), s(f.v.1, mv), h),
            ]
        };
        let (low, high) = (ring(f.h.0, 1.0), ring(f.h.1, f.top));
        for i in 0..4 {
            let j = (i + 1) % 4;
            self.line(low[i], low[j]);
            self.line(high[i], high[j]);
            self.line(low[i], high[i]);
            self.fills
                .extend([[low[i], low[j], high[j]], [low[i], high[j], high[i]]]);
        }
        self.fills.extend([
            [low[0], low[1], low[2]],
            [low[0], low[2], low[3]],
            [high[0], high[1], high[2]],
            [high[0], high[2], high[3]],
        ]);
    }

    fn line(&mut self, a: [f32; 3], b: [f32; 3]) {
        self.lines.push(Segment {
            a,
            b,
            kind: self.kind,
            rgb: RGB,
        });
    }
}
