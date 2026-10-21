//! The save that does not load (M8.5 7h4): what the loaded packs make of it, from the title,
//! the pause or the fallen party's screen. It opens only when the verdict refuses the load (a
//! name the packs lack, or a save that keeps no names on other packs); a save that loads, loads
//! at once. It lists the packs that changed, then what is missing and where, and closes. Its
//! words are ASCII: the built-in face has no arrow or ellipsis (owner's walk, 2026-10-19).
//!
//! The Debug page's **Reload packs** opens it too (M8.5 7h5), for fresh packs that lack a name
//! the running world holds: the reload is refused and nothing changed.
//!
//! A save or a running world the packs' migrations change opens it to ask (M8.5 7h6): it lists
//! each step, and **Apply and load** (or **Apply and reload**) applies them; Close changes
//! nothing.

use super::{Control, ControlId, Effect, Forms, HostOp, Panel, Row, Screen, ScreenId, Span};
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
    moved_on: &'static str,
    asks: &'static str,
    apply: &'static str,
    was: &'static str,
    now: &'static str,
    lists: &'static str,
}

const SAVE: Words = Words {
    title: "This save does not load",
    lacks: "It names what the loaded packs no longer have.",
    moved_on: "These packs moved on",
    asks: "The packs changed since this save, and their migrations change it. Apply them to load it; the world moves on.",
    apply: "Apply and load",
    was: "saved",
    now: "loaded",
    lists: "save_check",
};

const RELOAD: Words = Words {
    title: "These packs do not load",
    lacks: "The world names what the reloaded packs no longer have. Nothing changed.",
    moved_on: "These packs moved on",
    asks: "The reloaded packs' migrations change the running world. Apply them to reload; nothing changes until then.",
    apply: "Apply and reload",
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
    let asks = forms
        .load_report
        .as_ref()
        .is_some_and(|r| r.check.verdict == Verdict::NeedsMigration);
    if let Some(Refusal { check, .. }) = &forms.load_report {
        rows.push(Row::text(Span::alert(match check.verdict {
            Verdict::Unnamed => {
                "Made with other packs before saves kept names: a forced load through the dev tools loads it as written."
            }
            Verdict::NeedsMigration => words.asks,
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
        if asks {
            let steps: Vec<String> = check
                .migrations
                .iter()
                .flat_map(|m| {
                    m.steps
                        .iter()
                        .map(move |step| format!("{} {}: {step}", m.pack, m.file))
                })
                .collect();
            rows.push(Row::text(Span::body("Migrations:")));
            listed(&mut rows, words, steps.into_iter());
            if !check.places.is_empty() {
                rows.push(Row::text(Span::body("Then, places:")));
                listed(
                    &mut rows,
                    words,
                    check.places.iter().map(ToString::to_string),
                );
            }
        }
    }
    let mut controls = Vec::new();
    if asks {
        controls.push(Control::button(ControlId::Apply, words.apply));
    }
    controls.push(Control::button(ControlId::Close, "Close"));
    rows.push(Row::controls(controls));
    Screen {
        id: ScreenId::LoadReport,
        title: Span::title(if asks { words.moved_on } else { words.title }),
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

/// Close drops the report and goes back to the screen that asked for the load or the reload;
/// Apply, offered when the packs' migrations change the save or the world, loads or reloads
/// again with them (the host closes this screen).
pub fn apply(forms: &mut Forms, control: ControlId) -> Effect {
    match control {
        ControlId::Close => {
            forms.load_report = None;
            Effect::Back
        }
        ControlId::Apply => match forms.load_report.take() {
            Some(r) if r.check.verdict == Verdict::NeedsMigration => Effect::Host(if r.reload {
                HostOp::Reload { migrate: true }
            } else {
                HostOp::Load { migrate: true }
            }),
            other => {
                forms.load_report = other;
                Effect::None
            }
        },
        _ => Effect::None,
    }
}
