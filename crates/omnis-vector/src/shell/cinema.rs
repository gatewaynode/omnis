//! The fight's picture window (alt-ARCHITECTURE.md §9): `cinema::paint` uploaded into an
//! image on the fight screen's `Screen` node. Needs `Assets<Image>`, so a window or the offscreen
//! capture; headless, the node keeps its space and stays blank.

use super::VectorSet;
use super::minimap::image;
use crate::cinema::{SIZE, Scene, paint};
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

/// Give each new picture window its scene's image. The fight screen rebuilds its nodes whenever
/// it changes, so the last scene's image is kept and reused rather than painted again.
fn show(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    screens: Query<(Entity, &Screen), Added<Screen>>,
    mut last: Local<Option<(Scene, Handle<Image>)>>,
) {
    for (entity, screen) in &screens {
        let handle = match &*last {
            Some((scene, handle)) if *scene == screen.0 => handle.clone(),
            _ => {
                let handle = images.add(image(&paint(screen.0, SIZE.0, SIZE.1)));
                *last = Some((screen.0, handle.clone()));
                handle
            }
        };
        commands.entity(entity).insert(ImageNode::new(handle));
    }
}
