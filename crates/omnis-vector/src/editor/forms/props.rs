//! The stamps' own forms (the editor's column, step 3b, owner 2026-10-31): a form filled from a
//! held place as a new placement at a cell (`Form::for_place`), what a click drops and what the
//! properties modal edits; and that form's rows as the modal shows them (`Form::props_rows`),
//! without the fields a click decides (the id, x and y) or the kind the picker chose, a list's
//! rows read as the chosen thing beside its count.

use super::super::form::{Form, FormKind, Slot};
use super::{Placing, encounter, ground, id_taken, lore, object, portal, site};
use crate::editor::EditAct;
use crate::editor::form::At;
use crate::editor::forms::fields::free_id;
use crate::screen::{ControlId, Row, Span};
use omnis_sim::omnis_data::edit::Place;
use omnis_sim::omnis_data::{Data, MapDef};

/// The fields a stamp's click decides, or the picker chose: not the properties modal's.
const DECIDED: [&str; 4] = ["id", "x", "y", "kind"];

impl Form {
    /// A form filled from `place` as a new placement at `at`: its values `place`'s, its cell
    /// `at`, its id (for a kind keyed by one) minted free of the map's and of `minted`, the ids
    /// dropped beside it. Nothing to remove, so its keys are checked as a new placement's. None
    /// for a start, the random table, a rest event or a removal.
    #[must_use]
    pub fn for_place(
        place: &Place,
        def: &MapDef,
        data: &Data,
        at: At,
        minted: &[String],
    ) -> Option<Form> {
        let fresh = |kind: Placing, word: &str| {
            free_id(word, (at.x, at.y), |id| {
                id_taken(def, kind, id) || minted.iter().any(|m| m == id)
            })
        };
        Some(match place {
            Place::Portal(p) => portal(def, data, at, Some(p), None, true),
            Place::Encounter(e) => {
                let mut e = e.clone();
                e.id = fresh(Placing::Encounter, "encounter");
                encounter(def, data, at, Some(&e), None)
            }
            Place::Site(s) => site(data, at, Some(s), None),
            Place::Object(o) => {
                let mut o = o.clone();
                o.id = fresh(Placing::Object, "object");
                object(def, data, at, Some(&o), None)
            }
            Place::Ground(g) => ground(data, at, Some(g), None),
            Place::Lore(l) => {
                let mut l = l.clone();
                l.id = fresh(Placing::Lore, "lore");
                lore(def, data, at, Some(&l), None)
            }
            Place::Start { .. } | Place::Random(_) | Place::RestEvent(_) | Place::Remove(_) => {
                return None;
            }
        })
    }

    /// The form's rows as the properties modal shows form `n`: an object's kind as its heading,
    /// the form's own fields but the decided ones, each under `EditAct::Prop(n, i)`, then each
    /// list's rows as the thing's id beside its other fields (no Add row or Remove: the picker
    /// decides what is in the list).
    #[must_use]
    pub fn props_rows(&self, n: u8) -> Vec<Row> {
        let id = |i: usize| ControlId::Edit(EditAct::Prop(n, u8::try_from(i).unwrap_or(u8::MAX)));
        let mut rows = Vec::new();
        if self.kind == FormKind::Object {
            rows.push(Row::text(Span::body(self.choice_of("kind"))));
        }
        for (i, f) in self.fields.iter().enumerate() {
            if f.slot == Slot::Top && !DECIDED.contains(&f.label) {
                rows.push(Row::controls(vec![f.control_as(id(i))]));
            }
        }
        for l in 0..self.lists.len() {
            let l = u8::try_from(l).unwrap_or(u8::MAX);
            for row in self.rows_of(l) {
                let Some((first, rest)) = row.split_first() else {
                    continue;
                };
                rows.push(Row {
                    spans: vec![Span::body(self.text(*first))],
                    controls: rest
                        .iter()
                        .map(|i| self.fields[*i].control_as(id(*i)))
                        .collect(),
                    ..Row::default()
                });
            }
        }
        rows
    }
}
