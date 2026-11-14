//! The palette checks (presentation-ARCHITECTURE 7.3, XR8): the built-in default and every
//! biosphere the fixtures draw keep their roles apart for every kind of colour vision, and a
//! palette that does not is told so, pair by pair.

use crate::common;
use omnis_sim::omnis_data::{ColourRole, Data};
use omnis_vector::contrast::{APART, Clash, DRAWN, FROM_DARK, Vision, clashes, distance, shown};
use omnis_vector::style::Palette;
use omnis_vector::tones::{Side, Tone, ink};

fn biosphere(data: &Data, id: &str) -> Palette {
    Palette::of(Some(
        data.biospheres.get(id).unwrap_or_else(|| panic!("{id}")),
    ))
}

#[test]
fn the_default_and_the_drawn_fixture_biospheres_keep_their_roles_apart() {
    let objects = common::objects_data();
    let levels = common::levels_data();
    for (name, palette) in [
        ("the default", Palette::default()),
        ("ember", biosphere(&objects, "objects:biosphere:ember")),
        ("deep", biosphere(&levels, "levels:biosphere:deep")),
    ] {
        assert_eq!(clashes(&palette), [], "{name}");
        // Not by a hair: each pair and each role clears its threshold under every vision.
        for vision in Vision::ALL {
            for (i, role) in DRAWN.iter().enumerate() {
                for other in &DRAWN[i + 1..] {
                    let d = distance(palette.ink(*role), palette.ink(*other), vision);
                    assert!(
                        d >= APART,
                        "{name}: {role:?} and {other:?} to {vision:?}: {d}"
                    );
                }
            }
            for role in ColourRole::ALL {
                let d = distance(palette.ink(role), [0.0; 3], vision);
                assert!(
                    d >= FROM_DARK,
                    "{name}: {role:?} on the dark to {vision:?}: {d}"
                );
            }
        }
    }
    assert_eq!((APART, FROM_DARK), (0.1, 0.45));
}

#[test]
fn a_palette_that_collapses_two_roles_or_sinks_one_is_told_which_and_for_whom() {
    // The fixture frost, never drawn: its structure blue is the default thing's violet to a
    // green-blind player, and its ground is too dark for anyone.
    let levels = common::levels_data();
    let found = clashes(&biosphere(&levels, "levels:biosphere:frost"));
    let pair = |c: &Clash| (c.vision, c.role, c.other);
    assert!(
        found.iter().map(pair).any(|c| c
            == (
                Vision::Deuteranopia,
                ColourRole::Structure,
                Some(ColourRole::Thing)
            )),
        "{found:?}"
    );
    assert!(
        !found
            .iter()
            .any(|c| c.vision == Vision::Full && c.other.is_some()),
        "in full colour its roles are apart: {found:?}"
    );
    for vision in Vision::ALL {
        assert!(
            found
                .iter()
                .map(pair)
                .any(|c| c == (vision, ColourRole::Ground, None)),
            "the ground on the dark, to {vision:?}: {found:?}"
        );
    }
    assert!(found.iter().all(|c| c.distance < APART.max(FROM_DARK)));

    // Two roles in one ink clash for everyone; the party's and the interface's draw no line in
    // the world, so sharing a hue with a drawn role is not a clash.
    let mut same = Palette::default();
    same.inks[ColourRole::Danger.index()] = same.ink(ColourRole::Hostile);
    same.inks[ColourRole::Party.index()] = same.ink(ColourRole::Structure);
    let found = clashes(&same);
    assert_eq!(found.len(), Vision::ALL.len(), "{found:?}");
    assert!(found.iter().all(|c| {
        (c.role, c.other, c.distance) == (ColourRole::Hostile, Some(ColourRole::Danger), 0.0)
    }));
    // A role with no ink is on the dark.
    let mut unlit = Palette::default();
    unlit.inks[ColourRole::Interface.index()] = [0.0; 3];
    let found = clashes(&unlit);
    assert_eq!(found.len(), Vision::ALL.len(), "{found:?}");
    assert!(
        found
            .iter()
            .all(|c| (c.role, c.other) == (ColourRole::Interface, None))
    );
}

#[test]
fn a_distance_is_between_what_the_line_shows_as_the_vision_takes_it() {
    // An ink brighter than the screen shows its hue at the screen's peak.
    assert_eq!(shown([3.0, 1.5, 0.0]), [1.0, 0.5, 0.0]);
    assert_eq!(shown([0.2, 0.4, 0.1]), [0.2, 0.4, 0.1]);
    assert_eq!(
        distance([3.0, 1.5, 0.0], [1.0, 0.5, 0.0], Vision::Full),
        0.0
    );
    // White on black is Oklab's whole lightness, to every vision.
    for vision in Vision::ALL {
        let d = distance([1.0; 3], [0.0; 3], vision);
        assert!((d - 1.0).abs() < 0.01, "{vision:?}: {d}");
    }
    // Pure red and pure green: far apart in full colour, less than half as far to the
    // green-blind, and no nearer to the blue-blind. To the red-blind, red sinks toward the
    // dark, yet a pure red still clears the threshold.
    let (red, green) = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let full = distance(red, green, Vision::Full);
    assert!(full > 0.5, "{full}");
    assert!(distance(red, green, Vision::Deuteranopia) < full * 0.5);
    let dark = |vision| distance(red, [0.0; 3], vision);
    assert!(dark(Vision::Protanopia) < dark(Vision::Full) * 0.8);
    assert!(dark(Vision::Protanopia) >= FROM_DARK);
    assert!(dark(Vision::Deuteranopia) > dark(Vision::Full) * 0.95);
    assert!(distance(red, green, Vision::Tritanopia) > full * 0.75);
    // Blue and green fall together for the blue-blind only.
    let (blue, teal) = ([0.0, 0.3, 1.0], [0.0, 1.0, 0.4]);
    assert!(distance(blue, teal, Vision::Tritanopia) < distance(blue, teal, Vision::Full) * 0.75);
    assert_eq!(Vision::Full.sees([0.3, 0.2, 0.1]), [0.3, 0.2, 0.1]);
}

/// The fight draws the party's and the hostile ink side by side (owner, 2026-11-13): the two
/// are apart for every kind of colour vision in the default palette and in the yard's. (The
/// default's party ink is the interface's hue, twice as bright: the party's figures and the
/// bars differ there by brightness and shape, not hue.)
#[test]
fn the_two_sides_of_a_fight_are_apart_for_every_vision() {
    let objects = common::objects_data();
    for (name, palette) in [
        ("the default", Palette::default()),
        ("ember", biosphere(&objects, "objects:biosphere:ember")),
    ] {
        for vision in Vision::ALL {
            // As the fight draws them: calmed (paler, less glow), which brings them closer. The
            // default's two are 0.14 apart to a green-blind eye as written and 0.107 at
            // `tones::PALE` 0.15; at 0.2 they fall under the line, which is what holds it there.
            let d = distance(
                ink(Tone::Figure(Side::Party), &palette),
                ink(Tone::Figure(Side::Monsters), &palette),
                vision,
            );
            assert!(d >= APART, "{name}: the sides to {vision:?}: {d}");
        }
    }
}
