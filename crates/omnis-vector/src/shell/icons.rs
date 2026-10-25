//! Icons in UI nodes (the owner's drawing-GUI column, step 1, 2026-10-25): a `Kind::Icon`
//! control shows a Lucide icon drawn once into an image and kept by name, size and colour, so
//! a grid of icons costs one small image per distinct icon however often the column is
//! rebuilt. The kit's builders spawn the icon's node with `NeedsIcon` and never touch the
//! assets; `fill` here gives each such node its image on the next frame. Headless (no
//! `Assets<Image>`) the node stays as spawned, so `IconOf` still names it for the text readers.

use super::VectorSet;
use crate::raster::Raster;
use crate::style::Role;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use omnis_icon::Icon;
use std::sync::{Mutex, OnceLock};

/// The icon a node shows, by its Lucide name: what `screen_text` prints for it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconOf(pub &'static str);

/// A node waiting for its icon's image: the name, the side in pixels and the stroke colour.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeedsIcon {
    /// The Lucide name.
    pub name: &'static str,
    /// The image's side in physical pixels.
    pub px: u32,
    /// The strokes' colour, RGBA bytes.
    pub colour: [u8; 4],
}

/// The images drawn so far, by name, side and colour.
#[derive(Resource, Debug, Default)]
pub struct IconImages(HashMap<(&'static str, u32, [u8; 4]), Handle<Image>>);

impl IconImages {
    /// The image of `need`, drawn on the first ask.
    pub fn image(&mut self, images: &mut Assets<Image>, need: NeedsIcon) -> Handle<Image> {
        self.0
            .entry((need.name, need.px, need.colour))
            .or_insert_with(|| {
                let side = need.px.max(1);
                let mut raster = Raster::filled(side, side, 1, [0, 0, 0, 0]);
                raster.icon(strokes(need.name), (0, 0), side, need.colour);
                images.add(super::minimap::image(&raster))
            })
            .clone()
    }

    /// How many distinct images have been drawn.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether none has.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The strokes of the icon `name`, read from the set once and kept for the process.
fn strokes(name: &'static str) -> &'static Icon {
    static READ: OnceLock<Mutex<HashMap<&'static str, &'static Icon>>> = OnceLock::new();
    let mut read = READ
        .get_or_init(|| Mutex::new(HashMap::default()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(icon) = read.get(name).copied() {
        return icon;
    }
    let icon: &'static Icon = Box::leak(Box::new(omnis_icon::get(name).unwrap_or_else(|| Icon {
        name: name.to_owned(),
        strokes: Vec::new(),
    })));
    read.insert(name, icon);
    icon
}

/// A role's colour as the bytes an icon is drawn in.
#[must_use]
pub fn tint(role: Role) -> [u8; 4] {
    let [r, g, b] = role.rgb();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(r), byte(g), byte(b), 255]
}

/// The strokes' colour of a chosen icon: white, so the chosen state of a grid reads at a glance
/// beside the interface green of the others.
pub const CHOSEN: [u8; 4] = [255, 255, 255, 255];

/// Give every node waiting for an icon its image.
fn fill(
    mut commands: Commands,
    mut cache: ResMut<IconImages>,
    images: Option<ResMut<Assets<Image>>>,
    waiting: Query<(Entity, &NeedsIcon)>,
) {
    let Some(mut images) = images else {
        return;
    };
    for (entity, need) in &waiting {
        let handle = cache.image(&mut images, *need);
        commands
            .entity(entity)
            .insert(ImageNode::new(handle))
            .remove::<NeedsIcon>();
    }
}

/// The icon images and the system that places them.
pub struct IconsPlugin;

impl Plugin for IconsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IconImages>()
            .add_systems(Update, fill.in_set(VectorSet::Screens));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_role_s_tint_is_its_colour_in_bytes_and_the_chosen_tint_is_white() {
        assert_eq!(tint(Role::Interface), [140, 255, 166, 255]);
        assert_eq!(tint(Role::Dim), [64, 102, 77, 255]);
        assert_eq!(CHOSEN, [255, 255, 255, 255]);
    }

    #[test]
    fn an_icon_s_strokes_are_read_once_and_a_missing_name_draws_nothing() {
        let a = strokes("zoom-in");
        let b = strokes("zoom-in");
        assert!(std::ptr::eq(a, b));
        assert!(!a.strokes.is_empty());
        assert!(strokes("no-such-icon-in-the-set").strokes.is_empty());
    }
}
