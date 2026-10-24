//! The editor's forms (M5.6, `tasks/plans/m5-editor-v1.md` row 6): one generic form of labelled
//! fields and lists of rows, drawn as kit rows at the top of the inspector and read back into a
//! request of `omnis_data::edit` by the kinds in `forms.rs`. A field is text, a choice, a number
//! or a flag; a list is rows of the same fields, added and removed one at a time (the owner's
//! choice of row editors over RON text, 2026-10-24). The form holds what is typed; the layer's
//! words-in-place redraw keeps the caret. Bevy-free.

use super::EditAct;
use crate::screen::{Control, ControlId, Kind, Row, Span};
use crate::style::{Role, Size};
use omnis_sim::omnis_core::rng::Dice;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::{Edit, Place};

/// What a form's Apply or Remove asks of the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Place, replace or remove a placement: one undo step.
    Place(Place),
    /// Change the map's terrains or levels: one undo step.
    Edit(Edit),
    /// Write a note, or clear it with none: at once, outside the history.
    Note {
        /// The note's key.
        key: String,
        /// The note, or none to clear it.
        text: Option<String>,
    },
}

/// What a field holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Typed text, at most `max` bytes, on `lines` lines.
    Text {
        /// The text.
        text: String,
        /// The most bytes it takes.
        max: usize,
        /// The lines it shows.
        lines: u8,
    },
    /// One of `options`.
    Choice {
        /// The options' words.
        options: Vec<String>,
        /// The current one.
        at: usize,
    },
    /// A whole number from `min` to `max`.
    Number {
        /// The number.
        value: i64,
        /// The lowest it may be.
        min: i64,
        /// The highest it may be.
        max: i64,
    },
    /// Yes or no.
    Flag(bool),
}

/// Where a field stands: among the form's own, or in row `.1` of list `.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// The form's own.
    Top,
    /// In a list's row.
    Row(u8, u8),
}

/// A labelled field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Its name, unique among the form's own fields and among a list's.
    pub label: &'static str,
    /// What it holds.
    pub value: Value,
    /// Where it stands.
    pub slot: Slot,
}

/// A list of rows of the same fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    /// Its name.
    pub label: &'static str,
    /// The fields a row has, with their starting values.
    pub template: Vec<Field>,
    /// The most rows it takes.
    pub max: u8,
}

/// Which kind of thing a form edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormKind {
    /// A portal to another map.
    Portal,
    /// A fixed encounter.
    Encounter,
    /// The map's random encounter table.
    Random,
    /// A town service's site.
    Site,
    /// What may happen while the party rests on a terrain.
    RestEvent,
    /// A placed object.
    Object,
    /// A ground pile.
    Ground,
    /// A cell's lore.
    Lore,
    /// A terrain of the map's list, by its glyph; none for a new one (6c).
    Terrain(Option<char>),
    /// A new level.
    LevelAdd,
    /// A level of the map, by its number.
    Level(i8),
    /// The author's note on this key.
    Note(String),
}

/// Where a form places: the shown level, the cursor's cell and its facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    /// The level.
    pub level: Level,
    /// The cell.
    pub x: u16,
    /// The cell.
    pub y: u16,
    /// The way the cursor faces.
    pub facing: Facing,
}

/// A form being filled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    /// What it edits.
    pub kind: FormKind,
    /// The heading over the fields: what is edited and where.
    pub heading: String,
    /// A line under the heading, when something must be said (a note is not undone).
    pub hint: Option<&'static str>,
    /// Where it was opened.
    pub at: At,
    /// The fields, the form's own first, then the lists' rows in order.
    pub fields: Vec<Field>,
    /// The lists.
    pub lists: Vec<List>,
    /// What Remove asks when the form edits something already there; none for a new one.
    pub remove: Option<Change>,
    /// Just opened: its first field takes the keyboard when it is drawn.
    pub fresh: bool,
}

/// The most rows any list takes.
pub const MAX_ROWS: u8 = 32;

/// A text field.
#[must_use]
pub fn text(label: &'static str, text: &str, max: usize) -> Field {
    Field {
        label,
        value: Value::Text {
            text: text.to_owned(),
            max,
            lines: 1,
        },
        slot: Slot::Top,
    }
}

/// A text field of several lines.
#[must_use]
pub fn lines(label: &'static str, text: &str, max: usize, lines: u8) -> Field {
    Field {
        label,
        value: Value::Text {
            text: text.to_owned(),
            max,
            lines,
        },
        slot: Slot::Top,
    }
}

