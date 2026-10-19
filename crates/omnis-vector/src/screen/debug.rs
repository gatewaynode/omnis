//! The debug panel (parity P8, presentation-ARCHITECTURE §10.6; the retired client's debug panel):
//! a devtools world's edits on four pages, switched like the sheet's (owner, 2026-10-11): a
//! member's numbers and conditions; the party's money, food and items; the world's flags and a
//! teleport; the fight's stacks. Numbers are typed (digits only) and sent by the button beside
//! them, saturated to the command's type with no game limit, as the retired client did. Every
//! button is lit by a trial of the command it sends, so the engine's refusal stands where the
//! other client had its own greys ("no teleport in a fight"). A field shows what was typed, or
//! else the world's value; an accepted edit clears what was typed, so the field reads the world.

use super::{
    Control, ControlId, Effect, Game, HostOp, Kind, Panel, Payload, Refused, Row, Screen, ScreenId,
    Span,
};
use crate::labels;
use crate::style::Role;
use crate::trial;
use omnis_sim::api::{MemberView, StackView};
use omnis_sim::omnis_core::{CharacterId, Facing, Level};
use omnis_sim::omnis_data::{Ability, Data};
use omnis_sim::{Command, DevCommand};
use std::collections::BTreeMap;

/// Why the panel cannot be opened in this game: a world without devtools refuses every edit, and
/// a build without the `devtools` feature has no panel.
#[must_use]
pub fn unavailable(views: &crate::sim::Views) -> Option<String> {
    if !cfg!(feature = "devtools") {
        Some("This build has no debug panel".to_owned())
    } else if !views.here.settings.devtools {
        Some("Devtools are off in this game".to_owned())
    } else {
        None
    }
}

/// The most digits a number field takes.
pub const DIGITS: usize = 10;

/// One page of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Page {
    /// The member's numbers and conditions.
    #[default]
    Member,
    /// Money, food and items.
    Party,
    /// Flags and the teleport.
    World,
    /// The fight's stacks.
    Fight,
}

impl Page {
    /// Every page, in the choice's order.
    pub const ALL: [Page; 4] = [Page::Member, Page::Party, Page::World, Page::Fight];

    /// The page's name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Page::Member => "Member",
            Page::Party => "Party",
            Page::World => "World",
            Page::Fight => "Fight",
        }
    }
}

/// A number field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Num {
    /// The member's hit points.
    Hp,
    /// The member's spell points.
    Sp,
    /// The member's experience.
    Xp,
    /// The chosen ability's score.
    Score,
    /// The party's money, in copper.
    Copper,
    /// The party's food.
    Food,
    /// How many of the item to give.
    Count,
    /// The value to set the flag to.
    Flag,
    /// The teleport's column.
    X,
    /// The teleport's row.
    Y,
    /// The chosen stack's first living monster's hit points.
    Lead,
}

/// A control of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugAct {
    /// The page.
    Page,
    /// Whose numbers.
    Member,
    /// A number field.
    Field(Num),
    /// Send a number field's edit.
    Set(Num),
    /// Which ability's score.
    Ability,
    /// Which condition.
    Condition,
    /// Put the condition on or take it off.
    Toggle,
    /// Which item.
    Item,
    /// Give the item to the member.
    ToMember,
    /// Give the item to the party's stores.
    ToStores,
    /// Which flag.
    Flag,
    /// Which map to teleport to.
    Map,
    /// Which level of it (M8.5 step 1b).
    Level,
    /// Which way to face there.
    Facing,
    /// Teleport.
    Go,
    /// Which stack.
    Stack,
    /// Kill the stack.
    Kill,
    /// Cut the world into its parts and join them back (M8.5 7b): report, not a command.
    Split,
    /// Fold the world now (M8.5 7e).
    Fold,
    /// List the world-folder files no save names (M8.5 7e): the host's work.
    Tidy,
    /// Delete the files Tidy listed: the host's work, lit once Tidy found some.
    Delete,
    /// Reload the packs from disk under the running world, by name (M8.5 7h5): the host's work.
    Reload,
}

const FACINGS: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

