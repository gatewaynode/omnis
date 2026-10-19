//! The save that does not load (M8.5 7h4): what the loaded packs make of it, from the title,
//! the pause or the fallen party's screen. It opens only when the verdict refuses the load (a
//! name the packs lack, or a save that keeps no names on other packs); a save that loads, loads
//! at once. It lists the packs that changed, then what is missing and where, and closes. Its
//! words are ASCII: the built-in face has no arrow or ellipsis (owner's walk, 2026-10-19).
//!
//! The Debug page's **Reload packs** opens it too (M8.5 7h5), for fresh packs that lack a name
//! the running world holds: the reload is refused and nothing changed.

use super::{Control, ControlId, Effect, Forms, Panel, Row, Screen, ScreenId, Span};
use omnis_sim::api::{PackStamp, SaveCheck, Verdict};

/// How many rows of each list are shown; the rest are counted (`save_check` lists them all).
pub const SHOWN: usize = 12;

/// What the screen reports: a refused load or a refused reload, and what the packs made of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The report.
    pub check: SaveCheck,
    /// Packs reloaded under the running world, not a save loaded.
    pub reload: bool,
}

/// The words that differ between a save and a reload.
struct Words {
    title: &'static str,
    lacks: &'static str,
    was: &'static str,
    now: &'static str,
    lists: &'static str,
}

const SAVE: Words = Words {
    title: "This save does not load",
    lacks: "It names what the loaded packs no longer have.",
    was: "saved",
    now: "loaded",
    lists: "save_check",
};

const RELOAD: Words = Words {
    title: "These packs do not load",
    lacks: "The world names what the reloaded packs no longer have. Nothing changed.",
    was: "running",
    now: "reloaded",
    lists: "pack_reload",
};

fn version(stamp: Option<&PackStamp>) -> &str {
    stamp.map_or("none", |s| s.version.as_str())
}

/// The first rows of a list, then how many more.
fn listed(rows: &mut Vec<Row>, words: &Words, lines: impl ExactSizeIterator<Item = String>) {
    let n = lines.len();
    rows.extend(lines.take(SHOWN).map(|l| Row::text(Span::body(l))));
    if n > SHOWN {
        rows.push(Row::text(Span::body(format!(
            "and {} more; {} lists them all",
            n - SHOWN,
            words.lists
        ))));
    }
}

/// The screen, from the report the last Load left.
#[must_use]
pub fn content(forms: &Forms) -> Screen {
    let mut rows = Vec::new();
    let words = match &forms.load_report {
        Some(r) if r.reload => &RELOAD,
        _ => &SAVE,
    };
    if let Some(Refusal { check, .. }) = &forms.load_report {
        rows.push(Row::text(Span::alert(match check.verdict {
            Verdict::Unnamed => {
                "Made with other packs before saves kept names: a forced load through the dev tools loads it as written."
            }
            _ => words.lacks,
        })));
        packs(&mut rows, words, check);
        if !check.missing.is_empty() {
            rows.push(Row::text(Span::body("Missing:")));
            listed(
                &mut rows,
                words,
                check.missing.iter().map(ToString::to_string),
            );
        }
    }
    rows.push(Row::controls(vec![Control::button(
        ControlId::Close,
        "Close",
    )]));
    Screen {
        id: ScreenId::LoadReport,
        title: Span::title(words.title),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// The packs that changed: each as saved (or running), then as loaded (or reloaded).
fn packs(rows: &mut Vec<Row>, words: &Words, check: &SaveCheck) {
    let changed: Vec<String> = check
        .packs
        .iter()
        .filter(|p| p.saved != p.loaded)
        .map(|p| {
            format!(
                "{}: {} {}, {} {}",
                p.id,
                words.was,
                version(p.saved.as_ref()),
                words.now,
                version(p.loaded.as_ref())
            )
        })
        .collect();
    if !changed.is_empty() {
        rows.push(Row::text(Span::body("Packs changed:")));
        listed(rows, words, changed.into_iter());
    }
}

/// Close drops the report and goes back to the screen that asked for the load or the reload.
pub fn apply(forms: &mut Forms, control: ControlId) -> Effect {
    match control {
        ControlId::Close => {
            forms.load_report = None;
            Effect::Back
        }
        _ => Effect::None,
    }
}
