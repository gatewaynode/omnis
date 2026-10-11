//! The world, kept private: this crate reaches the simulation only through the engine's API
//! (ARCHITECTURE.md §4.9, A17), as the methods of [`SimWorld`], and reads it through
//! [`Views`]. `World`'s fields are not API. Tests reach the world as a fixture
//! (`test-fixtures`); the library is checked without it, so a field read does not compile.
//! Bevy-free.

use omnis_sim::api::{
    CastView, CombatView, Command, CreationView, Direction, Event, Facing, Here, Known, MapId,
    ModeKind, NewGameError, Op, OpError, PartyView, Position, Rejection, Reply, RestView,
    ServiceView, Settings, Status, World, apply, automap, cast_view, check_reload, combat_view,
    creation_view, dispatch, door_open, here, party_view, rest_view, save_text, service_view,
    status, step_lands,
};
use omnis_sim::omnis_data::Data;
use std::collections::BTreeMap;

/// The authoritative world, private to this module.
#[derive(Debug, Clone)]
pub struct SimWorld(World);

impl SimWorld {
    /// A running game on `world`.
    #[must_use]
    pub fn new(world: World) -> SimWorld {
        SimWorld(world)
    }

    /// A new game (`World::new`).
    ///
    /// # Errors
    /// The packs have no entry map, or the settings are refused.
    pub fn start(data: &Data, seed: u64, settings: Settings) -> Result<SimWorld, NewGameError> {
        World::new(data, seed, settings).map(SimWorld)
    }

    /// Apply one command (`api::apply`).
    ///
    /// # Errors
    /// The simulation's rejection; the world is unchanged.
    pub fn apply(&mut self, data: &Data, command: Command) -> Result<Vec<Event>, Rejection> {
        apply(&mut self.0, data, command)
    }

    /// What `command` would do now, tried on a copy; the world is unchanged.
    ///
    /// # Errors
    /// The simulation's rejection.
    pub fn trial(&self, data: &Data, command: &Command) -> Result<Vec<Event>, Rejection> {
        apply(&mut self.0.clone(), data, command.clone())
    }

    /// Answer one op of the JSON protocol (`api::dispatch`).
    ///
    /// # Errors
    /// As `api::dispatch`; host ops answer `HostOnly`.
    pub fn dispatch(&mut self, data: &Data, op: &Op) -> Result<Reply, OpError> {
        dispatch(&mut self.0, data, op)
    }

    /// Every member's spells castable outside a fight (`api::cast_view`).
    #[must_use]
    pub fn casts(&self, data: &Data) -> Vec<CastView> {
        cast_view(&self.0, data)
    }

    /// What a rest would do for each member, and why either rest is refused (`api::rest_view`).
    #[must_use]
    pub fn rest(&self, data: &Data) -> RestView {
        rest_view(&self.0, data)
    }

    /// Where the party is and what it is doing (`api::here`).
    #[must_use]
    pub fn here(&self, data: &Data) -> Here {
        here(&self.0, data)
    }

    /// The party's position as registry numbers: `here`'s place looked up in the packs.
    /// `None` only for a map no pack names, which a loaded world never stands on.
    #[must_use]
    pub fn position(&self, data: &Data) -> Option<Position> {
        position(data, &self.here(data))
    }

    /// The party's position while it explores; `None` in a fight, a service or anywhere
    /// movement stops.
    #[must_use]
    pub fn exploring_at(&self, data: &Data) -> Option<Position> {
        let here = self.here(data);
        if here.mode == ModeKind::Explore {
            position(data, &here)
        } else {
            None
        }
    }

    /// The party and every member's sheet (`api::party_view`).
    #[must_use]
    pub fn party(&self, data: &Data) -> PartyView {
        party_view(&self.0, data)
    }

    /// Every world flag and its value, 0 when unset, in registry order (`api::flags`).
    #[must_use]
    pub fn flags(&self, data: &Data) -> Vec<(String, i64)> {
        omnis_sim::api::flags(&self.0, data)
    }

    /// The encounter or fight, if one is on (`api::combat_view`).
    #[must_use]
    pub fn combat(&self, data: &Data) -> Option<CombatView> {
        combat_view(&self.0, data)
    }

    /// The service the party is inside, if any (`api::service_view`).
    #[must_use]
    pub fn service(&self, data: &Data) -> Option<ServiceView> {
        service_view(&self.0, data)
    }

    /// What the party knows of a map (`api::automap`).
    #[must_use]
    pub fn automap(&self, map: MapId) -> Option<&BTreeMap<(u16, u16), Known>> {
        automap(&self.0, map)
    }

    /// Whether a door is open in the world (`api::door_open`).
    #[must_use]
    pub fn door_open(&self, map: MapId, x: u16, y: u16, facing: Facing) -> bool {
        door_open(&self.0, map, x, y, facing)
    }

