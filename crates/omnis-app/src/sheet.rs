//! `SheetPlugin`: the character sheet over the world, opened from the SHEET button, the P
//! key, or the pause menu. Keys and clicks drive the state machine in `sheet_menu.rs`; the
//! member shown and the band's selection keep in step both ways. Headless-capable.

use crate::cursor::UiSet;
use crate::menu::MenuKey;
use crate::menus::{Active, Screens, Where, menu_key};
use crate::screen::{self, Target};
use crate::sim::{PlayState, SimWorld};
use crate::ui::{Selected, UiClick};
use crate::widget::Hit;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

/// The sheet plugin.
pub struct SheetPlugin;

impl Plugin for SheetPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<KeyboardInput>()
            .add_message::<UiClick>()
            .init_resource::<Selected>()
            .add_systems(OnEnter(PlayState::Sheet), open_sheet)
            .add_systems(Update, sheet_keys.in_set(UiSet::Dispatch))
            .add_systems(Update, sheet_model.in_set(UiSet::Model));
    }
}

/// The sheet opens on the band's selected member, or the first.
fn open_sheet(
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    mut selected: ResMut<Selected>,
) {
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    screens.sheet.open(selected.0, members);
    if members > 0 {
        selected.0 = Some(screens.sheet.member);
    }
}

/// Keys and clicks while the sheet is up; a member change selects that member on the band.
fn sheet_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<UiClick>,
    at: Where,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    mut selected: ResMut<Selected>,
    mut next: ResMut<NextState<PlayState>>,
) {
    // Read every frame so a key from before the sheet is never replayed into it.
    let mut pressed: Vec<MenuKey> = keys.read().filter_map(menu_key).collect();
    let hits: Vec<Hit> = clicks.read().map(|c| c.0).collect();
    if at.screen() != Active::Sheet {
        return;
    }
    let Some(world) = world else {
        return;
    };
    let members = world.0.party.members.len();
    for hit in hits {
        pressed.extend(screen::click(Target::Sheet(&mut screens.sheet), hit));
    }
    let before = screens.sheet.member;
    for key in pressed {
        if screens.sheet.key(key, members).is_some() {
            next.set(PlayState::for_mode(&world.0.mode));
        }
    }
    if screens.sheet.member != before {
        selected.0 = Some(screens.sheet.member);
    }
}

/// The sheet follows the band: a member clicked there is the member shown.
fn sheet_model(
    at: Where,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    selected: Res<Selected>,
) {
    if at.screen() != Active::Sheet {
        return;
    }
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    if let Some(slot) = selected.0.filter(|s| *s < members) {
        screens.sheet.member = slot;
    }
    screens.sheet.sync(members);
}
