//! The remap (M8.5 7h3): a world rebuilt in its save's numbering, renumbered by name for the
//! loaded packs. Each number is read through the name table of the world folder it was written
//! in (`Manifest.names`) and found again by its string id in the loaded registry; the rows a
//! save keeps by position (a map's terrains, an object kind's `once` interactions) are found
//! again by name and signature. Only the live world is remapped: its history (the journal, the
//! bases, the archive) stays in its own numbering under its own folder, and a load folds.
//! A name the loaded packs do not have is `Missing`, with where the world uses it; a load
//! refuses them all at once (nothing is dropped without a migration, owner 2026-10-15).

use super::NameTable;
use super::chain::Chain;
use crate::bus::Topic;
use crate::encounter::EncounterState;
use crate::log::Logged;
use crate::world::{Known, MapState, Mode, World};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt;
use omnis_core::{
    ActorId, BackgroundId, CharacterId, ClassId, ConditionId, FlagId, HolderId, ItemId, MapId,
    MonsterId, PartyId, ProjectId, RaceId, RegionId, ServiceId, SpellId, StreamName,
};
use omnis_data::Data;
use omnis_data::migration::{Kind, Target};
use omnis_data::registry::{Interner, Registry};
use omnis_rules::tactics::{Ids, Rename};
use omnis_rules::{ActiveEffect, Character};
use serde::{Deserialize, Serialize};

/// What kind of name a save holds that the loaded packs do not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameKind {
    /// A map.
    Map,
    /// A flag.
    Flag,
    /// A region.
    Region,
    /// An item.
    Item,
    /// A spell.
    Spell,
    /// A monster.
    Monster,
    /// A condition.
    Condition,
    /// A race.
    Race,
    /// A class.
    Class,
    /// A background.
    Background,
    /// A service.
    Service,
    /// A terrain of a map, by its key.
    Terrain,
    /// A `once` interaction of an object kind, by its verb, label and state.
    Interaction,
}

impl fmt::Display for NameKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NameKind::Map => "map",
            NameKind::Flag => "flag",
            NameKind::Region => "region",
            NameKind::Item => "item",
            NameKind::Spell => "spell",
            NameKind::Monster => "monster",
            NameKind::Condition => "condition",
            NameKind::Race => "race",
            NameKind::Class => "class",
            NameKind::Background => "background",
            NameKind::Service => "service",
            NameKind::Terrain => "terrain",
            NameKind::Interaction => "interaction",
        })
    }
}

/// A name a save holds that the loaded packs do not: what it is, and where the world uses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Missing {
    /// Its kind.
    pub kind: NameKind,
    /// Its string id, as the save's world folder names it.
    pub name: String,
    /// Where the world holds it (`party.members[2].known_spells`).
    pub at: String,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} '{}' in {}", self.kind, self.name, self.at)
    }
}

/// An id the registry numbers, with its kind's list in a name table.
trait Registered: Copy + From<u32> + Into<u32> {
    const KIND: NameKind;
    const MIGRATION: Kind;
    fn old(names: &NameTable) -> &[String];
    fn new(registry: &Registry) -> &Interner<Self>;
}

macro_rules! registered {
    ($($id:ty, $kind:ident, $field:ident;)*) => {$(
        impl Registered for $id {
            const KIND: NameKind = NameKind::$kind;
            const MIGRATION: Kind = Kind::$kind;
            fn old(names: &NameTable) -> &[String] {
                &names.$field
            }
            fn new(registry: &Registry) -> &Interner<Self> {
                &registry.$field
            }
        }
    )*};
}

registered! {
    MapId, Map, maps;
    FlagId, Flag, flags;
    RegionId, Region, regions;
    ItemId, Item, items;
    SpellId, Spell, spells;
    MonsterId, Monster, monsters;
    ConditionId, Condition, conditions;
    RaceId, Race, races;
    ClassId, Class, classes;
    BackgroundId, Background, backgrounds;
    ServiceId, Service, services;
}