/// The panel's state: the picks and what was typed, by field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DebugForm {
    /// The page shown.
    pub page: Page,
    /// Whose numbers; the first member when unset or gone.
    pub member: Option<CharacterId>,
    ability: usize,
    condition: usize,
    item: usize,
    flag: usize,
    /// The teleport's map, by registry order; the party's map when unset.
    map: Option<usize>,
    /// The teleport's level; the party's when it stays on its map, else the ground, when unset.
    level: Option<Level>,
    facing: Option<Facing>,
    stack: usize,
    /// What was typed, by field; a field without an edit shows the world's value.
    edits: BTreeMap<Num, String>,
    /// The last split's lines and the turn it was taken on; shown only while the world is
    /// still at that turn.
    split: Option<(u64, Vec<String>)>,
    /// What the last tidy listed: the files no save names (M8.5 7e).
    tidy: Option<Vec<String>>,
}

impl DebugForm {
    /// The host listed the world-folder files no save names, or deleted them (`None`).
    pub fn tidied(&mut self, files: Option<Vec<String>>) {
        self.tidy = files;
    }

    /// The command was accepted: its field reads the world again.
    pub fn accepted(&mut self, command: &DevCommand) {
        let num = match command {
            DevCommand::SetHp { .. } => Num::Hp,
            DevCommand::SetSpellPoints { .. } => Num::Sp,
            DevCommand::SetXp { .. } => Num::Xp,
            DevCommand::SetScore { .. } => Num::Score,
            DevCommand::SetGold { .. } => Num::Copper,
            DevCommand::SetFood { .. } => Num::Food,
            DevCommand::SetFlag { .. } => Num::Flag,
            DevCommand::SetMonsterHp { .. } => Num::Lead,
            DevCommand::Teleport { .. } => {
                self.map = None;
                self.level = None;
                self.facing = None;
                self.edits.remove(&Num::X);
                self.edits.remove(&Num::Y);
                return;
            }
            _ => return,
        };
        self.edits.remove(&num);
    }
}

/// What the panel reads, gathered once a build.
struct Look<'a> {
    data: &'a Data,
    game: Game<'a>,
    form: &'a DebugForm,
    member: Option<&'a MemberView>,
    stack: Option<&'a StackView>,
}

impl<'a> Look<'a> {
    fn of(data: &'a Data, game: Game<'a>, form: &'a DebugForm) -> Look<'a> {
        let members = &game.views.party.members;
        let member = form
            .member
            .and_then(|id| members.iter().find(|m| m.member == id))
            .or_else(|| members.first());
        let stack = game.views.combat.as_ref().and_then(|c| {
            c.stacks
                .iter()
                .filter(|s| !s.hps.is_empty())
                .nth(form.stack)
        });
        Look {
            data,
            game,
            form,
            member,
            stack,
        }
    }

    fn conditions(&self) -> Vec<&'a str> {
        self.data.registry.conditions.names().collect()
    }

