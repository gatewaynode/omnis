//! A stamp's detail chosen in a modal (the owner's drawing-GUI column, step 3b, 2026-10-31):
//! the picker lists what the loaded packs have of the stamp's kind, searched by a text field and
//! chosen several at once up to what one cell takes (`stamp::limit`); Finish holds the choice in
//! the stamp (`stamp::rebuild`) and, for a kind with properties (an encounter, an object, a
//! pile), opens the properties modal: the kind's own form a held place, without the fields a
//! click decides; its Finish reads the forms back into the stamp. Cancel on either closes it and
//! keeps what the stamp held; the state here is dropped unread. The portal picker (the entrances
//! and exits enhancement, W1 step 2b) lists the portals nothing links to, the open map's unsaved
//! ones among them, each as "id  -  map (x, y)" (`options`, `fits`: the list under a portal
//! form's link field shares them, `links`). Bevy-free: the screens in `screen/editor_pick.rs`
//! and `screen/editor_props.rs` draw it.

use super::brush::Entry;
use super::form::{Change, Form};
use super::forms::Placing;
use super::stamp::{self, Stamps};
use super::{Asked, EditAct, Editor};
use omnis_sim::omnis_core::Level;
use omnis_sim::omnis_data::{Data, MapDef, unlinked_portals};

/// A stamp's detail being chosen: the picker's filter and choices, then the properties forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picking {
    /// The stamp's kind.
    pub kind: Placing,
    /// What may be chosen, by id, in the order listed.
    pub options: Vec<String>,
    /// What each option's row shows (the id alone for most kinds; a portal's with its map and
    /// cell).
    pub labels: Vec<String>,
    /// The search field's text: an option is listed while its label holds it, in any case.
    pub filter: String,
    /// The ids chosen, in the order chosen.
    pub chosen: Vec<String>,
    /// The properties modal's forms, one a held place, while it is open.
    pub props: Vec<Form>,
}

impl Picking {
    /// The picker opened for `kind` over `options` (id and label each): its choices as the
    /// stamp holds them.
    #[must_use]
    pub fn open(kind: Placing, stamps: &Stamps, options: Vec<(String, String)>) -> Picking {
        Picking::new(kind, options, stamps.ids(kind))
    }

    fn new(kind: Placing, options: Vec<(String, String)>, chosen: Vec<String>) -> Picking {
        let (options, labels) = options.into_iter().unzip();
        Picking {
            kind,
            options,
            labels,
            filter: String::new(),
            chosen,
            props: Vec::new(),
        }
    }

    /// The positions among the options the filter leaves: every one on an empty filter, else
    /// those whose label holds the filter's text in any case.
    #[must_use]
    pub fn shown(&self) -> Vec<usize> {
        self.labels
            .iter()
            .enumerate()
            .filter(|(_, l)| fits(l, &self.filter))
            .map(|(i, _)| i)
            .collect()
    }

    /// Whether `id` is chosen.
    #[must_use]
    pub fn is_chosen(&self, id: &str) -> bool {
        self.chosen.iter().any(|c| c == id)
    }

    /// Choose `id`, or let it go when it is chosen. A kind that takes one moves its choice; at
    /// the kind's limit another is refused.
    ///
    /// # Errors
    /// The limit, in a line.
    pub fn toggle(&mut self, id: &str) -> Result<(), String> {
        if let Some(i) = self.chosen.iter().position(|c| c == id) {
            self.chosen.remove(i);
            return Ok(());
        }
        let limit = stamp::limit(self.kind);
        if limit == 1 {
            self.chosen.clear();
        } else if self.chosen.len() >= limit {
            return Err(format!("at most {limit} {}s", self.kind.what()));
        }
        self.chosen.push(id.to_owned());
        Ok(())
    }

    /// Whether a stamp of `kind` has properties the second modal sets: an encounter's counts,
    /// disposition and stand; an object's place on the cell; a pile's counts and place.
    #[must_use]
    pub const fn has_props(kind: Placing) -> bool {
        matches!(kind, Placing::Encounter | Placing::Object | Placing::Ground)
    }
}

/// What the picker of `kind` offers (id and label each): the packs' ids of the kind; for a
/// portal, every portal nothing links to among the loaded maps with `def` standing in for its
/// file (its unsaved portals among them), but the one at `except`, each labelled with its map's
/// name and cell.
#[must_use]
pub fn options(
    kind: Placing,
    def: &MapDef,
    data: &Data,
    except: Option<(Level, u16, u16)>,
) -> Vec<(String, String)> {
    if kind != Placing::Portal {
        return stamp::options(kind, data)
            .into_iter()
            .map(|id| (id.clone(), id))
            .collect();
    }
    unlinked_portals(def, data)
        .into_iter()
        .filter(|(_, p)| except != Some((p.level, p.x, p.y)))
        .map(|(map, p)| (p.id.clone(), portal_label(data, map, p)))
        .collect()
}