/// Renumbers from one name table to the loaded packs, through the save's migrations,
/// collecting what is missing.
struct Remapper<'a> {
    old: &'a NameTable,
    data: &'a Data,
    chain: &'a Chain<'a>,
    missing: RefCell<Vec<Missing>>,
    /// Where the tactics being renamed are, for `Rename`'s calls.
    here: RefCell<String>,
    /// The name a migration removed that `Rename` last met, for the tactics that name it.
    removed: RefCell<Option<(NameKind, String)>>,
}

/// A saved number found again.
enum Found<I> {
    /// Its number on the loaded packs.
    To(I),
    /// A migration removed it: its name.
    Removed(String),
    /// The loaded packs lack it: its name.
    Missing(String),
}

impl Remapper<'_> {
    fn miss(&self, kind: NameKind, name: String, at: &str) {
        self.missing.borrow_mut().push(Missing {
            kind,
            name,
            at: at.into(),
        });
    }

    /// The old number's name, if the table has it.
    fn name_of<I: Registered>(&self, id: I) -> Option<&str> {
        let index = usize::try_from(id.into()).ok()?;
        I::old(self.old).get(index).map(String::as_str)
    }

    /// `id` found again on the loaded packs, through the save's migrations.
    fn found<I: Registered>(&self, id: I) -> Found<I> {
        let Some(name) = self.name_of(id) else {
            return Found::Missing(format!("#{}", id.into()));
        };
        let name = match self.chain.name(I::MIGRATION, name) {
            Some(Target::Removed) => return Found::Removed(name.into()),
            Some(Target::To(to)) => to.as_str(),
            None => name,
        };
        I::new(&self.data.registry)
            .get(name)
            .map_or_else(|| Found::Missing(name.into()), Found::To)
    }

    /// `id`'s number on the loaded packs, where the world cannot do without it; missing or
    /// removed, it is noted and kept.
    fn id<I: Registered>(&self, id: I, at: &str) -> I {
        match self.found(id) {
            Found::To(new) => new,
            Found::Removed(name) => {
                self.miss(I::KIND, name, &format!("{at} (removed by a migration)"));
                id
            }
            Found::Missing(name) => {
                self.miss(I::KIND, name, at);
                id
            }
        }
    }

    /// `id` renumbered in a list that can do without it: false when a migration removed it.
    fn keep<I: Registered>(&self, id: &mut I, at: &str) -> bool {
        match self.found(*id) {
            Found::To(new) => *id = new,
            Found::Removed(_) => return false,
            Found::Missing(name) => self.miss(I::KIND, name, at),
        }
        true
    }

    /// A list of stacks renumbered, a removed item's stack gone. Stacks of one item stay apart,
    /// as a starting kit keeps them.
    fn items(&self, items: &mut Vec<(ItemId, u16)>, at: &str) {
        items.retain_mut(|(item, _)| self.keep(item, at));
    }

    /// A criteria set renamed failed only on a removed name: what it needs is missing.
    fn criteria_lost(&self) {
        if let Some((kind, name)) = self.removed.borrow_mut().take() {
            let at = format!(
                "{} (removed by a migration, named by its criteria)",
                self.here.borrow()
            );
            self.miss(kind, name, &at);
        }
    }

    fn holder(&self, holder: HolderId, at: &str) -> HolderId {
        match holder {
            HolderId::Region(r) => HolderId::Region(self.id(r, at)),
            other => other,
        }
    }

    /// A `time:<a>:<b>` stream's holders renumbered, the pair put back in order; any other
    /// name holds no number.
    fn stream(&self, name: StreamName) -> StreamName {
        let parts: Vec<&str> = name.0.split(':').collect();
        let holders = match parts.as_slice() {
            ["time", ka, na, kb, nb] => holder_of(ka, na).zip(holder_of(kb, nb)),
            _ => None,
        };
        match holders {
            Some((a, b)) => {
                let at = format!("rngs[{name}]");
                StreamName::time(self.holder(a, &at), self.holder(b, &at))
            }
            None => name,
        }
    }

    fn effects(&self, effects: &mut Vec<ActiveEffect>, at: &str) {
        effects.retain_mut(|effect| self.keep(&mut effect.source, at));
    }

    fn member(&self, m: &mut Character, at: &str) {
        m.race = self.id(m.race, &format!("{at}.race"));
        m.class = self.id(m.class, &format!("{at}.class"));
        m.background = self.id(m.background, &format!("{at}.background"));
        let spells = format!("{at}.known_spells");
        let known = m.known_spells.len();
        m.known_spells.retain_mut(|spell| self.keep(spell, &spells));
        let mut seen = BTreeSet::new();
        m.known_spells.retain(|spell| seen.insert(*spell));
        // A removed spell, or one replaced by a spell already known, gives its pick back;
        // nothing else is refunded (owner, 2026-10-15).
        let given = u8::try_from(known - m.known_spells.len()).unwrap_or(u8::MAX);
        m.spell_picks = m.spell_picks.saturating_add(given);
        m.legacy_auto_cast
            .retain_mut(|spell| self.keep(spell, &spells));
        self.items(&mut m.equipment, &format!("{at}.equipment"));
        let conditions = format!("{at}.conditions");
        m.conditions
            .retain_mut(|condition| self.keep(condition, &conditions));
        let equipped = format!("{at}.equipped");
        m.equipped.retain(|_, item| self.keep(item, &equipped));
        self.effects(&mut m.effects, &format!("{at}.effects"));
        let tactics = &mut m.tactics;
        for (i, set) in tactics.library.iter_mut().enumerate() {
            *self.here.borrow_mut() = format!("{at}.tactics.library[{i}]");
            match set.map(self) {
                Ok(mapped) => *set = mapped,
                Err(_) => self.criteria_lost(),
            }
        }
        for (i, book) in tactics.runbooks.iter_mut().enumerate() {
            *self.here.borrow_mut() = format!("{at}.tactics.runbooks[{i}]");
            match book.when.as_ref().map(|w| w.map(self)) {
                Some(Ok(when)) => book.when = Some(when),
                Some(Err(_)) => self.criteria_lost(),
                None => {}
            }
            // An action naming a removed spell or item leaves the runbook.
            book.entries
                .retain_mut(|(action, _)| match action.map(self) {
                    Ok(mapped) => {
                        *action = mapped;
                        true
                    }
                    Err(_) => {
                        self.removed.borrow_mut().take();
                        false
                    }
                });
        }
    }

    fn encounter(&self, e: &mut EncounterState, at: &str) {
        for stack in &mut e.stacks {
            stack.monster = self.id(stack.monster, &format!("{at}.stacks"));
        }
        e.retreat.map = self.id(e.retreat.map, &format!("{at}.retreat"));
    }

    fn mode(&self, mode: &mut Mode) {
        match mode {
            Mode::Explore => {}
            Mode::Encounter(e) => self.encounter(e, "mode.encounter"),
            Mode::Combat(c) => {
                self.encounter(&mut c.encounter, "mode.combat.encounter");
                self.items(&mut c.loot, "mode.combat.loot");
            }
            Mode::Town(t) => t.service = self.id(t.service, "mode.town.service"),
        }
    }

    /// A terrain row of the old map `old` as a row of the loaded map `new`.
    fn terrain(&self, old: MapId, new: MapId, row: u8, at: &str) -> u8 {
        let map = self.name_of(old).unwrap_or_default();
        let key = self
            .old
            .terrains
            .get(map)
            .and_then(|t| t.get(usize::from(row)));
        let found = key.and_then(|key| {
            let terrains = &self.data.maps.get(&new)?.def.terrains;
            let row = terrains.iter().position(|t| t.name == *key)?;
            u8::try_from(row).ok()
        });
        found.unwrap_or_else(|| {
            let name = key.map_or_else(|| format!("{map} #{row}"), Clone::clone);
            self.miss(NameKind::Terrain, name, at);
            row
        })
    }

    fn tiles(&self, old: MapId, new: MapId, tiles: &mut BTreeMap<(u16, u16), Known>, at: &str) {
        for known in tiles.values_mut() {
            known.terrain = self.terrain(old, new, known.terrain, at);
        }
    }

    /// A map's state: its piles' and containers' items, and its objects' spent interactions
    /// found again in their kinds by signature.
    fn map_state(&self, map: MapId, state: &mut MapState, at: &str) {
        for pile in state.ground.values_mut() {
            self.items(&mut pile.items, &format!("{at}.ground"));
        }
        for (id, object) in &mut state.objects {
            let at = format!("{at}.objects[{id}]");
            if let Some(contents) = &mut object.contents {
                self.items(contents, &at);
            }
            if !object.used.is_empty() {
                object.used = self.used(map, id, &object.used, &at);
            }
        }
    }

    fn used(&self, map: MapId, id: &str, used: &BTreeSet<u8>, at: &str) -> BTreeSet<u8> {
        let placement = self
            .data
            .maps
            .get(&map)
            .and_then(|m| m.objects.iter().find(|p| p.id == id));
        let kind = placement.map(|p| p.kind);
        let kind_name = kind.and_then(|k| self.data.registry.objects.name(k));
        let interactions = kind.and_then(|k| self.data.objects.get(&k));
        let mut out = BTreeSet::new();
        for row in used {
            let signature = kind_name
                .and_then(|n| self.old.interactions.get(n))
                .and_then(|rows| rows.get(row));
            let found = signature.zip(interactions).and_then(|(sig, kind)| {
                let row = kind.interactions.iter().position(|i| {
                    i.once
                        && i.verb == sig.verb
                        && i.label == sig.label
                        && i.when_state == sig.state
                })?;
                u8::try_from(row).ok()
            });
            match found {
                Some(new) => {
                    out.insert(new);
                }
                None => {
                    let name = format!("{} #{row}", kind_name.unwrap_or(id));
                    self.miss(NameKind::Interaction, name, at);
                    out.insert(*row);
                }
            }
        }
        out
    }
}

