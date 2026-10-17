//! The verbs (M8.5 step 2b; presentation-ARCHITECTURE §6.7): everything the party can do with
//! what is in reach, one button each, opened from the square's Use when there is more than one
//! thing to do or the one thing is refused. The faced door comes first, then each object in the
//! order Use reaches them (`ObjectsView`): its name and state, then a row per interaction its
//! state offers, dim with the view's refusal (shared with `Act`, so they agree) or a trial's,
//! its checks and minutes after. A verb sends its command and the screen stays open, so the next
//! thing (open, then search) is one press on.

use super::{Control, ControlId, Effect, Game, Key, Panel, Row, Screen, ScreenId, Span};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::{ObjectView, VerbView};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Rejection};

/// A small dim note.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// One button of the screen: what it says and sends, and why not.
struct Entry {
    label: String,
    command: Command,
    why_not: Option<String>,
    after: Vec<String>,
}

/// The engine's refusal of `command`, worded short; the view's when it gives one.
fn why_not(data: &Data, game: Game, view: Option<&Rejection>, command: &Command) -> Option<String> {
    let refusal = view
        .cloned()
        .or_else(|| game.world.trial(data, command).err())?;
    Some(labels::refused(data, &game.views.party, &refusal))
}

/// The faced door's button, when the party faces one.
fn door(data: &Data, game: Game) -> Option<Entry> {
    let open = game.world.faced_door(data)?;
    let command = Command::Door;
    Some(Entry {
        label: if open {
            "Close the door"
        } else {
            "Open the door"
        }
        .to_owned(),
        why_not: why_not(data, game, None, &command),
        command,
        after: Vec::new(),
    })
}

/// An object's interaction's button.
fn verb(data: &Data, game: Game, object: &ObjectView, v: &VerbView) -> Entry {
    let command = Command::Act {
        object: object.object.clone(),
        verb: v.verb,
        option: v.option,
    };
    let mut after: Vec<String> = v
        .checks
        .iter()
        .map(|c| labels::check(c.kind, c.dc))
        .collect();
    if v.minutes > 0 {
        after.push(format!("{} min", v.minutes));
    }
    Entry {
        label: v.label.as_deref().map_or_else(
            || labels::verb(v.verb),
            |key| labels::text(data, key).to_owned(),
        ),
        why_not: why_not(data, game, v.refusal.as_ref(), &command),
        command,
        after,
    }
}

/// Every button in screen order, each with the object it belongs to (none for the door).
fn entries<'g>(data: &Data, game: Game<'g>) -> Vec<(Option<&'g ObjectView>, Entry)> {
    let mut out: Vec<_> = door(data, game).map(|d| (None, d)).into_iter().collect();
    for object in &game.views.objects.objects {
        out.extend(
            object
                .verbs
                .iter()
                .map(|v| (Some(object), verb(data, game, object, v))),
        );
    }
    out
}

/// The screen: Close, then the door and each object with its buttons.
#[must_use]
pub fn content(data: &Data, game: Game) -> Screen {
    let mut rows = vec![Row::controls(vec![Control::button(
        ControlId::Close,
        "Close",
    )])];
    let entries = entries(data, game);
    if entries.is_empty() {
        rows.push(Row::text(Span::body("Nothing within reach")));
    }
    let mut keys = Vec::new();
    let mut heading: Option<Option<&str>> = None;
    for (at, (object, entry)) in (0u8..).zip(entries) {
        let this = object.map(|o| o.object.as_str());
        if heading != Some(this) {
            heading = Some(this);
            rows.push(match object {
                None => Row::text(Span::body("Door")),
                Some(o) => Row {
                    spans: vec![Span::body(labels::text(data, &o.name))],
                    after: (!o.state.is_empty())
                        .then(|| note(o.state.clone()))
                        .into_iter()
                        .collect(),
                    ..Row::default()
                },
            });
        }
        let id = ControlId::Verb(at);
        if at < 9 {
            keys.push((Key::Digit(at + 1), id));
        }
        rows.push(Row {
            controls: vec![Control {
                why_not: entry.why_not,
                ..Control::button(id, entry.label)
            }],
            after: entry.after.into_iter().map(note).collect(),
            ..Row::default()
        });
    }
    Screen {
        id: ScreenId::Verbs,
        title: Span::title("Within reach"),
        panels: vec![Panel { rows }],
        keys,
        message: None,
    }
}

/// A verb sends its command and the screen stays open; Close closes it. The control was found
/// live on the screen as it stands.
#[must_use]
pub fn apply(data: &Data, game: Game, control: ControlId) -> Effect {
    match control {
        ControlId::Verb(at) => {
            entries(data, game)
                .into_iter()
                .nth(usize::from(at))
                .map_or(Effect::None, |(_, e)| Effect::Command {
                    command: e.command,
                    close: false,
                })
        }
        _ => Effect::Back,
    }
}
