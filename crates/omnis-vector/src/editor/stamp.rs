//! Placements as palette stamps (the owner's drawing-GUI column, step 3, 2026-10-25; step 3b,
//! 2026-10-31): the palette holds an entry per placement kind (`forms::Placing`). Each stamp
//! holds what it drops as the data model's own places (`Place`), chosen in the picker modal
//! (`pick`) and given their properties there: an encounter with its stacks, disposition and
//! stand; a pile with its items and counts; an object a chosen kind, where it stands on the
//! cell; a lore a chosen text; a portal; a site. A click with the brush drops them on the cell
//! through the kinds' own forms (`Form::for_place`, `Form::request`): valid as they land, ids
//! minted apart, refused where a tile-keyed kind already is, one undo step. A right click on a
//! placement copies it whole into its stamp. Bevy-free.

use super::brush::Entry;
use super::form::{At, Change, Form};
use super::forms::{self, Found, Placing};
use super::{Editor, Here};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::{Place, Target};
use omnis_sim::omnis_data::encounter::FixedEncounter;
use omnis_sim::omnis_data::limits::{
    MAX_LORE_PER_CELL, MAX_OBJECTS_PER_CELL, MAX_PILE_STACKS, MAX_STACKS,
};
use omnis_sim::omnis_data::lore::CellLore;
use omnis_sim::omnis_data::placement::{GroundDef, Placement};
use omnis_sim::omnis_data::{Data, MapDef};

/// What each stamp drops, by `Placing::index`: the places as the model holds them, at a nominal
/// cell with nominal ids (the click decides both); empty until chosen, when the stamp drops the
/// packs' first of its kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stamps([Vec<Place>; 6]);

/// The nominal cell a held place stands at.
const NOWHERE: At = At {
    level: Level::GROUND,
    x: 0,
    y: 0,
    facing: Facing::North,
};

/// What a kind's detail is chosen among: the loaded packs' ids of that kind, sorted.
#[must_use]
pub fn options(kind: Placing, data: &Data) -> Vec<String> {
    let mut out: Vec<String> = match kind {
        Placing::Portal => data.maps.values().map(|m| m.def.id.clone()).collect(),
        Placing::Encounter => data.monsters.values().map(|m| m.id.clone()).collect(),
        Placing::Activity => data.services.values().map(|s| s.id.clone()).collect(),
        Placing::Object => data.objects.values().map(|o| o.id.clone()).collect(),
        Placing::Ground => data.items.values().map(|i| i.id.clone()).collect(),
        Placing::Lore => data.registry.text.names().map(str::to_owned).collect(),
    };
    out.sort();
    out.dedup();
    out
}

/// The most a stamp of `kind` holds: what one cell takes (one portal or site; the loader's
/// stacks, objects, pile stacks and lore per cell).
#[must_use]
pub const fn limit(kind: Placing) -> usize {
    match kind {
        Placing::Portal | Placing::Activity => 1,
        Placing::Encounter => MAX_STACKS,
        Placing::Object => MAX_OBJECTS_PER_CELL,
        Placing::Ground => MAX_PILE_STACKS,
        Placing::Lore => MAX_LORE_PER_CELL,
    }
}

/// The details `places` hold with their counts: a portal's map, the stacks' monsters, a site's
/// service, the objects' kinds, the items, the lore texts; one of each but a stack or an item.
#[must_use]
pub fn details(places: &[Place]) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for place in places {
        match place {
            Place::Portal(p) => out.push((p.to_map.clone(), 1)),
            Place::Encounter(e) => {
                out.extend(e.stacks.iter().map(|(id, n)| (id.clone(), u32::from(*n))));
            }
            Place::Site(s) => out.push((s.service.clone(), 1)),
            Place::Object(o) => out.push((o.kind.clone(), 1)),
            Place::Ground(g) => {
                out.extend(g.items.iter().map(|(id, n)| (id.clone(), u32::from(*n))));
            }
            Place::Lore(l) => out.push((l.text.clone(), 1)),
            Place::Start { .. } | Place::Random(_) | Place::RestEvent(_) | Place::Remove(_) => {}
        }
    }
    out
}

/// An id's last segment: `test:monster:goblin` reads `goblin` where room is short.
#[must_use]
pub fn short(id: &str) -> &str {
    id.rsplit(':').next().unwrap_or(id)
}

impl Stamps {
    /// What the stamp of `kind` holds.
    #[must_use]
    pub fn held(&self, kind: Placing) -> &[Place] {
        &self.0[kind.index()]
    }

