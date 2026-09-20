//! The Feathers experiment's typefaces (PRD D26: can a modern font replace the bitmap one).
//! Feathers embeds Fira Sans; Inter and Alegreya Sans are compiled in from `assets/fonts/`
//! (SIL OFL 1.1, sources and hashes in `assets/README.md`), so a headless app has them too.
//! Every Feathers control names its own font, so a swap rewrites every `TextFont` under the
//! panel; what a text was at first (regular, bold, the sliders' monospace) is remembered,
//! because Fira alone has a monospace face and the others answer with their regular one.

use crate::creation_panel::FONTS;
use crate::feathers_creation::{FontChoice, PanelRoot};
use bevy::feathers::constants::fonts;
use bevy::prelude::*;
use bevy::text::FontSource;

/// Which face of a family a text wears.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// The body face.
    Regular,
    /// Titles.
    Bold,
    /// The sliders' figures.
    Mono,
}

/// The families' faces, in `creation_panel::FONTS`' order: regular, bold, monospace.
#[derive(Resource, Debug, Default)]
pub struct PanelFonts(pub Vec<[Handle<Font>; 3]>);

impl PanelFonts {
    fn face_of(&self, handle: &Handle<Font>) -> Option<Face> {
        let fira = self.0.first()?;
        [Face::Regular, Face::Bold, Face::Mono]
            .into_iter()
            .zip(fira)
            .find_map(|(face, held)| (held.id() == handle.id()).then_some(face))
    }

    fn handle(&self, family: usize, face: Face) -> Option<Handle<Font>> {
        self.0.get(family).map(|faces| faces[face as usize].clone())
    }
}

macro_rules! face {
    ($fonts:expr, $path:literal) => {
        $fonts.add(Font::from_bytes(
            include_bytes!(concat!("../../../assets/fonts/", $path)).to_vec(),
        ))
    };
}

/// Register the families. Fira's handles are Feathers' own (the same paths, so the same ids).
pub fn register(
    server: Res<AssetServer>,
    mut fonts: ResMut<Assets<Font>>,
    mut held: ResMut<PanelFonts>,
) {
    let inter = face!(fonts, "inter/Inter-Regular.ttf");
    let alegreya = face!(fonts, "alegreya-sans/AlegreyaSans-Regular.ttf");
    held.0 = vec![
        [
            server.load(fonts::REGULAR),
            server.load(fonts::BOLD),
            server.load(fonts::MONO),
        ],
        [inter.clone(), face!(fonts, "inter/Inter-Bold.ttf"), inter],
        [
            alegreya.clone(),
            face!(fonts, "alegreya-sans/AlegreyaSans-Bold.ttf"),
            alegreya,
        ],
    ];
    debug_assert_eq!(held.0.len(), FONTS.len());
}

/// Dress every text under the panel in the chosen family: when the choice changes, and when
/// a text's font is written (a rebuilt panel; Feathers hands most texts their font by
/// inheritance a frame after they are spawned).
pub fn wear(
    mut commands: Commands,
    choice: Res<FontChoice>,
    held: Res<PanelFonts>,
    roots: Query<(), With<PanelRoot>>,
    parents: Query<&ChildOf>,
    mut texts: Query<(Entity, &mut TextFont, Option<&Face>)>,
) {
    for (entity, mut text, face) in &mut texts {
        if !(choice.is_changed() || text.is_changed()) {
            continue;
        }
        if !parents.iter_ancestors(entity).any(|e| roots.contains(e)) {
            continue;
        }
        let FontSource::Handle(now) = &text.font else {
            continue;
        };
        // A text is first seen as Feathers made it, in one of Fira's faces.
        let Some(face) = face.copied().or_else(|| held.face_of(now)) else {
            continue;
        };
        commands.entity(entity).insert(face);
        if let Some(wanted) = held.handle(choice.0, face)
            && wanted.id() != now.id()
        {
            text.font = FontSource::Handle(wanted);
        }
    }
}
