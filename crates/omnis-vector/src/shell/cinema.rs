//! The fight's picture window (presentation-ARCHITECTURE.md §9): `cinema::paint` uploaded into an
//! image on the fight screen's `Screen` node. Needs `Assets<Image>`, so a window or the offscreen
//! capture; headless, the node keeps its space and stays blank.

use super::VectorSet;
use super::combat::palette;
use super::minimap::image;
use super::session::Session;
use crate::cinema::{Inks, SIZE, Scene, paint};
use bevy::prelude::*;

/// The picture window, with the scene it shows. The node only holds the space; `show` paints
/// it where there is a window.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen(pub Scene);

/// Paints the picture window.
pub struct CinemaPlugin;

impl Plugin for CinemaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show.in_set(VectorSet::Draw));
    }
}

/// The last image painted, with the scene and inks it shows.
type Painted = ((Scene, Inks), Handle<Image>);

/// Give each new picture window its scene's image, in the inks of the palette the fight is
/// drawn in. The fight screen rebuilds its nodes whenever it changes, so the last image is kept
/// and reused while its scene and inks are the same, and painted again when either changes.
fn show(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    session: Option<Res<Session>>,
    screens: Query<(Entity, &Screen), Added<Screen>>,
    mut last: Local<Option<Painted>>,
) {
    for (entity, screen) in &screens {
        let palette = session.as_deref().map(palette).unwrap_or_default();
        let key = (screen.0, Inks::of(&palette));
        let handle = match &*last {
            Some((painted, handle)) if *painted == key => handle.clone(),
            _ => {
                let handle = images.add(image(&paint(key.0, SIZE.0, SIZE.1, key.1)));
                *last = Some((key, handle.clone()));
                handle
            }
        };
        commands.entity(entity).insert(ImageNode::new(handle));
    }
}
