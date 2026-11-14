//! The fight on the palette (owner, 2026-11-13; `tasks/plans/fight-on-the-palette.md`): a fight
//! is drawn in the palette of the map it is fought on, as the 3D view is.

use crate::common;
use bevy::prelude::*;
use common::app::{met_in_the_yard, met_with};
use omnis_sim::omnis_data::ColourRole;
use omnis_vector::cinema::Inks;
use omnis_vector::shell::cinema::{CinemaPlugin, Screen};
use omnis_vector::shell::combat::palette;
use omnis_vector::shell::icons::NeedsIcon;
use omnis_vector::shell::session::Session;
use omnis_vector::shell::theme::Theme;
use omnis_vector::style::{self, Palette, Role};
use omnis_vector::tones::{Side, Tone, calm, ink};

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
        // The palette's ink, calmed for the fight (owner, 2026-11-14): never the raw glow.
        assert_eq!(ink(tone, &drawn), calm(ember.ink(role)));
        assert_ne!(ink(tone, &drawn), ember.ink(role));
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
            // Ember's hostile ink as written in the pack is (255, 77, 187), its glow taken
            // out; the picture's is that, paler (owner, 2026-11-14): green and blue lifted.
            let [r, g, b, _] = inks.line;
            assert_eq!(r, 255);
            assert!((78..135).contains(&g) && (188..205).contains(&b), "{g} {b}");
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

/// Every colour an interface node of `app` shows: its text's, its border's and its
/// background's, alpha dropped, with what it is for a failure's message.
fn shown_colours(app: &mut App) -> Vec<(String, [f32; 3])> {
    let world = app.world_mut();
    let rgb = |c: Color| {
        let c = c.to_srgba();
        [c.red, c.green, c.blue]
    };
    let mut out = Vec::new();
    for (text, colour) in world.query::<(&Text, &TextColor)>().iter(world) {
        if colour.0.alpha() > 0.0 {
            out.push((format!("text {:?}", text.0), rgb(colour.0)));
        }
    }
    for border in world.query::<&BorderColor>().iter(world) {
        if border.top.alpha() > 0.0 {
            out.push(("a border".to_owned(), rgb(border.top)));
        }
    }
    for back in world.query::<&BackgroundColor>().iter(world) {
        if back.0.alpha() > 0.0 {
            out.push(("a background".to_owned(), rgb(back.0)));
        }
    }
    out
}

/// The colours of a theme that are its own: ordinary and dim text and the three backgrounds.
fn own(theme: &style::Theme) -> [[f32; 3]; 5] {
    [
        theme.rgb(Role::Interface),
        theme.rgb(Role::Dim),
        theme.shade(0),
        theme.shade(1),
        theme.shade(2),
    ]
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.002)
}

/// In a fight the interface follows the biosphere's interface ink (owner, 2026-11-13): in the
/// yard every text, border and background that is the interface's own is ember's, the title
/// among them, and none keeps the default green. In the dungeon, which names no biosphere, the
/// theme is the default.
#[test]
fn the_fight_s_interface_is_in_the_biosphere_s_interface_ink() {
    let mut dungeon = met_with(1);
    dungeon.update();
    assert_eq!(
        dungeon.world().resource::<Theme>().0,
        style::Theme::default()
    );
    let green = style::Theme::default();
    let colours = shown_colours(&mut dungeon);
    assert!(
        colours
            .iter()
            .any(|(_, c)| near(*c, green.rgb(Role::Interface)))
    );

    let mut yard = met_in_the_yard();
    yard.update();
    let ember = style::Theme::of(&palette(yard.world().resource::<Session>()));
    assert_ne!(ember, green);
    // Ember's interface ink as written in the pack.
    assert!(near(
        ember.rgb(Role::Interface),
        [1.0, 200.0 / 255.0, 140.0 / 255.0]
    ));
    assert_eq!(yard.world().resource::<Theme>().0, ember);
    let colours = shown_colours(&mut yard);
    for (what, colour) in &colours {
        assert!(
            !own(&green).iter().any(|g| near(*colour, *g)),
            "{what} keeps the default green {colour:?}"
        );
    }
    for (name, want) in [
        ("ordinary text", ember.rgb(Role::Interface)),
        ("a panel", ember.shade(0)),
    ] {
        assert!(
            colours.iter().any(|(_, c)| near(*c, want)),
            "{name} in ember's"
        );
    }
    let world = yard.world_mut();
    assert_eq!(
        world.query::<&NeedsIcon>().iter(world).count(),
        0,
        "no icon, which is drawn in a fixed colour, stands on the fight screen"
    );
}

/// Outside a fight every screen keeps the default green until the HUD's plan: on the yard, with
/// ember's biosphere, before the rats are met.
#[test]
fn outside_a_fight_the_interface_stays_the_default_green() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = common::app::config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let mut app = common::app::shell(
        omnis_vector::shell::session::Config {
            packs,
            ..common::app::config()
        },
        true,
    );
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = common::place(
            &mut session.world,
            map,
            0,
            2,
            omnis_sim::omnis_core::Facing::East,
        );
        session.refresh();
    }
    app.update();
    app.update();
    let session = app.world().resource::<Session>();
    assert_ne!(style::Theme::of(&palette(session)), style::Theme::default());
    assert_eq!(app.world().resource::<Theme>().0, style::Theme::default());
    let ember = style::Theme::of(&palette(session));
    let colours = shown_colours(&mut app);
    assert!(!colours.is_empty());
    for (what, colour) in &colours {
        assert!(
            !own(&ember).iter().any(|e| near(*colour, *e)),
            "{what} is ember's outside a fight"
        );
    }
}
