//! Placements as palette stamps (the owner's drawing-GUI column, step 3, 2026-10-25): the
//! palette holds an entry per placement kind (`forms::Placing`), each with one detail chosen
//! under the palette (the map a portal leads to, an encounter's monster, an activity's service,
//! an object's kind, a pile's item, a lore's text key). A click with the brush places one at once
//! through the kind's own form (`Form::placing`, `Form::request`): valid as it lands, refused
//! where its kind already is (`Form::keys`), one undo step; the inspector's Edit opens the form
//! for the rest. A right click on a placement picks its stamp and detail. Bevy-free.

use super::brush::Entry;
use super::form::{At, Form};
use super::forms::{self, Found, Placing};
use super::{Editor, Here};
use omnis_sim::omnis_data::{Data, MapDef};

/// The detail held for each stamp kind, by `Placing::index`; none holds the first option.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stamps([Option<String>; 6]);

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

impl Stamps {
    /// The options of `kind` and where the held detail stands among them (the first when none
    /// is held, or the held one is gone from the packs).
    #[must_use]
    pub fn chosen(&self, kind: Placing, data: &Data) -> (Vec<String>, usize) {
        let options = options(kind, data);
        let at = self.0[kind.index()]
            .as_deref()
            .and_then(|held| options.iter().position(|o| o == held))
            .unwrap_or(0);
        (options, at)
    }

    /// The detail a stamp of `kind` places: the held one, else the first; none when the packs
    /// have none of the kind.
    #[must_use]
    pub fn detail(&self, kind: Placing, data: &Data) -> Option<String> {
        let (options, at) = self.chosen(kind, data);
        options.get(at).cloned()
    }

    /// Step the detail of `kind` by `by` through its options, round.
    pub fn step(&mut self, kind: Placing, data: &Data, by: i8) {
        let (options, at) = self.chosen(kind, data);
        let Ok(n) = i64::try_from(options.len()) else {
            return;
        };
        if n == 0 {
            return;
        }
        let next = (i64::try_from(at).unwrap_or(0) + i64::from(by)).rem_euclid(n);
        self.0[kind.index()] = usize::try_from(next)
            .ok()
            .and_then(|i| options.get(i))
            .cloned();
    }

    /// Hold `id` as the detail of `kind` (a pick).
    pub fn set(&mut self, kind: Placing, id: String) {
        self.0[kind.index()] = Some(id);
    }
}

/// The stamp a listed placement is, with the detail it holds; none for a rest event or for one
/// the map no longer has.
#[must_use]
pub fn of_here(def: &MapDef, here: &Here) -> Option<(Placing, Option<String>)> {
    Some(match forms::find(def, &here.key)? {
        Found::Portal(p) => (Placing::Portal, Some(p.to_map.clone())),
        Found::Encounter(e) => (
            Placing::Encounter,
            e.stacks.first().map(|(id, _)| id.clone()),
        ),
        Found::Site(s) => (Placing::Activity, Some(s.service.clone())),
        Found::Object(o) => (Placing::Object, Some(o.kind.clone())),
        Found::Ground(g) => (Placing::Ground, g.items.first().map(|(id, _)| id.clone())),
        Found::Lore(l) => (Placing::Lore, Some(l.text.clone())),
        Found::RestEvent(_) => return None,
    })
}

impl Editor {
    /// Place the palette's stamp on `cell` of the shown level, one undo step, and put the
    /// cursor there: the kind's form, filled with the detail held, is requested and made as
    /// Apply would make it. A refusal (the cell already holds the kind, an id is taken, the
    /// packs have nothing of the detail's kind) is said and changes nothing; what landed is
    /// said, with the loader's first word on the map as it now stands. Nothing unless the entry
    /// is a stamp. Whether the map changed.
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
        let Some(detail) = self.stamps.detail(kind, data) else {
            self.message = Some(format!(
                "nothing to stamp: the packs have no {}",
                kind.what()
            ));
            return false;
        };
        let form = Form::placing(kind, &def, data, at, Some(&detail));
        match form.request(&def) {
            Ok(change) => {
                let before = self.here_at(cell);
                if !self.change(data, change) {
                    return false;
                }
                let label = self
                    .here_at(cell)
                    .into_iter()
                    .find(|h| !before.contains(h))
                    .map_or_else(|| kind.label().to_owned(), |h| h.label);
                self.report(format!("{label} placed at ({x}, {y})"));
                true
            }
            Err(why) => {
                self.message = Some(why);
                false
            }
        }
    }
}
