//! Which screen is up (presentation-ARCHITECTURE §6.3): a base that follows the world, and the
//! overlays the player opened on top of it. After every change the overlays are checked against
//! the world, so one the world no longer allows closes (the party moved, a fight began).

use super::ScreenId;
use crate::sim::Views;

/// What Esc did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    /// Closed the top overlay.
    Closed,
    /// Nothing to close: the fight's menu steps back, when the fight screen is up.
    FightBack,
    /// Nothing to do.
    Nothing,
}

/// The overlays, bottom first; the base comes from the world.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Nav {
    stack: Vec<ScreenId>,
}

/// The screen the world calls for under any overlay: the fallen party's, a service's, or none
/// (the 3D view, or the fight screen, which draws itself).
#[must_use]
pub fn base(views: &Views) -> Option<ScreenId> {
    if views.combat.is_some() {
        None
    } else if views.fallen() {
        Some(ScreenId::Defeat)
    } else if views.here.service.is_some() {
        Some(ScreenId::Service)
    } else {
        None
    }
}

/// Whether the world allows overlay `id` now.
fn allowed(id: ScreenId, views: &Views) -> bool {
    match id {
        ScreenId::Actions => base(views).is_none() && views.exploring(),
        ScreenId::Service | ScreenId::Defeat => false,
    }
}

impl Nav {
    /// The screen on top: the last overlay, else the base.
    #[must_use]
    pub fn top(&self, views: &Views) -> Option<ScreenId> {
        self.stack.last().copied().or_else(|| base(views))
    }

    /// Whether `id` is open as an overlay.
    #[must_use]
    pub fn is_open(&self, id: ScreenId) -> bool {
        self.stack.contains(&id)
    }

    /// Open overlay `id` on top, when the world allows it and it is not open already.
    pub fn open(&mut self, id: ScreenId, views: &Views) {
        if allowed(id, views) && !self.is_open(id) {
            self.stack.push(id);
        }
    }

    /// Open overlay `id`, or close it when it is on top.
    pub fn toggle(&mut self, id: ScreenId, views: &Views) {
        if self.stack.last() == Some(&id) {
            self.stack.pop();
        } else {
            self.open(id, views);
        }
    }

    /// Close the top overlay; `false` when there is none.
    pub fn back(&mut self) -> bool {
        self.stack.pop().is_some()
    }

    /// Close what the world no longer allows, and the square's actions when the party `moved`.
    pub fn settle(&mut self, views: &Views, moved: bool) {
        self.stack
            .retain(|&id| allowed(id, views) && !(moved && id == ScreenId::Actions));
    }

    /// Esc: close the top overlay; otherwise, in a fight, step the fight's menu back.
    pub fn escape(&mut self, views: &Views) -> Escape {
        if self.back() {
            Escape::Closed
        } else if views.combat.is_some() {
            Escape::FightBack
        } else {
            Escape::Nothing
        }
    }
}
