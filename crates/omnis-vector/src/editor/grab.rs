//! The move tool (the owner's drawing-GUI column, step 2, 2026-10-25): a press on a cell takes
//! the selection when the cell is inside it (step 4, `super::select`), else what the inspector
//! lists there first, the map's start or a placement; the drag carries it and letting go puts it
//! on the cell under the pointer (a selection by the drag's offset in one `Document::batch`, a
//! start in one `Place::Start`, a placement as a removal and a placement in one step,
//! `Document::place_many`); a press anywhere else drags the plan itself by whole cells
//! (`View::origin`). The middle button drags the plan with any tool, a shortcut for Move's pan.
//! The lasso's loop is read here too, being a drag. Bevy-free.

use super::run::{Pointer, Press};
use super::{Editor, Here, PlaceKey};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::Place;

/// What a press under the Move tool took: the map's start, or a placement as the inspector lists
/// it. The start is listed first, so a cell that has both gives the start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Held {
    /// The selection, pressed inside (step 4).
    Selection,
    /// The map's start (the owner could not move it in the step's hand test, 2026-10-25).
    Start,
    /// The first placement the inspector lists on the cell.
    Place(Here),
}

impl Held {
    /// What it is, in a line.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Held::Selection => "Selection",
            Held::Start => "Start",
            Held::Place(here) => &here.label,
        }
    }
}

/// A drag in progress under the Move tool or the middle button.
#[derive(Debug, Clone, PartialEq)]
pub struct Grab {
    /// The cell pressed on, when the press was on the map.
    pub from: Option<(u16, u16)>,
    /// What is carried, if the press took anything; none pans the plan.
    pub held: Option<Held>,
    /// The pane pixel of the press.
    pub press: (f32, f32),
    /// Where the plan looked at the press.
    pub origin: (i64, i64),
    /// Whether the middle button holds it (the left otherwise).
    pub middle: bool,
}

impl Editor {
    /// The plan's pointer for a drag: a grab in progress follows its button; the middle button
    /// going down starts a pan with any tool; with the Move tool a left press starts a grab of
    /// what is under it and a right click picks as elsewhere. Some when this frame was a drag's
    /// (whether a button went down on the plan), None for the tool's own pointer rules.
    pub(super) fn grabbing(&mut self, data: &Data, p: Pointer) -> Option<bool> {
        // The lasso's loop in progress follows the left button: the cell under the pointer
        // joins it, letting go selects (off the plan too).
        if self.lasso.is_some() {
            match p.left {
                Press::Held => {
                    if let Some(cell) = p
                        .pixel
                        .and_then(|(px, py)| self.view.hit(self.size(), px, py))
                    {
                        self.lasso_extend(cell);
                    }
                }
                Press::Up | Press::Down => self.lasso_end(),
            }
            return Some(false);
        }
        if let Some(grab) = &self.grab {
            let button = if grab.middle { p.middle } else { p.left };
            match button {
                Press::Held => self.drag_grab(p.pixel),
                Press::Up | Press::Down => self.end_grab(data, p.pixel),
            }
            return Some(false);
        }
        if p.middle == Press::Down
            && let Some(pixel) = p.pixel
        {
            self.start_grab(pixel, true);
            return Some(true);
        }
        if !matches!(self.tool, super::Tool::Move | super::Tool::Lasso) {
            return None;
        }
        if self.corner.is_some() {
            self.corner = None;
            self.revision = self.revision.wrapping_add(1);
        }
        let target = p.pixel.and_then(|(px, py)| self.aim(px, py));
        if self.hover != target {
            self.hover = target;
        }
        if p.right_down
            && let Some(target) = target
        {
            self.pick(target);
            return Some(true);
        }
        if p.left == Press::Down
            && let Some(pixel) = p.pixel
        {
            if self.tool == super::Tool::Lasso {
                if let Some(cell) = self.view.hit(self.size(), pixel.0, pixel.1) {
                    self.lasso_start(cell);
                }
            } else {
                self.start_grab(pixel, false);
            }
            return Some(true);
        }
        Some(false)
    }

    /// Start a drag at pane pixel `pixel`: of what is under it (`under`), unless `pan` or the
    /// cell has nothing, when the plan itself is dragged. The cursor goes to the cell.
    fn start_grab(&mut self, pixel: (f32, f32), pan: bool) {
        let from = self.view.hit(self.size(), pixel.0, pixel.1);
        let held = match from {
            Some(cell) if !pan => self.under(cell),
            _ => None,
        };
        if let Some(cell) = from
            && !pan
        {
            self.point(cell);
        }
        self.grab = Some(Grab {
            from,
            held,
            press: pixel,
            origin: self.view.origin,
            middle: pan,
        });
    }

    /// What the Move tool takes on `cell` of the shown level: the selection when the cell is
    /// inside it, else as the inspector lists it: the start when it is here (it is on the ground
    /// level), else the first listed placement; none on a bare cell.
    fn under(&self, cell: (u16, u16)) -> Option<Held> {
        if self.selected().is_some_and(|s| s.cells.contains(&cell)) {
            return Some(Held::Selection);
        }
        let start_here = self
            .def()
            .is_some_and(|d| self.level.is_ground() && (d.start.0, d.start.1) == cell);
        if start_here {
            return Some(Held::Start);
        }
        self.here_at(cell).into_iter().next().map(Held::Place)
    }