/// A choice among `options`, at `at`.
#[must_use]
pub fn choice(label: &'static str, options: Vec<String>, at: usize) -> Field {
    Field {
        label,
        value: Value::Choice { options, at },
        slot: Slot::Top,
    }
}

/// A number from `min` to `max`, at `value`.
#[must_use]
pub fn number(label: &'static str, value: i64, min: i64, max: i64) -> Field {
    Field {
        label,
        value: Value::Number {
            value: value.clamp(min, max),
            min,
            max,
        },
        slot: Slot::Top,
    }
}

/// A yes or no.
#[must_use]
pub fn flag(label: &'static str, on: bool) -> Field {
    Field {
        label,
        value: Value::Flag(on),
        slot: Slot::Top,
    }
}

impl Form {
    /// A form just opened: `fields` its own, `lists` empty until `fill`ed.
    #[must_use]
    pub fn new(
        kind: FormKind,
        heading: String,
        at: At,
        fields: Vec<Field>,
        lists: Vec<List>,
    ) -> Form {
        Form {
            kind,
            heading,
            hint: None,
            at,
            fields,
            lists,
            remove: None,
            fresh: true,
        }
    }

    /// The words on the button that asks `remove`.
    fn remove_label(&self) -> &'static str {
        match self.kind {
            FormKind::Note(_) => "Clear note",
            FormKind::Terrain(_) => "Remove terrain",
            FormKind::Level(_) => "Remove level",
            _ => "Remove",
        }
    }

    /// Append rows to list `l`, each from the template with `values` in the template's order.
    pub fn fill(&mut self, l: u8, rows: Vec<Vec<Value>>) {
        for values in rows {
            let r = self.rows_of(l).len();
            let Some(list) = self.lists.get(usize::from(l)) else {
                return;
            };
            let row: Vec<Field> = list
                .template
                .iter()
                .enumerate()
                .map(|(i, f)| Field {
                    label: f.label,
                    value: values.get(i).cloned().unwrap_or_else(|| f.value.clone()),
                    slot: Slot::Row(l, u8::try_from(r).unwrap_or(u8::MAX)),
                })
                .collect();
            self.fields.extend(row);
        }
    }

    /// The indices of the fields of each row of list `l`, in order.
    #[must_use]
    pub fn rows_of(&self, l: u8) -> Vec<Vec<usize>> {
        let mut rows: Vec<Vec<usize>> = Vec::new();
        for (i, f) in self.fields.iter().enumerate() {
            if let Slot::Row(list, r) = f.slot
                && list == l
            {
                let r = usize::from(r);
                if rows.len() <= r {
                    rows.resize(r + 1, Vec::new());
                }
                rows[r].push(i);
            }
        }
        rows
    }

    /// The form's own field named `label`.
    fn own(&self, label: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| f.slot == Slot::Top && f.label == label)
    }

    /// The text of the form's own text field `label`; empty when there is none.
    #[must_use]
    pub fn text_of(&self, label: &str) -> &str {
        self.own(label).map_or("", |f| f.text())
    }

    /// The number of the form's own number field `label`; 0 when there is none.
    #[must_use]
    pub fn number_of(&self, label: &str) -> i64 {
        self.own(label).map_or(0, Field::number)
    }

    /// The option of the form's own choice `label`; 0 when there is none.
    #[must_use]
    pub fn chosen_of(&self, label: &str) -> usize {
        self.own(label).map_or(0, Field::chosen)
    }

    /// The words of the option of the form's own choice `label`; empty when there is none.
    #[must_use]
    pub fn choice_of(&self, label: &str) -> &str {
        self.own(label).map_or("", Field::option)
    }

    /// Whether the form's own flag `label` is set.
    #[must_use]
    pub fn flag_of(&self, label: &str) -> bool {
        self.own(label).is_some_and(Field::flag)
    }

    /// The text of field `i`; empty when it is no text field.
    #[must_use]
    pub fn text(&self, i: usize) -> &str {
        self.fields.get(i).map_or("", |f| f.text())
    }

    /// Field `i` was typed into: the form holds `text` now. Whether anything changed.
    pub fn typed(&mut self, i: usize, text: String) -> bool {
        match self.fields.get_mut(i).map(|f| &mut f.value) {
            Some(Value::Text { text: held, .. }) if *held != text => {
                *held = text;
                true
            }
            _ => false,
        }
    }

    /// Field `i` stepped by `by` (a press steps right): a choice goes round, a number stops at
    /// its ends, a flag turns.
    pub fn step(&mut self, i: usize, by: i8) {
        let by = if by == 0 { 1 } else { i64::from(by) };
        match self.fields.get_mut(i).map(|f| &mut f.value) {
            Some(Value::Choice { options, at }) if !options.is_empty() => {
                let n = i64::try_from(options.len()).unwrap_or(1);
                let next = (i64::try_from(*at).unwrap_or(0) + by).rem_euclid(n);
                *at = usize::try_from(next).unwrap_or(0);
            }
            Some(Value::Number { value, min, max }) => {
                *value = value.saturating_add(by).clamp(*min, *max);
            }
            Some(Value::Flag(on)) => *on = !*on,
            _ => {}
        }
    }

    /// Number field `i` set outright (a slider), held to its range.
    pub fn set(&mut self, i: usize, n: i64) {
        if let Some(Value::Number { value, min, max }) =
            self.fields.get_mut(i).map(|f| &mut f.value)
        {
            *value = n.clamp(*min, *max);
        }
    }

    /// A row added to list `l` from its template.
    ///
    /// # Errors
    /// The list is full.
    pub fn add_row(&mut self, l: u8) -> Result<(), String> {
        let Some(list) = self.lists.get(usize::from(l)) else {
            return Err("no such list".to_owned());
        };
        let (label, max) = (list.label, list.max);
        if self.rows_of(l).len() >= usize::from(max) {
            return Err(format!("at most {max} rows of {label}"));
        }
        self.fill(l, vec![Vec::new()]);
        Ok(())
    }

    /// Row `r` of list `l` removed; the rows after it move up.
    pub fn remove_row(&mut self, l: u8, r: u8) {
        self.fields.retain(|f| f.slot != Slot::Row(l, r));
        for f in &mut self.fields {
            if let Slot::Row(list, row) = &mut f.slot
                && *list == l
                && *row > r
            {
                *row -= 1;
            }
        }
    }

    /// The form as kit rows: the heading, the form's own fields one to a row, each list's
    /// heading with Add row, its rows each with Remove, then Apply, Cancel and, when it edits
    /// something already there, Remove.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let edit = |act| ControlId::Edit(act);
        let mut rows = vec![Row::text(Span {
            text: self.heading.clone(),
            role: Role::Title,
            size: Size::Body,
        })];
        if let Some(hint) = self.hint {
            rows.push(Row::text(Span::body(hint)));
        }
        for (i, f) in self.fields.iter().enumerate() {
            if f.slot == Slot::Top {
                rows.push(Row::controls(vec![f.control(i)]));
            }
        }
        for (l, list) in self.lists.iter().enumerate() {
            let l = u8::try_from(l).unwrap_or(u8::MAX);
            rows.push(Row {
                spans: vec![Span::body(list.label)],
                controls: vec![Control::button(edit(EditAct::ListAdd(l)), "Add row")],
                ..Row::default()
            });
            for (r, row) in self.rows_of(l).into_iter().enumerate() {
                let r = u8::try_from(r).unwrap_or(u8::MAX);
                let mut controls: Vec<Control> =
                    row.iter().map(|i| self.fields[*i].control(*i)).collect();
                controls.push(Control::button(edit(EditAct::ListRemove(l, r)), "Remove"));
                rows.push(Row::controls(controls));
            }
        }
        let mut last = vec![
            Control::button(edit(EditAct::Apply), "Apply"),
            Control::button(edit(EditAct::Cancel), "Cancel"),
        ];
        if self.remove.is_some() {
            last.push(Control::button(edit(EditAct::Discard), self.remove_label()));
        }
        rows.push(Row::controls(last));
        rows
    }
}

