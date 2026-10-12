//! Which screen is up (presentation-ARCHITECTURE §6.3): a base that follows the world, and the
//! overlays the player opened on top of it. After every change the overlays are checked against
//! the world, so one the world no longer allows closes (the party moved, a fight began, the game
//! ended). A new game is created member by member before it is played: until Begin, its base is
//! party creation.

use super::ScreenId;
use crate::sim::Views;

/// What Esc did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    /// Closed the top overlay.
    Closed,
    /// The fight's menu steps back: it was not at its first step.
    FightBack,
    /// Opened the pause.
    Paused,
    /// Leave party creation for the new game's form, as its Back does.
    Leave,
    /// Leave the service the party is in, as its Leave does.
    LeaveService,
    /// Nothing to do.
    Nothing,
}

/// The overlays, bottom first; the base comes from the world.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Nav {
    stack: Vec<ScreenId>,
    /// The game is a new one whose party is being created; Begin ends it.
    creating: bool,
}

impl Nav {
    /// The screen the world calls for under any overlay: the title with no game, party creation
    /// while the party is created or has no one (an empty party is not a fallen one), the
    /// fallen party's, a service's, or none (the 3D view, or the fight screen, which draws
    /// itself).
    #[must_use]
    pub fn base(&self, game: Option<&Views>) -> Option<ScreenId> {
        let Some(views) = game else {
            return Some(ScreenId::Title);
        };
        if views.combat.is_some() {
            None
        } else if self.creating || views.party.members.is_empty() {
            Some(ScreenId::CreateParty)
        } else if views.fallen() {
            Some(ScreenId::Defeat)
        } else if views.here.service.is_some() {
            Some(ScreenId::Service)
        } else {
            None
        }
    }

    /// Whether the world allows overlay `id` now (the Camp button is lit by it).
    #[must_use]
    pub fn allowed(&self, id: ScreenId, game: Option<&Views>) -> bool {
        let base = self.base(game);
        match (id, game) {
            (ScreenId::NewGame, None) => true,
            // From the title, or from the pause wherever it is allowed.
            (ScreenId::Settings, None) => true,
            (ScreenId::Settings, Some(_)) => {
                !matches!(base, Some(ScreenId::Defeat | ScreenId::CreateParty))
            }
            // Exploring, met, fighting or in a service, in a devtools world.
            (ScreenId::Debug, Some(views)) => {
                matches!(base, None | Some(ScreenId::Service))
                    && super::debug::unavailable(views).is_none()
            }
            (ScreenId::Pause, Some(_)) => {
                !matches!(base, Some(ScreenId::Defeat | ScreenId::CreateParty))
            }
            (ScreenId::Actions, Some(views)) => base.is_none() && views.exploring(),
            // From the party column, exploring or in a fight (to look); not over a service.
            (ScreenId::Inventory | ScreenId::Cast | ScreenId::Sheet, Some(_)) => base.is_none(),
            // Resting is the map's: not in a service (the room is the rest indoors) or a fight.
            (ScreenId::Camp, Some(views)) => base.is_none() && views.exploring(),
            // Tactics are written outside a fight (the fight's React switch is its own).
            (ScreenId::Tactics, Some(views)) => base.is_none() && views.combat.is_none(),
            _ => false,
        }
    }

    /// Whether the party is being created.
    #[must_use]
    pub fn creating(&self) -> bool {
        self.creating
    }

    /// The party is made: play begins.
    pub fn begin(&mut self) {
        self.creating = false;
    }

    /// The screen on top: the last overlay, else the base.
    #[must_use]
    pub fn top(&self, game: Option<&Views>) -> Option<ScreenId> {
        self.stack.last().copied().or_else(|| self.base(game))
    }

    /// Whether the game waits on the screen on top: nothing moves or acts under the pause, the
    /// fallen party's screen, or one of the screens beside the party column.
    #[must_use]
    pub fn paused(&self, game: Option<&Views>) -> bool {
        matches!(
            self.top(game),
            Some(
                ScreenId::Pause
                    | ScreenId::Inventory
                    | ScreenId::Cast
                    | ScreenId::Camp
                    | ScreenId::Sheet
                    | ScreenId::Tactics
                    | ScreenId::Defeat
                    | ScreenId::Title
                    | ScreenId::NewGame
                    | ScreenId::CreateParty
                    | ScreenId::Settings
                    | ScreenId::Debug
            )
        )
    }

    /// Whether `id` is open as an overlay.
    #[must_use]
    pub fn is_open(&self, id: ScreenId) -> bool {
        self.stack.contains(&id)
    }

    /// Open overlay `id` on top, when the world allows it and it is not open already.
    pub fn open(&mut self, id: ScreenId, game: Option<&Views>) {
        if self.allowed(id, game) && !self.is_open(id) {
            self.stack.push(id);
        }
    }

    /// Open overlay `id`, or close it when it is on top.
    pub fn toggle(&mut self, id: ScreenId, game: Option<&Views>) {
        if self.stack.last() == Some(&id) {
            self.stack.pop();
        } else {
            self.open(id, game);
        }
    }

    /// Show the column's screen `id` (the inventory, the cast screen or the sheet) in place of
    /// whatever stands beside the column, or on top, when the world allows it.
    pub fn show(&mut self, id: ScreenId, game: Option<&Views>) {
        while self.stack.last().is_some_and(|top| top.beside_column()) {
            self.stack.pop();
        }
        self.open(id, game);
    }

    /// Close the top overlay; `false` when there is none.
    pub fn back(&mut self) -> bool {
        self.stack.pop().is_some()
    }

    /// Close every overlay for a game that has just begun, been loaded or ended (`None`): a
    /// game with no party yet is created first.
    pub fn reset(&mut self, game: Option<&Views>) {
        self.stack.clear();
        self.creating = game.is_some_and(|v| v.party.members.is_empty());
    }

    /// Close what the world no longer allows, and the square's actions when the party `moved`.
    pub fn settle(&mut self, game: Option<&Views>, moved: bool) {
        let stack = core::mem::take(&mut self.stack);
        self.stack = stack
            .into_iter()
            .filter(|&id| self.allowed(id, game) && !(moved && id == ScreenId::Actions))
            .collect();
    }

    /// Esc: close the top overlay; otherwise step a fight's menu back when it is not at its
    /// first step (`fight_at_root` false); otherwise leave party creation or the service, or
    /// pause a running game.
    pub fn escape(&mut self, game: Option<&Views>, fight_at_root: bool) -> Escape {
        if self.back() {
            return Escape::Closed;
        }
        let Some(views) = game else {
            return Escape::Nothing;
        };
        if views.combat.is_some() && !fight_at_root {
            return Escape::FightBack;
        }
        match self.base(game) {
            Some(ScreenId::CreateParty) => return Escape::Leave,
            Some(ScreenId::Service) => return Escape::LeaveService,
            _ => {}
        }
        if self.allowed(ScreenId::Pause, game) {
            self.stack.push(ScreenId::Pause);
            Escape::Paused
        } else {
            Escape::Nothing
        }
    }
}
