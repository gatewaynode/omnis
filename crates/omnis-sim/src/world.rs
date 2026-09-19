//! The complete game state (ARCHITECTURE.md §4.1). Ordered collections, integers, serde. A save
//! is this struct as RON text; the fingerprint hashes that text.

use crate::combat::CombatState;
use crate::encounter::EncounterState;
use crate::event::{ActorRef, Event};
use crate::migrate::{WorldV1, v1_to_v2, v2_to_v3, v3_to_v4};
use crate::party::Party;
use crate::{LOG_CAPACITY, PARTY};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{
    Clock, Edges, EraId, Facing, FlagId, HolderId, MapId, Pcg32, Position, StreamName, fnv1a64,
};
use omnis_data::{Data, DataError, PackFingerprint};
use serde::{Deserialize, Serialize};

/// The save schema this build writes. Schema 1 (no party, a save switch), schema 2 (no combat)
/// and schema 3 (no equipment slots, no effects, no devtools bit) migrate on load.
pub const SAVE_SCHEMA: u32 = 4;

/// Mutable state of one map. Static tiles come from data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapState {
    /// Open doors by canonical edge (see `door_key`).
    pub open_doors: BTreeSet<(u16, u16, Facing)>,
    /// Fixed encounters cleared for good, by their index in the map file.
    #[serde(default)]
    pub cleared: BTreeSet<u16>,
}

/// A door edge is shared by two tiles; this names it once, from the north or west tile.
#[must_use]
pub fn door_key(x: u16, y: u16, facing: Facing) -> (u16, u16, Facing) {
    match facing {
        Facing::East | Facing::South => (x, y, facing),
        Facing::West => (x.saturating_sub(1), y, Facing::East),
        Facing::North => (x, y.saturating_sub(1), Facing::South),
    }
}

impl MapState {
    /// Whether the door on `facing` of tile `(x, y)` is open.
    #[must_use]
    pub fn door_open(&self, x: u16, y: u16, facing: Facing) -> bool {
        self.open_doors.contains(&door_key(x, y, facing))
    }
}

/// Knowledge layers recorded on an automap tile.
pub mod layer {
    /// Terrain type known.
    pub const TERRAIN: u8 = 1;
    /// Walls and doors known.
    pub const STRUCTURE: u8 = 2;
    /// The party has stood here.
    pub const VISITED: u8 = 4;
    /// The latest knowledge came from afar (a spyglass), not from the party's own eyes; a
    /// direct sighting clears it.
    pub const REMOTE: u8 = 8;
}

/// What the party knows about one tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Known {
    /// Terrain index into the map's terrains.
    pub terrain: u8,
    /// Walls seen.
    pub walls: Edges,
    /// Doors seen.
    pub doors: Edges,
    /// `layer::*` bits.
    pub layers: u8,
    /// Party clock when last seen; the automap shows what was seen and when, not what is.
    pub seen_at: i64,
}

/// The party's persistent record of the world (PRD §7.2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Automap {
    /// Per map, per tile.
    pub maps: BTreeMap<MapId, BTreeMap<(u16, u16), Known>>,
}

impl Automap {
    /// Record a tile: the terrain when `TERRAIN` is carried, the walls and doors when
    /// `STRUCTURE` is, layers merged (`VISITED` sticks), the time the newest. `REMOTE` is set
    /// by a remote recording and cleared by a direct one.
    pub fn record(&mut self, map: MapId, x: u16, y: u16, known: Known) {
        let entry = self
            .maps
            .entry(map)
            .or_default()
            .entry((x, y))
            .or_insert(Known {
                terrain: known.terrain,
                walls: Edges::default(),
                doors: Edges::default(),
                layers: 0,
                seen_at: known.seen_at,
            });
        if known.layers & layer::TERRAIN != 0 {
            entry.terrain = known.terrain;
        }
        if known.layers & layer::STRUCTURE != 0 {
            entry.walls = known.walls;
            entry.doors = known.doors;
        }
        entry.layers = (entry.layers | known.layers) & !layer::REMOTE;
        if known.layers & layer::REMOTE != 0 {
            entry.layers |= layer::REMOTE;
        }
        entry.seen_at = known.seen_at;
    }

    /// Known tiles of a map.
    #[must_use]
    pub fn map(&self, map: MapId) -> Option<&BTreeMap<(u16, u16), Known>> {
        self.maps.get(&map)
    }
}

/// What the party is doing. Town and journal modes arrive with their milestones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Walking the map.
    Explore,
    /// Monsters ahead; the party chooses what to do.
    Encounter(EncounterState),
    /// Fighting.
    Combat(CombatState),
}