    fn items(&self) -> Vec<&'a str> {
        self.data.registry.items.names().collect()
    }

    fn maps(&self) -> Vec<&'a str> {
        self.data.registry.maps.names().collect()
    }

    /// The teleport's map: the one picked, else the party's.
    fn map(&self) -> usize {
        let here = &self.game.views.here.position.map;
        self.form
            .map
            .unwrap_or_else(|| self.maps().iter().position(|m| m == here).unwrap_or(0))
    }

    /// The levels of the teleport's map, lowest first; the ground alone for a map not loaded.
    fn levels(&self) -> Vec<Level> {
        self.loaded(self.maps()[self.map()]).map_or_else(
            || vec![Level::GROUND],
            |m| m.levels.iter().map(|l| l.level).collect(),
        )
    }

    /// The teleport's level: the one picked, else the party's on its own map, else the ground.
    fn level(&self) -> Level {
        let here = &self.game.views.here.position;
        self.form.level.unwrap_or_else(|| {
            if self.maps()[self.map()] == here.map {
                here.level
            } else {
                Level::GROUND
            }
        })
    }

    /// A loaded map by its string id.
    fn loaded(&self, id: &str) -> Option<&'a omnis_sim::omnis_data::MapData> {
        self.data
            .registry
            .maps
            .get(id)
            .and_then(|m| self.data.maps.get(&m))
    }

    /// A map's size in cells.
    fn size(&self, id: &str) -> (u16, u16) {
        self.data
            .registry
            .maps
            .get(id)
            .and_then(|m| self.data.maps.get(&m))
            .map_or((1, 1), |m| (m.def.width, m.def.height))
    }

    /// The world's value a field shows when nothing was typed.
    fn value(&self, num: Num) -> String {
        let m = self.member;
        let here = &self.game.views.here.position;
        match num {
            Num::Hp => m.map_or(String::new(), |m| m.hp.max(0).to_string()),
            Num::Sp => m.map_or(String::new(), |m| m.spell_points.to_string()),
            Num::Xp => m.map_or(String::new(), |m| m.xp.to_string()),
            Num::Score => m.map_or(String::new(), |m| m.scores[self.form.ability].to_string()),
            Num::Copper => self.game.views.party.gold.to_string(),
            Num::Food => self.game.views.party.food.to_string(),
            Num::Count | Num::Flag => "1".to_owned(),
            Num::X if self.form.map.is_none() => here.x.to_string(),
            Num::Y if self.form.map.is_none() => here.y.to_string(),
            Num::X | Num::Y => "0".to_owned(),
            Num::Lead => self
                .stack
                .and_then(|s| s.hps.first())
                .map_or(String::new(), |hp| hp.max(&0).to_string()),
        }
    }

    /// What a field holds: the edit, else the world's value.
    fn text(&self, num: Num) -> String {
        self.form
            .edits
            .get(&num)
            .cloned()
            .unwrap_or_else(|| self.value(num))
    }

    /// A field's number, saturated to `T`; `None` while it is empty.
    fn number<T: TryFrom<u64> + Bounded>(&self, num: Num) -> Option<T> {
        let text = self.text(num);
        if text.is_empty() {
            return None;
        }
        let n: u64 = text.parse().unwrap_or(u64::MAX);
        Some(T::try_from(n).unwrap_or(T::MAX))
    }

    /// The command a button sends now, or why it cannot be built.
    fn command(&self, act: DebugAct) -> Result<DevCommand, String> {
        let typed = "type a number".to_owned();
        let member = || self.member.map(|m| m.member).ok_or("no member to edit");
        Ok(match act {
            DebugAct::Set(Num::Hp) => DevCommand::SetHp {
                member: member()?,
                hp: self.number(Num::Hp).ok_or(typed)?,
            },
            DebugAct::Set(Num::Sp) => DevCommand::SetSpellPoints {
                member: member()?,
                points: self.number(Num::Sp).ok_or(typed)?,
            },
            DebugAct::Set(Num::Xp) => DevCommand::SetXp {
                member: member()?,
                xp: self.number(Num::Xp).ok_or(typed)?,
            },
            DebugAct::Set(Num::Score) => DevCommand::SetScore {
                member: member()?,
                ability: Ability::ALL[self.form.ability],
                score: self.number(Num::Score).ok_or(typed)?,
            },
            DebugAct::Set(Num::Copper) => DevCommand::SetGold {
                gold: self.number(Num::Copper).ok_or(typed)?,
            },
            DebugAct::Set(Num::Food) => DevCommand::SetFood {
                food: self.number(Num::Food).ok_or(typed)?,
            },
            DebugAct::Set(Num::Flag) => DevCommand::SetFlag {
                flag: self
                    .game
                    .views
                    .flags
                    .get(self.form.flag)
                    .map(|(id, _)| id.clone())
                    .ok_or("the packs declare no flags")?,
                value: self.number(Num::Flag).ok_or(typed)?,
            },
            DebugAct::Set(Num::Lead) => DevCommand::SetMonsterHp {
                stack: self.stack.map(|s| s.stack).ok_or("no fight is on")?,
                index: 0,
                hp: self.number(Num::Lead).ok_or(typed)?,
            },
            DebugAct::Toggle => {
                let m = self.member.ok_or("no member to edit")?;
                let condition = self
                    .conditions()
                    .get(self.form.condition)
                    .map(|c| (*c).to_owned())
                    .ok_or("the packs declare no conditions")?;
                DevCommand::SetCondition {
                    member: m.member,
                    applied: !m.conditions.contains(&condition),
                    condition,
                }
            }
            DebugAct::ToMember | DebugAct::ToStores => DevCommand::GiveItem {
                member: if act == DebugAct::ToMember {
                    Some(member()?)
                } else {
                    None
                },
                item: self
                    .items()
                    .get(self.form.item)
                    .map(|i| (*i).to_owned())
                    .ok_or("the packs declare no items")?,
                count: self.number::<u16>(Num::Count).ok_or(typed)?.max(1),
            },
            DebugAct::Go => DevCommand::Teleport {
                map: self.maps()[self.map()].to_owned(),
                level: self.level(),
                x: self.number(Num::X).ok_or(typed.clone())?,
                y: self.number(Num::Y).ok_or(typed)?,
                facing: self.facing(),
            },
            DebugAct::Kill => DevCommand::KillStack {
                stack: self.stack.map(|s| s.stack).ok_or("no fight is on")?,
            },
            DebugAct::Fold => DevCommand::Fold,
            _ => return Err(String::new()),
        })
    }

    fn facing(&self) -> Facing {
        self.form
            .facing
            .unwrap_or(self.game.views.here.position.facing)
    }

    /// A button that sends `act`'s command, lit by its trial.
    fn button(&self, act: DebugAct, label: &str) -> Control {
        let why_not = match self.command(act) {
            Ok(dev) => trial::why(
                self.game.world,
                self.data,
                &self.game.views.party,
                &Command::Dev(dev),
            ),
            Err(why) => Some(why),
        };
        Control {
            why_not,
            ..Control::button(ControlId::Debug(act), label)
        }
    }

    fn field(&self, num: Num, label: &str) -> Control {
        Control {
            id: ControlId::Debug(DebugAct::Field(num)),
            label: label.to_owned(),
            kind: Kind::Text {
                value: self.text(num),
                max: DIGITS,
            },
            why_not: None,
        }
    }

    /// A field, its Set button, and a note after them.
    fn set_row(&self, num: Num, label: &str, after: Option<String>) -> Row {
        Row {
            controls: vec![
                self.field(num, label),
                self.button(DebugAct::Set(num), "Set"),
            ],
            after: after.map(note).into_iter().collect(),
            ..Row::default()
        }
    }
}

