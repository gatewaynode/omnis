//! The party column (presentation-ARCHITECTURE §6.6; from the retired client's roster band): each
//! member in marching order under a "front" or "back" word, with their class and level, hit
//! points, spell points, armour class and the first condition or effect. A member is selected
//! by pressing their entry; the selected member's screens open from the row under it. The
//! column stands at the right in explore and in a fight. Everything shown comes from the views;
//! the packs give names only (VA9).

use super::{Control, ControlId, Game, Panel, Refused, Row, Span};
use crate::combat_menu::acting;
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::MemberView;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;

/// A screen a member's entry opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// The character sheet.
    Sheet,
    /// The member's kit and the party's stores.
    Inventory,
    /// The spells the member knows.
    Spells,
    /// The party's quests.
    Journal,
}

impl Tool {
    /// Every tool, in the order the row shows them.
    pub const ALL: [Tool; 4] = [Tool::Sheet, Tool::Inventory, Tool::Spells, Tool::Journal];

    /// The button's word.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Tool::Sheet => "Sheet",
            Tool::Inventory => "Inventory",
            Tool::Spells => "Spells",
            Tool::Journal => "Journal",
        }
    }

    /// Why it cannot be opened for `member` now: the journal arrives with quests, and Spells
    /// needs a spell for the road.
    #[must_use]
    pub fn why_not(self, game: Game, member: CharacterId) -> Option<&'static str> {
        match self {
            Tool::Sheet | Tool::Inventory => None,
            Tool::Spells => (!game.views.casts.iter().any(|c| c.caster == member))
                .then_some("No spells for the road"),
            Tool::Journal => Some("No quests yet"),
        }
    }

    /// The screen it opens, once built.
    #[must_use]
    pub const fn screen(self) -> Option<super::ScreenId> {
        match self {
            Tool::Inventory => Some(super::ScreenId::Inventory),
            Tool::Spells => Some(super::ScreenId::Cast),
            Tool::Sheet => Some(super::ScreenId::Sheet),
            Tool::Journal => None,
        }
    }
}

/// What using the column asks of the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterEffect {
    /// Nothing beyond the selection, which `apply` changed.
    None,
    /// Open `tool` on `member`; in a fight it only looks (the action column acts).
    Open {
        /// Which screen.
        tool: Tool,
        /// Whose.
        member: CharacterId,
        /// In a fight: look, don't act.
        read_only: bool,
    },
}

/// A small dim span.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// How the member's entry is drawn: red under a quarter of their hit points (down among them),
/// yellow under half, dim when dead.
fn health(m: &MemberView) -> Role {
    if m.dead {
        Role::Dim
    } else if m.hp * 4 < m.hp_max {
        Role::Alert
    } else if m.hp * 2 < m.hp_max {
        Role::Caution
    } else {
        Role::Interface
    }
}

/// The member's second line, in their health's role: hit points, spell points, armour class,
/// then how they stand, or else the first condition or effect.
fn numbers(data: &Data, m: &MemberView) -> Row {
    let role = health(m);
    let out = m.dead || m.down;
    let span = |text: String, role: Role| Span {
        text,
        role,
        size: Size::Small,
    };
    let mut spans = vec![
        span(format!("HP {}/{}", m.hp, m.hp_max), role),
        span(
            format!("SP {}/{}  AC {}", m.spell_points, m.spell_points_max, m.ac),
            role,
        ),
    ];
    let state = if m.dead {
        Some("dead".to_owned())
    } else if m.down {
        Some("down".to_owned())
    } else {
        m.conditions
            .first()
            .map(|c| labels::condition(data, c).to_owned())
            .or_else(|| {
                m.effects
                    .first()
                    .map(|e| labels::spell(data, &e.spell).to_owned())
            })
    };
    if let Some(state) = state {
        spans.push(span(state, if out { Role::Alert } else { Role::Dim }));
    }
    Row {
        spans,
        ..Row::default()
    }
}

/// The column, with the open row under `selected`.
#[must_use]
pub fn content(data: &Data, game: Game, selected: Option<CharacterId>) -> Panel {
    let party = &game.views.party;
    let actor = game.views.combat.as_ref().and_then(acting);
    let mut rows = vec![Row::text(Span::title("Party"))];
    let mut row_group = None;
    for m in &party.members {
        if row_group != Some(m.in_front) {
            row_group = Some(m.in_front);
            rows.push(Row::text(note(if m.in_front { "front" } else { "back" })));
        }
        let mark = if actor == Some(m.member) { "> " } else { "" };
        rows.push(Row {
            tint: Some(health(m)),
            ..Row::controls(vec![Control::button(
                ControlId::Member(m.member),
                format!(
                    "{mark}{}  {} {}",
                    m.name,
                    labels::class(data, &m.class),
                    m.level
                ),
            )])
        });
        rows.push(numbers(data, m));
        if selected == Some(m.member) {
            rows.push(Row::controls(
                Tool::ALL
                    .iter()
                    .map(|t| Control {
                        why_not: t.why_not(game, m.member).map(str::to_owned),
                        ..Control::button(ControlId::Open(*t), t.label())
                    })
                    .collect(),
            ));
        }
    }
    Panel { rows }
}

/// The selection held only while its member is in the party.
#[must_use]
pub fn settle(game: Game, selected: Option<CharacterId>) -> Option<CharacterId> {
    selected.filter(|id| game.views.party.members.iter().any(|m| m.member == *id))
}

/// Use `control`: a member's entry selects them, or clears the selection when they are
/// already selected; an open button asks for the screen. A dim control is refused with its
/// reason, as on any screen.
///
/// # Errors
/// [`Refused::NotHere`] when the column has no such control, [`Refused::Dim`] when it is dim.
pub fn apply(
    data: &Data,
    game: Game,
    selected: &mut Option<CharacterId>,
    control: ControlId,
) -> Result<RosterEffect, Refused> {
    let panel = content(data, game, *selected);
    let found = panel
        .rows
        .iter()
        .flat_map(|r| &r.controls)
        .find(|c| c.id == control)
        .ok_or(Refused::NotHere)?;
    if let Some(why) = &found.why_not {
        return Err(Refused::Dim(why.clone()));
    }
    Ok(match control {
        ControlId::Member(id) => {
            *selected = (*selected != Some(id)).then_some(id);
            RosterEffect::None
        }
        ControlId::Open(tool) => match *selected {
            Some(member) => RosterEffect::Open {
                tool,
                member,
                read_only: game.views.combat.is_some(),
            },
            None => RosterEffect::None,
        },
        _ => RosterEffect::None,
    })
}
