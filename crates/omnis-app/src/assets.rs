//! `PackAssetPlugin`: pack images by pack-relative path through a `pack://` asset source, and
//! a magenta placeholder for anything that fails to load (PRD §10).

use bevy::asset::RenderAssetUsages;
use bevy::asset::io::{AssetSource, AssetSourceBuilder};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::BTreeMap;
use std::path::Path;

/// The asset source name packs are served under.
pub const SOURCE: &str = "pack";

/// Register the `pack://` source. Must run before `DefaultPlugins` is added.
pub fn register_pack_source(app: &mut App, pack_root: &Path) {
    let root = pack_root
        .canonicalize()
        .unwrap_or_else(|_| pack_root.to_path_buf());
    let root = root.to_string_lossy().into_owned();
    app.register_asset_source(
        SOURCE,
        AssetSourceBuilder::new(AssetSource::get_default_reader(root)),
    );
}

/// Handles by pack-relative path, plus the placeholder.
#[derive(Resource, Default)]
pub struct PackImages {
    handles: BTreeMap<String, Handle<Image>>,
    placeholder: Handle<Image>,
}

impl PackImages {
    /// The image at a pack-relative path, loading it on first request.
    pub fn get(&mut self, server: &AssetServer, path: &str) -> Handle<Image> {
        self.handles
            .entry(path.to_owned())
            .or_insert_with(|| server.load(format!("{SOURCE}://{path}")))
            .clone()
    }

    /// The magenta placeholder.
    #[must_use]
    pub fn placeholder(&self) -> Handle<Image> {
        self.placeholder.clone()
    }
}

/// The pack asset plugin.
pub struct PackAssetPlugin;

impl Plugin for PackAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PackImages>()
            .add_systems(Startup, make_placeholder)
            .add_systems(PostUpdate, substitute_failed);
    }
}

fn make_placeholder(mut images: ResMut<Assets<Image>>, mut pack: ResMut<PackImages>) {
    let size = Extent3d {
        width: 8,
        height: 8,
        ..default()
    };
    let image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[255, 0, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    pack.placeholder = images.add(image);
}

/// Sprites and UI images whose file failed to load show the placeholder instead of nothing.
fn substitute_failed(
    server: Res<AssetServer>,
    pack: Res<PackImages>,
    mut sprites: Query<&mut Sprite>,
    mut nodes: Query<&mut ImageNode>,
) {
    for mut sprite in &mut sprites {
        if sprite.image != pack.placeholder && server.load_state(&sprite.image).is_failed() {
            warn!("missing pack image, using placeholder");
            sprite.image = pack.placeholder();
            sprite.custom_size = Some(Vec2::new(16.0, 16.0));
        }
    }
    for mut node in &mut nodes {
        if node.image != pack.placeholder && server.load_state(&node.image).is_failed() {
            warn!("missing pack UI image, using placeholder");
            node.image = pack.placeholder();
        }
    }
}
