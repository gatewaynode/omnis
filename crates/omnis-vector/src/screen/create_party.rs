//! Party creation (presentation-ARCHITECTURE §6.3, §6.7; from the other client's creation
//! panel): a new game's party is made one member at a time. The draft's name, race, class,
//! background and alignment on the left, with the class's skill picks; the six bought scores on
//! the right with their cost, the points spent and the party so far; then Add member, Begin and
//! Back. What a draft may be comes from the engine's creation view, never from the rules
//! (VA9): the score range, the costs, the picks and what a race or background grants. Whether
//! Add would be accepted is a trial of the command itself, so the reason shown is the engine's
//! first, in the words this screen uses.

use super::{
    Control, ControlId, Effect, Game, HostOp, Kind, Panel, Payload, Row, Screen, ScreenId, Span,
    stepped,
};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::{CreationView, Draft};
use omnis_sim::omnis_data::{Ability, Data, Skill};
use omnis_sim::{Command, PartyCommand};

/// The next member's draft, as the player has it so far. Indices and scores are held inside
/// what the creation view allows when read, so a fresh form starts at the first of each and
/// every score at the lowest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreateForm {
    /// The name as typed.
    pub name: String,
    /// The race, by place in the view's list.
    pub race: usize,
    /// The class, likewise.
    pub class: usize,
    /// The background, likewise.
    pub background: usize,
    /// The alignment, likewise.
    pub alignment: usize,
    /// The bought scores, SRD order.
    pub scores: [u8; 6],
    /// The class's skills picked, in the order picked.
    pub picks: Vec<Skill>,
}

/// Place `at` held inside a list of `len`.
fn within(at: usize, len: usize) -> usize {
    at.min(len.saturating_sub(1))
}

/// One step along a list of `len`, wrapping; `Left` steps back.
fn step(at: usize, len: usize, payload: &Payload) -> usize {
    if len == 0 {
        return 0;
    }
    let at = within(at, len);
    match payload {
        Payload::Left => (at + len - 1) % len,
        _ => (at + 1) % len,
    }
}

impl CreateForm {
    /// Bought score `i`, inside the view's range.
    #[must_use]
    pub fn score(&self, i: usize, view: &CreationView) -> u8 {
        self.scores[i].clamp(view.score_min, view.score_max)
    }

    /// What the six scores cost, from the view's table.
    #[must_use]
    pub fn spent(&self, view: &CreationView) -> i64 {
        (0..6)
            .map(|i| {
                let at = usize::from(self.score(i, view) - view.score_min);
                view.costs.get(at).copied().unwrap_or(0)
            })
            .sum()
    }

    /// The skills the draft's race and background already give, each with where it comes from.
    fn granted(&self, view: &CreationView) -> Vec<(Skill, &'static str)> {
        let mut out = Vec::new();
        if let Some(r) = view.races.get(within(self.race, view.races.len())) {
            out.extend(r.grants.iter().map(|s| (*s, "from the race")));
        }
        if let Some(b) = view
            .backgrounds
            .get(within(self.background, view.backgrounds.len()))
        {
            out.extend(b.grants.iter().map(|s| (*s, "from the background")));
        }
        out
    }

    /// Drop the picks the race or background now grants.
    fn drop_granted(&mut self, view: &CreationView) {
        let granted: Vec<Skill> = self.granted(view).into_iter().map(|(s, _)| s).collect();
        self.picks.retain(|s| !granted.contains(s));
    }

    /// The draft the form describes.
    #[must_use]
    pub fn draft(&self, view: &CreationView) -> Draft {
        let id = |list: Vec<&String>, at: usize| {
            list.get(within(at, list.len()))
                .map_or_else(String::new, |s| (*s).clone())
        };
        Draft {
            name: self.name.trim().to_owned(),
            race: id(view.races.iter().map(|r| &r.id).collect(), self.race),
            class: id(view.classes.iter().map(|c| &c.id).collect(), self.class),
            background: id(
                view.backgrounds.iter().map(|b| &b.id).collect(),
                self.background,
            ),
            alignment: view.alignments[within(self.alignment, view.alignments.len())],
            scores: core::array::from_fn(|i| self.score(i, view)),
            skills: self.picks.clone(),
        }
    }
}

/// A choice over `options`, at `at`.
fn choice(id: ControlId, label: &str, options: Vec<String>, at: usize) -> Control {
    let at = within(at, options.len());
    Control {
        id,
        label: label.to_owned(),
        kind: Kind::Choice { options, at },
        why_not: None,
    }
}

