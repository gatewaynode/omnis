//! The game paused (presentation-ARCHITECTURE §6.3, §6.7, from the retired client's pause): the
//! game's seed and settings, then Resume, Save, Load, Save log, Settings, Debug (dim outside a
//! devtools world) and Quit to title, and Quit in its own row, set apart from the rest (PRD §7). A
//! control that cannot be used now says why: the save rule forbids saving here, there is no save
//! to load, the log no longer replays. In a playtest from the editor (M5.8) nothing is saved or
//! loaded and the way out is Back to editor.

use super::{Control, ControlId, Ctx, Effect, Game, HostOp, Panel, Row, Screen, ScreenId, Span};
use crate::labels;

const fn on(flag: bool) -> &'static str {
    if flag { "on" } else { "off" }
}

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx, game: Game) -> Screen {
    let here = &game.views.here;
    let s = here.settings;
    let mut save = Control::button(ControlId::Save, "Save");
    save.why_not = (!here.may_save).then(|| "The save rule forbids saving here".to_owned());
    let mut load = Control::button(ControlId::Load, "Load");
    load.why_not = ctx.save.missing();
    let mut log = Control::button(ControlId::SaveLog, "Save log");
    log.why_not = game
        .unreplayable
        .map(|why| format!("the log no longer replays: {why}"));
    if ctx.playtest {
        // A playtest is the editor's: nothing of it is kept (M5.8).
        save.why_not = Some("a playtest is not saved".to_owned());
        load.why_not = Some("a playtest is not loaded".to_owned());
        log.why_not = Some("a playtest's log is not saved".to_owned());
    }
    let leave = if ctx.playtest {
        Control::button(ControlId::BackToEditor, "Back to editor")
    } else {
        Control::button(ControlId::QuitToTitle, "Quit to title")
    };
    Screen {
        id: ScreenId::Pause,
        title: Span::title("PAUSED"),
        panels: vec![Panel {
            rows: vec![
                Row::text(Span::body(format!("Seed {}", here.seed))),
                Row::text(Span::body(format!(
                    "Saving: {}",
                    labels::save_rule(s.save_rule)
                ))),
                Row::text(Span::body(format!("Permadeath: {}", on(s.permadeath)))),
                Row::text(Span::body(format!("Devtools: {}", on(s.devtools)))),
                Row::controls(vec![
                    Control::button(ControlId::Resume, "Resume"),
                    save,
                    load,
                    log,
                    Control::button(ControlId::Settings, "Settings"),
                    Control {
                        why_not: super::debug::unavailable(game.views),
                        ..Control::button(ControlId::OpenDebug, "Debug")
                    },
                ]),
                Row::controls(vec![leave]),
                Row::controls(vec![Control::button(ControlId::Quit, "Quit")]),
            ],
        }],
        keys: Vec::new(),
        message: None,
    }
}

/// What each control asks of the shell.
#[must_use]
pub fn apply(control: ControlId) -> Effect {
    match control {
        ControlId::Resume => Effect::Back,
        ControlId::Save => Effect::Host(HostOp::Save),
        ControlId::Load => Effect::Host(HostOp::Load { migrate: false }),
        ControlId::SaveLog => Effect::Host(HostOp::SaveLog),
        ControlId::Settings => Effect::Open(ScreenId::Settings),
        ControlId::OpenDebug => Effect::Open(ScreenId::Debug),
        ControlId::QuitToTitle => Effect::ToTitle,
        ControlId::BackToEditor => Effect::ToEditor,
        ControlId::Quit => Effect::Quit,
        _ => Effect::None,
    }
}