    /// The button is still held at `pixel`: a carried placement's cursor follows the cell under
    /// the pointer; a pan moves the plan by the whole cells the pointer has crossed since the
    /// press. Off the plan nothing moves.
    fn drag_grab(&mut self, pixel: Option<(f32, f32)>) {
        let (Some(grab), Some((px, py))) = (self.grab.clone(), pixel) else {
            return;
        };
        if grab.held.is_some() {
            if let Some(cell) = self.view.hit(self.size(), px, py) {
                self.point(cell);
            }
            return;
        }
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let crossed = {
            let s = self.view.scale() as f32;
            (
                ((px - grab.press.0) / s).trunc() as i64,
                ((py - grab.press.1) / s).trunc() as i64,
            )
        };
        let origin = (grab.origin.0 - crossed.0, grab.origin.1 - crossed.1);
        if self.view.origin != origin {
            self.view.origin = origin;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// The button is let go at `pixel`: a carried placement lands on the cell under the pointer
    /// when that is a cell of the map other than the one it came from; elsewhere, or a pan,
    /// nothing more happens.
    fn end_grab(&mut self, data: &Data, pixel: Option<(f32, f32)>) {
        let Some(grab) = self.grab.take() else {
            return;
        };
        let (Some(held), Some((px, py))) = (grab.held, pixel) else {
            return;
        };
        if let Some(to) = self.view.hit(self.size(), px, py)
            && Some(to) != grab.from
        {
            if held == Held::Selection {
                if let Some(from) = grab.from {
                    let d = (
                        i32::from(to.0) - i32::from(from.0),
                        i32::from(to.1) - i32::from(from.1),
                    );
                    self.move_selection(data, d);
                }
                return;
            }
            self.moved(data, &held, to);
        }
    }

    /// Put what `held` names on `to` of the shown level: the start keeps its facing (one
    /// `Place::Start`); a placement is a removal and a placement in one step, a tile-keyed kind
    /// (a portal, a site, a ground pile) refused where one of its kind already is, the refusal
    /// said. The cursor goes to `to`. Whether the map changed.
    pub fn moved(&mut self, data: &Data, held: &Held, to: (u16, u16)) -> bool {
        let (x, y) = to;
        let taken = match held {
            // A selection moves by an offset (`end_grab`), never to one cell.
            Held::Selection => return false,
            Held::Start => {
                let Some(facing) = self.def().map(|d| d.start.2) else {
                    return false;
                };
                self.placed(data, Place::Start { x, y, facing })
            }
            Held::Place(here) => self.moved_place(data, here, to),
        };
        if !taken {
            return false;
        }
        self.cursor = to;
        self.message = Some(format!("{} moved to ({x}, {y})", held.label()));
        true
    }

    /// The placement `here` names put on `to`: `Place::Remove` and the def at the new cell in one
    /// `place_many`; the refusals of `moved`. Whether the map changed.
    fn moved_place(&mut self, data: &Data, here: &Here, to: (u16, u16)) -> bool {
        let Some(def) = self.def() else {
            return false;
        };
        let (level, (x, y)) = (self.level, to);
        let there = |l, px, py| (l, px, py) == (level, x, y);
        let occupied = |what: &str| format!("{what} is already at ({x}, {y})");
        let new = match &here.key {
            PlaceKey::Portal {
                level: l,
                x: fx,
                y: fy,
            } => {
                if def.portals.iter().any(|p| there(p.level, p.x, p.y)) {
                    self.message = Some(occupied("a portal"));
                    return false;
                }
                def.portals
                    .iter()
                    .find(|p| (p.level, p.x, p.y) == (*l, *fx, *fy))
                    .cloned()
                    .map(|mut p| {
                        (p.x, p.y) = (x, y);
                        Place::Portal(p)
                    })
            }
            PlaceKey::Site {
                level: l,
                x: fx,
                y: fy,
            } => {
                if def.sites.iter().any(|s| there(s.level, s.x, s.y)) {
                    self.message = Some(occupied("a site"));
                    return false;
                }
                def.sites
                    .iter()
                    .find(|s| (s.level, s.x, s.y) == (*l, *fx, *fy))
                    .cloned()
                    .map(|mut s| {
                        (s.x, s.y) = (x, y);
                        Place::Site(s)
                    })
            }
            PlaceKey::Ground {
                level: l,
                x: fx,
                y: fy,
            } => {
                if def.ground.iter().any(|g| there(g.level, g.x, g.y)) {
                    self.message = Some(occupied("a ground pile"));
                    return false;
                }
                def.ground
                    .iter()
                    .find(|g| (g.level, g.x, g.y) == (*l, *fx, *fy))
                    .cloned()
                    .map(|mut g| {
                        (g.x, g.y) = (x, y);
                        Place::Ground(g)
                    })
            }
            PlaceKey::Encounter { id } => {
                def.encounters
                    .iter()
                    .find(|e| e.id == *id)
                    .cloned()
                    .map(|mut e| {
                        (e.x, e.y) = (x, y);
                        Place::Encounter(e)
                    })
            }
            PlaceKey::Object { id } => {
                def.objects
                    .iter()
                    .find(|o| o.id == *id)
                    .cloned()
                    .map(|mut o| {
                        (o.x, o.y) = (x, y);
                        Place::Object(o)
                    })
            }
            PlaceKey::Lore { id } => def.lore.iter().find(|l| l.id == *id).cloned().map(|mut l| {
                (l.x, l.y) = (x, y);
                Place::Lore(l)
            }),
            PlaceKey::RestEvent { .. } => None,
        };
        let Some(new) = new else {
            self.message = Some(format!("nothing to move: {}", here.label));
            return false;
        };
        let places = vec![Place::Remove(here.key.clone()), new];
        let result = self
            .session
            .document_mut()
            .and_then(|d| d.place_many(places));
        self.took(data, result)
    }
}
