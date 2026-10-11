//! Tactics (parity P6; the other client's tactics panel): one member's declared reactions, in
//! the order they are considered, with Edit and Remove; the reactions switch; and a form that
//! writes a reaction: an action the member could declare (`TacticsView.answers`), the trigger
//! it answers, and conditions joined by all or any. Opened over the sheet outside a fight, the
//! party column beside it; Close and Esc go back to the sheet. A reaction set deeper than the
//! form writes (over the dev socket) loads locked: it may be removed here, not edited. Every
//! command is lit by a trial; the screen stays open with the outcome.

use super::inventory::{owner, owner_choice, step_owner};
use super::tactics_draft::{
    Choices, Draft, Field, Kind as Condition, action_name, criteria_text, trigger_name,
};
use super::{
    Control, ControlId, Effect, Forms, Game, Kind, Panel, Payload, Row, Screen, ScreenId, Span,
    stepped,
};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::{Criteria, CriteriaSet, MemberView, Named, TacticsCommand, TacticsView};
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_rules::tactics::{CRITERIA_NODES, RUNBOOK_ENTRIES, TACTICS_NAME_BYTES};
use omnis_sim::{Command, PartyCommand};

/// The conditions one reaction may list: a flat list under one node (`CRITERIA_NODES`).
pub const CONDITIONS_MAX: usize = CRITERIA_NODES - 1;

/// Why the form is dim when it holds a reaction it cannot write.
pub const LOCKED: &str = "Set elsewhere: it can be removed here, not edited";

/// A control of the tactics screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TacticAct {
    /// The reactions switch.
    Reactions,
    /// Load a declared reaction into the form, by row.
    Edit(u8),
    /// Remove a declared reaction, by row.
    Remove(u8),
    /// The action, a row of the answers.
    Action,
    /// The trigger, a row of the action's triggers.
    Trigger,
    /// All or any of the conditions.
    Join,
    /// Add a condition.
    Add,
    /// A condition's kind, by condition row.
    Kind(u8),
    /// One of a condition's fields: condition row, then the field's place in its kind's fields.
    Field(u8, u8),
    /// A condition's number.
    Number(u8),
    /// Drop a condition.
    Drop(u8),
    /// Declare the reaction: as a new row, or over the one being edited.
    Save,
    /// Start a new reaction.
    New,
}

/// The reaction being written, and whose.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TacticsForm {
    /// Whose tactics; a form for another member reads as a new one.
    pub member: Option<CharacterId>,
    /// The declared row the form writes over; `None` writes a new row.
    pub editing: Option<u8>,
    /// Row in the member's answers.
    pub action: usize,
    /// Row in the chosen answer's triggers.
    pub trigger: usize,
    /// Any rather than all.
    pub any: bool,
    /// The conditions.
    pub conditions: Vec<Draft>,
    /// The row loaded was set deeper than this form writes: shown, not saved.
    pub locked: bool,
}

impl TacticsForm {
    /// A new reaction for `member`.
    #[must_use]
    pub fn new(member: CharacterId) -> Self {
        TacticsForm {
            member: Some(member),
            ..TacticsForm::default()
        }
    }

    /// The criteria the conditions make: always when there are none.
    fn criteria(&self, choices: &Choices) -> Option<Criteria<Named>> {
        let list = self
            .conditions
            .iter()
            .map(|d| d.predicate(choices).map(Criteria::Is))
            .collect::<Option<Vec<_>>>()?;
        Some(match (list.is_empty(), self.any) {
            (true, _) => Criteria::Always,
            (false, false) => Criteria::All(list),
            (false, true) => Criteria::Any(list),
        })
    }

    /// The set Save declares; `None` when the form cannot make one.
    #[must_use]
    pub fn set(
        &self,
        data: &Data,
        view: &TacticsView,
        choices: &Choices,
    ) -> Option<CriteriaSet<Named>> {
        let answer = view.answers.get(self.action)?;
        let trigger = *answer.triggers.get(self.trigger)?;
        Some(CriteriaSet {
            name: set_name(&action_name(data, &answer.action), trigger_name(trigger)),
            action: answer.action.clone(),
            trigger,
            when: self.criteria(choices)?,
        })
    }