impl Mode {
    /// The mode without its state.
    #[must_use]
    pub const fn kind(&self) -> ModeKind {
        match self {
            Mode::Explore => ModeKind::Explore,
            Mode::Encounter(_) => ModeKind::Encounter,
            Mode::Combat(_) => ModeKind::Combat,
        }
    }
}

/// A mode by name, for status lines and state machines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ModeKind {
    /// Walking the map.
    Explore,
    /// Choosing before a fight.
    Encounter,
    /// Fighting.
    Combat,
}

/// Where the player may save (PRD D17), easiest first.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SaveRule {
    /// Anywhere, any time.
    #[default]
    Anywhere,
    /// At inns, and wherever an item or spell grants a save (M6).
    Relief,
    /// At inns only.
    InnOnly,
}

/// Difficulty options (D17), fixed when the game starts and kept in the save.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Where saving is allowed.
    pub save_rule: SaveRule,
    /// A dead member stays dead; enforced when death arrives (M4).
    pub permadeath: bool,
    /// `Dev` commands are accepted (ARCHITECTURE.md §12). Set by dev builds of the app when a
    /// game starts and by the headless driver, never by a command; a save says what it was.
    #[serde(default)]
    pub devtools: bool,
}

impl Settings {
    /// Whether a save is allowed here. There are no inns yet, so only `Anywhere` says yes.
    #[must_use]
    pub const fn may_save(&self, at_inn: bool) -> bool {
        match self.save_rule {
            SaveRule::Anywhere => true,
            SaveRule::Relief | SaveRule::InnOnly => at_inn,
        }
    }
}

/// The world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    /// Save schema version.
    pub schema: u32,
    /// The world seed every RNG stream derives from (A14).
    pub seed: u64,
    /// The packs this world was created with.
    pub packs: Vec<PackFingerprint>,
    /// Every RNG stream used so far, with its state and draw count.
    pub rngs: BTreeMap<StreamName, Pcg32>,
    /// Subjective time per holder; no global clock (§4.4).
    pub clocks: BTreeMap<HolderId, Clock>,
    /// Where the party is.
    pub position: Position,
    /// Mutable per-map state.
    pub maps: BTreeMap<MapId, MapState>,
    /// What the party knows.
    pub automap: Automap,
    /// What the party is doing.
    pub mode: Mode,
    /// Named integer flags.
    pub flags: BTreeMap<FlagId, i64>,
    /// Difficulty options.
    pub settings: Settings,
    /// The party.
    pub party: Party,
    /// Commands applied so far.
    pub turn: u64,
    /// Recent events for clients that poll (`events.tail`). Not part of the save or the
    /// fingerprint.
    #[serde(skip)]
    pub log: Vec<Event>,
}

/// Why a new game could not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewGameError {
    /// No loaded pack names an entry map.
    NoEntryMap,
}

impl fmt::Display for NewGameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("no loaded pack names an entry map")
    }
}

/// Why a save could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The text is not a world.
    Parse(DataError),
    /// The save was written by a schema this build has no migration for.
    Schema(u32),
    /// The save's packs differ from the loaded ones; pass `force` to load anyway.
    PackMismatch,
    /// The party stands on a map no loaded pack defines, or outside it.
    BadPosition(Position),
    /// The saved encounter or fight is not one this build can continue.
    BadCombat(&'static str),
    /// A member's sheet contradicts itself (an equipped item that is not carried).
    BadParty(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Parse(e) => write!(f, "save is not readable: {e}"),
            LoadError::Schema(s) => write!(
                f,
                "save schema {s} is not supported; this build reads {SAVE_SCHEMA}"
            ),
            LoadError::PackMismatch => f.write_str("save was made with different packs"),
            LoadError::BadPosition(p) => write!(f, "save position {p} is not on a loaded map"),
            LoadError::BadCombat(why) => write!(f, "save combat cannot continue: {why}"),
            LoadError::BadParty(why) => write!(f, "save party is inconsistent: {why}"),
        }
    }
}

impl World {
    /// A new game on the packs' entry map, with an empty party and the given settings.
    pub fn new(data: &Data, seed: u64, settings: Settings) -> Result<World, NewGameError> {
        let map = data.entry.ok_or(NewGameError::NoEntryMap)?;
        let (x, y, facing) = data.maps[&map].def.start;
        let mut clocks = BTreeMap::new();
        clocks.insert(PARTY, Clock::new(EraId(0)));
        Ok(World {
            schema: SAVE_SCHEMA,
            seed,
            packs: data.fingerprints.clone(),
            rngs: BTreeMap::new(),
            clocks,
            position: Position { map, x, y, facing },
            maps: BTreeMap::new(),
            automap: Automap::default(),
            mode: Mode::Explore,
            flags: BTreeMap::new(),
            settings,
            party: Party::default(),
            turn: 0,
            log: Vec::new(),
        })
    }

