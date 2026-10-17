//! What a client reads of objects (M8.5 step 2a). `objects.get` lists the objects the party
//! reaches, each with every interaction its kind offers in the state it is in now and, when
//! `Act` would refuse one, why: the same checks the command makes (`objects::admit`), each on
//! its own copy of the `object` stream, so looking draws nothing. It also lists the pile the
//! party stands on, which `PickUp` takes from. The viewport carries what is in sight: objects
//! and piles on the visible tiles, to be drawn.

use crate::INTERACT_MINUTES;
use crate::beings::{self, BeingView};
use crate::combat::Roller;
use crate::command::Rejection;
use crate::event::CheckKind;
use crate::ground::pile_at;
use crate::names::id_of;
use crate::objects::{
    admit, check_of, contents, found_in, offered, reachable, removed_in, state_in, state_of,
};
use crate::world::{MapState, Mode, World};
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Facing, ItemId, Level, MapId};
use omnis_data::placement::{ResolvedPlacement, in_state};
use omnis_data::{Data, Look, ObjectKind, Verb};
use serde::{Deserialize, Serialize};

/// A count of one item, named by its string id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCount {
    /// The item's id.
    pub item: String,
    /// How many.
    pub count: u16,
}

/// A check an interaction rolls, by the member best at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckView {
    /// The skill or ability.
    pub kind: CheckKind,
    /// The difficulty to meet.
    pub dc: i64,
}

/// One interaction an object offers now: the `verb` and `option` that `Act` takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbView {
    /// The verb.
    pub verb: Verb,
    /// Which of the verb's offered interactions, counted from 0.
    pub option: u8,
    /// Text key naming this way of doing it, when the kind gives one.
    pub label: Option<String>,
    /// The checks it rolls, in order; the first to fail ends the attempt.
    pub checks: Vec<CheckView>,
    /// Minutes an attempt takes, success or not.
    pub minutes: u32,
    /// Why `Act` would refuse it now, if it would.
    pub refusal: Option<Rejection>,
}

/// One object the party reaches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectView {
    /// The placement's id, the `object` that `Act` takes.
    pub object: String,
    /// Its kind's id.
    pub object_kind: String,
    /// Text key of its name: the placement's own, else the kind's.
    pub name: String,
    /// The state it is in; empty when its kind has none.
    pub state: String,
    /// Kinds of item it holds at most; 0 when it is not a container.
    pub capacity: u16,
    /// A container's items when it is open; `None` when shut or not a container.
    pub contents: Option<Vec<ItemCount>>,
    /// Every interaction offered in its state, a spent one with its refusal.
    pub verbs: Vec<VerbView>,
}

/// What `objects.get` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectsView {
    /// The objects in reach, in the order Use reaches them: the party's cell, then the one it
    /// faces across an open edge.
    pub objects: Vec<ObjectView>,
    /// The pile on the party's cell, what `PickUp` takes from.
    pub ground: Vec<ItemCount>,
    /// The groups standing in reach, which `Approach` meets (M8.5 step 2c).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beings: Vec<BeingView>,
}

/// An object in sight, as the viewport carries it to be drawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeenObject {
    /// The placement's id.
    pub object: String,
    /// Its kind's id.
    pub object_kind: String,
    /// Its home cell.
    pub x: u16,
    /// Its home cell.
    pub y: u16,
    /// Offset from the home cell's centre, eighths east and south.
    pub pos: (i8, i8),
    /// The way it faces; its look's width runs across it.
    pub facing: Facing,
    /// The state it is in; empty when its kind has none.
    pub state: String,
    /// How it looks in that state.
    pub look: Look,
    /// Whether it blocks a step in that state; a client draws a shut arch closed.
    pub blocks: bool,
}

/// A pile in sight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PileView {
    /// Its cell.
    pub x: u16,
    /// Its cell.
    pub y: u16,
    /// Offset from the cell's centre, eighths east and south.
    pub pos: (i8, i8),
    /// Its items, in the order they were laid down.
    pub items: Vec<ItemCount>,
}

/// The objects the party reaches and the pile it stands on.
#[must_use]
pub fn objects_view(world: &World, data: &Data) -> ObjectsView {
    let objects = reachable(world, data)
        .filter_map(|p| Some((p, data.objects.get(&p.kind)?)))
        .map(|(p, kind)| object(world, data, p, kind))
        .collect();
    let pos = world.position;
    let ground = pile_at(world, data, pos.map, (pos.level, pos.x, pos.y))
        .map(|pile| counted(data, &pile.items))
        .unwrap_or_default();
    ObjectsView {
        objects,
        ground,
        beings: beings::reached(world, data),
    }
}

