//! Monster silhouettes in the viewport: where each living stack stands and the plain shapes
//! that stand in for art until a CC0 set arrives (owner decision 2026-09-13, no layout change
//! when it does). Shared by the sprite layer (`viewport.rs`) and the UI frame
//! (`combat_screen.rs` centres the counts on them). Bevy-free; imports only the layout and
//! the draw ops.

use crate::layout::{Camera, Rect, VIEWPORT, VIEWPORT_SIZE};
use crate::plan::{DrawOp, Paint};
use omnis_sim::omnis_data::Size;

/// Lateral slot in tiles by stack index: the first stack ahead, then left, right, further
/// left, further right.
pub const SLOTS: [f32; 5] = [0.0, -1.0, 1.0, -2.0, 2.0];
/// Distance of the front stacks: feet on the clear window's floor line.
pub const FRONT_Z: f32 = 3.0;
/// Distance of the back stacks.
pub const BACK_Z: f32 = 4.5;
/// The front silhouettes' colour.
pub const FRONT_COLOR: (u8, u8, u8) = (20, 14, 24);
/// The canvas y of the front silhouettes' feet: the floor line at `FRONT_Z`.
pub const FRONT_FEET_Y: i32 = (Camera::new(VIEWPORT_SIZE).sy(0.0, FRONT_Z) + 0.5) as i32;
/// The back silhouettes' colour, a shade lighter for depth.
pub const BACK_COLOR: (u8, u8, u8) = (36, 30, 44);
/// The one-pixel rim around a front silhouette, so a dark shape reads against a dark wall.
pub const RIM_FRONT: (u8, u8, u8) = (128, 120, 140);
/// The rim around a back silhouette.
pub const RIM_BACK: (u8, u8, u8) = (88, 82, 100);

/// A living stack as the silhouettes need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    /// Its index in the encounter.
    pub index: u8,
    /// The monster's size.
    pub size: Size,
    /// Whether it stands in front.
    pub front: bool,
}

/// Where a silhouette stands on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Silhouette {
    /// The stack.
    pub index: u8,
    /// Its bounding box, feet on the bottom edge.
    pub rect: Rect,
    /// Whether it stands in front.
    pub front: bool,
}

/// A creature's height in tiles by size.
#[must_use]
pub const fn height_tiles(size: Size) -> f32 {
    match size {
        Size::Tiny => 0.3,
        Size::Small => 0.55,
        Size::Medium => 0.8,
        Size::Large => 1.0,
        Size::Huge => 1.25,
        Size::Gargantuan => 1.5,
    }
}

/// Where each actor stands: the camera of the viewport places its slot at its distance, the
/// height follows its size, the width is half the height.
#[must_use]
pub fn silhouettes(actors: &[Actor]) -> Vec<Silhouette> {
    let camera = Camera::new(VIEWPORT_SIZE);
    actors
        .iter()
        .map(|actor| {
            let z = if actor.front { FRONT_Z } else { BACK_Z };
            let slot = SLOTS[usize::from(actor.index) % SLOTS.len()];
            let height = (height_tiles(actor.size) * camera.focal / z) as u32;
            let width = (height / 2).max(2);
            // The lateral offset is truncated, so the left and right slots mirror exactly.
            let centre = (camera.width / 2.0) as i32 + (slot * camera.focal / z) as i32;
            let feet = camera.sy(0.0, z).round() as i32;
            Silhouette {
                index: actor.index,
                rect: Rect::new(
                    centre - width as i32 / 2,
                    feet - height as i32,
                    width,
                    height,
                ),
                front: actor.front,
            }
        })
        .collect()
}

/// A silhouette's parts: head, body, left leg, right leg, as `(x, y, width, height)`.
fn parts(s: &Silhouette) -> [(i32, i32, u32, u32); 4] {
    let (w, h) = (s.rect.w, s.rect.h);
    let (head_h, body_h) = (h / 4, h / 2);
    let leg_h = h - head_h - body_h;
    let leg_w = (w * 35 / 100).max(1);
    let head_w = (w / 2).max(1);
    [
        (s.rect.x + (w - head_w) as i32 / 2, s.rect.y, head_w, head_h),
        (s.rect.x, s.rect.y + head_h as i32, w, body_h),
        (s.rect.x, s.rect.bottom() - leg_h as i32, leg_w, leg_h),
        (
            s.rect.right() - leg_w as i32,
            s.rect.bottom() - leg_h as i32,
            leg_w,
            leg_h,
        ),
    ]
}

fn fill(color: (u8, u8, u8), (x, y, width, height): (i32, i32, u32, u32)) -> DrawOp {
    DrawOp {
        paint: Paint::Fill {
            color,
            width,
            height,
        },
        x,
        y,
    }
}

