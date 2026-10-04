//! The tactics panel, Bevy-free (M7c step 7): which control is which (`TacticsPanelId`), the
//! texts it rewrites (`TacticsLabelId`), the reaction being written (`TacticsForm`), and
//! `apply`, which turns a control's report into the tactics command to send. What a member may
//! declare comes from `omnis_sim::party_view` (`TacticsView.answers`), so the panel offers only
//! actions and triggers the rules accept; the conditions are a flat list joined by all or any
//! (an entry set deeper, over MCP, is shown and may be removed but not edited here; the
//! conditions themselves are `tactics_draft.rs`). The
//! widgets are not trusted: numbers are clamped, picks out of range are ignored, and the
//! simulation checks every command again. `feathers_tactics.rs` draws it.

pub use crate::tactics_draft::{Choices, Draft, Field, Kind, criteria_text, trigger_name};
use crate::ui_model::Payload;
use omnis_sim::TacticsView;
use omnis_sim::omnis_core::fnv1a64;
use omnis_sim::omnis_rules::tactics::{CRITERIA_NODES, RUNBOOK_ENTRIES, TACTICS_NAME_BYTES};
use omnis_sim::omnis_rules::{Criteria, CriteriaSet, Trigger};
use omnis_sim::tactics::TacticsCommand;

/// The conditions one reaction may list: a flat list under one node (`CRITERIA_NODES`).
pub const CONDITIONS_MAX: usize = CRITERIA_NODES - 1;

/// One control of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TacticsPanelId {
    /// The member menu's button, and a member in it by party slot.
    Member,
    /// A member in the member menu.
    MemberPick(usize),
    /// The reactions switch.
    Reactions,
    /// Load a declared reaction into the form, by row.
    Edit(usize),
    /// Remove a declared reaction, by row.
    Remove(usize),
    /// The action menu's button.
    Action,
    /// An action, by its row in the answers.
    ActionPick(usize),
    /// The trigger menu's button.
    Trigger,
    /// A trigger, by its row in the chosen action's triggers.
    TriggerPick(usize),
    /// The all/any menu's button.
    Combine,
    /// All (0) or any (1).
    CombinePick(usize),
    /// A condition's kind menu, by condition row.
    Kind(usize),
    /// A kind, by condition row and kind.
    KindPick(usize, usize),
    /// A condition's field menu.
    Field(usize, Field),
    /// A field's option, by condition row, field and option row.
    FieldPick(usize, Field, usize),
    /// A condition's number.
    Number(usize),
    /// Drop a condition.
    Drop(usize),
    /// Add a condition.
    Add,
    /// Declare the reaction: as a new row, or over the one being edited.
    Save,
    /// Start a new reaction.
    New,
    /// Back to the sheet.
    #[default]
    Close,
}

/// A text the panel rewrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TacticsLabelId {
    /// The member menu's caption.
    #[default]
    Member,
    /// A declared reaction's line, by row.
    Entry(usize),
    /// What the form is writing: a new reaction, or which row.
    Editing,
    /// The action menu's caption.
    Action,
    /// The trigger menu's caption.
    Trigger,
    /// The all/any menu's caption.
    Combine,
    /// A condition's kind caption.
    Kind(usize),
    /// A condition's field caption.
    Field(usize, Field),
    /// The last refusal or note.
    Message,
}

/// What the panel keeps between frames: whose tactics, the reaction being written, and the
/// last refusal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TacticsForm {
    /// The member, by party slot.
    pub member: usize,
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
    /// The row being looked at was set deeper than this panel writes: shown, not saved.
    pub locked: bool,
    /// The last refusal or note, or empty.
    pub message: String,
}

impl TacticsForm {
    /// A new reaction for `member`.
    #[must_use]
    pub fn new(member: usize) -> Self {
        TacticsForm {
            member,
            ..TacticsForm::default()
        }
    }

    /// The trigger the form names, when the action has it.
    #[must_use]
    pub fn chosen_trigger(&self, view: &TacticsView) -> Option<Trigger> {
        view.answers
            .get(self.action)?
            .triggers
            .get(self.trigger)
            .copied()
    }