/// Numbers a field saturates to.
pub trait Bounded {
    /// The largest.
    const MAX: Self;
}

macro_rules! bounded {
    ($($t:ty),*) => { $(impl Bounded for $t { const MAX: Self = <$t>::MAX; })* };
}
bounded!(u8, u16, u32, i32, i64);

fn note(text: String) -> Span {
    Span {
        role: Role::Dim,
        ..Span::body(text)
    }
}

fn choice(act: DebugAct, label: &str, options: Vec<String>, at: usize) -> Control {
    Control {
        id: ControlId::Debug(act),
        label: label.to_owned(),
        kind: Kind::Choice { options, at },
        why_not: None,
    }
}

fn member_rows(look: &Look) -> Vec<Row> {
    let Some(m) = look.member else {
        return vec![Row::text(note("No member to edit".to_owned()))];
    };
    let conditions = look.conditions();
    let condition = conditions.get(look.form.condition).copied().unwrap_or("");
    let on = m.conditions.iter().any(|c| c == condition);
    vec![
        look.set_row(Num::Hp, "Hit points", Some(format!("of {}", m.hp_max))),
        look.set_row(
            Num::Sp,
            "Spell points",
            Some(format!("of {}", m.spell_points_max)),
        ),
        look.set_row(Num::Xp, "Experience", Some(format!("level {}", m.level))),
        Row {
            controls: vec![
                choice(
                    DebugAct::Ability,
                    "Ability",
                    Ability::ALL.iter().map(|a| a.short().to_owned()).collect(),
                    look.form.ability,
                ),
                look.field(Num::Score, "Score"),
                look.button(DebugAct::Set(Num::Score), "Set"),
            ],
            ..Row::default()
        },
        Row {
            controls: vec![
                choice(
                    DebugAct::Condition,
                    "Condition",
                    conditions
                        .iter()
                        .map(|c| labels::condition(look.data, c).to_owned())
                        .collect(),
                    look.form.condition,
                ),
                look.button(DebugAct::Toggle, "On / off"),
            ],
            after: vec![note(if on { "on" } else { "off" }.to_owned())],
            ..Row::default()
        },
    ]
}

