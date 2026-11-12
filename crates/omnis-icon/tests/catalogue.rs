//! The whole Lucide set reads: every icon, within its box, with the shapes the set is known to
//! hold; the licence travels with it.

use omnis_icon::{Icon, LICENSE, get, names, text};

/// The set as added by the owner (2026-10-24): a pin, moved on purpose when the set is updated.
const ICONS: usize = 1903;

#[test]
fn every_icon_of_the_set_reads_within_its_box() {
    let mut count = 0;
    for name in names() {
        let icon = get(name).unwrap_or_else(|| {
            panic!(
                "{name}: {}",
                Icon::parse(name, text(name).unwrap()).err().unwrap()
            )
        });
        assert!(!icon.strokes.is_empty(), "{name} draws nothing");
        let [x0, y0, x1, y1] = icon.bounds().unwrap();
        assert!(
            x0 >= -0.5 && y0 >= -0.5 && x1 <= 24.5 && y1 <= 24.5,
            "{name} leaves its box: {x0} {y0} {x1} {y1}"
        );
        for stroke in &icon.strokes {
            assert!(
                stroke.points.len() >= 2 || !stroke.closed,
                "{name}: a closed point"
            );
            for p in &stroke.points {
                assert!(p[0].is_finite() && p[1].is_finite(), "{name}: not a number");
            }
        }
        // A filled child is the set's dot: a circle no wider than the stroke's half width, so its
        // outline stroke is the same disc.
        for element in omnis_icon::svg::read(text(name).unwrap()).unwrap() {
            if element.attr("fill").is_some() {
                assert_eq!(element.name, "circle", "{name}: a filled {}", element.name);
                assert!(
                    element.number("r", 0.0).unwrap() <= omnis_icon::WIDTH / 2.0,
                    "{name}: a filled shape wider than the stroke"
                );
            }
        }
        count += 1;
    }
    assert_eq!(count, ICONS, "the set's size (a pin: move it with the set)");
    assert_eq!(omnis_icon::catalogue::count(), ICONS);
    assert!(get("no-such-icon").is_none());
    assert!(text("door-open").unwrap().starts_with("<svg"));
}

#[test]
fn the_shapes_read_as_the_set_draws_them() {
    // `circle` is one circle: one closed stroke of 32 points at radius 10 about (12, 12).
    let circle = get("circle").expect("the circle icon");
    assert_eq!(circle.strokes.len(), 1);
    let s = &circle.strokes[0];
    assert!(s.closed && s.points.len() == omnis_icon::shape::SIDES);
    for p in &s.points {
        let r = (p[0] - 12.0).hypot(p[1] - 12.0);
        assert!((r - 10.0).abs() < 0.05, "{r}");
    }
    // `door-open` is five paths.
    assert_eq!(get("door-open").unwrap().strokes.len(), 5);
    // A square's rect is a closed stroke of four points; a rounded one is longer and lies within.
    let square = get("square").expect("square");
    assert!(
        square
            .strokes
            .iter()
            .any(|s| s.closed && s.points.len() == 4 || s.closed && s.points.len() > 4)
    );
    // The lines come out scaled: `minus` at 48 pixels a side is one line 24 pixels wide.
    let minus = get("minus").expect("minus");
    let lines: Vec<_> = minus.segments(48.0).collect();
    assert_eq!(lines.len(), 1);
    assert!((lines[0][1][0] - lines[0][0][0]).abs() > 20.0, "{lines:?}");
    assert_eq!(Icon::width(48.0), 4.0);
}

#[test]
fn the_licence_travels_with_the_set() {
    assert!(LICENSE.starts_with("ISC License"), "{LICENSE}");
    assert!(LICENSE.contains("Lucide"));
}
