//! The forms' shared parts (M5.6): the limits typed values keep to, the choice fields several
//! kinds share (a facing, a disposition, a map) and the readers that turn a form's text and
//! numbers back into the data's types. Held by `forms.rs` until M5.10.

use crate::editor::form::{self, Form, List, Value};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_core::rng::Dice;
use omnis_sim::omnis_data::encounter::Disposition;
use omnis_sim::omnis_data::map::Join;
use omnis_sim::omnis_data::rest_event::RestKind;

/// The most bytes an id or a text key typed in a form takes.
pub(super) const ID_MAX: usize = 96;
/// The most bytes a line of creatures and dice takes.
pub(super) const CREATURES_MAX: usize = 400;
/// The most of a stack or an item.
pub(super) const COUNT_MAX: i64 = 255;
/// The most of an item in a container or a pile.
pub(super) const ITEMS_MAX: i64 = 65_535;
/// The highest lore DC.
pub(super) const DC_MAX: i64 = 30;
/// The deepest a hidden object hides.
pub(super) const HIDDEN_MAX: i64 = 30;
/// The most a random entry weighs.
pub(super) const WEIGHT_MAX: i64 = 65_535;
/// The rest events' chance scale: per mille.
pub(super) const PER_MILLE: i64 = 1000;
/// The most minutes a step on a terrain may take.
pub(super) const STEP_MINUTES_MAX: i64 = 1440;
/// The most a terrain's climb may be, in eighths.
pub(super) const CLIMB_MAX: i64 = 64;
/// The levels a map may have, above and below the ground.
pub(super) const LEVEL_SPAN: i64 = 16;
/// The lines a note's field shows.
pub(super) const NOTE_LINES: u8 = 4;

pub(super) const JOINS: [Option<Join>; 5] = [
    None,
    Some(Join::Up),
    Some(Join::Down),
    Some(Join::Ladder),
    Some(Join::Hole),
];

pub(super) const DISPOSITIONS: [Disposition; 4] = [
    Disposition::Hostile,
    Disposition::Wary,
    Disposition::Neutral,
    Disposition::Friendly,
];
pub(super) const RESTS: [RestKind; 3] = [RestKind::Short, RestKind::Long, RestKind::Any];

fn facing_name(facing: Facing) -> String {
    format!("{facing:?}").to_lowercase()
}

pub(super) fn facings(at: Facing) -> form::Field {
    let options = Facing::ALL.iter().map(|f| facing_name(*f)).collect();
    let i = Facing::ALL.iter().position(|f| *f == at).unwrap_or(0);
    form::choice("facing", options, i)
}

pub(super) fn facing_of(form: &Form, label: &str) -> Facing {
    Facing::ALL
        .get(form.chosen_of(label))
        .copied()
        .unwrap_or(Facing::North)
}

pub(super) fn dispositions(at: Disposition) -> form::Field {
    let options = DISPOSITIONS.iter().map(|d| format!("{d:?}")).collect();
    let i = DISPOSITIONS.iter().position(|d| *d == at).unwrap_or(0);
    form::choice("disposition", options, i)
}

/// `options` sorted, with `held` among them even when the loaded packs lack it (so editing keeps
/// what the file says), and where `held` stands.
pub(super) fn among(mut options: Vec<String>, held: Option<&str>) -> (Vec<String>, usize) {
    options.sort();
    options.dedup();
    if let Some(h) = held
        && !options.iter().any(|o| o == h)
    {
        options.push(h.to_owned());
    }
    let at = held
        .and_then(|h| options.iter().position(|o| o == h))
        .unwrap_or(0);
    (options, at)
}

pub(super) fn coordinate(label: &'static str, value: u16, side: u16) -> form::Field {
    form::number(
        label,
        i64::from(value),
        0,
        i64::from(side.saturating_sub(1).max(1)),
    )
}

pub(super) fn u16_of(n: i64) -> u16 {
    u16::try_from(n).unwrap_or(0)
}

pub(super) fn u8_of(n: i64) -> u8 {
    u8::try_from(n).unwrap_or(0)
}

pub(super) fn i8_of(n: i64) -> i8 {
    i8::try_from(n).unwrap_or(0)
}

/// A text that is none when empty.
pub(super) fn optional(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The rows of a list of (id, count) pairs, as the template's values.
pub(super) fn pairs<N: Copy + Into<i64>>(items: &[(String, N)]) -> Vec<Vec<Value>> {
    items
        .iter()
        .map(|(id, n)| {
            vec![
                Value::Text {
                    text: id.clone(),
                    max: ID_MAX,
                    lines: 1,
                },
                Value::Number {
                    value: (*n).into(),
                    min: 1,
                    max: ITEMS_MAX,
                },
            ]
        })
        .collect()
}

/// A list of (id, count) rows, at most `rows` of them.
pub(super) fn pair_list(label: &'static str, what: &'static str, max: i64, rows: u8) -> List {
    List {
        label,
        template: vec![
            form::text(what, "", ID_MAX),
            form::number("count", 1, 1, max),
        ],
        max: rows,
    }
}

/// An id for a new thing at `(x, y)` that nothing on the map has yet: `<what>_<x>_<y>`, then
/// `_2`, `_3` and so on.
pub(super) fn free_id(what: &str, (x, y): (u16, u16), taken: impl Fn(&str) -> bool) -> String {
    let base = format!("{what}_{x}_{y}");
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

/// The (id, count) pairs of list `l`, each id one word.
pub(super) fn read_pairs<N: TryFrom<i64>>(form: &Form, l: u8) -> Result<Vec<(String, N)>, String> {
    form.rows_of(l)
        .iter()
        .map(|row| {
            let id = form::id(form.text(row[0]))?;
            let n = form.fields.get(row[1]).map_or(1, form::Field::number);
            let n = N::try_from(n).map_err(|_| format!("{n} is too many"))?;
            Ok((id, n))
        })
        .collect()
}

/// Creatures and their dice as one line: `goblin 1d4, wolf 2d6`.
pub(super) fn creatures_line(stacks: &[(String, Dice)]) -> String {
    stacks
        .iter()
        .map(|(id, d)| format!("{id} {d}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The creatures and dice of a line.
pub(super) fn read_creatures(line: &str) -> Result<Vec<(String, Dice)>, String> {
    line.split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let (id, dice) = part.trim().split_once(char::is_whitespace).ok_or_else(|| {
                format!("a creature and its dice, like goblin 1d4, not \"{part}\"")
            })?;
            Ok((form::id(id)?, form::dice(dice)?))
        })
        .collect()
}
