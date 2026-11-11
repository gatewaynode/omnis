//! The rules list (rule priority, owner 2026-11-11): a modal over the editor naming every slot,
//! value and table the loaded packs define, the file and pack whose definition stands and its
//! priority, and under a rule that beat others each definition it beat. A search narrows the
//! list; a rule's file opens as text when the pack being edited wrote it. Read from
//! `Data::standings`, which the loader fills as it merges the rules files by priority.

use super::{Control, ControlId, Ctx, Effect, Kind, Panel, Payload, Row, Screen, ScreenId, Span};
use crate::editor::EditAct;

/// The search field's limit in bytes.
const FILTER_MAX: usize = 64;

/// The screen.
#[must_use]
pub fn content(ctx: &Ctx) -> Screen {
    let edit = |act| ControlId::Edit(act);
    let (filter, pack) = ctx.editing.map_or(("", ""), |e| (e.rules_filter, e.pack));
    let wanted = filter.to_lowercase();
    let standings = &ctx.data.standings;
    let overriding = standings.values().filter(|s| !s.beats.is_empty()).count();
    let mut rows = vec![
        Row::controls(vec![Control {
            kind: Kind::Text {
                value: filter.to_owned(),
                max: FILTER_MAX,
                lines: 1,
            },
            ..Control::button(edit(EditAct::RulesFilter), "search")
        }]),
        Row::text(Span::body(format!(
            "{} rules, {overriding} overriding another",
            standings.len()
        ))),
    ];
    let mut shown = 0;
    for (n, ((kind, name), stands)) in standings.iter().enumerate() {
        let matches = [kind.word(), name, &stands.file, &stands.pack]
            .iter()
            .any(|text| text.to_lowercase().contains(&wanted))
            || (wanted == "overrides" && !stands.beats.is_empty());
        if !matches {
            continue;
        }
        shown += 1;
        let mark = if stands.beats.is_empty() {
            ""
        } else {
            "OVERRIDES  "
        };
        rows.push(Row::text(Span::body(format!("{mark}{kind} {name}"))));
        let mut file = Control::button(
            edit(EditAct::RuleFile(u16::try_from(n).unwrap_or(u16::MAX))),
            format!("{}  priority {}", stands.file, stands.priority),
        );
        if stands.pack != pack {
            file.why_not = Some(format!(
                "the pack {}'s, not the one being edited",
                stands.pack
            ));
        }
        rows.push(Row::controls(vec![file]));
        for beaten in &stands.beats {
            rows.push(Row::text(Span::body(format!(
                "  over {}  priority {}",
                beaten.file, beaten.priority
            ))));
        }
    }
    if shown == 0 {
        rows.push(Row::text(Span::body("No rule matches.")));
    }
    rows.push(Row::controls(vec![Control::button(
        ControlId::Resume,
        "Close",
    )]));
    Screen {
        id: ScreenId::EditorRules,
        title: Span::title("RULES"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// A control used: Close goes back; the search and a rule's file are the editor's acts.
#[must_use]
pub fn apply(control: ControlId, payload: &Payload) -> Effect {
    match control {
        ControlId::Resume => Effect::Back,
        ControlId::Edit(act) => Effect::Editor(act, payload.clone()),
        _ => Effect::None,
    }
}
