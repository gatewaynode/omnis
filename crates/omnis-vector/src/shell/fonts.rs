//! The interface's typeface (presentation-ARCHITECTURE §7.4, §10.5): Bevy's built-in font, or Inter
//! or Alegreya Sans, compiled in from `assets/fonts/` (SIL OFL 1.1; sources and hashes in
//! `assets/README.md`), so a headless or offscreen run has them too. The faces live at fixed
//! handles, and one system puts the chosen one on every text: on text as it appears, and on all
//! text when the setting changes. No spawn site names a font. Headless-safe: without the text
//! plugin the handles are still set, only nothing is rasterised.

use super::settings::Current;
use crate::settings::Face;
use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::text::FontSource;

/// Inter's handle.
pub const INTER: Handle<Font> = uuid_handle!("cedf1100-5f91-4750-879d-9d8d344e8ba4");
/// Alegreya Sans's handle.
pub const ALEGREYA: Handle<Font> = uuid_handle!("8c7fcf28-3994-4726-978c-803c19b42211");

/// The handle a face is drawn with: the built-in font's is the default handle.
#[must_use]
pub fn handle(face: Face) -> Handle<Font> {
    match face {
        Face::BuiltIn => Handle::default(),
        Face::Inter => INTER,
        Face::Alegreya => ALEGREYA,
    }
}

/// Registers the faces and keeps every text in the chosen one.
pub struct FontsPlugin;

impl Plugin for FontsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Current>()
            .add_systems(Startup, register)
            .add_systems(PostUpdate, wear.before(bevy::ui::UiSystems::Content));
    }
}

fn register(fonts: Option<ResMut<Assets<Font>>>) {
    let Some(mut fonts) = fonts else {
        return;
    };
    let faces: [(&Handle<Font>, &[u8]); 2] = [
        (
            &INTER,
            include_bytes!("../../../../assets/fonts/inter/Inter-Regular.ttf"),
        ),
        (
            &ALEGREYA,
            include_bytes!("../../../../assets/fonts/alegreya-sans/AlegreyaSans-Regular.ttf"),
        ),
    ];
    for (at, bytes) in faces {
        if let Err(e) = fonts.insert(at.id(), Font::from_bytes(bytes.to_vec())) {
            warn!("the interface font could not be registered: {e}");
        }
    }
}

/// The chosen face on every text whose face differs: new text, and all text after the setting
/// changed. Reading compares only, so untouched text is not marked changed.
fn wear(current: Res<Current>, mut texts: Query<&mut TextFont>) {
    let want = FontSource::Handle(handle(current.0.font));
    for mut font in &mut texts {
        if font.font != want {
            font.font = want.clone();
        }
    }
}