    /// Load declared row `at`: its action and trigger, and its conditions when the tree is flat;
    /// otherwise locked.
    fn load(&mut self, view: &TacticsView, at: usize, choices: &Choices) {
        let Some(entry) = view.reactions.get(at) else {
            return;
        };
        let member = self.member;
        *self = TacticsForm {
            member,
            editing: u8::try_from(at).ok(),
            ..TacticsForm::default()
        };
        let action = view.answers.iter().position(|a| a.action == entry.action);
        let trigger = action.and_then(|a| {
            view.answers[a]
                .triggers
                .iter()
                .position(|t| *t == entry.trigger)
        });
        let flat = match &entry.when {
            Criteria::Always => Some((false, Vec::new())),
            Criteria::Is(p) => Some((false, vec![p])),
            Criteria::All(list) | Criteria::Any(list) => list
                .iter()
                .map(|c| match c {
                    Criteria::Is(p) => Some(p),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(|ps| (matches!(entry.when, Criteria::Any(_)), ps)),
        };
        let drafts = flat.and_then(|(any, ps)| {
            let drafts = ps
                .into_iter()
                .map(|p| Draft::of(p, choices))
                .collect::<Option<Vec<_>>>()?;
            Some((any, drafts))
        });
        match (action, trigger, drafts) {
            (Some(action), Some(trigger), Some((any, conditions))) => {
                (self.action, self.trigger, self.any, self.conditions) =
                    (action, trigger, any, conditions);
            }
            _ => self.locked = true,
        }
    }
}

/// A set's name: the action on the trigger, cut to the rules' limit at a character.
#[must_use]
pub fn set_name(action: &str, trigger: &str) -> String {
    let mut name = format!("{action} on {trigger}");
    while name.len() > TACTICS_NAME_BYTES {
        name.pop();
    }
    name
}

/// The form for `member`: the one kept, or a new one when it was another member's.
fn form_of(forms: &Forms, member: CharacterId) -> TacticsForm {
    if forms.tactics.member == Some(member) {
        forms.tactics.clone()
    } else {
        TacticsForm::new(member)
    }
}

/// A small dim note.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

fn act(act: TacticAct) -> ControlId {
    ControlId::Tactic(act)
}

/// A choice of `options` at `at`, dim with `why_not`.
fn choice(
    id: TacticAct,
    label: &str,
    options: Vec<String>,
    at: usize,
    why_not: Option<&str>,
) -> Control {
    Control {
        id: act(id),
        label: label.to_owned(),
        kind: Kind::Choice { options, at },
        why_not: why_not.map(str::to_owned),
    }
}

fn button(id: TacticAct, label: &str, why_not: Option<String>) -> Control {
    Control {
        why_not,
        ..Control::button(act(id), label)
    }
}

/// A tactics command as the engine takes it.
fn command(tactics: TacticsCommand) -> Command {
    Command::Party(PartyCommand::Tactics(tactics))
}

/// Why `command` would be refused, worded short.
fn tried(data: &Data, game: Game, command: &Command) -> Option<String> {
    game.world
        .trial(data, command)
        .err()
        .map(|r| labels::reason(&game.views.party, &r))
}

/// A declared row's line: "1. Shield on attacked, when me HP < 50%".
fn entry_line(data: &Data, view: &TacticsView, at: usize, choices: &Choices) -> String {
    view.reactions.get(at).map_or_else(String::new, |entry| {
        format!(
            "{}. {} on {}, when {}",
            at + 1,
            action_name(data, &entry.action),
            trigger_name(entry.trigger),
            criteria_text(&entry.when, choices)
        )
    })
}

/// The list of declared reactions, each with Edit and Remove.
fn list(data: &Data, game: Game, m: &MemberView, choices: &Choices) -> Vec<Row> {
    let view = &m.tactics;
    let mut rows = vec![Row::text(note(
        "Declared reactions, in the order they are considered",
    ))];
    if view.reactions.is_empty() {
        rows.push(Row::text(Span::body("None declared")));
    }
    for (i, entry) in view.reactions.iter().enumerate() {
        let Ok(at) = u8::try_from(i) else { break };
        let remove = command(TacticsCommand::RemoveReaction {
            member: m.member,
            entry: entry.entry,
        });
        rows.push(Row {
            spans: vec![Span::body(entry_line(data, view, i, choices))],
            controls: vec![
                button(TacticAct::Edit(at), "Edit", None),
                button(TacticAct::Remove(at), "Remove", tried(data, game, &remove)),
            ],
            after: Vec::new(),
        });
    }
    rows
}

/// One condition's row: its kind, its fields, its number, Drop.
fn condition_row(i: u8, draft: &Draft, choices: &Choices, lock: Option<&str>) -> Row {
    let mut controls = vec![choice(
        TacticAct::Kind(i),
        "If",
        Condition::ALL
            .iter()
            .map(|k| k.label().to_owned())
            .collect(),
        Condition::ALL
            .iter()
            .position(|k| *k == draft.kind)
            .unwrap_or(0),
        lock,
    )];
    for (j, field) in draft.kind.fields().iter().enumerate() {
        let label = match field {
            Field::Who => "Who",
            Field::Monster => "Monster",
            Field::Condition => "Condition",
            Field::Cmp => "Is",
            Field::Row => "Row",
        };
        controls.push(choice(
            TacticAct::Field(i, u8::try_from(j).unwrap_or(0)),
            label,
            choices.options(*field),
            draft.pick(*field),
            lock,
        ));
    }
    if let Some(max) = draft.kind.number_max() {
        controls.push(Control {
            id: act(TacticAct::Number(i)),
            label: "Number".to_owned(),
            kind: Kind::Number {
                value: i64::from(draft.n),
                min: 0,
                max: i64::from(max),
            },
            why_not: lock.map(str::to_owned),
        });
    }
    controls.push(button(TacticAct::Drop(i), "Drop", lock.map(str::to_owned)));
    Row::controls(controls)
}

/// The form: what it writes, the action and trigger, the conditions, Save and New.
fn form_rows(
    data: &Data,
    game: Game,
    m: &MemberView,
    form: &TacticsForm,
    choices: &Choices,
) -> Vec<Row> {
    let view = &m.tactics;
    let lock = form.locked.then_some(LOCKED);
    let mut rows = vec![Row::text(Span::body(match form.editing {
        Some(at) => format!("Editing reaction {}", at + 1),
        None => "New reaction".to_owned(),
    }))];
    if form.locked {
        rows.push(Row::text(Span::alert(LOCKED)));
    }
    match view.answers.get(form.action) {
        None => rows.push(Row {
            spans: vec![Span::body("Action")],
            controls: Vec::new(),
            after: vec![note("nothing to declare yet")],
        }),
        Some(answer) => rows.push(Row::controls(vec![
            choice(
                TacticAct::Action,
                "Action",
                view.answers
                    .iter()
                    .map(|a| action_name(data, &a.action))
                    .collect(),
                form.action,
                lock,
            ),
            choice(
                TacticAct::Trigger,
                "on",
                answer
                    .triggers
                    .iter()
                    .map(|t| trigger_name(*t).to_owned())
                    .collect(),
                form.trigger,
                lock,
            ),
        ])),
    }
    let add_why = lock.map(str::to_owned).or_else(|| {
        (form.conditions.len() >= CONDITIONS_MAX).then(|| "Fifteen conditions at most".to_owned())
    });
    rows.push(Row {
        spans: Vec::new(),
        controls: vec![
            choice(
                TacticAct::Join,
                "When",
                vec!["all of these".to_owned(), "any of these".to_owned()],
                usize::from(form.any),
                lock,
            ),
            button(TacticAct::Add, "Add condition", add_why),
        ],
        after: vec![note("hold (none: always)")],
    });
    for (i, draft) in form.conditions.iter().enumerate() {
        if let Ok(i) = u8::try_from(i) {
            rows.push(condition_row(i, draft, choices, lock));
        }
    }
    let save_why = if let Some(lock) = lock {
        Some(lock.to_owned())
    } else if let Some(set) = form.set(data, view, choices) {
        if form.editing.is_none() && view.reactions.len() >= RUNBOOK_ENTRIES {
            Some("Sixteen reactions at most".to_owned())
        } else {
            let put = TacticsCommand::PutReaction {
                member: m.member,
                entry: form.editing,
                set,
            };
            tried(data, game, &command(put))
        }
    } else {
        Some("Nothing this member can declare".to_owned())
    };
    rows.push(Row::controls(vec![
        button(TacticAct::Save, "Save", save_why),
        button(TacticAct::New, "New", None),
    ]));
    rows
}

/// The tactics screen as it stands.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    let Some(m) = owner(game, forms) else {
        return super::empty(ScreenId::Tactics);
    };
    let choices = Choices::from_data(data);
    let form = form_of(forms, m.member);
    let switch = command(TacticsCommand::SetReactions {
        member: m.member,
        on: !m.tactics.reactions_on,
    });
    let mut rows = vec![Row::controls(vec![
        owner_choice(game, m.member, "Member"),
        Control {
            id: act(TacticAct::Reactions),
            label: "Reactions".to_owned(),
            kind: Kind::Choice {
                options: vec!["off".to_owned(), "on".to_owned()],
                at: usize::from(m.tactics.reactions_on),
            },
            why_not: tried(data, game, &switch),
        },
        Control::button(ControlId::Close, "Close"),
    ])];
    rows.extend(list(data, game, m, &choices));
    let form_panel = form_rows(data, game, m, &form, &choices);
    Screen {
        id: ScreenId::Tactics,
        title: Span::title("Tactics"),
        panels: vec![Panel { rows }, Panel { rows: form_panel }],
        keys: Vec::new(),
        message: None,
    }
}