fn party_rows(look: &Look) -> Vec<Row> {
    let items = look.items();
    vec![
        look.set_row(
            Num::Copper,
            "Copper",
            Some(labels::coins(look.game.views.party.gold)),
        ),
        look.set_row(Num::Food, "Food", None),
        Row::controls(vec![
            choice(
                DebugAct::Item,
                "Item",
                items
                    .iter()
                    .map(|i| labels::item(look.data, i).to_owned())
                    .collect(),
                look.form.item,
            ),
            look.field(Num::Count, "Count"),
        ]),
        Row::controls(vec![
            look.button(DebugAct::ToMember, "To member"),
            look.button(DebugAct::ToStores, "To stores"),
        ]),
    ]
}

fn world_rows(look: &Look) -> Vec<Row> {
    let flags = &look.game.views.flags;
    let mut rows = if flags.is_empty() {
        vec![Row::text(note("The packs declare no flags".to_owned()))]
    } else {
        vec![Row::controls(vec![
            choice(
                DebugAct::Flag,
                "Flag",
                flags
                    .iter()
                    .map(|(id, v)| {
                        let whose = match omnis_sim::api::flag_scope(look.data, id) {
                            Some(omnis_sim::api::FlagScope::Party) => "party",
                            _ => "world",
                        };
                        format!("{id} ({whose}, now {v})")
                    })
                    .collect(),
                look.form.flag,
            ),
            look.field(Num::Flag, "Value"),
            look.button(DebugAct::Set(Num::Flag), "Set"),
        ])]
    };
    let maps = look.maps();
    rows.push(Row::controls(vec![choice(
        DebugAct::Map,
        "Map",
        maps.iter()
            .map(|m| {
                let (w, h) = look.size(m);
                format!("{m} ({w} by {h})")
            })
            .collect(),
        look.map(),
    )]));
    let levels = look.levels();
    let level = look.level();
    let map = look.loaded(maps[look.map()]);
    rows.push(Row::controls(vec![choice(
        DebugAct::Level,
        "Level",
        levels
            .iter()
            .map(|&l| {
                map.and_then(|m| crate::labels::level(look.data, m, l))
                    .unwrap_or_else(|| l.to_string())
            })
            .collect(),
        levels.iter().position(|&l| l == level).unwrap_or(0),
    )]));
    rows.push(Row::controls(vec![
        look.field(Num::X, "x"),
        look.field(Num::Y, "y"),
    ]));
    let facing = look.facing();
    rows.push(Row::controls(vec![
        choice(
            DebugAct::Facing,
            "Facing",
            FACINGS.iter().map(ToString::to_string).collect(),
            FACINGS.iter().position(|f| *f == facing).unwrap_or(0),
        ),
        look.button(DebugAct::Go, "Go"),
    ]));
    if let Some((turn, lines)) = &look.form.split
        && *turn == look.game.views.here.turn
    {
        rows.extend(lines.iter().map(|line| Row::text(note(line.clone()))));
    }
    rows.push(Row::controls(vec![Control::button(
        ControlId::Debug(DebugAct::Split),
        "Split",
    )]));
    rows.push(Row::text(note(journal_line(
        look.game.views.journal,
        look.game.views.archive,
    ))));
    rows.push(Row::text(note(names_line(look.game.views.names))));
    rows.push(Row::controls(vec![
        look.button(DebugAct::Fold, "Fold"),
        Control::button(ControlId::Debug(DebugAct::Tidy), "Tidy"),
        Control::button(ControlId::Debug(DebugAct::Reload), "Reload packs"),
        Control {
            why_not: match &look.form.tidy {
                Some(files) if !files.is_empty() => None,
                Some(_) => Some("nothing to delete".to_owned()),
                None => Some("tidy first".to_owned()),
            },
            ..Control::button(ControlId::Debug(DebugAct::Delete), "Delete them")
        },
    ]));
    if let Some(files) = &look.form.tidy {
        rows.extend(tidy_lines(files).into_iter().map(|l| Row::text(note(l))));
    }
    rows
}

/// What the last tidy listed: how many, then the first few.
fn tidy_lines(files: &[String]) -> Vec<String> {
    const SHOWN: usize = 8;
    let mut lines = vec![match files.len() {
        0 => "Every world file is named by a save".to_owned(),
        1 => "1 file no save names".to_owned(),
        n => format!("{n} files no save names"),
    }];
    lines.extend(files.iter().take(SHOWN).cloned());
    if files.len() > SHOWN {
        lines.push(format!("and {} more", files.len() - SHOWN));
    }
    lines
}