fn object(world: &World, data: &Data, p: &ResolvedPlacement, kind: &ObjectKind) -> ObjectView {
    let state = state_of(world, world.position.map, p);
    let open = kind.states.is_empty() || kind.states.get(state).is_some_and(|s| s.open);
    let capacity = kind.attributes.capacity;
    let mut verbs: Vec<VerbView> = Vec::new();
    for (index, interaction) in offered(kind, state) {
        let earlier = verbs.iter().filter(|v| v.verb == interaction.verb).count();
        let refusal = if matches!(world.mode, Mode::Explore) {
            let mut roller = Roller::take_stream(world, "object");
            admit(world, data, p, index, interaction, &mut roller).err()
        } else {
            Some(Rejection::WrongMode)
        };
        verbs.push(VerbView {
            verb: interaction.verb,
            option: u8::try_from(earlier).unwrap_or(u8::MAX),
            label: interaction.label.clone(),
            checks: interaction
                .requires
                .iter()
                .filter_map(check_of)
                .map(|(_, kind, dc)| CheckView { kind, dc })
                .collect(),
            minutes: interaction.minutes.unwrap_or(INTERACT_MINUTES),
            refusal,
        });
    }
    ObjectView {
        object: p.id.clone(),
        object_kind: id_of(&data.registry.objects, p.kind),
        name: p
            .name
            .map_or_else(|| kind.name.clone(), |key| id_of(&data.registry.text, key)),
        state: String::from(state),
        capacity,
        contents: (capacity > 0 && open).then(|| counted(data, contents(world, p))),
        verbs,
    }
}

/// The objects and piles on the cells in sight, `tiles` nearest first, on the party's level:
/// an object once, though its footprint spans several of them; hidden ones only when found.
pub(crate) fn in_sight(
    world: &World,
    data: &Data,
    tiles: &[(u16, u16)],
) -> (Vec<SeenObject>, Vec<PileView>) {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return (Vec::new(), Vec::new());
    };
    let state = world.maps.get(&pos.map);
    let mut seen = BTreeSet::new();
    let objects = tiles
        .iter()
        .flat_map(|&(x, y)| map.objects_at(pos.level, x, y))
        .filter(|(_, p)| seen.insert(p.id.as_str()))
        .filter_map(|(_, p)| drawn(state, data, p))
        .collect();
    let piles = tiles
        .iter()
        .filter_map(|&(x, y)| pile(world, data, pos.map, (pos.level, x, y)))
        .collect();
    (objects, piles)
}

/// Every object and pile on `level` of `map` as the party would see them, wherever it stands:
/// for a client that draws the whole level, not only what is in sight. Removed objects are
/// left out, and hidden ones until found, as in the viewport. Objects in file order, piles
/// north to south, then west to east.
#[must_use]
pub fn on_level(
    world: &World,
    data: &Data,
    map: MapId,
    level: Level,
) -> (Vec<SeenObject>, Vec<PileView>) {
    let Some(def) = data.maps.get(&map) else {
        return (Vec::new(), Vec::new());
    };
    let state = world.maps.get(&map);
    let objects = def
        .objects
        .iter()
        .filter(|p| p.level == level)
        .filter_map(|p| drawn(state, data, p))
        .collect();
    let saved = state.into_iter().flat_map(|s| s.ground.keys());
    let cells: BTreeSet<(u16, u16)> = def
        .ground
        .keys()
        .chain(saved)
        .filter(|&&(l, _, _)| l == level)
        .map(|&(_, x, y)| (y, x))
        .collect();
    let piles = cells
        .into_iter()
        .filter_map(|(y, x)| pile(world, data, map, (level, x, y)))
        .collect();
    (objects, piles)
}

/// A placement as drawn, unless removed or still hidden.
fn drawn(state: Option<&MapState>, data: &Data, p: &ResolvedPlacement) -> Option<SeenObject> {
    if removed_in(state, p) || !found_in(state, p) {
        return None;
    }
    let kind = data.objects.get(&p.kind)?;
    let now = state_in(state, p);
    let (look, blocks, _) = in_state(kind, now);
    Some(SeenObject {
        object: p.id.clone(),
        object_kind: id_of(&data.registry.objects, p.kind),
        x: p.x,
        y: p.y,
        pos: p.pos,
        facing: p.facing,
        state: String::from(now),
        look: *look,
        blocks,
    })
}

fn pile(world: &World, data: &Data, map: MapId, cell: (Level, u16, u16)) -> Option<PileView> {
    let pile = pile_at(world, data, map, cell)?;
    Some(PileView {
        x: cell.1,
        y: cell.2,
        pos: pile.pos,
        items: counted(data, &pile.items),
    })
}

fn counted(data: &Data, items: &[(ItemId, u16)]) -> Vec<ItemCount> {
    items
        .iter()
        .map(|&(item, count)| ItemCount {
            item: id_of(&data.registry.items, item),
            count,
        })
        .collect()
}
