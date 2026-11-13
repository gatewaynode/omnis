//! A scene's lines as vertex buffers (rendering size 1, `tasks/plans/rendering-size-1.md`):
//! each segment is a quad of four vertices and two triangles that the line shader widens to a
//! constant width in pixels, so the view's lines are a retained mesh and not a loop over
//! segments each frame. No Bevy here: `shell/lines.rs` wraps the buffers in a mesh.
//!
//! A vertex carries both of its segment's ends, which corner of the quad it is, a colour with
//! its level's brightness in the alpha, and a `style` the shader reads:
//! `[weight, pattern, role, level]`. The weight and the pattern are the kind's ([`channels`];
//! rendering size 2, part 2); the role is whose colour the line is drawn in ([`role`]; part 3);
//! the level is kept for the depth levels, so the mesh's layout does not change again.

use crate::geometry::{SegKind, Segment};
use crate::style::{Palette, Pattern, Weight};
use omnis_sim::omnis_data::ColourRole;

/// The corners of a segment's quad, in the order its vertices are written: how far along the
/// segment (0 at `a`, 1 at `b`) and which side of it (-1 or +1).
pub const CORNERS: [[f32; 2]; 4] = [[0.0, -1.0], [0.0, 1.0], [1.0, 1.0], [1.0, -1.0]];

/// The two triangles of a quad over [`CORNERS`].
pub const QUAD: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// How a line is drawn: the role its colour comes from, how strongly that colour is laid on,
/// and the channels that carry its meaning without colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Whose colour, from the palette.
    pub role: ColourRole,
    /// A factor on the role's ink: a ceiling grid is fainter than the floor's.
    pub strength: f32,
    /// How heavy.
    pub weight: Weight,
    /// Solid, dashed, dotted or double.
    pub pattern: Pattern,
}

impl Look {
    /// What the shader reads of it: `[weight, pattern, role, level]`, the level the party's.
    #[must_use]
    pub fn style(&self) -> [f32; 4] {
        // A role's number is far below f32's exact range.
        #[allow(clippy::cast_precision_loss)]
        let role = self.role.index() as f32;
        [self.weight.factor(), self.pattern.code(), role, 0.0]
    }
}

/// A crest's line, where the ground turns away from the eye: the ground's, brighter than the
/// floor grid, so a rise's top shows against the sky.
pub const CREST: Look = Look {
    role: ColourRole::Ground,
    strength: 2.5,
    weight: Weight::Normal,
    pattern: Pattern::Solid,
};

/// The outline of a pile's cell's floor under the pointer: a thing's, plain.
pub const AIM: Look = Look {
    role: ColourRole::Thing,
    strength: 1.0,
    weight: Weight::Normal,
    pattern: Pattern::Solid,
};

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
            strokes.push(segment.a, segment.b, look(segment), dim);
        }
        strokes
    }

    /// One more line from `a` to `b`, drawn as `look` says at brightness `dim`.
    pub fn push(&mut self, a: [f32; 3], b: [f32; 3], look: Look, dim: f32) {
        // A scene's vertex count is far below u32's range.
        #[allow(clippy::cast_possible_truncation)]
        let first = self.a.len() as u32;
        // Until the palette is the material's (step 3), the default's ink is laid in here.
        let [r, g, b_] = Palette::default().ink(look.role).map(|c| c * look.strength);
        for corner in CORNERS {
            self.a.push(a);
            self.b.push(b);
            self.corner.push(corner);
            self.colour.push([r, g, b_, dim]);
            self.style.push(look.style());
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

/// A kind's weight and pattern, the channels that carry its meaning without colour
/// (presentation-PRD X19). Provisional until the roles (M8.5 step 5), after the PRD's own
/// examples (5.3): structure and terrain plain, the grids hairline as scenery, what can be used
/// heavy and dashed, beings heavy with two rails (until there is a sawtooth), a portal's mark
/// dotted.
#[must_use]
pub fn channels(kind: SegKind) -> (Weight, Pattern) {
    match kind {
        SegKind::Wall
        | SegKind::Door
        | SegKind::Block
        | SegKind::Barrier
        | SegKind::Cliff
        | SegKind::Ramp
        | SegKind::Stair
        | SegKind::Ladder
        | SegKind::Hole
        | SegKind::Activity => (Weight::Normal, Pattern::Solid),
        SegKind::Floor | SegKind::Ceiling => (Weight::Hairline, Pattern::Solid),
        SegKind::Object | SegKind::Pile => (Weight::Heavy, Pattern::Dashed),
        SegKind::Being => (Weight::Heavy, Pattern::Double),
        SegKind::Portal => (Weight::Normal, Pattern::Dotted),
    }
}

/// A segment's look: its role, its strength and its kind's channels.
#[must_use]
pub fn look(segment: &Segment) -> Look {
    let (weight, pattern) = channels(segment.kind);
    Look {
        role: role(segment),
        strength: strength(segment.kind),
        weight,
        pattern,
    }
}

/// The role a segment's colour comes from (presentation-PRD 5.2; the set and the table the
/// owner's, 2026-11-13): what stands is structure, what is walked on ground, a way through a
/// passage, what can be used a thing, a being hostile, a hole a danger. A terrain that names a
/// role gives it to its own grids and solid forms.
#[must_use]
pub fn role(segment: &Segment) -> ColourRole {
    let named = |own: ColourRole| segment.role.unwrap_or(own);
    match segment.kind {
        SegKind::Wall | SegKind::Cliff => ColourRole::Structure,
        SegKind::Block | SegKind::Barrier => named(ColourRole::Structure),
        SegKind::Floor | SegKind::Ceiling | SegKind::Ramp => named(ColourRole::Ground),
        SegKind::Door | SegKind::Portal | SegKind::Stair | SegKind::Ladder => ColourRole::Passage,
        SegKind::Object | SegKind::Pile | SegKind::Activity => ColourRole::Thing,
        SegKind::Being => ColourRole::Hostile,
        SegKind::Hole => ColourRole::Danger,
    }
}

/// How strongly a kind lays on its role's ink.
#[must_use]
pub fn strength(kind: SegKind) -> f32 {
    match kind {
        // The ceiling grid is fainter than the floor's.
        SegKind::Ceiling => 0.5,
        // A ramp's treads are the ground's, and must show on its dim grid.
        SegKind::Ramp => 3.0,
        SegKind::Barrier => 0.85,
        _ => 1.0,
    }
}
