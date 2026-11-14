//! The target step (owner, 2026-11-14; PRD §7.3; `tasks/plans/targeting.md`): after an option
//! is chosen (a spell, an item's use), the player chooses from its possible targets, which the
//! engine lists (`omnis_sim::targets`), each dim with the engine's reason when it would be
//! refused. The target is never implied by anything else on the screen. Bevy-free.

use crate::labels;
use crate::screen::{Control, ControlId, Game, Panel, Row, Screen, ScreenId, Span};
use crate::style::{Role, Size};
use omnis_sim::api::{Chosen, Sight, TargetView};
use omnis_sim::omnis_core::{CharacterId, Facing};
use omnis_sim::omnis_data::placement::ResolvedPlacement;
use omnis_sim::omnis_data::{Data, ResolvedEncounter};
use omnis_sim::{Command, ItemCommand, Target};

/// An option chosen on the road, waiting for its target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aim {
    /// Who acts.
    pub member: CharacterId,
    /// What they chose.
    pub option: Chosen,
}

impl Aim {
    /// A spell `caster` is about to cast.
    #[must_use]
    pub fn spell(caster: CharacterId, spell: &str) -> Aim {
        Aim {
            member: caster,
            option: Chosen::Spell {
                spell: spell.to_owned(),
                pay: omnis_sim::Pay::Action,
            },
        }
    }

    /// A weapon attack `member` is about to make from the road.
    #[must_use]
    pub const fn attack(member: CharacterId) -> Aim {
        Aim {
            member,
            option: Chosen::Attack,
        }
    }

    /// An item `member` is about to use.
    #[must_use]
    pub fn item(member: CharacterId, item: &str) -> Aim {
        Aim {
            member,
            option: Chosen::Item {
                item: item.to_owned(),
            },
        }
    }

    /// The possible targets, as the engine lists them now.
    #[must_use]
    pub fn targets(&self, data: &Data, game: Game) -> Vec<TargetView> {
        game.world.targets(data, self.member, &self.option)
    }

    /// The command the road sends for this option on `target`, when the two fit.
    #[must_use]
    pub fn command(&self, target: Target) -> Option<Command> {
        match (&self.option, target) {
            (Chosen::Spell { spell, .. }, target) => Some(Command::Cast {
                caster: self.member,
                spell: spell.clone(),
                target,
            }),
            (Chosen::Item { item }, Target::Member(receiver)) => {
                Some(Command::Item(ItemCommand::Use {
                    member: self.member,
                    item: item.clone(),
                    receiver: Some(receiver),
                }))
            }
            (Chosen::Attack, target @ Target::Being { .. }) => Some(Command::Attack {
                member: self.member,
                target,
            }),
            _ => None,
        }
    }

    /// The option in words: "Cure Wounds", "Potion of Healing".
    fn named<'d>(&'d self, data: &'d Data) -> &'d str {
        match &self.option {
            Chosen::Spell { spell, .. } => labels::spell(data, spell),
            Chosen::Item { item } => labels::item(data, item),
            Chosen::Feature { feature, .. } => labels::text(data, feature),
            Chosen::Attack => "Attack",
            Chosen::Exchange => "Swap",
        }
    }

    /// Why the option has no target to choose now, if it has none: the first row's reason, or
    /// that nothing is in reach. What dims the option's own button.
    #[must_use]
    pub fn blocked(&self, data: &Data, game: Game) -> Option<String> {
        let rows = self.targets(data, game);
        if rows.iter().any(|r| r.refusal.is_none()) {
            return None;
        }
        Some(rows.first().and_then(|r| r.refusal.as_ref()).map_or_else(
            || "nothing in reach".to_owned(),
            |r| labels::refused(data, &game.views.party, r),
        ))
    }

    /// The command of the target at row `at` of the list as it stands, when that row is free.
    #[must_use]
    pub fn chosen(&self, data: &Data, game: Game, at: u8) -> Option<Command> {
        let rows = self.targets(data, game);
        let row = rows.get(usize::from(at)).filter(|r| r.refusal.is_none())?;
        self.command(row.target.clone())
    }

    /// The command for `member`, when the list offers them free: a click on their entry in the
    /// party column during the step.
    #[must_use]
    pub fn on_member(&self, data: &Data, game: Game, member: CharacterId) -> Option<Command> {
        let target = Target::Member(member);
        let rows = self.targets(data, game);
        rows.iter()
            .find(|r| r.target == target && r.refusal.is_none())
            .and_then(|r| self.command(r.target.clone()))
    }

    /// The cell of the map the target at row `at` stands on, when it is a thing of the map:
    /// what the 3D view marks while the pointer is on the row.
    #[must_use]
    pub fn marked(&self, data: &Data, game: Game, at: u8) -> Option<(u16, u16)> {
        let rows = self.targets(data, game);
        cell(data, game, &rows.get(usize::from(at))?.target)
    }
}

/// The cell a thing of the map stands on; none for a member or a combatant.
fn cell(data: &Data, game: Game, target: &Target) -> Option<(u16, u16)> {
    match target {
        Target::Door { x, y, .. } => Some((*x, *y)),
        Target::Object { object } => placed(data, game, object).map(|p| (p.x, p.y)),
        Target::Being { being } => stood(data, game, being).map(|e| (e.x, e.y)),
        Target::Member(_) | Target::Stack(_) => None,
    }
}

