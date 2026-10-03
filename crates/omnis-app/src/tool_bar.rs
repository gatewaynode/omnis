//! The tool bar's model (M7 step 8a): the doors to the screens outside a fight, which button is
//! live on which screen, and the one gate every press passes. The bar itself is a `bevy_ui`
//! scene (`feathers_tools.rs`) over the canvas's tool strip (`layout::TOOLS`); its states are a
//! resource kept here every frame, so an app without `bevy_ui` (the `MinimalPlugins` tests) has
//! them too, and a press is refused here, by the states, whatever the widget did.

use crate::look::look_command;
use crate::menus::{Active, Where};
use crate::sim::{PackData, ShellCommand, SimWorld};
use crate::spell_menu::cast_rows;
use crate::widget::PadState;
use bevy::prelude::*;

/// A tool bar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolButton {
    /// The inventory (M6b).
    Items,
    /// The cast menu.
    Spells,
    /// The character sheet (E3).
    Sheet,
    /// The camp: rests outside a service (M7 step 8b).
    Camp,
    /// Look through a sense item (M6c).
    Look,
    /// The automap.
    Map,
    /// The pause menu.
    Menu,
}

impl ToolButton {
    /// Every button, in the bar's reading order: four on the first row, three on the second.
    pub const ALL: [ToolButton; 7] = [
        ToolButton::Items,
        ToolButton::Spells,
        ToolButton::Sheet,
        ToolButton::Camp,
        ToolButton::Look,
        ToolButton::Map,
        ToolButton::Menu,
    ];

    /// The word on the button.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            ToolButton::Items => "Items",
            ToolButton::Spells => "Spells",
            ToolButton::Sheet => "Sheet",
            ToolButton::Camp => "Camp",
            ToolButton::Look => "Look",
            ToolButton::Map => "Map",
            ToolButton::Menu => "Menu",
        }
    }
}

/// Each button's state, in `ToolButton::ALL` order; kept every frame by `track`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolStates(pub [PadState; 7]);

impl Default for ToolStates {
    fn default() -> Self {
        ToolStates::all(PadState::Hidden)
    }
}

impl ToolStates {
    /// Every button in one state.
    #[must_use]
    pub const fn all(state: PadState) -> ToolStates {
        ToolStates([state; 7])
    }

    /// The state of one button.
    #[must_use]
    pub const fn get(self, button: ToolButton) -> PadState {
        self.0[button as usize]
    }

    /// Set the state of one button.
    pub const fn set(&mut self, button: ToolButton, state: PadState) {
        self.0[button as usize] = state;
    }

    /// Whether the bar is shown at all.
    #[must_use]
    pub fn shown(self) -> bool {
        self.0.iter().any(|s| *s != PadState::Hidden)
    }
}

/// The shell action a tool button stands for; none for CAMP until its panel (step 8b).
#[must_use]
pub const fn tool_for(button: ToolButton) -> Option<ShellCommand> {
    Some(match button {
        ToolButton::Items => ShellCommand::Inventory,
        ToolButton::Spells => ShellCommand::Cast,
        ToolButton::Sheet => ShellCommand::Sheet,
        ToolButton::Camp => return None,
        ToolButton::Look => ShellCommand::Look,
        ToolButton::Map => ShellCommand::ToggleAutomap,
        ToolButton::Menu => ShellCommand::Pause,
    })
}

/// A press on a tool bar button, from the bar or a test; `answer` gates it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolPressed(pub ToolButton);

/// The tool bar's states: hidden without a world; MENU live wherever Escape pauses (the map,
/// an encounter, a fight) and inside a service; MAP live on the map; SPELLS on the map and
/// inside a service when someone has a spell for the road; SHEET on the map, inside a service
/// and in a fight once the party has a member; ITEMS on the map and inside a service with a
/// member; LOOK on the map when a member who can act carries a sense item; CAMP dim until its
/// panel (step 8b).
#[must_use]
pub fn tool_states(
    active: Active,
    has_world: bool,
    has_casts: bool,
    has_members: bool,
    has_look: bool,
) -> ToolStates {
    if !has_world {
        return ToolStates::default();
    }
    let live = |on: bool| {
        if on {
            PadState::Enabled
        } else {
            PadState::Disabled
        }
    };
    let exploring = active == Active::None;
    let mut tools = ToolStates::all(PadState::Disabled);
    tools.set(
        ToolButton::Menu,
        live(matches!(
            active,
            Active::None | Active::Service | Active::Encounter | Active::Combat
        )),
    );
    tools.set(ToolButton::Map, live(exploring));
    let indoors = exploring || active == Active::Service;
    tools.set(ToolButton::Items, live(indoors && has_members));
    tools.set(ToolButton::Look, live(exploring && has_look));
    tools.set(ToolButton::Spells, live(indoors && has_casts));
    tools.set(
        ToolButton::Sheet,
        live(has_members && matches!(active, Active::None | Active::Service | Active::Combat)),
    );
    tools
}