    /// The criteria the conditions make: always when there are none.
    #[must_use]
    pub fn criteria(&self, choices: &Choices) -> Option<Criteria> {
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
    pub fn set(&self, view: &TacticsView, choices: &Choices) -> Option<CriteriaSet> {
        let answer = view.answers.get(self.action)?;
        let trigger = self.chosen_trigger(view)?;
        Some(CriteriaSet {
            name: set_name(&choices.action_name(&answer.action, &answer.name), trigger),
            action: answer.action.clone(),
            trigger,
            when: self.criteria(choices)?,
        })
    }

    /// Load a declared row: its action and trigger, and its conditions when the tree is flat.
    fn load(&mut self, view: &TacticsView, at: usize, choices: &Choices) {
        let Some(entry) = view.reactions.get(at) else {
            return;
        };
        let member = self.member;
        *self = TacticsForm::new(member);
        self.editing = u8::try_from(at).ok();
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
            _ => {
                self.locked = true;
                "Set elsewhere: it can be removed here, not edited".clone_into(&mut self.message);
            }
        }
    }
}

/// What a control asks of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TacticsAsk {
    /// Send this tactics command.
    Send(TacticsCommand),
    /// Back to the sheet.
    Close,
}

/// Apply one control's report. `view` is the member's tactics as they are now, `members` the
/// party's size.
pub fn apply(
    id: TacticsPanelId,
    payload: &Payload,
    view: &TacticsView,
    members: usize,
    choices: &Choices,
    form: &mut TacticsForm,
) -> Option<TacticsAsk> {
    let member = u8::try_from(form.member).ok()?;
    let editable = !form.locked;
    match (id, payload) {
        (TacticsPanelId::MemberPick(slot), Payload::Activate) if slot < members => {
            *form = TacticsForm::new(slot);
        }
        (TacticsPanelId::Reactions, Payload::Flag(on)) => {
            return Some(TacticsAsk::Send(TacticsCommand::SetReactions {
                member,
                on: *on,
            }));
        }
        (TacticsPanelId::Edit(at), Payload::Activate) => form.load(view, at, choices),
        (TacticsPanelId::Remove(at), Payload::Activate) if at < view.reactions.len() => {
            return Some(TacticsAsk::Send(TacticsCommand::RemoveReaction {
                member,
                at: u8::try_from(at).ok()?,
            }));
        }
        (TacticsPanelId::ActionPick(at), Payload::Activate)
            if editable && at < view.answers.len() =>
        {
            (form.action, form.trigger) = (at, 0);
        }
        (TacticsPanelId::TriggerPick(at), Payload::Activate)
            if editable
                && view
                    .answers
                    .get(form.action)
                    .is_some_and(|a| at < a.triggers.len()) =>
        {
            form.trigger = at;
        }
        (TacticsPanelId::CombinePick(at), Payload::Activate) if editable && at < 2 => {
            form.any = at == 1;
        }
        (TacticsPanelId::KindPick(row, kind), Payload::Activate) if editable => {
            let (draft, kind) = (form.conditions.get_mut(row)?, *Kind::ALL.get(kind)?);
            draft.kind = kind;
            draft.n = draft.n.min(kind.number_max().unwrap_or(0));
        }
        (TacticsPanelId::FieldPick(row, field, at), Payload::Activate) if editable => {
            if at >= choices.options(field).len() {
                return None;
            }
            let draft = form.conditions.get_mut(row)?;
            match field {
                Field::Who => draft.who = at,
                Field::Monster => draft.monster = at,
                Field::Condition => draft.condition = at,
                Field::Cmp => draft.cmp = at,
                Field::Row => draft.row = at,
            }
        }
        (TacticsPanelId::Number(row), Payload::Number(value)) if editable => {
            let draft = form.conditions.get_mut(row)?;
            let max = i64::from(draft.kind.number_max()?);
            draft.n = u16::try_from((*value).clamp(0, max)).ok()?;
        }
        (TacticsPanelId::Drop(row), Payload::Activate)
            if editable && row < form.conditions.len() =>
        {
            form.conditions.remove(row);
        }
        (TacticsPanelId::Add, Payload::Activate)
            if editable && form.conditions.len() < CONDITIONS_MAX =>
        {
            form.conditions.push(Draft::default());
        }
        (TacticsPanelId::Save, Payload::Activate) if editable => {
            let Some(set) = form.set(view, choices) else {
                "Nothing this member can declare".clone_into(&mut form.message);
                return None;
            };
            return Some(TacticsAsk::Send(TacticsCommand::PutReaction {
                member,
                at: form.editing,
                set,
            }));
        }
        (TacticsPanelId::New, Payload::Activate) => *form = TacticsForm::new(form.member),
        (TacticsPanelId::Close, Payload::Activate) => return Some(TacticsAsk::Close),
        _ => {}
    }
    None
}