/// The placement `id` of the party's map and level.
fn placed<'d>(data: &'d Data, game: Game, id: &str) -> Option<&'d ResolvedPlacement> {
    let at = game.views.position;
    let objects = data.maps.get(&at.map)?.objects.iter();
    objects
        .into_iter()
        .find(|p| p.id == id && p.level == at.level)
}

/// The placed group `id` of the party's map.
fn stood<'d>(data: &'d Data, game: Game, id: &str) -> Option<&'d ResolvedEncounter> {
    data.maps.get(&game.views.position.map)?.encounter(id)
}

/// Which way a point lies from the party, in half tiles east and south of it: one of the
/// eight winds, a diagonal when neither axis is twice the other.
fn bearing(east: i32, south: i32) -> &'static str {
    let (wide, tall) = (east.abs() >= 2 * south.abs(), south.abs() >= 2 * east.abs());
    match (east.signum(), south.signum()) {
        (0, 0) => "here",
        (1, _) if wide => "east",
        (-1, _) if wide => "west",
        (_, 1) if tall => "south",
        (_, -1) if tall => "north",
        (1, 1) => "south-east",
        (1, _) => "north-east",
        (_, 1) => "south-west",
        _ => "north-west",
    }
}

/// Which way the middle of a tile, or of one of its edges, lies from the party.
fn toward(game: Game, (x, y): (u16, u16), edge: Option<Facing>) -> &'static str {
    let at = game.views.position;
    let (dx, dy) = edge.map_or((0, 0), Facing::delta);
    bearing(
        2 * (i32::from(x) - i32::from(at.x)) + dx,
        2 * (i32::from(y) - i32::from(at.y)) + dy,
    )
}

fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// A target in words: a member by name, the actor marked as such; a door or an object by
/// what it is and which way it lies.
fn label(data: &Data, game: Game, aim: &Aim, target: &Target) -> (String, Option<&'static str>) {
    match target {
        Target::Member(id) => {
            let members = &game.views.party.members;
            let name = members.iter().find(|m| m.member == *id);
            (
                name.map_or_else(|| "someone".to_owned(), |m| m.name.clone()),
                (*id == aim.member).then_some("self"),
            )
        }
        Target::Stack(stack) => (format!("Stack {}", u16::from(*stack) + 1), None),
        Target::Door { x, y, facing } => {
            let way = toward(game, (*x, *y), Some(*facing));
            (format!("Door, {way}"), None)
        }
        Target::Object { object } => {
            let placement = placed(data, game, object);
            let kind = placement.and_then(|p| data.objects.get(&p.kind));
            let key = placement.and_then(|p| data.registry.text.name(p.name?));
            let name = key.or(kind.map(|k| k.name.as_str()));
            let name = name.map_or("Something", |key| labels::text(data, key));
            let way = placement.map_or("here", |p| toward(game, (p.x, p.y), None));
            (format!("{name}, {way}"), None)
        }
        Target::Being { being } => {
            let Some(group) = stood(data, game, being) else {
                return ("A group".to_owned(), None);
            };
            let monsters = &data.registry.monsters;
            let front = group.front().and_then(|id| monsters.name(id));
            let name = front.map_or("Something", |id| labels::monster(data, id));
            let count = group.stacks.iter().map(|s| u16::from(s.1)).sum();
            let way = toward(game, (group.x, group.y), None);
            (format!("{}, {way}", labels::count_label(name, count)), None)
        }
    }
}

/// The step's screen, standing in for the screen `id` it was opened from: what was chosen and
/// by whom, Back, then one row per possible target with its Choose button.
#[must_use]
pub fn content(data: &Data, game: Game, aim: &Aim, id: ScreenId) -> Screen {
    let (actor, _) = label(data, game, aim, &Target::Member(aim.member));
    let mut rows = vec![
        Row::controls(vec![Control::button(ControlId::Back, "Back")]),
        Row::text(Span::body(format!("{}, by {actor}", aim.named(data)))),
    ];
    let targets = aim.targets(data, game);
    for (i, row) in targets.iter().enumerate() {
        let Ok(at) = u8::try_from(i) else {
            break;
        };
        let (name, own) = label(data, game, aim, &row.target);
        let mut marks: Vec<String> = own.map(str::to_owned).into_iter().collect();
        if row.distance > 0 {
            let unit = if row.distance == 1 { "tile" } else { "tiles" };
            marks.push(format!("{} {unit}", row.distance));
        }
        if row.sight == Sight::Obscured {
            marks.push("out of sight".to_owned());
        }
        rows.push(Row {
            spans: vec![Span::body(name)],
            controls: vec![Control {
                why_not: row
                    .refusal
                    .as_ref()
                    .map(|r| labels::refused(data, &game.views.party, r)),
                ..Control::button(ControlId::Target(at), "Choose")
            }],
            after: if marks.is_empty() {
                Vec::new()
            } else {
                vec![note(marks.join("  "))]
            },
            ..Row::default()
        });
    }
    if targets.is_empty() {
        rows.push(Row::text(note("Nothing in reach")));
    }
    Screen {
        id,
        title: Span::title("Choose a target"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}
