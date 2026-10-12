//! The settings (parity P8, presentation-ARCHITECTURE §10.5): the player's own choices, edited as a
//! draft and kept only when Apply is pressed (owner, 2026-10-11); Close and Esc drop the draft.
//! Opened from the title and from the pause. A scale larger than the window holds says how far it
//! fits (`settings::fits`); the interface draws at that. The settings are the client's: they never reach
//! the world or a save.

use super::{
    Control, ControlId, Ctx, Effect, Forms, Kind, Panel, Payload, Row, Screen, ScreenId, Span,
};
use crate::settings::{Face, Prefs, Quality, SCALE_MAX, SCALE_MIN, SCALE_STEP};
use crate::style::Role;

/// One setting on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    /// Reduced motion.
    Motion,
    /// The floor grid.
    Grid,
    /// The quality preset.
    Quality,
    /// The typeface.
    Font,
    /// The interface scale.
    Scale,
}

/// The scales offered, smallest first.
fn scales() -> impl Iterator<Item = u16> {
    (SCALE_MIN..=SCALE_MAX).step_by(usize::from(SCALE_STEP))
}

/// The draft shown: the one being edited, else the settings as applied.
#[must_use]
pub fn draft(ctx: &Ctx, forms: &Forms) -> Prefs {
    forms.settings.unwrap_or(ctx.prefs)
}

fn choice(field: Field, label: &str, options: Vec<String>, at: usize) -> Row {
    Row::controls(vec![Control {
        id: ControlId::Setting(field),
        label: label.to_owned(),
        kind: Kind::Choice { options, at },
        why_not: None,
    }])
}

fn words(options: &[&str]) -> Vec<String> {
    options.iter().map(|&o| o.to_owned()).collect()
}

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx, forms: &Forms) -> Screen {
    let d = draft(ctx, forms);
    let mut apply = Control::button(ControlId::Apply, "Apply");
    apply.why_not = (d == ctx.prefs).then(|| "nothing changed".to_owned());
    Screen {
        id: ScreenId::Settings,
        title: Span::title("SETTINGS"),
        panels: vec![Panel {
            rows: vec![
                choice(
                    Field::Motion,
                    "Reduced motion",
                    words(&["off", "on"]),
                    usize::from(d.reduced_motion),
                ),
                choice(
                    Field::Grid,
                    "Floor grid",
                    words(&["hidden", "shown"]),
                    usize::from(d.grid),
                ),
                choice(
                    Field::Quality,
                    "Quality",
                    Quality::ALL.iter().map(|q| q.word().to_owned()).collect(),
                    Quality::ALL
                        .iter()
                        .position(|q| *q == d.quality)
                        .unwrap_or(0),
                ),
                choice(
                    Field::Font,
                    "Font",
                    Face::ALL.iter().map(|f| f.label().to_owned()).collect(),
                    Face::ALL.iter().position(|f| *f == d.font).unwrap_or(0),
                ),
                Row {
                    after: (d.scale > ctx.fits)
                        .then(|| Span {
                            role: Role::Dim,
                            ..Span::body(format!("{}% fits this window", ctx.fits))
                        })
                        .into_iter()
                        .collect(),
                    ..choice(
                        Field::Scale,
                        "Interface scale",
                        scales().map(|s| format!("{s}%")).collect(),
                        scales().position(|s| s == d.scale).unwrap_or(0),
                    )
                },
                Row::controls(vec![apply, Control::button(ControlId::Close, "Close")]),
            ],
        }],
        keys: Vec::new(),
        message: None,
    }
}

/// `options` stepped once from `at`: left goes back, a press or right goes on, round.
fn step<T: Copy + PartialEq>(options: &[T], now: T, payload: &Payload) -> T {
    let n = options.len();
    let at = options.iter().position(|o| *o == now).unwrap_or(0);
    options[match payload {
        Payload::Left => (at + n - 1) % n,
        _ => (at + 1) % n,
    }]
}

/// What each control asks of the shell.
#[must_use]
pub fn apply(ctx: &Ctx, forms: &mut Forms, control: ControlId, payload: &Payload) -> Effect {
    let mut d = draft(ctx, forms);
    match control {
        ControlId::Setting(field) => {
            match field {
                Field::Motion => d.reduced_motion = step(&[false, true], d.reduced_motion, payload),
                Field::Grid => d.grid = step(&[false, true], d.grid, payload),
                Field::Quality => d.quality = step(&Quality::ALL, d.quality, payload),
                Field::Font => d.font = step(&Face::ALL, d.font, payload),
                Field::Scale => {
                    let all: Vec<u16> = scales().collect();
                    d.scale = step(&all, d.scale, payload);
                }
            }
            forms.settings = Some(d);
            Effect::None
        }
        ControlId::Apply => Effect::Settings(d),
        ControlId::Close => Effect::Back,
        _ => Effect::None,
    }
}