/// Whether Save and Add would do anything now.
#[must_use]
pub fn can_save(view: &TacticsView, form: &TacticsForm, choices: &Choices) -> bool {
    !form.locked
        && form.set(view, choices).is_some()
        && (form.editing.is_some() || view.reactions.len() < RUNBOOK_ENTRIES)
}

/// Whether another condition fits.
#[must_use]
pub fn can_add(form: &TacticsForm) -> bool {
    !form.locked && form.conditions.len() < CONDITIONS_MAX
}

/// A set's name: the action on the trigger, cut to the rules' limit at a character.
#[must_use]
pub fn set_name(action: &str, trigger: Trigger) -> String {
    let mut name = format!("{action} on {}", trigger_name(trigger));
    while name.len() > TACTICS_NAME_BYTES {
        name.pop();
    }
    name
}

/// A declared row's line: `1. Shield on attacked, when me HP < 50%`.
#[must_use]
pub fn entry_line(view: &TacticsView, at: usize, choices: &Choices) -> String {
    view.reactions.get(at).map_or_else(String::new, |entry| {
        format!(
            "{}. {} on {}, when {}",
            at + 1,
            choices.action_name(&entry.action, &entry.action_name),
            trigger_name(entry.trigger),
            criteria_text(&entry.when, choices)
        )
    })
}

/// What the form is writing.
#[must_use]
pub fn editing_line(form: &TacticsForm) -> String {
    match form.editing {
        Some(at) => format!("Editing reaction {}", at + 1),
        None => "New reaction".to_owned(),
    }
}

/// The action menu's caption.
#[must_use]
pub fn action_caption(view: &TacticsView, form: &TacticsForm, choices: &Choices) -> String {
    view.answers.get(form.action).map_or_else(
        || "nothing to declare".to_owned(),
        |a| choices.action_name(&a.action, &a.name),
    )
}

/// The trigger menu's caption.
#[must_use]
pub fn trigger_caption(view: &TacticsView, form: &TacticsForm) -> String {
    form.chosen_trigger(view)
        .map_or("", trigger_name)
        .to_owned()
}

/// The all/any menu's caption.
#[must_use]
pub const fn combine_caption(form: &TacticsForm) -> &'static str {
    if form.any {
        "any of these"
    } else {
        "all of these"
    }
}

