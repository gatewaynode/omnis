//! The list under a portal form's link field (the entrances and exits enhancement, W1 step 2c,
//! the owner's direction of 2026-11-07): every portal nothing links to across the loaded maps
//! (the open map's unsaved ones among them, the edited portal left out) and the edited portal's
//! current partner, each "id  -  Map (x, y)", listed under the field whenever a portal form is
//! open and narrowed by what is typed in it (the picker's rule, `pick::fits`: the label holds
//! the text in any case). When the text is a listed id the whole list shows with that one marked
//! and first, so a chosen link still lets you switch. A row pressed fills the field (the marked
//! one clears it), Unlink clears it; the field is drawn anew either way (`Form::set_text`), since
//! the window pushes a drawn field's text back into the model. Bevy-free: `view::inspector`
//! draws the rows through `with_list`.

use super::form::{Form, FormKind};
use super::forms::Placing;
use super::pick::{self, fits};
use super::{EditAct, Editor};
use crate::screen::{Control, ControlId, Row, Span};
use omnis_sim::omnis_data::{Data, MapDef, partner_of};

/// The most rows the list draws; the rest are counted.
pub const SHOWN: usize = 10;

/// A row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The portal's id, what the field takes.
    pub id: String,
    /// "id  -  Map (x, y)".
    pub label: String,
    /// Whether the field's text is this id.
    pub chosen: bool,
}

/// The list as narrowed by the field's text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Listed {
    /// The rows drawn, the chosen one first.
    pub shown: Vec<Match>,
    /// How many more fit the text beyond `SHOWN`.
    pub more: usize,
    /// Whether anything could be listed at all (the text aside).
    pub any: bool,
}

/// What a portal form at `form.at` may link to (id and label each), by label: the portals
/// nothing links to among the loaded maps but this one (`pick::options`), and its partner
/// when it has one.
#[must_use]
pub fn candidates(form: &Form, def: &MapDef, data: &Data) -> Vec<(String, String)> {
    let at = (form.at.level, form.at.x, form.at.y);
    let mut options = pick::options(Placing::Portal, def, data, Some(at));
    let own = def.portals.iter().find(|p| (p.level, p.x, p.y) == at);
    if let Some((map, q)) = own.and_then(|p| partner_of(p, def, data))
        && !options.iter().any(|(id, _)| *id == q.id)
    {
        options.push((q.id.clone(), pick::portal_label(data, map, q)));
    }
    options.sort_by(|a, b| a.1.cmp(&b.1));
    options
}

/// The list for `form`: every candidate while the field's text is a candidate's id (that one
/// chosen and first), else those whose label holds the text; the first `SHOWN` of them.
#[must_use]
pub fn matches(form: &Form, def: &MapDef, data: &Data) -> Listed {
    let text = form.text_of("link").trim();
    let all = candidates(form, def, data);
    let any = !all.is_empty();
    let exact = !text.is_empty() && all.iter().any(|(id, _)| id == text);
    let mut fitting: Vec<Match> = all
        .into_iter()
        .filter(|(_, label)| exact || fits(label, text))
        .map(|(id, label)| Match {
            chosen: exact && id == text,
            id,
            label,
        })
        .collect();
    fitting.sort_by_key(|m| !m.chosen);
    let more = fitting.len().saturating_sub(SHOWN);
    fitting.truncate(SHOWN);
    Listed {
        shown: fitting,
        more,
        any,
    }
}

/// The rows of the list for the open form, when it is a portal's: a button a match ("[x]" the
/// chosen one), a line for the rest, or why there is none.
#[must_use]
fn rows(editor: &Editor, data: &Data) -> Vec<Row> {
    let Some((form, def)) = editor.form.as_ref().zip(editor.def()) else {
        return Vec::new();
    };
    if form.kind != FormKind::Portal {
        return Vec::new();
    }
    let listed = matches(form, &def, data);
    let mut rows = Vec::new();
    if listed.shown.is_empty() {
        rows.push(Row::text(Span::body(if listed.any {
            "No portal matches."
        } else {
            "No portal to link: stamp one on any map."
        })));
    }
    for (n, m) in listed.shown.iter().enumerate() {
        let mark = if m.chosen { "[x]" } else { "[ ]" };
        rows.push(Row::controls(vec![Control::button(
            ControlId::Edit(EditAct::Link(u16::try_from(n).unwrap_or(u16::MAX))),
            format!("{mark} {}", m.label),
        )]));
    }
    if listed.more > 0 {
        rows.push(Row::text(Span::body(format!(
            "and {} more: type to narrow",
            listed.more
        ))));
    }
    rows
}

/// The form's `rows` with the list spliced in under the link field's row (a portal form),
/// as they are.
#[must_use]
pub fn with_list(editor: &Editor, data: &Data, mut rows: Vec<Row>) -> Vec<Row> {
    let list = self::rows(editor, data);
    if list.is_empty() {
        return rows;
    }
    let field = editor
        .form
        .as_ref()
        .and_then(|f| f.fields.iter().position(|f| f.label == "link"))
        .map(|i| ControlId::Edit(EditAct::Field(u8::try_from(i).unwrap_or(u8::MAX))));
    let at = rows
        .iter()
        .position(|r| r.controls.iter().any(|c| Some(c.id) == field))
        .map_or(rows.len(), |i| i + 1);
    let tail = rows.split_off(at);
    rows.extend(list);
    rows.extend(tail);
    rows
}

impl Editor {
    /// Row `n` of the list pressed: the field takes its id, or lets it go when that is its text.
    pub(super) fn link(&mut self, n: u16, data: &Data) {
        let text = {
            let Some((form, def)) = self.form.as_ref().zip(self.def()) else {
                return;
            };
            let listed = matches(form, &def, data);
            let Some(m) = listed.shown.get(usize::from(n)) else {
                return;
            };
            if m.chosen {
                String::new()
            } else {
                m.id.clone()
            }
        };
        self.fill_link(text);
    }

    /// The open portal form's Unlink: its link field cleared.
    pub(super) fn unlink(&mut self) {
        self.fill_link(String::new());
    }

    /// The link field set by the model: drawn anew.
    fn fill_link(&mut self, text: String) {
        if let Some(form) = &mut self.form
            && let Some(i) = form.fields.iter().position(|f| f.label == "link")
        {
            form.set_text(i, text);
            self.revision = self.revision.wrapping_add(1);
        }
    }
}
