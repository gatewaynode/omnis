//! Path data as the set writes it: every command, implicit repeats, the compact syntax, and arcs
//! that land where they should.

use omnis_icon::path::parse;

fn last(d: &str) -> [f32; 2] {
    *parse(d).unwrap().last().unwrap().points.last().unwrap()
}

#[test]
fn every_command_reads_absolute_and_relative() {
    assert_eq!(last("M1 2L3 4"), [3.0, 4.0]);
    assert_eq!(last("M1 2l3 4"), [4.0, 6.0]);
    assert_eq!(last("M1 2H5V7"), [5.0, 7.0]);
    assert_eq!(last("M1 2h5v7"), [6.0, 9.0]);
    assert_eq!(last("M0 0C1 1 2 1 3 0"), [3.0, 0.0]);
    assert_eq!(last("M0 0c1 1 2 1 3 0s1 -1 2 0"), [5.0, 0.0]);
    assert_eq!(last("M0 0Q1 1 2 0t2 0"), [4.0, 0.0]);
    assert_eq!(last("M0 0A1 1 0 0 1 2 0"), [2.0, 0.0]);
    // A move's trailing numbers are lines; a close returns the pen to the subpath's start.
    let strokes = parse("M0 0 1 0 1 1Z m5 5 1 0").unwrap();
    assert_eq!(strokes.len(), 2);
    assert!(strokes[0].closed && strokes[0].points.len() == 3);
    assert_eq!(strokes[1].points, vec![[5.0, 5.0], [6.0, 5.0]]);
    // Unknown commands and short commands are refused.
    assert!(parse("M0 0 K1").is_err());
    assert!(parse("M0").is_err());
    assert!(parse("1 2").is_err());
}

#[test]
fn the_compact_syntax_of_the_set_reads() {
    // From `door-open`: flags run into the next number, fractions run into fractions.
    let s = parse("M10 3H7a2 2 0 00-2 2v16").unwrap();
    assert_eq!(*s[0].points.last().unwrap(), [5.0, 21.0]);
    let s = parse("M19 21V5a2 2 0 00-1.675-1.974l-6.163-1.013A1 1 0 0010 3v18a1 1 0 001.124.992z")
        .unwrap();
    assert!(s[0].closed);
    let end = s[0].points[s[0].points.len() - 1];
    assert!(
        (end[0] - 11.124).abs() < 1e-3 && (end[1] - 21.992).abs() < 1e-3,
        "{end:?}"
    );
    assert_eq!(last("M14 12h.01"), [14.01, 12.0]);
    assert_eq!(last("M1 1l1e1 0"), [11.0, 1.0]);
}

#[test]
fn an_arc_lands_on_its_endpoint_and_bows_the_right_way() {
    // A half circle of radius 2 from (0, 0) to (4, 0), sweeping clockwise (positive), bows down (y up
    // the screen is negative, so the midpoint is at y = 2 for sweep 1).
    let s = parse("M0 0A2 2 0 0 1 4 0").unwrap();
    let points = &s[0].points;
    assert_eq!(*points.last().unwrap(), [4.0, 0.0]);
    let mid = points[points.len() / 2];
    assert!((mid[0] - 2.0).abs() < 0.3 && mid[1] < -1.5, "{mid:?}");
    let s = parse("M0 0A2 2 0 0 0 4 0").unwrap();
    let mid = s[0].points[s[0].points.len() / 2];
    assert!(mid[1] > 1.5, "the other sweep bows the other way: {mid:?}");
    // Radii too small for the chord grow to fit: the arc still lands.
    assert_eq!(last("M0 0A1 1 0 0 1 10 0"), [10.0, 0.0]);
    // A zero radius is a line.
    let s = parse("M0 0A0 0 0 0 1 3 3").unwrap();
    assert_eq!(s[0].points, vec![[0.0, 0.0], [3.0, 3.0]]);
}