/// Whether a row's `label` holds `needle` in any case; every label holds an empty one (trimmed).
#[must_use]
pub fn fits(label: &str, needle: &str) -> bool {
    let needle = needle.trim().to_lowercase();
    needle.is_empty() || label.to_lowercase().contains(&needle)
}

/// A portal's row in the picker: "id  -  map (x, y)".
pub(super) fn portal_label(data: &Data, map: &str, p: &omnis_sim::omnis_data::Portal) -> String {
    format!(
        "{}  -  {} ({}, {})",
        p.id,
        crate::labels::map(data, map),
        p.x,
        p.y
    )
}

impl Editor {
    /// The chosen stamp's kind, if the entry is a stamp.
    fn stamp_kind(&self) -> Option<Placing> {
        match self.entry {
            Entry::Stamp(kind) => Some(kind),
            _ => None,
        }
    }

    /// Open the picker for the chosen stamp (`EditAct::Pick`): its choices start as the stamp
    /// holds them.
    pub(super) fn pick_open(&mut self, data: &Data) -> Asked {
        let Some((kind, def)) = self.stamp_kind().zip(self.def()) else {
            return Asked::Nothing;
        };
        let options = options(kind, &def, data, None);
        self.picking = Some(Picking::open(kind, &self.stamps, options));
        self.message = None;
        Asked::PickDetail
    }