/// The screen.
#[must_use]
pub fn content(data: &Data, game: Game, form: &CreateForm) -> Screen {
    let view = &game.views.creation;
    let members = &game.views.party.members;
    let class = view.classes.get(within(form.class, view.classes.len()));
    let choose = class.map_or(0, |c| c.choose);
    let names = |keys: Vec<&String>| -> Vec<String> {
        keys.into_iter()
            .map(|k| labels::text(data, k).to_owned())
            .collect()
    };

    let mut left = vec![
        Row::controls(vec![Control {
            id: ControlId::Name,
            label: "Name".to_owned(),
            kind: Kind::Text {
                value: form.name.clone(),
                max: view.name_max,
            },
            why_not: None,
        }]),
        Row::controls(vec![choice(
            ControlId::Race,
            "Race",
            names(view.races.iter().map(|r| &r.name).collect()),
            form.race,
        )]),
        Row::controls(vec![choice(
            ControlId::Class,
            "Class",
            names(view.classes.iter().map(|c| &c.name).collect()),
            form.class,
        )]),
        Row::controls(vec![choice(
            ControlId::Background,
            "Background",
            names(view.backgrounds.iter().map(|b| &b.name).collect()),
            form.background,
        )]),
        Row::controls(vec![choice(
            ControlId::Alignment,
            "Alignment",
            view.alignments
                .iter()
                .map(|a| labels::alignment(*a))
                .collect(),
            form.alignment,
        )]),
        Row::text(Span::body(format!("Skills (pick {choose})"))),
    ];
    let granted = form.granted(view);
    let full = form.picks.len() >= usize::from(choose);
    let skills = class.map_or_else(Vec::new, |c| {
        c.skill_list
            .iter()
            .map(|s| {
                let picked = form.picks.contains(s);
                let mark = if picked { "[x]" } else { "[ ]" };
                let index = Skill::ALL.iter().position(|k| k == s).unwrap_or(0);
                let mut control = Control::button(
                    ControlId::Skill(u8::try_from(index).unwrap_or(0)),
                    format!("{mark} {}", labels::skill(*s)),
                );
                control.why_not = granted
                    .iter()
                    .find(|(g, _)| g == s)
                    .map(|(_, from)| (*from).to_owned())
                    .or_else(|| {
                        (full && !picked).then(|| format!("This class picks {choose} skills"))
                    });
                control
            })
            .collect()
    });
    left.push(Row::controls(skills));

    let mut right: Vec<Row> = Ability::ALL
        .iter()
        .enumerate()
        .map(|(i, ability)| {
            let score = form.score(i, view);
            let mut after = Vec::new();
            let bonus = view
                .races
                .get(within(form.race, view.races.len()))
                .map_or(0, |r| r.bonuses[i]);
            if bonus != 0 {
                after.push(Span {
                    text: format!("{bonus:+}"),
                    role: Role::Dim,
                    size: Size::Small,
                });
            }
            let cost = view
                .costs
                .get(usize::from(score - view.score_min))
                .copied()
                .unwrap_or(0);
            after.push(Span {
                text: format!("cost {cost}"),
                role: Role::Dim,
                size: Size::Small,
            });
            Row {
                controls: vec![Control {
                    id: ControlId::Score(u8::try_from(i).unwrap_or(0)),
                    label: ability.short().to_owned(),
                    kind: Kind::Number {
                        value: i64::from(score),
                        min: i64::from(view.score_min),
                        max: i64::from(view.score_max),
                    },
                    why_not: None,
                }],
                after,
                ..Row::default()
            }
        })
        .collect();
    right.push(Row::text(Span::body(format!(
        "{} points spent of {}",
        form.spent(view),
        view.points
    ))));
    right.push(Row::text(Span::body("Party")));
    if members.is_empty() {
        right.push(Row::text(Span {
            text: "No one yet".to_owned(),
            role: Role::Dim,
            size: Size::Body,
        }));
    }
    for m in members {
        right.push(Row::text(Span::body(m.name.clone())));
    }

    let draft = form.draft(view);
    let mut add = Control::button(ControlId::AddMember, "Add member");
    add.why_not = game
        .world
        .trial(data, &Command::Party(PartyCommand::Create(draft)))
        .err()
        .map(|r| labels::creation_refusal(&r, form.picks.len(), choose));
    let mut begin = Control::button(ControlId::Begin, "Begin");
    if members.is_empty() {
        begin.why_not = Some("Add at least one member".to_owned());
    }
    right.push(Row::controls(vec![
        add,
        begin,
        Control::button(ControlId::Back, "Back"),
    ]));

    Screen {
        id: ScreenId::CreateParty,
        title: Span::title(format!(
            "CREATE YOUR PARTY   {} of {} members",
            members.len(),
            view.slots
        )),
        panels: vec![Panel { rows: left }, Panel { rows: right }],
        keys: Vec::new(),
        message: None,
    }
}

/// What each control does to the draft, or asks of the shell.
pub fn apply(form: &mut CreateForm, game: Game, control: ControlId, payload: &Payload) -> Effect {
    let view = &game.views.creation;
    match (control, payload) {
        (ControlId::Name, Payload::Text(text)) => form.name.clone_from(text),
        (ControlId::Race, _) => {
            form.race = step(form.race, view.races.len(), payload);
            form.drop_granted(view);
        }
        (ControlId::Class, _) => {
            let was = within(form.class, view.classes.len());
            form.class = step(form.class, view.classes.len(), payload);
            if form.class != was {
                form.picks.clear();
            }
        }
        (ControlId::Background, _) => {
            form.background = step(form.background, view.backgrounds.len(), payload);
            form.drop_granted(view);
        }
        (ControlId::Alignment, _) => {
            form.alignment = step(form.alignment, view.alignments.len(), payload);
        }
        (ControlId::Score(i), _) => {
            let i = usize::from(i).min(5);
            let (min, max) = (i64::from(view.score_min), i64::from(view.score_max));
            let n = stepped(i64::from(form.score(i, view)), min, max, payload);
            form.scores[i] = u8::try_from(n).unwrap_or(view.score_min);
        }
        (ControlId::Skill(i), _) => {
            if let Some(skill) = Skill::ALL.get(usize::from(i)) {
                if let Some(at) = form.picks.iter().position(|s| s == skill) {
                    form.picks.remove(at);
                } else {
                    form.picks.push(*skill);
                }
            }
        }
        (ControlId::AddMember, _) => {
            return Effect::Command {
                command: Command::Party(PartyCommand::Create(form.draft(view))),
                close: false,
            };
        }
        (ControlId::Begin, _) => return Effect::Host(HostOp::Begin),
        (ControlId::Back, _) => return Effect::ToNewGame,
        _ => {}
    }
    Effect::None
}