/// The world's parts by size and whether they join back (PRD D28, M8.5 7b).
fn split_lines(data: &Data, world: &crate::sim::SimWorld) -> Vec<String> {
    let report = match world.split_report(data) {
        Ok(report) => report,
        Err(e) => return vec![format!("Split failed: {e}")],
    };
    let mut lines = vec![format!(
        "World base: {} bytes; checkpoint: {} bytes",
        report.world_bytes, report.checkpoint_bytes
    )];
    lines.extend(report.regions.iter().map(|r| {
        let maps = if r.maps == 1 { "map" } else { "maps" };
        format!("{}: {} bytes, {} {maps}", r.id, r.bytes, r.maps)
    }));
    lines.push(format!(
        "Flags set: {} the party's, {} the world's",
        report.party_flags, report.world_flags
    ));
    if report.regions.is_empty() {
        lines.push("No region has changed: each is as its pack wrote it".to_owned());
    }
    lines.push(
        if report.round_trip {
            "Round trip: ok"
        } else {
            "Round trip: differs"
        }
        .to_owned(),
    );
    lines
}

/// The journal a load lays over the world's bases (PRD D28, M8.5 7d): the interactions since
/// the last fold, not the moves; and the archive the folds put away (7e).
fn journal_line(journal: usize, archive: usize) -> String {
    let interactions = if journal == 1 {
        "interaction"
    } else {
        "interactions"
    };
    let segments = if archive == 1 { "segment" } else { "segments" };
    format!("Journal: {journal} {interactions} since the last fold; archive: {archive} {segments}")
}

/// The names a save writes into its world's manifest (M8.5 7h2): what its numbers stand for.
fn names_line(names: usize) -> String {
    format!("Manifest: {names} names, what the save's numbers stand for")
}

fn fight_rows(look: &Look) -> Vec<Row> {
    let living: Vec<&StackView> = look
        .game
        .views
        .combat
        .as_ref()
        .map(|c| c.stacks.iter().filter(|s| !s.hps.is_empty()).collect())
        .unwrap_or_default();
    if living.is_empty() {
        return vec![Row::text(note("No fight is on".to_owned()))];
    }
    vec![
        Row::controls(vec![choice(
            DebugAct::Stack,
            "Stack",
            living
                .iter()
                .map(|s| {
                    format!(
                        "{} ({} of {})",
                        labels::text(look.data, &s.name),
                        s.hps.len(),
                        s.initial
                    )
                })
                .collect(),
            look.form.stack.min(living.len() - 1),
        )]),
        look.set_row(Num::Lead, "Lead hit points", None),
        Row::controls(vec![look.button(DebugAct::Kill, "Kill")]),
    ]
}