/// The draw ops per silhouette: the four parts widened by a pixel in the rim colour, then the
/// head, body, and two legs as dark fills over them, so a one-pixel light edge outlines the
/// union of the parts.
#[must_use]
pub fn ops(silhouettes: &[Silhouette]) -> Vec<DrawOp> {
    let mut ops = Vec::with_capacity(silhouettes.len() * 8);
    for s in silhouettes {
        let (rim, color) = if s.front {
            (RIM_FRONT, FRONT_COLOR)
        } else {
            (RIM_BACK, BACK_COLOR)
        };
        let parts = parts(s);
        for (x, y, w, h) in parts {
            ops.push(fill(rim, (x - 1, y - 1, w + 2, h + 2)));
        }
        for part in parts {
            ops.push(fill(color, part));
        }
    }
    ops
}

/// Whether a draw op lies inside the viewport.
#[must_use]
pub fn inside_viewport(op: &DrawOp) -> bool {
    match op.paint {
        Paint::Fill { width, height, .. } => {
            VIEWPORT.encloses(Rect::new(op.x, op.y, width, height))
        }
        Paint::Sprite(_) => VIEWPORT.contains(op.x, op.y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: [Size; 6] = [
        Size::Tiny,
        Size::Small,
        Size::Medium,
        Size::Large,
        Size::Huge,
        Size::Gargantuan,
    ];

    fn actor(index: u8, size: Size, front: bool) -> Actor {
        Actor { index, size, front }
    }

    #[test]
    fn every_size_stands_on_its_floor_line_inside_the_viewport() {
        let camera = Camera::new(VIEWPORT_SIZE);
        for size in SIZES {
            for front in [true, false] {
                let actors: Vec<Actor> = (0..5).map(|i| actor(i, size, front)).collect();
                let placed = silhouettes(&actors);
                assert_eq!(placed.len(), 5);
                for s in &placed {
                    let feet = camera.sy(0.0, if front { FRONT_Z } else { BACK_Z }).round();
                    assert_eq!(s.rect.bottom(), feet as i32, "{size:?}");
                    assert!(!front || s.rect.bottom() == FRONT_FEET_Y);
                    assert!(VIEWPORT.encloses(s.rect), "{size:?} {s:?}");
                }
                let fills = ops(&placed);
                assert_eq!(fills.len(), 40, "four rims and four fills per silhouette");
                assert!(fills.iter().all(inside_viewport), "{size:?} {front}");
            }
        }
        let front = silhouettes(&[actor(0, Size::Tiny, true), actor(1, Size::Gargantuan, true)]);
        let tiles = |size| (height_tiles(size) * camera.focal / FRONT_Z) as u32;
        let expected = (tiles(Size::Tiny), tiles(Size::Gargantuan));
        assert_eq!((front[0].rect.h, front[1].rect.h), expected);
        let back = silhouettes(&[actor(0, Size::Large, false)]);
        assert!(back[0].rect.h < front[1].rect.h, "distance shrinks");
    }

    #[test]
    fn slots_fan_out_from_the_centre_and_fills_tile_the_box() {
        let actors: Vec<Actor> = (0..5).map(|i| actor(i, Size::Medium, true)).collect();
        let placed = silhouettes(&actors);
        let centres: Vec<i32> = placed
            .iter()
            .map(|s| s.rect.x + s.rect.w as i32 / 2)
            .collect();
        assert_eq!(centres[0], i32::from(VIEWPORT_SIZE.0) / 2);
        assert!(centres[3] < centres[1] && centres[1] < centres[0]);
        assert!(centres[0] < centres[2] && centres[2] < centres[4]);
        assert_eq!(centres[0] - centres[1], centres[2] - centres[0]);
        let all = ops(&placed[..1]);
        let size = |o: &DrawOp| match o.paint {
            Paint::Fill { width, height, .. } => (width, height),
            Paint::Sprite(_) => (0, 0),
        };
        let color = |o: &DrawOp| match o.paint {
            Paint::Fill { color, .. } => color,
            Paint::Sprite(_) => (0, 0, 0),
        };
        let (rims, fills) = all.split_at(4);
        let heights: Vec<u32> = fills.iter().map(|o| size(o).1).collect();
        assert_eq!(heights[0] + heights[1] + heights[2], placed[0].rect.h);
        assert_eq!(fills[0].y, placed[0].rect.y, "the head is on top");
        assert_eq!(fills[2].y + heights[2] as i32, placed[0].rect.bottom());
        assert!(fills[3].x > fills[2].x, "the legs stand apart");
        assert!(fills.iter().all(|o| color(o) == FRONT_COLOR));
        // Each rim is its part widened by one pixel on every side, painted underneath.
        for (rim, part) in rims.iter().zip(fills) {
            assert_eq!((rim.x, rim.y), (part.x - 1, part.y - 1));
            assert_eq!(size(rim), (size(part).0 + 2, size(part).1 + 2));
            assert_eq!(color(rim), RIM_FRONT);
        }
        let back = ops(&silhouettes(&[actor(0, Size::Medium, false)]));
        assert_eq!(color(&back[0]), RIM_BACK);
        assert_eq!(color(&back[4]), BACK_COLOR);
    }
}