    /// Whether the settings allow a save where the party stands.
    #[must_use]
    pub const fn may_save(&self) -> bool {
        self.settings.may_save(false)
    }

    /// The RNG stream `name`, created from the seed on first use.
    pub fn stream(&mut self, name: &StreamName) -> &mut Pcg32 {
        let seed = self.seed;
        self.rngs
            .entry(name.clone())
            .or_insert_with(|| Pcg32::for_stream(seed, name))
    }

    /// The party's clock.
    #[must_use]
    pub fn party_clock(&self) -> Clock {
        self.clocks
            .get(&PARTY)
            .copied()
            .unwrap_or(Clock::new(EraId(0)))
    }

    /// Mutable state of the map the party is on, created on first touch.
    pub fn map_state(&mut self, map: MapId) -> &mut MapState {
        self.maps.entry(map).or_default()
    }

    /// Append events to the bounded log.
    pub fn record(&mut self, events: &[Event]) {
        self.log.extend_from_slice(events);
        if self.log.len() > LOG_CAPACITY {
            let excess = self.log.len() - LOG_CAPACITY;
            self.log.drain(..excess);
        }
    }

    /// The save text.
    pub fn to_ron(&self) -> Result<String, DataError> {
        omnis_data::ron_io::to_string(self)
    }

    /// The hash of the canonical save text. Equal fingerprints on two platforms is the
    /// determinism test.
    pub fn fingerprint(&self) -> Result<u64, DataError> {
        Ok(fnv1a64(self.to_ron()?.as_bytes()))
    }

    /// A world from save text, migrated from an older schema when one applies and checked
    /// against the loaded packs. `force` skips the pack fingerprint comparison, not the
    /// structural checks.
    pub fn from_ron(text: &str, data: &Data, force: bool) -> Result<World, LoadError> {
        #[derive(Deserialize)]
        struct Header {
            schema: u32,
        }
        let header: Header = omnis_data::ron_io::parse(text).map_err(LoadError::Parse)?;
        let world: World = match header.schema {
            1 => omnis_data::ron_io::parse::<WorldV1>(text)
                .map(v1_to_v2)
                .map(v2_to_v3)
                .map(|w| v3_to_v4(w, data))
                .map_err(LoadError::Parse)?,
            2 => omnis_data::ron_io::parse::<World>(text)
                .map(v2_to_v3)
                .map(|w| v3_to_v4(w, data))
                .map_err(LoadError::Parse)?,
            3 => omnis_data::ron_io::parse::<World>(text)
                .map(|w| v3_to_v4(w, data))
                .map_err(LoadError::Parse)?,
            SAVE_SCHEMA => omnis_data::ron_io::parse(text).map_err(LoadError::Parse)?,
            other => return Err(LoadError::Schema(other)),
        };
        if !force && world.packs != data.fingerprints {
            return Err(LoadError::PackMismatch);
        }
        let p = world.position;
        if !data
            .maps
            .get(&p.map)
            .is_some_and(|map| map.cell(p.x, p.y).is_some())
        {
            return Err(LoadError::BadPosition(p));
        }
        world.check_mode(data)?;
        world.check_party()?;
        Ok(world)
    }

    /// Every equipped item must be carried: the rules read slots, so a slot naming an item
    /// the member does not have is a hand-edited save.
    fn check_party(&self) -> Result<(), LoadError> {
        for member in &self.party.members {
            if member
                .equipped
                .values()
                .any(|item| !member.equipment.iter().any(|(id, n)| id == item && *n > 0))
            {
                return Err(LoadError::BadParty("an equipped item is not carried"));
            }
        }
        Ok(())
    }

    /// A saved encounter or fight must name known monsters and, in a fight, wait on a living
    /// member: the turn loop parks there between commands, so anything else is a hand-edited
    /// save.
    fn check_mode(&self, data: &Data) -> Result<(), LoadError> {
        let stacks = match &self.mode {
            Mode::Explore => return Ok(()),
            Mode::Encounter(e) => &e.stacks,
            Mode::Combat(c) => &c.encounter.stacks,
        };
        if stacks
            .iter()
            .any(|s| !data.monsters.contains_key(&s.monster))
        {
            return Err(LoadError::BadCombat("a stack names an unknown monster"));
        }
        if let Mode::Combat(c) = &self.mode {
            let waiting = c
                .order
                .get(usize::from(c.current))
                .and_then(|entry| match entry.actor {
                    ActorRef::Member(id) => self.party.members.iter().find(|m| m.id == id),
                    _ => None,
                });
            if !waiting.is_some_and(|m| !m.is_down()) {
                return Err(LoadError::BadCombat(
                    "the fight is not waiting on a living member",
                ));
            }
        }
        Ok(())
    }
}
