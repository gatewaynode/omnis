//! The fight on the palette (owner, 2026-11-13; `tasks/plans/fight-on-the-palette.md`): a fight
//! is drawn in the palette of the map it is fought on, as the 3D view is.

use crate::common;
use bevy::prelude::*;
use common::app::{met_in_the_yard, met_with};
use omnis_sim::omnis_data::ColourRole;
use omnis_vector::cinema::Inks;
use omnis_vector::shell::cinema::{CinemaPlugin, Screen};
use omnis_vector::shell::combat::palette;
use omnis_vector::shell::session::Session;
use omnis_vector::style::Palette;
use omnis_vector::tones::{Side, Tone, ink};

/// In the dungeon, which names no biosphere, the default; in the yard, its region's ember, so
/// the two sides and the bars are drawn in ember's inks.
#[test]
fn a_fight_is_drawn_in_the_palette_of_its_map() {
    let dungeon = met_with(1);
    assert_eq!(
        palette(dungeon.world().resource::<Session>()),
        Palette::default()
    );
    let yard = met_in_the_yard();
    let session = yard.world().resource::<Session>();
    let ember = Palette::of(session.data.biospheres.get("objects:biosphere:ember"));
    assert_ne!(ember, Palette::default());
    let drawn = palette(session);
    assert_eq!(drawn, ember);
    for (tone, role) in [
        (Tone::Figure(Side::Party), ColourRole::Party),
        (Tone::Figure(Side::Monsters), ColourRole::Hostile),
    ] {
        assert_eq!(ink(tone, &drawn), ember.ink(role));
        assert_ne!(ink(tone, &drawn), ink(tone, &Palette::default()));
    }
}

/// The picture window's image on `app`'s fight screen: its handle and how many of its pixels
/// are `colour`.
fn picture(app: &mut App, colour: [u8; 4]) -> (Handle<Image>, usize) {
    let world = app.world_mut();
    let handle = world
        .query_filtered::<&ImageNode, With<Screen>>()
        .single(world)
        .expect("one picture window, painted")
        .image
        .clone();
    let image = world
        .resource::<Assets<Image>>()
        .get(&handle)
        .expect("the image");
    let data = image.data.as_ref().expect("pixels");
    let n = data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| **p == colour)
        .count();
    (handle, n)
}

/// The picture window paints the enemy in the hostile ink of the fight's palette, and keeps
/// its image while the screen is only built again.
#[test]
fn the_picture_window_is_painted_in_the_fight_s_hostile_ink() {
    let default = Inks::of(&Palette::default());
    for (mut app, ember) in [(met_with(1), false), (met_in_the_yard(), true)] {
        app.add_plugins(AssetPlugin::default())
            .init_asset::<Image>()
            .add_plugins(CinemaPlugin);
        app.update();
        app.update();
        let inks = Inks::of(&palette(app.world().resource::<Session>()));
        assert_eq!(inks != default, ember);
        let (first, core) = picture(&mut app, inks.line);
        assert!(core > 100, "the enemy's lines in the fight's ink");
        if ember {
            // Ember's hostile ink as written in the pack, its glow taken out.
            assert_eq!(inks.line, [255, 77, 187, 255]);
            assert_eq!(
                picture(&mut app, default.line).1,
                0,
                "none in the default's"
            );
        }
        // The screen builds its nodes again: a new picture window of the same scene takes the
        // same image, not a new one.
        let world = app.world_mut();
        let (old, scene) = world
            .query::<(Entity, &Screen)>()
            .single(world)
            .map(|(e, s)| (e, *s))
            .expect("the picture window");
        world.despawn(old);
        world.spawn(scene);
        app.update();
        app.update();
        assert_eq!(
            picture(&mut app, inks.line).0,
            first,
            "kept, not painted again"
        );
    }
}