    /// Where a step in `direction` would leave the party (`api::step_lands`).
    #[must_use]
    pub fn step_lands(&self, data: &Data, direction: Direction) -> Option<Position> {
        step_lands(&self.0, data, direction)
    }

    /// The save file's text, or why the save rule refuses it (`api::save_text`).
    ///
    /// # Errors
    /// The save rule refuses a save here.
    pub fn save_text(&self) -> Result<String, OpError> {
        save_text(&self.0)
    }

    /// `game.status` (`api::status`).
    ///
    /// # Errors
    /// The world could not be fingerprinted.
    pub fn status(&self, data: &Data) -> Result<Status, OpError> {
        status(&self.0, data)
    }

    /// Whether freshly loaded packs can replace the current ones (`api::check_reload`).
    ///
    /// # Errors
    /// The fresh packs would change what the world's numbers mean.
    pub fn check_reload(&self, fresh: &Data) -> Result<(), OpError> {
        check_reload(&self.0, fresh)
    }

    /// Put a loaded world in place of this one.
    pub fn replace(&mut self, world: World) {
        self.0 = world;
    }

    /// Every view the shell reads.
    #[must_use]
    pub fn views(&self, data: &Data) -> Views {
        Views::of(self, data)
    }
}

/// The world itself, for tests that build or inspect a fixture.
#[cfg(any(test, feature = "test-fixtures"))]
impl core::ops::Deref for SimWorld {
    type Target = World;
    fn deref(&self) -> &World {
        &self.0
    }
}

/// The world itself, mutable, for tests that build a fixture.
#[cfg(any(test, feature = "test-fixtures"))]
impl core::ops::DerefMut for SimWorld {
    fn deref_mut(&mut self) -> &mut World {
        &mut self.0
    }
}

/// A place as registry numbers, when a pack names its map.
fn position(data: &Data, here: &Here) -> Option<Position> {
    Some(Position {
        map: data.registry.maps.get(&here.position.map)?,
        x: here.position.x,
        y: here.position.y,
        facing: here.position.facing,
    })
}

/// The settings a game starts with: the player's choices, and `Dev` commands accepted in builds
/// with the developer tools (as the other client's `game_settings`), so the dev socket's
/// `time.reconcile` and the debug tools work there. A save says what it was started with.
#[must_use]
pub fn game_settings(base: Settings) -> Settings {
    Settings {
        devtools: cfg!(feature = "devtools"),
        ..base
    }
}

/// What the shell reads of the world each frame: the engine's views, refreshed by the session
/// after every accepted command and every new world.
#[derive(Debug, Clone, PartialEq)]
pub struct Views {
    /// Where the party is and what it is doing.
    pub here: Here,
    /// `here`'s place as registry numbers, for the maps the packs hold.
    pub position: Position,
    /// The party and every member's sheet.
    pub party: PartyView,
    /// The encounter or fight, if one is on.
    pub combat: Option<CombatView>,
    /// The service the party is inside: every offer with its command, price and refusal.
    pub service: Option<ServiceView>,
    /// Every member's spells castable outside a fight, with cost and refusal.
    pub casts: Vec<CastView>,
    /// What a short or a long rest would do now, and why either is refused.
    pub rest: RestView,
    /// What a new member's draft may be (the loaded rules', so a rule set over the dev socket
    /// shows at the next refresh).
    pub creation: CreationView,
    /// Every world flag with its value, in registry order; only in a devtools world, for the
    /// debug panel (the other client's rule).
    pub flags: Vec<(String, i64)>,
}

impl Views {
    /// Every view of `world`, read through the engine's API only.
    #[must_use]
    pub fn of(world: &SimWorld, data: &Data) -> Views {
        let here = world.here(data);
        let here_devtools = here.settings.devtools;
        // A map no pack names (never the case for a loaded world) reads as one no pack holds.
        let position = position(data, &here).unwrap_or(Position {
            map: MapId(u32::MAX),
            x: here.position.x,
            y: here.position.y,
            facing: here.position.facing,
        });
        Views {
            here,
            position,
            party: world.party(data),
            combat: world.combat(data),
            service: world.service(data),
            casts: world.casts(data),
            rest: world.rest(data),
            creation: creation_view(data),
            flags: if here_devtools {
                world.flags(data)
            } else {
                Vec::new()
            },
        }
    }

    /// Whether the party is exploring (not in an encounter, a fight or a service).
    #[must_use]
    pub fn exploring(&self) -> bool {
        self.here.mode == ModeKind::Explore
    }

    /// Whether the party can no longer fight: every member is down, dead or incapacitated (or
    /// none is left). A fallen party walks no further (`screen::defeat`).
    #[must_use]
    pub fn fallen(&self) -> bool {
        !self.party.members.iter().any(|m| m.can_fight)
    }
}
