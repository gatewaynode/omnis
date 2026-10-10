//! The minimap, top right: `minimap::paint` uploaded into an image shown as a UI node
//! (presentation-ARCHITECTURE.md §8). Repainted only when what it shows changes.

use super::VectorSet;
use super::session::Session;
use crate::minimap::{Raster, paint};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use omnis_sim::omnis_core::MapId;

/// Marks the minimap's node.
#[derive(Component)]
pub struct Minimap;

/// What the picture last showed: the map, the accepted-command count (the automap only changes
/// when a command is accepted), and the marker's pixel and heading.
type Shown = (MapId, usize, (i64, i64), i64);

/// The minimap. Needs `Assets<Image>`, so a window or the offscreen capture.
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn)
            .add_systems(Update, repaint.in_set(VectorSet::Draw));
    }
}

fn spawn(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(image(&Raster {
        width: 1,
        height: 1,
        scale: 1,
        rgba: vec![0; 4],
    }));
    commands.spawn((
        Minimap,
        ImageNode::new(handle),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(super::controls::GREEN),
    ));
}

/// A raster as an image the UI can show.
pub fn image(raster: &Raster) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: raster.width,
            height: raster.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        raster.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn repaint(
    session: Res<Session>,
    mut images: ResMut<Assets<Image>>,
    mut node: Query<(&ImageNode, &mut Node), With<Minimap>>,
    mut last: Local<Option<Shown>>,
) {
    let Ok((picture, mut layout)) = node.single_mut() else {
        return;
    };
    let p = session.world.position;
    let Some(map) = session.data.maps.get(&p.map) else {
        return;
    };
    let scale = crate::minimap::scale_for(map.def.width, map.def.height) as f32;
    let pose = session.pose;
    // A sixteenth of a turn is as fine as the heading line can show.
    #[allow(clippy::cast_possible_truncation)]
    let shown = (
        p.map,
        session.binder.log.len(),
        ((pose.x * scale) as i64, (pose.z * scale) as i64),
        (pose.yaw * 8.0 / core::f32::consts::PI).round() as i64,
    );
    if *last == Some(shown) {
        return;
    }
    *last = Some(shown);
    let raster = paint(map, session.world.automap.map(p.map), pose);
    #[allow(clippy::cast_precision_loss)]
    let (w, h) = (raster.width as f32, raster.height as f32);
    layout.width = Val::Px(w);
    layout.height = Val::Px(h);
    if let Some(mut target) = images.get_mut(&picture.image) {
        *target = image(&raster);
    }
}
