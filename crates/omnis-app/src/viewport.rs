//! `ViewportPlugin`: after every simulation event batch, rebuild the viewport sprites from the
//! draw plan, and the automap overlay when it is shown.

use crate::assets::PackImages;
use crate::layout::{
    CANVAS_HEIGHT, CANVAS_WIDTH, OVERLAY_MAP_CLIP, OVERLAY_MAP_SCALE, SIDEBAR_MAP,
    SIDEBAR_MAP_SCALE, VIEWPORT_ORIGIN, VIEWPORT_SIZE,
};
use crate::plan::{self, DrawOp, Paint};
use crate::sim::{PackData, ShellCommand, SimEvent, SimSet, SimWorld, WorldReplaced};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use omnis_sim::Event;
use omnis_sim::query;

/// A viewport sprite; despawned on every redraw.
#[derive(Component)]
pub struct ViewportSprite;

/// An automap sprite; despawned on every redraw.
#[derive(Component)]
pub struct AutomapSprite;

/// Whether the large automap overlay is shown over the viewport. The sidebar minimap is
/// always shown.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutomapShown(pub bool);

/// The viewport plugin.
pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutomapShown>().add_systems(
            Update,
            (toggle_automap, redraw).chain().in_set(SimSet::Publish),
        );
    }
}

fn toggle_automap(mut shell: MessageReader<ShellCommand>, mut shown: ResMut<AutomapShown>) {
    for command in shell.read() {
        if *command == ShellCommand::ToggleAutomap {
            shown.0 = !shown.0;
        }
    }
}

/// Canvas pixel coordinates to world coordinates for a top-left anchored sprite.
#[must_use]
pub fn canvas_to_world(x: i32, y: i32, z: f32) -> Vec3 {
    Vec3::new(
        x as f32 - CANVAS_WIDTH as f32 / 2.0,
        CANVAS_HEIGHT as f32 / 2.0 - y as f32,
        z,
    )
}

fn spawn_ops<M: Component + Default>(
    commands: &mut Commands,
    server: &AssetServer,
    images: &mut PackImages,
    ops: &[DrawOp],
    origin: (i32, i32),
    z0: f32,
) {
    for (i, op) in ops.iter().enumerate() {
        let translation = canvas_to_world(origin.0 + op.x, origin.1 + op.y, z0 + i as f32 * 0.001);
        let sprite = match &op.paint {
            Paint::Sprite(path) => Sprite::from_image(images.get(server, path)),
            Paint::Fill {
                color,
                width,
                height,
            } => Sprite::from_color(
                Color::srgb_u8(color.0, color.1, color.2),
                Vec2::new(*width as f32, *height as f32),
            ),
        };
        commands.spawn((
            sprite,
            Anchor::TOP_LEFT,
            Transform::from_translation(translation),
            M::default(),
        ));
    }
}

impl Default for ViewportSprite {
    fn default() -> Self {
        ViewportSprite
    }
}

impl Default for AutomapSprite {
    fn default() -> Self {
        AutomapSprite
    }
}

#[allow(clippy::too_many_arguments)]
fn redraw(
    mut commands: Commands,
    mut events: MessageReader<SimEvent>,
    mut replaced: MessageReader<WorldReplaced>,
    shown: Res<AutomapShown>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    server: Res<AssetServer>,
    mut images: ResMut<PackImages>,
    old_view: Query<Entity, With<ViewportSprite>>,
    old_map: Query<Entity, With<AutomapSprite>>,
) {
    let saw_visible = events.read().any(|e| matches!(e.0, Event::Visible { .. }));
    let was_replaced = replaced.read().count() > 0;
    if !(saw_visible || was_replaced || shown.is_changed()) {
        return;
    }
    let (Some(world), Some(data)) = (world, data) else {
        return;
    };
    for entity in &old_view {
        commands.entity(entity).despawn();
    }
    for entity in &old_map {
        commands.entity(entity).despawn();
    }
    let Some(view) = query::viewport(&world.0, &data.0) else {
        return;
    };
    // Sky or darkness inside the viewport only; the panel colour shows around it.
    let backdrop = plan::backdrop(&data.0, world.0.position.map);
    let sky = DrawOp {
        paint: Paint::Fill {
            color: backdrop,
            width: VIEWPORT_SIZE.0,
            height: VIEWPORT_SIZE.1,
        },
        x: 0,
        y: 0,
    };
    spawn_ops::<ViewportSprite>(
        &mut commands,
        &server,
        &mut images,
        &[sky],
        VIEWPORT_ORIGIN,
        0.5,
    );
    spawn_ops::<ViewportSprite>(
        &mut commands,
        &server,
        &mut images,
        &plan::viewport(&view, &data.0),
        VIEWPORT_ORIGIN,
        1.0,
    );
    let sidebar = plan::automap_window(&world.0, &data.0, SIDEBAR_MAP, SIDEBAR_MAP_SCALE);
    spawn_ops::<AutomapSprite>(&mut commands, &server, &mut images, &sidebar, (0, 0), 10.0);
    if shown.0 {
        // Clipped to the viewport so a large map never covers the column or the band.
        let overlay = plan::automap_window(
            &world.0,
            &data.0,
            OVERLAY_MAP_CLIP.tuple(),
            OVERLAY_MAP_SCALE,
        );
        spawn_ops::<AutomapSprite>(&mut commands, &server, &mut images, &overlay, (0, 0), 20.0);
    }
}
