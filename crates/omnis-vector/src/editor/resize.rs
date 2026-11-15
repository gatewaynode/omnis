//! The map's resize as the editor holds it (owner, 2026-11-14;
//! `tasks/plans/editor-resize.md`): cells to add on each of the four sides, a negative number
//! cutting them, and, when a cut would destroy something, the list of it waiting on a yes or a
//! no. The engine's rule is `omnis_data::edit::Edit::Extend`.

use super::{Asked, Editor};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::{Edit, Sides};

/// The sides in the order the form lists them.
pub const SIDES: [&str; 4] = ["North", "East", "South", "West"];

/// How many lines of a cut's losses the warning shows before it counts the rest.
pub const SHOWN: usize = 8;

/// A resize being asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resizing {
    /// What each side's field holds, in [`SIDES`]' order.
    pub typed: [String; 4],
    /// The map's size as the form opened on it.
    pub size: (u16, u16),
    /// What the cut would destroy, a line each, while the editor waits on Yes or No.
    pub asking: Option<Vec<String>>,
}

impl Resizing {
    /// The cells asked for on each side; an empty field is none.
    ///
    /// # Errors
    /// The side whose field is not a whole number.
    pub fn sides(&self) -> Result<Sides, String> {
        let mut cells = [0i16; 4];
        for ((cell, text), side) in cells.iter_mut().zip(&self.typed).zip(SIDES) {
            let text = text.trim();
            *cell = match text {
                "" => 0,
                _ => text
                    .parse()
                    .map_err(|_| format!("{side} is not a whole number"))?,
            };
        }
        let [north, east, south, west] = cells;
        Ok(Sides {
            north,
            east,
            south,
            west,
        })
    }

    /// The size the map would come to.
    ///
    /// # Errors
    /// Why Apply cannot be pressed: a field that is not a number, nothing asked for, or a size
    /// a map may not have.
    pub fn becomes(&self) -> Result<(u16, u16), String> {
        let sides = self.sides()?;
        if sides == Sides::default() {
            return Err("nothing to add or cut".to_owned());
        }
        sides
            .size(self.size)
            .map_err(|_| "a side must stay 1 to 256 cells".to_owned())
    }

    /// The edit the form asks for.
    fn edit(&self) -> Option<Edit> {
        let sides = self.sides().ok()?;
        Some(Edit::Extend {
            north: sides.north,
            east: sides.east,
            south: sides.south,
            west: sides.west,
        })
    }
}

impl Editor {
    /// Open the resize on the map as it stands, every side at nothing.
    pub(super) fn resize_open(&mut self) -> Asked {
        let Some(def) = self.def() else {
            return Asked::Nothing;
        };
        self.resizing = Some(Resizing {
            size: (def.width, def.height),
            ..Resizing::default()
        });
        Asked::Resize
    }

    /// Text typed into side `side`'s field.
    pub(super) fn resize_typed(&mut self, side: u8, text: String) {
        if let Some(field) = self
            .resizing
            .as_mut()
            .and_then(|r| r.typed.get_mut(usize::from(side)))
        {
            *field = text;
        }
    }

    /// The form's Apply: a grow, or a cut of empty cells, is made at once; a cut that would
    /// destroy something is not, and the list of it waits on Yes or No.
    pub(super) fn resize_apply(&mut self, data: &Data) -> Asked {
        let Some(edit) = self.resizing.as_ref().and_then(Resizing::edit) else {
            return Asked::Nothing;
        };
        match self.session.losses(&edit) {
            Ok(lost) if lost.is_empty() => self.resize_make(data, edit),
            Ok(lost) => {
                if let Some(resizing) = &mut self.resizing {
                    resizing.asking = Some(lost.lines());
                }
                Asked::Nothing
            }
            Err(e) => {
                self.message = Some(e.to_string());
                Asked::Nothing
            }
        }
    }

    /// The warning's answer: Yes makes the cut, No returns to the fields as they were.
    pub(super) fn resize_answer(&mut self, data: &Data, yes: bool) -> Asked {
        let asked = self.resizing.as_mut().and_then(|r| r.asking.take());
        match (asked, self.resizing.as_ref().and_then(Resizing::edit)) {
            (Some(_), Some(edit)) if yes => self.resize_make(data, edit),
            _ => Asked::Nothing,
        }
    }

    /// Make the resize, one undo step: the cursor goes with its cell, a selection and a run
    /// under way are dropped, and the plan is fitted to the new size.
    fn resize_make(&mut self, data: &Data, edit: Edit) -> Asked {
        let by = match edit {
            Edit::Extend { north, west, .. } => (i32::from(west), i32::from(north)),
            _ => (0, 0),
        };
        if !self.edited(data, edit) {
            return Asked::Nothing;
        }
        let moved = |at: u16, by: i32| u16::try_from((i32::from(at) + by).max(0)).unwrap_or(0);
        self.cursor = (moved(self.cursor.0, by.0), moved(self.cursor.1, by.1));
        self.selection = None;
        self.lasso = None;
        self.resizing = None;
        self.changed(data);
        if let Some(def) = self.def() {
            self.view = super::plan::View::fitted((def.width, def.height), self.pane);
        }
        let words = self
            .session
            .document()
            .and_then(|d| d.steps().last().map(|s| (*s).to_owned()));
        self.report(words.unwrap_or_else(|| "Resized".to_owned()));
        Asked::Picked
    }
}