/// What the panel's entity tree depends on: the member and the party's size, the declared
/// rows, what the member may declare, the action chosen (its trigger menu), and each
/// condition's kind (its controls). Captions, numbers and the switch are rewritten in place.
#[must_use]
pub fn shape(view: &TacticsView, form: &TacticsForm, members: usize) -> u64 {
    let mut bytes = Vec::new();
    for n in [form.member, members, view.reactions.len(), form.action] {
        bytes.extend(n.to_le_bytes());
    }
    for answer in &view.answers {
        bytes.extend(answer.name.as_bytes());
        bytes.push(u8::try_from(answer.triggers.len()).unwrap_or(u8::MAX));
    }
    bytes.push(u8::from(form.locked));
    bytes.extend(form.conditions.iter().map(|d| d.kind as u8));
    fnv1a64(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_core::{ConditionId, MonsterId, SpellId};
    use omnis_sim::omnis_rules::{ActionRef, Cmp, Predicate, Row, Who};
    use omnis_sim::{AnswerView, ReactionView};

    fn choices() -> Choices {
        Choices {
            monsters: vec![
                (MonsterId(2), "Giant rat".to_owned()),
                (MonsterId(5), "Bob the Rat King".to_owned()),
            ],
            conditions: vec![
                (ConditionId(1), "Poisoned".to_owned()),
                (ConditionId(7), "Prone".to_owned()),
            ],
            actions: Vec::new(),
        }
    }

    const SHIELD: ActionRef = ActionRef::Spell(SpellId(3));

    /// A wizard's tactics: attack answers an enemy fleeing, Shield an attack; one row is
    /// declared flat, one deeper.
    fn view() -> TacticsView {
        let flat = Criteria::All(vec![Criteria::Is(Predicate::Hp {
            who: Who::Me,
            cmp: Cmp::Lt,
            percent: 50,
        })]);
        let deep = Criteria::Any(vec![Criteria::All(vec![Criteria::Always])]);
        let row = |index, when| ReactionView {
            index,
            name: "Shield on attacked".to_owned(),
            action: SHIELD,
            action_name: "Shield".to_owned(),
            trigger: Trigger::Attacked,
            when,
        };
        TacticsView {
            reactions_on: true,
            auto: false,
            reactions: vec![row(0, flat), row(1, deep)],
            answers: vec![
                AnswerView {
                    action: ActionRef::Attack,
                    name: "attack".to_owned(),
                    triggers: vec![Trigger::EnemyFlees],
                },
                AnswerView {
                    action: SHIELD,
                    name: "Shield".to_owned(),
                    triggers: vec![Trigger::Attacked],
                },
            ],
        }
    }

    fn press(id: TacticsPanelId, form: &mut TacticsForm) -> Option<TacticsAsk> {
        apply(id, &Payload::Activate, &view(), 4, &choices(), form)
    }

    #[test]
    fn every_kind_of_condition_comes_back_as_it_went() {
        let choices = choices();
        let predicates = [
            Predicate::MonsterCount {
                monster: MonsterId(5),
                cmp: Cmp::Ge,
                n: 3,
            },
            Predicate::MonsterShare {
                monster: MonsterId(2),
                cmp: Cmp::Gt,
                percent: 60,
            },
            Predicate::Hp {
                who: Who::Subject,
                cmp: Cmp::Le,
                percent: 25,
            },
            Predicate::SpellPoints {
                who: Who::Me,
                cmp: Cmp::Eq,
                percent: 100,
            },
            Predicate::HasCondition {
                who: Who::Subject,
                condition: ConditionId(7),
            },
            Predicate::Row {
                who: Who::Me,
                row: Row::Back,
            },
            Predicate::Round { cmp: Cmp::Lt, n: 4 },
            Predicate::WouldChangeOutcome,
        ];
        for (predicate, kind) in predicates.iter().zip(Kind::ALL) {
            let draft = Draft::of(predicate, &choices).unwrap();
            assert_eq!(draft.kind, kind);
            assert_eq!(draft.predicate(&choices).as_ref(), Some(predicate));
        }
        let unknown = Predicate::HasCondition {
            who: Who::Me,
            condition: ConditionId(99),
        };
        assert_eq!(Draft::of(&unknown, &choices), None, "not loaded");
        let draft = Draft {
            kind: Kind::MonsterCount,
            ..Draft::default()
        };
        assert_eq!(draft.predicate(&Choices::default()), None, "no monsters");
    }

    #[test]
    fn numbers_are_held_to_the_kind_s_range() {
        let mut form = TacticsForm::new(2);
        press(TacticsPanelId::Add, &mut form);
        let number = |value, form: &mut TacticsForm| {
            apply(
                TacticsPanelId::Number(0),
                &Payload::Number(value),
                &view(),
                4,
                &choices(),
                form,
            )
        };
        number(250, &mut form);
        assert_eq!(form.conditions[0].n, 100, "a percentage stops at 100");
        number(-3, &mut form);
        assert_eq!(form.conditions[0].n, 0);
        press(TacticsPanelId::KindPick(0, 6), &mut form);
        number(70_000, &mut form);
        assert_eq!(form.conditions[0].n, u16::MAX, "a round stops at u16");
        press(TacticsPanelId::KindPick(0, 2), &mut form);
        assert_eq!(
            form.conditions[0].n, 100,
            "a kind change keeps the number in range"
        );
        press(TacticsPanelId::KindPick(0, 7), &mut form);
        assert_eq!(number(5, &mut form), None);
        assert_eq!(
            form.conditions[0].n, 0,
            "a kind without a number holds none"
        );
        press(TacticsPanelId::FieldPick(0, Field::Monster, 9), &mut form);
        assert_eq!(
            form.conditions[0].monster, 0,
            "a pick past the list is ignored"
        );
        press(TacticsPanelId::FieldPick(0, Field::Monster, 1), &mut form);
        assert_eq!(form.conditions[0].monster, 1);
        for _ in 0..20 {
            press(TacticsPanelId::Add, &mut form);
        }
        assert_eq!(form.conditions.len(), CONDITIONS_MAX);
        assert!(!can_add(&form));
        press(TacticsPanelId::Drop(0), &mut form);
        assert_eq!(form.conditions.len(), CONDITIONS_MAX - 1);
    }

    #[test]
    fn save_declares_the_action_on_its_trigger_with_the_conditions() {
        let mut form = TacticsForm::new(2);
        press(TacticsPanelId::ActionPick(1), &mut form);
        assert_eq!(trigger_caption(&view(), &form), "attacked");
        press(TacticsPanelId::TriggerPick(1), &mut form);
        assert_eq!(form.trigger, 0, "Shield answers one trigger");
        press(TacticsPanelId::Add, &mut form);
        press(TacticsPanelId::Add, &mut form);
        press(TacticsPanelId::KindPick(1, 7), &mut form);
        press(TacticsPanelId::CombinePick(1), &mut form);
        let asked = press(TacticsPanelId::Save, &mut form);
        let hp = Predicate::Hp {
            who: Who::Me,
            cmp: Cmp::Lt,
            percent: 0,
        };
        assert_eq!(
            asked,
            Some(TacticsAsk::Send(TacticsCommand::PutReaction {
                member: 2,
                at: None,
                set: CriteriaSet {
                    name: "Shield on attacked".to_owned(),
                    action: SHIELD,
                    trigger: Trigger::Attacked,
                    when: Criteria::Any(vec![
                        Criteria::Is(hp),
                        Criteria::Is(Predicate::WouldChangeOutcome)
                    ]),
                },
            }))
        );
        assert!(can_save(&view(), &form, &choices()));
        press(TacticsPanelId::New, &mut form);
        assert_eq!(form, TacticsForm::new(2));
        let Some(TacticsAsk::Send(TacticsCommand::PutReaction { set, .. })) =
            press(TacticsPanelId::Save, &mut form)
        else {
            panic!("a new form saves the first answer")
        };
        assert_eq!(
            (set.action, set.trigger, set.when),
            (ActionRef::Attack, Trigger::EnemyFlees, Criteria::Always)
        );
        let bare = TacticsView {
            answers: Vec::new(),
            ..view()
        };
        let mut form = TacticsForm::new(0);
        let asked = apply(
            TacticsPanelId::Save,
            &Payload::Activate,
            &bare,
            4,
            &choices(),
            &mut form,
        );
        assert_eq!(
            (asked, form.message.as_str()),
            (None, "Nothing this member can declare")
        );
    }

    #[test]
    fn edit_loads_a_flat_row_and_locks_a_deeper_one() {
        let mut form = TacticsForm::new(2);
        press(TacticsPanelId::Edit(0), &mut form);
        assert_eq!(
            (
                form.editing,
                form.action,
                form.trigger,
                form.any,
                form.locked
            ),
            (Some(0), 1, 0, false, false)
        );
        assert_eq!(form.conditions.len(), 1);
        assert_eq!(editing_line(&form), "Editing reaction 1");
        let Some(TacticsAsk::Send(TacticsCommand::PutReaction { at, set, .. })) =
            press(TacticsPanelId::Save, &mut form)
        else {
            panic!("an edited row saves")
        };
        assert_eq!((at, &set.when), (Some(0), &view().reactions[0].when));
        press(TacticsPanelId::Edit(1), &mut form);
        assert!(form.locked && form.editing == Some(1), "{form:?}");
        assert!(form.message.starts_with("Set elsewhere"));
        assert_eq!(press(TacticsPanelId::Save, &mut form), None, "not saved");
        assert!(!can_save(&view(), &form, &choices()));
        press(TacticsPanelId::Add, &mut form);
        assert!(form.conditions.is_empty(), "nor changed");
        assert_eq!(
            press(TacticsPanelId::Remove(1), &mut form),
            Some(TacticsAsk::Send(TacticsCommand::RemoveReaction {
                member: 2,
                at: 1
            })),
            "but removed"
        );
        assert_eq!(
            press(TacticsPanelId::Remove(2), &mut form),
            None,
            "no row 3"
        );
    }

    #[test]
    fn the_switch_the_member_and_close() {
        let mut form = TacticsForm::new(2);
        let flag = apply(
            TacticsPanelId::Reactions,
            &Payload::Flag(false),
            &view(),
            4,
            &choices(),
            &mut form,
        );
        assert_eq!(
            flag,
            Some(TacticsAsk::Send(TacticsCommand::SetReactions {
                member: 2,
                on: false
            }))
        );
        form.conditions.push(Draft::default());
        press(TacticsPanelId::MemberPick(1), &mut form);
        assert_eq!(form, TacticsForm::new(1), "another member starts afresh");
        press(TacticsPanelId::MemberPick(4), &mut form);
        assert_eq!(form.member, 1, "no fifth member");
        assert_eq!(
            press(TacticsPanelId::Close, &mut form),
            Some(TacticsAsk::Close)
        );
    }

    #[test]
    fn lines_and_names_read_as_english_and_fit_the_rules() {
        let choices = choices();
        assert_eq!(
            entry_line(&view(), 0, &choices),
            "1. Shield on attacked, when me HP < 50%"
        );
        assert_eq!(
            entry_line(&view(), 1, &choices),
            "2. Shield on attacked, when ((always))"
        );
        let when = Criteria::All(vec![
            Criteria::Is(Predicate::MonsterCount {
                monster: MonsterId(5),
                cmp: Cmp::Ge,
                n: 1,
            }),
            Criteria::Is(Predicate::HasCondition {
                who: Who::Subject,
                condition: ConditionId(1),
            }),
        ]);
        assert_eq!(
            criteria_text(&when, &choices),
            "Bob the Rat King standing >= 1 and the subject Poisoned"
        );
        let name = set_name("Éclair éclatant de la tempête", Trigger::MemberAttacked);
        assert!(name.len() <= TACTICS_NAME_BYTES, "{name}");
        assert!(name.starts_with("Éclair éclatant"), "{name}");
        assert_eq!(set_name("Shield", Trigger::Attacked), "Shield on attacked");
        let form = TacticsForm::new(0);
        assert_eq!(action_caption(&view(), &form, &choices), "attack");
        let named = Choices {
            actions: vec![(SHIELD, "Shield of the Mage".to_owned())],
            ..choices.clone()
        };
        let shield = TacticsForm {
            action: 1,
            ..TacticsForm::new(0)
        };
        assert_eq!(
            action_caption(&view(), &shield, &named),
            "Shield of the Mage"
        );
        assert!(entry_line(&view(), 0, &named).starts_with("1. Shield of the Mage on"));
        assert_eq!(combine_caption(&form), "all of these");
        let moved = TacticsForm {
            action: 1,
            ..TacticsForm::new(0)
        };
        assert_ne!(shape(&view(), &form, 4), shape(&view(), &moved, 4));
        let mut kinds = moved.clone();
        kinds.conditions.push(Draft::default());
        assert_ne!(shape(&view(), &moved, 4), shape(&view(), &kinds, 4));
        let mut number = kinds.clone();
        number.conditions[0].n = 40;
        assert_eq!(
            shape(&view(), &kinds, 4),
            shape(&view(), &number, 4),
            "a number is rewritten in place"
        );
    }
}
