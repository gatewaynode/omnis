//! `InventoryPlugin`: the inventory overlay over the world, opened from the ITEMS button or
//! the I key. Keys and clicks drive the model in `inventory_menu.rs`; every action is an
//! `ItemCommand` for the simulation and the overlay stays open, so the band's selection is
//! the target for Use, Take and Give. Headless-capable.

use crate::cursor::UiSet;
use crate::inventory_menu::{InventoryIntent, inventory_view};
use crate::menu::MenuKey;
use crate::menus::{Active, Screens, Where, menu_key};
use crate::screen::{self, Target};
use crate::sim::{CommandRefused, PackData, PlayState, PlayerCommand, SimWorld};
use crate::ui::{Selected, UiClick};
use crate::widget::Hit;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

/// The inventory plugin.
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<KeyboardInput>()
            .add_message::<UiClick>()
            .add_message::<PlayerCommand>()
            .add_message::<CommandRefused>()
            .init_resource::<Selected>()
            .add_systems(OnEnter(PlayState::Inventory), open_inventory)
            .add_systems(Update, inventory_keys.in_set(UiSet::Dispatch))
            .add_systems(Update, inventory_refused.in_set(UiSet::Model));
    }
}

/// The overlay opens on the band's selected member, or the first pane.
fn open_inventory(
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    selected: Res<Selected>,
) {
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    screens.inventory.open(selected.0, members);
}

/// Keys and clicks while the overlay is up.
#[allow(clippy::too_many_arguments)]
fn inventory_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<UiClick>,
    at: Where,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    selected: Res<Selected>,
    mut player: MessageWriter<PlayerCommand>,
    mut next: ResMut<NextState<PlayState>>,
) {
    // Read every frame so a key from before the overlay is never replayed into it.
    let mut pressed: Vec<MenuKey> = keys.read().filter_map(menu_key).collect();
    let hits: Vec<Hit> = clicks.read().map(|c| c.0).collect();
    if at.screen() != Active::Inventory {
        return;
    }
    let Some((world, data)) = world.zip(data) else {
        return;
    };
    let view = inventory_view(&world.0, &data.0);
    screens.inventory.sync(&view);
    for hit in hits {
        pressed.extend(screen::click(
            Target::Inventory(&mut screens.inventory),
            hit,
        ));
    }
    for key in pressed {
        match screens.inventory.key(key, &view, selected.0) {
            Some(InventoryIntent::Command(command)) => {
                player.write(PlayerCommand(command));
            }
            Some(InventoryIntent::Close) => next.set(PlayState::for_mode(&world.0.mode)),
            None => {}
        }
    }
}

/// A refused item command is the overlay's message.
fn inventory_refused(
    mut refused: MessageReader<CommandRefused>,
    at: Where,
    mut screens: ResMut<Screens>,
) {
    let last = refused.read().last().cloned();
    if let (Active::Inventory, Some(CommandRefused(rejection))) = (at.screen(), last) {
        screens.inventory.message = rejection.to_string();
    }
}