/// Step `at` among `len` options as `payload` asks (a press steps right).
fn step(at: usize, len: usize, payload: &Payload) -> usize {
    if len == 0 {
        return 0;
    }
    match payload {
        Payload::Left => (at + len - 1) % len,
        _ => (at + 1) % len,
    }
}

/// Use a control. The control was found live on the screen as it stands, so a locked form's
/// controls never reach here.
pub fn apply(
    data: &Data,
    game: Game,
    forms: &mut Forms,
    control: ControlId,
    payload: &Payload,
) -> Effect {
    let Some(m) = owner(game, forms) else {
        return Effect::None;
    };
    let member = m.member;
    let view = &m.tactics;
    let choices = Choices::from_data(data);
    let mut form = form_of(forms, member);
    let effect = match control {
        ControlId::Owner => {
            step_owner(game, forms, payload);
            return Effect::None;
        }
        ControlId::Close => return Effect::Back,
        ControlId::Tactic(TacticAct::Reactions) => Effect::Command {
            command: command(TacticsCommand::SetReactions {
                member,
                on: !view.reactions_on,
            }),
            close: false,
        },
        ControlId::Tactic(TacticAct::Edit(at)) => {
            form.load(view, usize::from(at), &choices);
            Effect::None
        }
        ControlId::Tactic(TacticAct::Remove(at)) => match view.reactions.get(usize::from(at)) {
            Some(entry) => {
                let entry = entry.entry;
                form = TacticsForm::new(member);
                Effect::Command {
                    command: command(TacticsCommand::RemoveReaction { member, entry }),
                    close: false,
                }
            }
            None => Effect::None,
        },
        ControlId::Tactic(TacticAct::Action) => {
            form.action = step(form.action, view.answers.len(), payload);
            form.trigger = 0;
            Effect::None
        }
        ControlId::Tactic(TacticAct::Trigger) => {
            let len = view
                .answers
                .get(form.action)
                .map_or(0, |a| a.triggers.len());
            form.trigger = step(form.trigger, len, payload);
            Effect::None
        }
        ControlId::Tactic(TacticAct::Join) => {
            form.any = !form.any;
            Effect::None
        }
        ControlId::Tactic(TacticAct::Add) => {
            if form.conditions.len() < CONDITIONS_MAX {
                form.conditions.push(Draft::default());
            }
            Effect::None
        }
        ControlId::Tactic(TacticAct::Kind(i)) => {
            if let Some(draft) = form.conditions.get_mut(usize::from(i)) {
                let at = Condition::ALL
                    .iter()
                    .position(|k| *k == draft.kind)
                    .unwrap_or(0);
                draft.kind = Condition::ALL[step(at, Condition::ALL.len(), payload)];
                draft.n = draft.n.min(draft.kind.number_max().unwrap_or(0));
            }
            Effect::None
        }
        ControlId::Tactic(TacticAct::Field(i, j)) => {
            if let Some(draft) = form.conditions.get_mut(usize::from(i))
                && let Some(field) = draft.kind.fields().get(usize::from(j)).copied()
            {
                let len = choices.options(field).len();
                draft.set(field, step(draft.pick(field), len, payload));
            }
            Effect::None
        }
        ControlId::Tactic(TacticAct::Number(i)) => {
            if let Some(draft) = form.conditions.get_mut(usize::from(i)) {
                let max = i64::from(draft.kind.number_max().unwrap_or(0));
                let n = stepped(i64::from(draft.n), 0, max, payload);
                draft.n = u16::try_from(n).unwrap_or(0);
            }
            Effect::None
        }
        ControlId::Tactic(TacticAct::Drop(i)) => {
            if usize::from(i) < form.conditions.len() {
                form.conditions.remove(usize::from(i));
            }
            Effect::None
        }
        ControlId::Tactic(TacticAct::Save) => match form.set(data, view, &choices) {
            Some(set) => {
                let entry = form.editing;
                form = TacticsForm::new(member);
                Effect::Command {
                    command: command(TacticsCommand::PutReaction { member, entry, set }),
                    close: false,
                }
            }
            None => Effect::None,
        },
        ControlId::Tactic(TacticAct::New) => {
            form = TacticsForm::new(member);
            Effect::None
        }
        _ => Effect::None,
    };
    forms.tactics = form;
    effect
}