/// Keep the states for the screen and the world as they are now.
pub fn track(
    at: Where,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut states: ResMut<ToolStates>,
) {
    let active = at.screen();
    let loaded = world.as_ref().zip(data.as_ref());
    let has_casts = (at.exploring() || active == Active::Service)
        && loaded.is_some_and(|(w, d)| !cast_rows(&w.0, &d.0).is_empty());
    let has_look =
        at.exploring() && loaded.is_some_and(|(w, d)| look_command(&w.0, &d.0).is_some());
    let has_members = world
        .as_ref()
        .is_some_and(|w| !w.0.party.members.is_empty());
    let wanted = tool_states(
        active,
        world.is_some() && at.playing(),
        has_casts,
        has_members,
        has_look,
    );
    if *states != wanted {
        *states = wanted;
    }
}

/// A press on a live button, whatever the screen: the states gate it, not the widget.
pub fn answer(
    mut presses: MessageReader<ToolPressed>,
    states: Res<ToolStates>,
    mut shell: MessageWriter<ShellCommand>,
) {
    for ToolPressed(button) in presses.read() {
        if states.get(*button) == PadState::Enabled
            && let Some(command) = tool_for(*button)
        {
            shell.write(command);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_bar_follows_the_screen() {
        assert_eq!(
            tool_states(Active::None, false, true, true, true),
            ToolStates::default()
        );
        assert!(!ToolStates::default().shown());
        let map = tool_states(Active::None, true, true, true, true);
        assert!(map.shown());
        for button in ToolButton::ALL {
            let wanted = if button == ToolButton::Camp {
                PadState::Disabled
            } else {
                PadState::Enabled
            };
            assert_eq!(map.get(button), wanted, "{button:?}");
        }
        assert_eq!(
            tool_states(Active::None, true, true, true, false).get(ToolButton::Look),
            PadState::Disabled,
            "no spyglass"
        );
        let nobody = tool_states(Active::None, true, false, false, false);
        assert_eq!(nobody.get(ToolButton::Spells), PadState::Disabled);
        assert_eq!(nobody.get(ToolButton::Sheet), PadState::Disabled);
        assert_eq!(nobody.get(ToolButton::Items), PadState::Disabled);
        assert_eq!(nobody.get(ToolButton::Look), PadState::Disabled);
        assert_eq!(nobody.get(ToolButton::Map), PadState::Enabled);
        let fight = tool_states(Active::Combat, true, true, true, true);
        assert_eq!(fight.get(ToolButton::Menu), PadState::Enabled);
        assert_eq!(fight.get(ToolButton::Sheet), PadState::Enabled);
        assert_eq!(fight.get(ToolButton::Map), PadState::Disabled);
        assert_eq!(fight.get(ToolButton::Spells), PadState::Disabled);
        assert_eq!(fight.get(ToolButton::Items), PadState::Disabled);
        assert_eq!(fight.get(ToolButton::Look), PadState::Disabled);
        let shop = tool_states(Active::Service, true, true, true, true);
        for button in [
            ToolButton::Items,
            ToolButton::Spells,
            ToolButton::Sheet,
            ToolButton::Menu,
        ] {
            assert_eq!(
                shop.get(button),
                PadState::Enabled,
                "{button:?} in a service"
            );
        }
        assert_eq!(shop.get(ToolButton::Map), PadState::Disabled);
        assert_eq!(shop.get(ToolButton::Look), PadState::Disabled);
        for active in [
            Active::Paused,
            Active::Cast,
            Active::Debug,
            Active::Defeat,
            Active::Sheet,
            Active::Inventory,
        ] {
            assert_eq!(
                tool_states(active, true, true, true, true),
                ToolStates::all(PadState::Disabled),
                "{active:?}"
            );
        }
    }

    #[test]
    fn tool_buttons_send_what_their_keys_send() {
        use crate::input::shell_for;
        assert_eq!(tool_for(ToolButton::Spells), shell_for(KeyCode::KeyC));
        assert_eq!(tool_for(ToolButton::Map), shell_for(KeyCode::KeyM));
        assert_eq!(tool_for(ToolButton::Sheet), shell_for(KeyCode::KeyP));
        assert_eq!(tool_for(ToolButton::Menu), shell_for(KeyCode::Escape));
        assert_eq!(tool_for(ToolButton::Items), shell_for(KeyCode::KeyI));
        assert_eq!(tool_for(ToolButton::Look), shell_for(KeyCode::KeyL));
        assert_eq!(tool_for(ToolButton::Camp), None, "until step 8b");
    }

    #[test]
    fn the_states_hold_one_slot_per_button() {
        let mut states = ToolStates::default();
        assert_eq!(states.get(ToolButton::Menu), PadState::Hidden);
        states.set(ToolButton::Menu, PadState::Enabled);
        assert_eq!(states.get(ToolButton::Menu), PadState::Enabled);
        assert_eq!(states.get(ToolButton::Map), PadState::Hidden);
        assert!(states.shown());
        for (index, button) in ToolButton::ALL.iter().enumerate() {
            assert_eq!(*button as usize, index, "{button:?}");
        }
    }
}