impl Remapper<'_> {
    /// A name in tactics: renumbered, or missing and noted; a removed one is `Err`, for the
    /// caller to drop the action or note the criteria.
    fn tactic<I: Registered>(&self, id: I) -> Result<I, String> {
        match self.found(id) {
            Found::To(new) => Ok(new),
            Found::Removed(name) => {
                *self.removed.borrow_mut() = Some((I::KIND, name.clone()));
                Err(name)
            }
            Found::Missing(name) => {
                self.miss(I::KIND, name, &self.here.borrow());
                Ok(id)
            }
        }
    }
}

impl Rename<Ids, Ids> for Remapper<'_> {
    fn spell(&self, spell: &SpellId) -> Result<SpellId, String> {
        self.tactic(*spell)
    }
    fn item(&self, item: &ItemId) -> Result<ItemId, String> {
        self.tactic(*item)
    }
    fn monster(&self, monster: &MonsterId) -> Result<MonsterId, String> {
        self.tactic(*monster)
    }
    fn condition(&self, condition: &ConditionId) -> Result<ConditionId, String> {
        self.tactic(*condition)
    }
}

/// A holder as a stream name writes it (`region:3`).
fn holder_of(kind: &str, number: &str) -> Option<HolderId> {
    let n = number.parse::<u32>().ok()?;
    Some(match kind {
        "party" => HolderId::Party(PartyId(n)),
        "region" => HolderId::Region(RegionId(n)),
        "actor" => HolderId::Actor(ActorId(n)),
        "character" => HolderId::Character(CharacterId(n)),
        "project" => HolderId::Project(ProjectId(n)),
        _ => return None,
    })
}

