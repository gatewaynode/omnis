//! A scene's lines as vertex buffers (rendering size 1, `tasks/plans/rendering-size-1.md`):
//! each segment is a quad of four vertices and two triangles that the line shader widens to a
//! constant width in pixels, so the view's lines are a retained mesh and not a loop over
//! segments each frame. No Bevy here: `shell/lines.rs` wraps the buffers in a mesh.
//!
//! A vertex carries both of its segment's ends, which corner of the quad it is, the line's
//! colour (linear, above 1.0 where it glows) with its level's brightness in the alpha, and a
//! `style` the shader reads: `[weight, pattern, role, level]`. Size 1 writes `[1, 0, 0, 0]`;
//! the other three are kept for size 2 (patterns, colour roles, depth levels) so the mesh's
//! layout does not change again.

use crate::geometry::{SegKind, Segment};

/// The corners of a segment's quad, in the order its vertices are written: how far along the
/// segment (0 at `a`, 1 at `b`) and which side of it (-1 or +1).
pub const CORNERS: [[f32; 2]; 4] = [[0.0, -1.0], [0.0, 1.0], [1.0, 1.0], [1.0, -1.0]];

/// The two triangles of a quad over [`CORNERS`].
pub const QUAD: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// The style of every line in size 1: weight 1, no pattern, no role, the party's depth level.
pub const PLAIN: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

/// A crest's line, where the ground turns away from the eye. Provisional until the colour roles
/// (M8.5 step 5): brighter than the floor grid, so a rise's top shows against the sky.
pub const CREST: (f32, f32, f32) = (1.1, 1.1, 1.1);

/// Lines as the line shader takes them: one entry a vertex in every list but `indices`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Strokes {
    /// The segment's first end.
    pub a: Vec<[f32; 3]>,
    /// The segment's other end.
    pub b: Vec<[f32; 3]>,
    /// Which corner of the quad: along (0 or 1) and side (-1 or +1).
    pub corner: Vec<[f32; 2]>,
    /// The line's linear colour, its level's brightness in the alpha.
    pub colour: Vec<[f32; 4]>,
    /// Weight, pattern, role and level.
    pub style: Vec<[f32; 4]>,
    /// Two triangles a segment.
    pub indices: Vec<u32>,
}

impl Strokes {
    /// The buffers of `lines`, each with its brightness.
    pub fn of<'s>(lines: impl IntoIterator<Item = (&'s Segment, f32)>) -> Strokes {
        let mut strokes = Strokes::default();
        for (segment, dim) in lines {
            strokes.push(segment.a, segment.b, colour(segment), dim);
        }
        strokes
    }

    /// One more line from `a` to `b` in `rgb` at brightness `dim`.
    pub fn push(&mut self, a: [f32; 3], b: [f32; 3], (r, g, b_): (f32, f32, f32), dim: f32) {
        // A scene's vertex count is far below u32's range.
        #[allow(clippy::cast_possible_truncation)]
        let first = self.a.len() as u32;
        for corner in CORNERS {
            self.a.push(a);
            self.b.push(b);
            self.corner.push(corner);
            self.colour.push([r, g, b_, dim]);
            self.style.push(PLAIN);
        }
        self.indices.extend(QUAD.iter().map(|i| first + i));
    }

    /// How many lines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.a.len() / CORNERS.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.a.is_empty()
    }
}

/// Line colour by kind: walls bright green, doors amber, solid and impassable terrain in the
/// terrain's colour, the floor and ceiling grids dim. Values above 1.0 bloom.
#[must_use]
pub fn colour(segment: &Segment) -> (f32, f32, f32) {
    let (r, g, b) = segment.rgb;
    let t = |k: f32| {
        (
            f32::from(r) / 255.0 * k,
            f32::from(g) / 255.0 * k,
            f32::from(b) / 255.0 * k,
        )
    };
    match segment.kind {
        SegKind::Wall => (0.15, 3.0, 0.5),
        SegKind::Door => (3.2, 1.6, 0.2),
        // A portal's floor mark in the doors' amber (owner, 2026-10-24); an activity's
        // placeholder smiley in a plain gold until it has a look of its own.
        SegKind::Portal => (3.2, 1.6, 0.2),
        SegKind::Activity => (2.6, 2.3, 0.5),
        SegKind::Block => t(3.0),
        SegKind::Barrier => t(2.5),
        SegKind::Floor => t(0.6),
        SegKind::Ceiling => t(0.3),
        // Provisional until the colour roles (M8.5 step 5): rock warm, ramps pale.
        SegKind::Cliff => (2.2, 1.4, 0.6),
        SegKind::Ramp => (0.9, 1.8, 2.2),
        // Joins (M8.5 step 1b), provisional as cliffs and ramps: stairs and ladders pale gold,
        // a hole a warning red.
        SegKind::Stair | SegKind::Ladder => (2.4, 2.1, 0.8),
        SegKind::Hole => (2.6, 0.5, 0.4),
        // Objects and piles (M8.5 step 2b), provisional: objects violet, piles rose.
        SegKind::Object => (1.9, 0.9, 2.8),
        SegKind::Pile => (2.8, 1.1, 1.7),
        // Beings (M8.5 step 2c), provisional: an alarm orange, apart from objects and holes.
        SegKind::Being => (3.0, 0.9, 0.15),
    }
}
