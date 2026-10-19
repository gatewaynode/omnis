//! The save that does not load (M8.5 7h4): what the loaded packs make of it, from the title,
//! the pause or the fallen party's screen. It opens only when the verdict refuses the load (a
//! name the packs lack, or a save that keeps no names on other packs); a save that loads, loads
//! at once. It lists the packs that changed, then what is missing and where, and closes.

use super::{Control, ControlId, Effect, Forms, Panel, Row, Screen, ScreenId, Span};
use omnis_sim::api::{PackStamp, SaveCheck, Verdict};

/// How many rows of each list are shown; the rest are counted (`save_check` lists them all).
pub const SHOWN: usize = 12;

fn version(stamp: Option<&PackStamp>) -> &str {
    stamp.map_or("none", |s| s.version.as_str())
}

/// The first rows of a list, then how many more.
fn listed(rows: &mut Vec<Row>, lines: impl ExactSizeIterator<Item = String>) {
    let n = lines.len();
    rows.extend(lines.take(SHOWN).map(|l| Row::text(Span::body(l))));
    if n > SHOWN {
        rows.push(Row::text(Span::body(format!(
            "…and {} more; save_check lists them all",
            n - SHOWN
        ))));
    }
}

/// The screen, from the report the last Load left.
#[must_use]
pub fn content(forms: &Forms) -> Screen {
    let mut rows = Vec::new();
    if let Some(check) = &forms.load_report {
        rows.push(Row::text(Span::alert(match check.verdict {
            Verdict::Unnamed => {
                "Made with other packs before saves kept names: a forced load through the dev tools loads it as written."
            }
            _ => "It names what the loaded packs no longer have.",
        })));
        packs(&mut rows, check);
        if !check.missing.is_empty() {
            rows.push(Row::text(Span::body("Missing:")));
            listed(&mut rows, check.missing.iter().map(ToString::to_string));
        }
    }
    rows.push(Row::controls(vec![Control::button(
        ControlId::Close,
        "Close",
    )]));
    Screen {
        id: ScreenId::LoadReport,
        title: Span::title("This save does not load"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// The packs that changed: each as saved, then as loaded.
fn packs(rows: &mut Vec<Row>, check: &SaveCheck) {
    let changed: Vec<String> = check
        .packs
        .iter()
        .filter(|p| p.saved != p.loaded)
        .map(|p| {
            format!(
                "{}: {} → {}",
                p.id,
                version(p.saved.as_ref()),
                version(p.loaded.as_ref())
            )
        })
        .collect();
    if !changed.is_empty() {
        rows.push(Row::text(Span::body("Packs, saved → loaded:")));
        listed(rows, changed.into_iter());
    }
}

/// Close drops the report and goes back to the screen that asked for the load.
pub fn apply(forms: &mut Forms, control: ControlId) -> Effect {
    match control {
        ControlId::Close => {
            forms.load_report = None;
            Effect::Back
        }
        _ => Effect::None,
    }
}