/// Renumber `world`, written under the names `old`, for the loaded packs. Keyed maps are built
/// anew, so two numbers that trade places never meet. The bus's region subscriptions are the
/// loaded packs' (`time::subscriptions`); its other topics are kept. Refused, every name the
/// packs lack is listed and the world is left half-renumbered: the caller drops it.
pub(crate) fn world(
    world: &mut World,
    old: &NameTable,
    data: &Data,
    chain: &Chain<'_>,
) -> Result<(), Vec<Missing>> {
    let r = Remapper {
        old,
        data,
        chain,
        missing: RefCell::new(Vec::new()),
        here: RefCell::new(String::new()),
        removed: RefCell::new(None),
    };
    world.position.map = r.id(world.position.map, "position");
    let mut maps = BTreeMap::new();
    for (map, mut state) in core::mem::take(&mut world.maps) {
        let mut new = map;
        if !r.keep(&mut new, "maps") {
            continue;
        }
        r.map_state(
            new,
            &mut state,
            &format!("maps[{}]", r.name_of(map).unwrap_or("?")),
        );
        maps.insert(new, state);
    }
    world.maps = maps;
    let automap = &mut world.automap;
    let mut tiles = BTreeMap::new();
    for (map, mut known) in core::mem::take(&mut automap.maps) {
        let mut new = map;
        if !r.keep(&mut new, "automap") {
            continue;
        }
        r.tiles(map, new, &mut known, "automap");
        tiles.insert(new, known);
    }
    automap.maps = tiles;
    let mut levels = BTreeMap::new();
    for ((map, level), mut known) in core::mem::take(&mut automap.levels) {
        let mut new = map;
        if !r.keep(&mut new, "automap.levels") {
            continue;
        }
        r.tiles(map, new, &mut known, "automap.levels");
        levels.insert((new, level), known);
    }
    automap.levels = levels;
    let known = &mut world.known;
    for (set, at) in [
        (&mut known.noticed, "known.noticed"),
        (&mut known.told, "known.told"),
        (&mut known.lore, "known.lore"),
    ] {
        *set = core::mem::take(set)
            .into_iter()
            .filter_map(|(mut map, id)| r.keep(&mut map, at).then_some((map, id)))
            .collect();
    }
    r.mode(&mut world.mode);
    world.flags = core::mem::take(&mut world.flags)
        .into_iter()
        .filter_map(|(mut flag, value)| r.keep(&mut flag, "flags").then_some((flag, value)))
        .collect();
    world.clocks = core::mem::take(&mut world.clocks)
        .into_iter()
        .map(|(holder, clock)| (r.holder(holder, "clocks"), clock))
        .collect();
    world.contacts = core::mem::take(&mut world.contacts)
        .into_iter()
        .map(|((a, b), mut contact)| {
            contact.other = r.holder(contact.other, "contacts");
            ((r.holder(a, "contacts"), r.holder(b, "contacts")), contact)
        })
        .collect();
    world.rngs = core::mem::take(&mut world.rngs)
        .into_iter()
        .map(|(name, stream)| (r.stream(name), stream))
        .collect();
    world.bus.retain(|topic| !matches!(topic, Topic::Region(_)));
    for region in data.regions.keys() {
        world
            .bus
            .subscribe(Topic::Region(*region), crate::bus::Subscriber::Reconcile);
    }
    let party = &mut world.party;
    r.items(&mut party.inventory, "party.inventory");
    r.effects(&mut party.effects, "party.effects");
    for (i, member) in party.members.iter_mut().enumerate() {
        r.member(member, &format!("party.members[{i}]"));
    }
    let missing = r.missing.into_inner();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

/// The map a log line was said on, renumbered; a map the packs no longer have becomes a number
/// no loaded map has, so the line is kept and retold with its objects unnamed (owner,
/// 2026-10-18).
pub(crate) fn log(log: &mut [Logged], old: &NameTable, data: &Data, chain: &Chain<'_>) {
    for line in log {
        let name = usize::try_from(line.map.0)
            .ok()
            .and_then(|i| old.maps.get(i))
            .and_then(|n| match chain.lead(Kind::Map, n) {
                Some(Target::To(to)) => Some(to),
                Some(Target::Removed) => None,
                None => Some(n),
            });
        line.map = name
            .and_then(|n| data.registry.maps.get(n))
            .unwrap_or(MapId(u32::MAX));
    }
}
