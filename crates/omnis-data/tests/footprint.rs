//! The footprint geometry (M8.5 step 2): boxes in sixteenths of a cell, turned by facing,
//! spilling into neighbours, open on every side; the path of a step; rays through a box.

use omnis_core::Facing;
use omnis_data::footprint::{Along, Footprint};

const fn fp(x0: i32, y0: i32, x1: i32, y1: i32) -> Footprint {
    Footprint { x0, y0, x1, y1 }
}

#[test]
fn a_footprint_is_centred_in_its_cell_and_turned_by_its_facing() {
    // Cell (2, 3): centre (40, 56). Width 6 across, depth 4 along.
    for facing in [Facing::North, Facing::South] {
        assert_eq!(
            Footprint::of(2, 3, (0, 0), facing, 6, 4),
            fp(34, 52, 46, 60)
        );
    }
    for facing in [Facing::East, Facing::West] {
        assert_eq!(
            Footprint::of(2, 3, (0, 0), facing, 6, 4),
            fp(36, 50, 44, 62)
        );
    }
    // Three eighths east and two north move the centre six and four sixteenths.
    assert_eq!(
        Footprint::of(2, 3, (3, -2), Facing::North, 2, 2),
        fp(44, 50, 48, 54)
    );
}

#[test]
fn a_footprint_reaches_only_the_cells_it_covers() {
    let centred = Footprint::of(1, 1, (0, 0), Facing::North, 8, 8);
    assert_eq!(centred, fp(16, 16, 32, 32), "a whole cell, no more");
    assert_eq!(centred.cells().collect::<Vec<_>>(), [(1, 1)]);
    let spill = Footprint::of(1, 1, (4, 0), Facing::North, 4, 2);
    assert_eq!(spill, fp(28, 22, 36, 26));
    assert_eq!(spill.cells().collect::<Vec<_>>(), [(1, 1), (2, 1)]);
    let corner = Footprint::of(0, 0, (-4, -4), Facing::North, 2, 2);
    assert_eq!(
        corner.cells().collect::<Vec<_>>(),
        [(-1, -1), (0, -1), (-1, 0), (0, 0)],
        "off the map's edge the cells are still named; the loader refuses them"
    );
}

#[test]
fn boxes_that_only_touch_do_not_overlap() {
    let a = fp(0, 0, 8, 8);
    assert!(!a.overlaps(&fp(8, 0, 16, 8)), "sharing the east edge");
    assert!(!a.overlaps(&fp(0, 8, 8, 16)), "sharing the south edge");
    assert!(a.overlaps(&fp(7, 7, 16, 16)), "one sixteenth in");
    assert!(fp(7, 7, 16, 16).overlaps(&a), "either way round");
}

#[test]
fn a_step_s_path_is_half_a_cell_wide_from_centre_to_centre() {
    assert_eq!(Footprint::path(0, 0, Facing::East), fp(8, 4, 24, 12));
    assert_eq!(Footprint::path(1, 1, Facing::West), fp(8, 20, 24, 28));
    assert_eq!(Footprint::path(1, 1, Facing::North), fp(20, 8, 28, 24));
    assert_eq!(Footprint::path(1, 1, Facing::South), fp(20, 24, 28, 40));
    // A barrel in the north-west corner is passed; a table in the middle is not.
    let barrel = Footprint::of(1, 1, (-3, -3), Facing::North, 2, 2);
    let table = Footprint::of(1, 1, (0, 0), Facing::North, 4, 4);
    for heading in [Facing::North, Facing::East, Facing::South, Facing::West] {
        assert!(
            !Footprint::path(1, 1, heading).overlaps(&barrel),
            "{heading:?}"
        );
        assert!(
            Footprint::path(1, 1, heading).overlaps(&table),
            "{heading:?}"
        );
    }
}

fn along(num: i64, den: i64) -> Along {
    Along { num, den }
}

#[test]
fn a_ray_through_a_box_gives_where_it_enters_and_leaves() {
    let post = fp(16, 16, 32, 32);
    assert_eq!(
        post.ray_span((0, 24), (48, 24)),
        Some((along(16, 48), along(32, 48)))
    );
    assert_eq!(
        post.ray_span((48, 24), (0, 24)),
        Some((along(16, 48), along(32, 48))),
        "the same span walked the other way"
    );
    assert_eq!(
        post.ray_span((24, 0), (24, 20)),
        Some((along(16, 20), along(1, 1))),
        "a ray ending inside the box leaves at its end"
    );
    assert_eq!(
        post.ray_span((0, 0), (48, 48)),
        Some((along(16, 48), along(32, 48))),
        "corner to corner"
    );
}

#[test]
fn a_ray_that_misses_or_only_grazes_does_not_pass_through() {
    let post = fp(16, 16, 32, 32);
    assert_eq!(
        post.ray_span((0, 16), (48, 16)),
        None,
        "along the north edge"
    );
    assert_eq!(post.ray_span((0, 40), (48, 40)), None, "beside it");
    assert_eq!(post.ray_span((0, 0), (16, 16)), None, "to its corner");
    assert_eq!(post.ray_span((0, 24), (10, 24)), None, "short of it");
    assert_eq!(
        post.ray_span((0, 32), (32, 0)),
        None,
        "across a corner only"
    );
}