impl Field {
    /// The field as a control, under `EditAct::Field(i)`.
    fn control(&self, i: usize) -> Control {
        let id = ControlId::Edit(EditAct::Field(u8::try_from(i).unwrap_or(u8::MAX)));
        let kind = match &self.value {
            Value::Text { text, max, lines } => Kind::Text {
                value: text.clone(),
                max: *max,
                lines: *lines,
            },
            Value::Choice { options, at } => Kind::Choice {
                options: options.clone(),
                at: *at,
            },
            Value::Number { value, min, max } => Kind::Number {
                value: *value,
                min: *min,
                max: *max,
            },
            Value::Flag(on) => Kind::Choice {
                options: vec!["no".to_owned(), "yes".to_owned()],
                at: usize::from(*on),
            },
        };
        Control {
            kind,
            ..Control::button(id, self.label)
        }
    }

    /// Its text; empty unless it is a text field.
    #[must_use]
    pub fn text(&self) -> &str {
        match &self.value {
            Value::Text { text, .. } => text,
            _ => "",
        }
    }

    /// Its number; 0 unless it is a number field.
    #[must_use]
    pub fn number(&self) -> i64 {
        match &self.value {
            Value::Number { value, .. } => *value,
            _ => 0,
        }
    }

    /// Its option's index; 0 unless it is a choice.
    #[must_use]
    pub fn chosen(&self) -> usize {
        match &self.value {
            Value::Choice { at, .. } => *at,
            _ => 0,
        }
    }