/// The screen.
#[must_use]
pub fn content(data: &Data, game: Game, form: &DebugForm) -> Screen {
    let look = Look::of(data, game, form);
    let mut head = vec![choice(
        DebugAct::Page,
        "Page",
        Page::ALL.iter().map(|p| p.label().to_owned()).collect(),
        Page::ALL.iter().position(|p| *p == form.page).unwrap_or(0),
    )];
    let members = &game.views.party.members;
    if matches!(form.page, Page::Member | Page::Party) && !members.is_empty() {
        head.push(choice(
            DebugAct::Member,
            "Member",
            members.iter().map(|m| m.name.clone()).collect(),
            look.member
                .and_then(|m| members.iter().position(|x| x.member == m.member))
                .unwrap_or(0),
        ));
    }
    let mut rows = vec![Row::controls(head)];
    rows.extend(match form.page {
        Page::Member => member_rows(&look),
        Page::Party => party_rows(&look),
        Page::World => world_rows(&look),
        Page::Fight => fight_rows(&look),
    });
    rows.push(Row::controls(vec![Control::button(
        ControlId::Close,
        "Close",
    )]));
    Screen {
        id: ScreenId::Debug,
        title: Span::title("DEBUG"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// One step of a choice over `n` options from `at`: left goes back, a press or right on, round.
fn step(at: usize, n: usize, payload: &Payload) -> usize {
    if n == 0 {
        return 0;
    }
    match payload {
        Payload::Left => (at + n - 1) % n,
        _ => (at + 1) % n,
    }
}

/// What each control does.
///
/// # Errors
/// [`Refused::NotDigits`] when a number field is given anything but digits.
pub fn apply(
    data: &Data,
    game: Game,
    form: &mut DebugForm,
    control: ControlId,
    payload: &Payload,
) -> Result<Effect, Refused> {
    let ControlId::Debug(act) = control else {
        return Ok(match control {
            ControlId::Close => Effect::Back,
            _ => Effect::None,
        });
    };
    // The form as it was, to read from while it changes.
    let was = form.clone();
    let look = Look::of(data, game, &was);
    let members = &game.views.party.members;
    match act {
        DebugAct::Page => {
            let at = Page::ALL.iter().position(|p| *p == form.page).unwrap_or(0);
            form.page = Page::ALL[step(at, Page::ALL.len(), payload)];
        }
        DebugAct::Member => {
            let at = look
                .member
                .and_then(|m| members.iter().position(|x| x.member == m.member))
                .unwrap_or(0);
            form.member = members
                .get(step(at, members.len(), payload))
                .map(|m| m.member);
            form.edits
                .retain(|n, _| !matches!(n, Num::Hp | Num::Sp | Num::Xp | Num::Score));
        }
        DebugAct::Ability => {
            form.ability = step(form.ability, Ability::ALL.len(), payload);
            form.edits.remove(&Num::Score);
        }
        DebugAct::Condition => {
            form.condition = step(form.condition, look.conditions().len(), payload)
        }
        DebugAct::Item => form.item = step(form.item, look.items().len(), payload),
        DebugAct::Flag => {
            form.flag = step(form.flag, game.views.flags.len(), payload);
            form.edits.remove(&Num::Flag);
        }
        DebugAct::Map => {
            let maps = look.maps();
            let at = step(look.map(), maps.len(), payload);
            let (w, h) = look.size(maps[at]);
            let held = |num: Num, most: u16| {
                let n: u16 = look.number(num).unwrap_or(0);
                n.min(most.saturating_sub(1)).to_string()
            };
            let (x, y) = (held(Num::X, w), held(Num::Y, h));
            form.map = Some(at);
            form.level = None;
            form.edits.insert(Num::X, x);
            form.edits.insert(Num::Y, y);
        }
        DebugAct::Level => {
            let levels = look.levels();
            let at = levels.iter().position(|&l| l == look.level()).unwrap_or(0);
            form.level = levels.get(step(at, levels.len(), payload)).copied();
        }
        DebugAct::Facing => {
            let at = FACINGS
                .iter()
                .position(|f| *f == look.facing())
                .unwrap_or(0);
            form.facing = Some(FACINGS[step(at, FACINGS.len(), payload)]);
        }
        DebugAct::Stack => {
            let n = game
                .views
                .combat
                .as_ref()
                .map_or(0, |c| c.stacks.iter().filter(|s| !s.hps.is_empty()).count());
            form.stack = step(form.stack, n, payload);
            form.edits.remove(&Num::Lead);
        }
        DebugAct::Field(num) => {
            let Payload::Text(text) = payload else {
                return Ok(Effect::None);
            };
            if !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(Refused::NotDigits);
            }
            form.edits.insert(num, text.clone());
        }
        DebugAct::Split => {
            form.split = Some((game.views.here.turn, split_lines(data, game.world)));
        }
        DebugAct::Tidy => return Ok(Effect::Host(HostOp::Tidy { delete: false })),
        DebugAct::Delete => return Ok(Effect::Host(HostOp::Tidy { delete: true })),
        DebugAct::Reload => return Ok(Effect::Host(HostOp::Reload { migrate: false })),
        DebugAct::Set(_)
        | DebugAct::Toggle
        | DebugAct::ToMember
        | DebugAct::ToStores
        | DebugAct::Go
        | DebugAct::Kill
        | DebugAct::Fold => {
            return Ok(look
                .command(act)
                .map_or(Effect::None, |dev| Effect::Command {
                    command: Command::Dev(dev),
                    close: false,
                }));
        }
    }
    Ok(Effect::None)
}