    /// Hold `places` as what the stamp of `kind` drops.
    pub fn set(&mut self, kind: Placing, places: Vec<Place>) {
        self.0[kind.index()] = places;
    }

    /// The details the stamp of `kind` holds, in order.
    #[must_use]
    pub fn ids(&self, kind: Placing) -> Vec<String> {
        details(self.held(kind))
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }

    /// What the stamp drops, in a line: the details' last segments, "x N" after a count above
    /// one; the packs' first of the kind while nothing is chosen, said so.
    #[must_use]
    pub fn summary(&self, kind: Placing, data: &Data) -> String {
        let held = details(self.held(kind));
        if held.is_empty() {
            return match options(kind, data).first() {
                Some(first) => format!("nothing chosen, the first: {}", short(first)),
                None => format!("nothing chosen: the packs have no {}", kind.what()),
            };
        }
        held.iter()
            .map(|(id, n)| {
                if *n > 1 {
                    format!("{} x{n}", short(id))
                } else {
                    short(id).to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The places a stamp of `kind` holds for `chosen`, carrying what `prev` held for a detail kept
/// (a stack's count, the group's disposition, once and stand; an object's place on the cell,
/// state, hidden DC, name, contents and lore; an item's count; the pile's place; a lore's DC and
/// once). A portal and a site come through their forms (the landing at the map's start and the
/// tileset's marker are the form's), so a map whose tileset has no marker holds none.
#[must_use]
pub fn rebuild(
    kind: Placing,
    chosen: &[String],
    prev: &[Place],
    def: &MapDef,
    data: &Data,
) -> Vec<Place> {
    let one =
        |kind: Placing, id: &str| match Form::placing(kind, def, data, NOWHERE, Some(id)).value() {
            Ok(Change::Place(place)) => Some(place),
            _ => None,
        };
    match kind {
        Placing::Portal | Placing::Activity => chosen
            .first()
            .and_then(|id| one(kind, id))
            .into_iter()
            .collect(),
        Placing::Encounter => {
            if chosen.is_empty() {
                return Vec::new();
            }
            let was = prev.iter().find_map(|p| match p {
                Place::Encounter(e) => Some(e),
                _ => None,
            });
            let count = |id: &str| {
                was.and_then(|e| e.stacks.iter().find(|(m, _)| m == id))
                    .map_or(1, |(_, n)| *n)
            };
            vec![Place::Encounter(FixedEncounter {
                id: "encounter".to_owned(),
                level: NOWHERE.level,
                x: NOWHERE.x,
                y: NOWHERE.y,
                stacks: chosen.iter().map(|id| (id.clone(), count(id))).collect(),
                disposition: was.map(|e| e.disposition).unwrap_or_default(),
                once: was.is_some_and(|e| e.once),
                stand: was.and_then(|e| e.stand.clone()),
            })]
        }
        Placing::Object => chosen
            .iter()
            .map(|kind| {
                prev.iter()
                    .find_map(|p| match p {
                        Place::Object(o) if o.kind == *kind => Some(p.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        Place::Object(Placement {
                            id: "object".to_owned(),
                            kind: kind.clone(),
                            level: NOWHERE.level,
                            x: NOWHERE.x,
                            y: NOWHERE.y,
                            pos: (0, 0),
                            facing: Facing::North,
                            state: None,
                            hidden: None,
                            name: None,
                            contents: Vec::new(),
                            lore: None,
                        })
                    })
            })
            .collect(),
        Placing::Ground => {
            if chosen.is_empty() {
                return Vec::new();
            }
            let was = prev.iter().find_map(|p| match p {
                Place::Ground(g) => Some(g),
                _ => None,
            });
            let count = |id: &str| {
                was.and_then(|g| g.items.iter().find(|(i, _)| i == id))
                    .map_or(1, |(_, n)| *n)
            };
            vec![Place::Ground(GroundDef {
                level: NOWHERE.level,
                x: NOWHERE.x,
                y: NOWHERE.y,
                pos: was.map_or((0, 0), |g| g.pos),
                items: chosen.iter().map(|id| (id.clone(), count(id))).collect(),
            })]
        }
        Placing::Lore => chosen
            .iter()
            .map(|text| {
                prev.iter()
                    .find_map(|p| match p {
                        Place::Lore(l) if l.text == *text => Some(p.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        Place::Lore(CellLore {
                            id: "lore".to_owned(),
                            level: NOWHERE.level,
                            x: NOWHERE.x,
                            y: NOWHERE.y,
                            text: text.clone(),
                            dc: None,
                            once: false,
                        })
                    })
            })
            .collect(),
    }
}

/// The stamp a listed placement is, holding that placement whole (every object or lore on its
/// cell when it is one of those); none for a rest event or for one the map no longer has.
#[must_use]
pub fn of_here(def: &MapDef, here: &Here) -> Option<(Placing, Vec<Place>)> {
    let on = |level: Level, x: u16, y: u16| {
        move |l: Level, px: u16, py: u16| (l, px, py) == (level, x, y)
    };
    Some(match forms::find(def, &here.key)? {
        Found::Portal(p) => (Placing::Portal, vec![Place::Portal(p.clone())]),
        Found::Encounter(e) => (Placing::Encounter, vec![Place::Encounter(e.clone())]),
        Found::Site(s) => (Placing::Activity, vec![Place::Site(s.clone())]),
        Found::Object(o) => {
            let here = on(o.level, o.x, o.y);
            (
                Placing::Object,
                def.objects
                    .iter()
                    .filter(|x| here(x.level, x.x, x.y))
                    .cloned()
                    .map(Place::Object)
                    .collect(),
            )
        }
        Found::Ground(g) => (Placing::Ground, vec![Place::Ground(g.clone())]),
        Found::Lore(l) => {
            let here = on(l.level, l.x, l.y);
            (
                Placing::Lore,
                def.lore
                    .iter()
                    .filter(|x| here(x.level, x.x, x.y))
                    .cloned()
                    .map(Place::Lore)
                    .collect(),
            )
        }
        Found::RestEvent(_) => return None,
    })
}

/// The id a place is keyed by, when its kind is (an encounter, an object, a lore).
fn id_of(place: &Place) -> Option<String> {
    match place.key()? {
        Target::Encounter { id } | Target::Object { id } | Target::Lore { id } => Some(id),
        _ => None,
    }
}

impl Editor {
    /// What the stamp of `kind` drops: what it holds, else the packs' first of its kind.
    #[must_use]
    pub fn stamp_places(&self, kind: Placing, data: &Data) -> Vec<Place> {
        let held = self.stamps.held(kind);
        if !held.is_empty() {
            return held.to_vec();
        }
        let Some(def) = self.def() else {
            return Vec::new();
        };
        let first: Vec<String> = options(kind, data).into_iter().take(1).collect();
        rebuild(kind, &first, &[], &def, data)
    }

    /// Drop the palette's stamp on `cell` of the shown level, one undo step, and put the cursor
    /// there: each held place is filled into the kind's form at the cell (a fresh id, minted
    /// apart from the map's and the others dropped with it), requested as Apply would, and all
    /// are placed together. A refusal (the cell already holds a tile-keyed kind or an encounter,
    /// the packs have nothing of the kind) is said and changes nothing; what landed is said,
    /// with the loader's first word on the map as it now stands. Nothing unless the entry is a
    /// stamp. Whether the map changed.
    pub fn stamp(&mut self, data: &Data, cell: (u16, u16)) -> bool {
        let (Some(def), Entry::Stamp(kind)) = (self.def(), self.entry) else {
            return false;
        };
        let (x, y) = cell;
        let at = At {
            level: self.level,
            x,
            y,
            facing: self.facing,
        };
        self.point(cell);
        let held = self.stamp_places(kind, data);
        if held.is_empty() {
            self.message = Some(format!(
                "nothing to stamp: the packs have no {}",
                kind.what()
            ));
            return false;
        }
        let mut minted: Vec<String> = Vec::new();
        let mut places = Vec::new();
        for place in &held {
            let Some(form) = Form::for_place(place, &def, data, at, &minted) else {
                continue;
            };
            match form.request(&def) {
                Ok(Change::Place(place)) => {
                    minted.extend(id_of(&place));
                    places.push(place);
                }
                Ok(Change::Edit(_) | Change::Note { .. }) => {}
                Err(why) => {
                    self.message = Some(why);
                    return false;
                }
            }
        }
        let before = self.here_at(cell);
        let result = self
            .session
            .document_mut()
            .and_then(|d| d.place_many(places));
        if !self.took(data, result) {
            return false;
        }
        let landed: Vec<String> = self
            .here_at(cell)
            .into_iter()
            .filter(|h| !before.contains(h))
            .map(|h| h.label)
            .collect();
        let label = if landed.is_empty() {
            kind.label().to_owned()
        } else {
            landed.join(", ")
        };
        self.report(format!("{label} placed at ({x}, {y})"));
        true
    }
}
