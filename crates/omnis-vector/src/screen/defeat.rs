//! The fallen party (presentation-ARCHITECTURE §6.3, §6.7): when no member can fight, the party
//! walks no further and this screen offers the ways on: the last save, the same game again, or
//! the title. Monsters met or fought have the fight screen (`shell/combat.rs`).

use super::{Control, ControlId, Ctx, Effect, Game, HostOp, Panel, Row, Screen, ScreenId, Span};

/// How many of the fight's last lines the screen repeats.
const LAST_LINES: usize = 4;

/// The screen: how it ended, then Load last save (dim when there is none), Start again, Back to
/// editor in a playtest (M5.8), and Quit to title.
#[must_use]
pub fn content(ctx: &Ctx, game: Game) -> Screen {
    let mut rows: Vec<Row> = game.log[game.log.len().saturating_sub(LAST_LINES)..]
        .iter()
        .map(|line| Row::text(Span::body(line.clone())))
        .collect();
    if rows.is_empty() {
        rows.push(Row::text(Span::body("Every member is down.")));
    }
    let mut load = Control::button(ControlId::Load, "Load last save");
    load.why_not = ctx.save.missing();
    if ctx.playtest {
        load.why_not = Some("a playtest is not loaded".to_owned());
    }
    let mut controls = vec![load, Control::button(ControlId::StartAgain, "Start again")];
    if ctx.playtest {
        controls.push(Control::button(ControlId::BackToEditor, "Back to editor"));
    }
    controls.push(Control::button(ControlId::QuitToTitle, "Quit to title"));
    rows.push(Row::controls(controls));
    Screen {
        id: ScreenId::Defeat,
        title: Span::title("The party has fallen"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// What each control asks of the shell.
#[must_use]
pub fn apply(control: ControlId) -> Effect {
    match control {
        ControlId::Load => Effect::Host(HostOp::Load { migrate: false }),
        ControlId::StartAgain => Effect::Host(HostOp::Restart),
        ControlId::QuitToTitle => Effect::ToTitle,
        ControlId::BackToEditor => Effect::ToEditor,
        _ => Effect::None,
    }
}
