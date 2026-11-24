//! The minimap, top right: `minimap::paint` uploaded into an image shown as a UI node
//! (presentation-ARCHITECTURE.md §8). Repainted only when what it shows changes. Always the same
//! square with the party at its middle; the two buttons on it step the zoom (owner, 2026-11-15).

use super::VectorSet;
use super::controls::{Control, Pressing, pressable};
use super::session::Session;
use super::theme::{edge, fill, ink};
use super::tips::Tip;
use crate::minimap::{Raster, SIDE, Zoom, marks, paint_marked};
use crate::style::Role;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use omnis_sim::omnis_core::{Level, MapId};

/// Marks the minimap's node.
#[derive(Component)]
pub struct Minimap;

/// Marks the line beside the minimap, on its left, that names the party's level on a map of
/// several (M8.5 step 1b). Under it is the party panel's place.
#[derive(Component)]
pub struct MinimapCaption;

/// How close the minimap looks, for as long as the game runs.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Zooming(pub Zoom);

/// A zoom button on the minimap: closer (`+`) or farther (`-`). Dim at the end of its range.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomButton {
    /// Whether it steps closer.
    pub closer: bool,
}

impl ZoomButton {
    /// The button's mark.
    #[must_use]
    pub const fn label(self) -> &'static str {
        if self.closer { "+" } else { "-" }
    }

    /// What it is for, or why it is dim.
    #[must_use]
    pub fn tip(self, zoom: Zoom) -> &'static str {
        match (self.closer, zoom.step(self.closer).is_some()) {
            (true, true) => "Zoom the map in",
            (false, true) => "Zoom the map out",
            (true, false) => "The map is as close as it goes",
            (false, false) => "The map is as far out as it goes",
        }
    }
}

/// What the picture last showed: the map, the accepted-command count (the automap only changes
/// when a command is accepted), the picture's corner on the map, the heading and the zoom.
type Shown = ((MapId, Level), usize, (i64, i64), i64, u32);

/// The caption's text and place, apart from the picture's node.
type Caption<'w, 's> =
    Query<'w, 's, (&'static mut Text, &'static mut Node), (With<MinimapCaption>, Without<Minimap>)>;

/// The minimap. Needs `Assets<Image>`, so a window or the offscreen capture.
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Zooming>()
            .add_systems(Startup, spawn)
            .add_systems(Update, (zoom, light).chain().in_set(VectorSet::Input))
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
    #[allow(clippy::cast_precision_loss)]
    let side = Val::Px(SIDE as f32);
    commands
        .spawn((
            Minimap,
            ImageNode::new(handle),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                width: side,
                height: side,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            edge(Role::Interface),
        ))
        .with_children(|map| {
            // In the picture's bottom corners, so they come and go with it.
            for button in [ZoomButton { closer: false }, ZoomButton { closer: true }] {
                let (left, right) = if button.closer {
                    (Val::Auto, Val::Px(2.0))
                } else {
                    (Val::Px(2.0), Val::Auto)
                };
                map.spawn((
                    button,
                    pressable(),
                    Tip {
                        text: button.tip(Zoom::default()).into(),
                        beside: true,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left,
                        right,
                        bottom: Val::Px(2.0),
                        width: Val::Px(26.0),
                        height: Val::Px(26.0),
                        border: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    edge(Role::Interface),
                    fill(0),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(button.label()),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        ink(Role::Interface),
                    ));
                });
            }
        });
    commands.spawn((
        MinimapCaption,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        ink(Role::Interface),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        },
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

/// A pressed zoom button steps the zoom.
fn zoom(pressed: Query<&ZoomButton, Pressing>, mut zooming: ResMut<Zooming>) {
    for button in &pressed {
        if let Some(to) = zooming.0.step(button.closer) {
            zooming.0 = to;
        }
    }
}

/// A button with no step left is dim and takes no press, and its tip says why.
fn light(
    zooming: Res<Zooming>,
    mut buttons: Query<(Entity, &ZoomButton, &mut Tip, &Children)>,
    mut commands: Commands,
) {
    if !zooming.is_changed() {
        return;
    }
    for (entity, button, mut tip, children) in &mut buttons {
        let live = zooming.0.step(button.closer).is_some();
        let role = if live { Role::Interface } else { Role::Dim };
        tip.text = button.tip(zooming.0).into();
        let mut node = commands.entity(entity);
        node.insert(edge(role));
        if live {
            node.insert(pressable());
        } else {
            // The pointer's hover stays: the tip that says why hangs on it.
            node.remove::<(Control, bevy::ui::Pressed)>();
        }
        for child in children {
            commands.entity(*child).insert(ink(role));
        }
    }
}

fn repaint(
    session: Res<Session>,
    zoom: Res<Zooming>,
    mut images: ResMut<Assets<Image>>,
    node: Query<&ImageNode, With<Minimap>>,
    mut caption: Caption,
    mut last: Local<Option<Shown>>,
) {
    let Ok(picture) = node.single() else {
        return;
    };
    let p = session.views.position;
    let Some(map) = session.data.maps.get(&p.map) else {
        return;
    };
    let scale = zoom.0.scale();
    let pose = session.pose;
    // A sixteenth of a turn is as fine as the heading line can show.
    #[allow(clippy::cast_possible_truncation)]
    let shown = (
        (p.map, p.level),
        session.binder.log.len(),
        crate::minimap::corner(scale, pose),
        (pose.yaw * 8.0 / core::f32::consts::PI).round() as i64,
        scale,
    );
    if *last == Some(shown) {
        return;
    }
    *last = Some(shown);
    let known = session.world.automap(p.map, p.level);
    // A door the party broke, and a doorway that fell in, as the world has them now.
    let worn = marks(known, |x, y, facing| {
        (
            session.world.door_broken(p.map, p.level, x, y, facing),
            session.world.door_collapsed(p.map, p.level, x, y, facing),
        )
    });
    let raster = paint_marked(map, p.level, known, pose, scale, &worn);
    #[allow(clippy::cast_precision_loss)]
    let w = raster.width as f32;
    if let Ok((mut text, mut place)) = caption.single_mut() {
        text.0 = crate::labels::level(&session.data, map, p.level).unwrap_or_default();
        // Left of the picture and its one-pixel border.
        place.right = Val::Px(16.0 + w + 10.0);
    }
    if let Some(mut target) = images.get_mut(&picture.image) {
        *target = image(&raster);
    }
}