    /// The picker's search field was typed into.
    pub fn pick_filter(&mut self, text: String) {
        if let Some(p) = &mut self.picking {
            p.filter = text;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// The picker's row `i` (a position among its options) was pressed: chosen or let go.
    pub(super) fn pick_toggle(&mut self, i: u16) {
        let Some(p) = &mut self.picking else {
            return;
        };
        let Some(id) = p.options.get(usize::from(i)).cloned() else {
            return;
        };
        self.message = p.toggle(&id).err();
    }

    /// The picker's Finish: the stamp holds the choice (what it held carries over for a detail
    /// kept) and a kind with properties goes on to the properties modal, the rest close.
    pub(super) fn pick_finish(&mut self, data: &Data) -> Asked {
        let Some((p, def)) = self.picking.as_ref().zip(self.def()) else {
            return Asked::Nothing;
        };
        let kind = p.kind;
        let places = stamp::rebuild(kind, &p.chosen, self.stamps.held(kind), &def, data);
        self.stamps.set(kind, places);
        self.message = None;
        if Picking::has_props(kind) {
            return self.props_open(data);
        }
        Asked::Picked
    }

    /// Open the properties modal for the chosen stamp (`EditAct::Props`, or the picker's
    /// Finish): a form a held place (the packs' first of the kind while nothing is chosen), at
    /// the cursor; the forms are copies, so Cancel changes nothing.
    pub(super) fn props_open(&mut self, data: &Data) -> Asked {
        let Some((kind, def)) = self.stamp_kind().zip(self.def()) else {
            return Asked::Nothing;
        };
        let at = self.at();
        let props: Vec<Form> = self
            .stamp_places(kind, data)
            .iter()
            .filter_map(|p| Form::for_place(p, &def, data, at, &[]))
            .collect();
        if props.is_empty() {
            self.message = Some(format!("nothing to set: the packs have no {}", kind.what()));
            return Asked::Nothing;
        }
        let mut picking = self
            .picking
            .take()
            .filter(|p| p.kind == kind)
            .unwrap_or_else(|| Picking::open(kind, &self.stamps, options(kind, &def, data, None)));
        picking.props = props;
        self.picking = Some(picking);
        self.message = None;
        Asked::Props
    }

    /// Field `i` of properties form `n` stepped (a press steps right).
    pub(super) fn prop(&mut self, n: u8, i: u8, step: i8) {
        if let Some(form) = self
            .picking
            .as_mut()
            .and_then(|p| p.props.get_mut(usize::from(n)))
        {
            form.step(usize::from(i), step);
        }
    }

    /// The properties modal's Finish: every form read back into the stamp; a field that does
    /// not read is said and the modal stands.
    pub(super) fn props_finish(&mut self) -> Asked {
        let Some(p) = &self.picking else {
            return Asked::Nothing;
        };
        let mut places = Vec::new();
        for form in &p.props {
            match form.value() {
                Ok(Change::Place(place)) => places.push(place),
                Ok(Change::Edit(_) | Change::Note { .. }) => {}
                Err(why) => {
                    self.message = Some(why);
                    return Asked::Nothing;
                }
            }
        }
        let kind = p.kind;
        self.stamps.set(kind, places);
        self.message = None;
        Asked::Picked
    }

    /// Text typed into the control `act` of a modal over the editor: the picker's search, a
    /// properties form's field; the inspector's form and the open file take theirs through the
    /// layer's own path, but are answered here too for the socket and the tests.
    pub fn typed_in(&mut self, act: EditAct, text: String) {
        match act {
            EditAct::PickFilter => self.pick_filter(text),
            EditAct::RulesFilter => self.rules_filter = text,
            EditAct::Prop(n, i) => {
                if let Some(form) = self
                    .picking
                    .as_mut()
                    .and_then(|p| p.props.get_mut(usize::from(n)))
                {
                    form.typed(usize::from(i), text);
                }
            }
            EditAct::Field(i) => self.typed(i, text),
            EditAct::FileText => self.typed_file(text),
            _ => {}
        }
    }

    /// A number set outright (a slider) on the control `act` of a modal over the editor.
    pub fn set_in(&mut self, act: EditAct, n: i64) {
        match act {
            EditAct::Prop(f, i) => {
                if let Some(form) = self
                    .picking
                    .as_mut()
                    .and_then(|p| p.props.get_mut(usize::from(f)))
                {
                    form.set(usize::from(i), n);
                    self.revision = self.revision.wrapping_add(1);
                }
            }
            EditAct::Field(i) => self.set_field(i, n),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picking(kind: Placing) -> Picking {
        Picking::open(kind, &Stamps::default(), Vec::new())
    }

    #[test]
    fn the_filter_keeps_the_rows_whose_label_holds_it_in_any_case() {
        let options: Vec<(String, String)> =
            ["a:monster:giant_rat", "a:monster:goblin", "b:monster:rat"]
                .iter()
                .map(|s| ((*s).to_owned(), (*s).to_owned()))
                .collect();
        let mut p = Picking::open(Placing::Encounter, &Stamps::default(), options);
        assert_eq!(p.shown(), vec![0, 1, 2], "everything on an empty filter");
        p.filter = "RAT".to_owned();
        assert_eq!(p.shown(), vec![0, 2]);
        p.filter = " b: ".to_owned();
        assert_eq!(p.shown(), vec![2], "trimmed");
        p.filter = "wolf".to_owned();
        assert!(p.shown().is_empty());
        // A portal's row carries its map and cell: the search finds it by any of them.
        let portals = vec![("gate".to_owned(), "gate  -  Town (5, 2)".to_owned())];
        let mut p = Picking::open(Placing::Portal, &Stamps::default(), portals);
        for needle in ["gate", "town", "(5, 2)"] {
            p.filter = needle.to_owned();
            assert_eq!(p.shown(), vec![0], "{needle}");
        }
    }

    #[test]
    fn a_label_fits_a_needle_it_holds_in_any_case_and_any_needle_that_is_blank() {
        assert!(fits("gate  -  Town (5, 2)", "TOWN"));
        assert!(fits("gate  -  Town (5, 2)", " (5, 2) "));
        assert!(fits("gate  -  Town (5, 2)", "") && fits("gate  -  Town (5, 2)", "   "));
        assert!(!fits("gate  -  Town (5, 2)", "meadow"));
    }

    #[test]
    fn a_choice_toggles_moves_for_a_kind_that_takes_one_and_stops_at_the_limit() {
        let mut p = picking(Placing::Encounter);
        for id in ["a", "b", "c", "d"] {
            p.toggle(id).unwrap();
        }
        assert_eq!(
            p.toggle("e").unwrap_err(),
            format!("at most {} monsters", stamp::limit(Placing::Encounter))
        );
        p.toggle("b").unwrap();
        assert!(!p.is_chosen("b") && p.chosen.len() == 3);
        p.toggle("e").unwrap();
        assert_eq!(p.chosen, ["a", "c", "d", "e"]);
        let mut one = picking(Placing::Portal);
        one.toggle("x").unwrap();
        one.toggle("y").unwrap();
        assert_eq!(one.chosen, ["y"], "a portal's choice moves");
        assert!(
            Picking::has_props(Placing::Ground) && !Picking::has_props(Placing::Lore),
            "piles have counts; a lore has nothing a click does not decide"
        );
    }
}