    /// Its option's words; empty unless it is a choice.
    #[must_use]
    pub fn option(&self) -> &str {
        match &self.value {
            Value::Choice { options, at } => options.get(*at).map_or("", String::as_str),
            _ => "",
        }
    }

    /// Whether it is a flag that is set.
    #[must_use]
    pub fn flag(&self) -> bool {
        matches!(self.value, Value::Flag(true))
    }
}

/// An id as typed: one word, not empty. The loader says whether anything answers to it.
///
/// # Errors
/// Empty, or more than one word.
pub fn id(text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("an id is needed".to_owned());
    }
    if text.chars().any(char::is_whitespace) {
        return Err(format!("an id is one word, not \"{text}\""));
    }
    Ok(text.to_owned())
}

/// A dice expression as typed, `2d6`, `1d4+1` or `3d8-2`.
///
/// # Errors
/// Anything else.
pub fn dice(text: &str) -> Result<Dice, String> {
    let bad = || format!("dice read like 2d6 or 1d4+1, not \"{text}\"");
    let text = text.trim();
    let (count, rest) = text.split_once('d').ok_or_else(bad)?;
    let split = rest.find(['+', '-']);
    let (sides, modifier) = match split {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let count: u16 = count.trim().parse().map_err(|_| bad())?;
    let sides: u16 = sides.trim().parse().map_err(|_| bad())?;
    let modifier: i32 = if modifier.is_empty() {
        0
    } else {
        modifier
            .trim_start_matches('+')
            .parse()
            .map_err(|_| bad())?
    };
    if count == 0 || sides == 0 {
        return Err(bad());
    }
    Ok(Dice {
        count,
        sides,
        modifier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dice_read_as_they_are_written() {
        for text in ["2d6", "1d4+1", "3d8-2", "10d10+12"] {
            let d = dice(text).unwrap();
            assert_eq!(d.to_string(), text);
        }
        for text in ["", "d6", "2d", "2d0", "0d6", "two dice", "2x6"] {
            assert!(dice(text).is_err(), "{text}");
        }
        assert_eq!(dice(" 1d4 ").unwrap(), Dice::new(1, 4));
    }

    #[test]
    fn rows_are_added_removed_and_renumbered() {
        let at = At {
            level: Level::GROUND,
            x: 0,
            y: 0,
            facing: Facing::North,
        };
        let list = List {
            label: "Stacks",
            template: vec![text("creature", "", 8), number("count", 1, 1, 9)],
            max: 2,
        };
        let mut form = Form::new(
            FormKind::Encounter,
            "E".into(),
            at,
            vec![text("id", "", 8)],
            vec![list],
        );
        form.add_row(0).unwrap();
        form.add_row(0).unwrap();
        assert!(form.add_row(0).is_err(), "full");
        let rows = form.rows_of(0);
        assert_eq!(rows.len(), 2);
        assert!(form.typed(rows[1][0], "wolf".into()));
        form.step(rows[1][1], 1);
        form.remove_row(0, 0);
        let rows = form.rows_of(0);
        assert_eq!(rows.len(), 1);
        assert_eq!(form.fields[rows[0][0]].slot, Slot::Row(0, 0), "moved up");
        assert_eq!(form.text(rows[0][0]), "wolf");
        assert_eq!(form.fields[rows[0][1]].number(), 2);
        assert!(id("two words").is_err() && id(" ").is_err());
        assert_eq!(id(" wolf ").unwrap(), "wolf");
        // The kit rows: the heading, the id, the list's heading, its row, the buttons.
        assert_eq!(form.rows().len(), 5);
        form.hint = Some("said");
        assert_eq!(form.rows().len(), 6);
    }
}
