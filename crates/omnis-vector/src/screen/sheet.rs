//! The character sheet (parity P6; the other client's sheet): one member's numbers on three
//! pages, Stats, Magic and Gear, picked from a choice under the member's. Opened from the party
//! column's Sheet, which stands beside it; in a fight it only looks, and its Tactics button,
//! which opens the member's declared reactions over it, is dim. Every figure is the member
//! view's; the words are labels.

use super::inventory::{owner, owner_choice, step_owner};
use super::{
    Control, ControlId, Effect, Forms, Game, Kind, Panel, Payload, Row, Screen, ScreenId, Span,
};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::{EffectView, MemberView};
use omnis_sim::omnis_data::{Ability, Data, EquipSlot};

/// One page of the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Page {
    /// Hit points, scores, saves, skills, conditions.
    #[default]
    Stats,
    /// Casting, known spells with their costs, effects in force.
    Magic,
    /// What is worn and wielded, and what is carried.
    Gear,
}

impl Page {
    /// Every page, in the choice's order.
    pub const ALL: [Page; 3] = [Page::Stats, Page::Magic, Page::Gear];

    /// The page's name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Page::Stats => "Stats",
            Page::Magic => "Magic",
            Page::Gear => "Gear",
        }
    }

    /// Where it stands in [`Page::ALL`].
    fn at(self) -> usize {
        Page::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }
}

/// A small dim note, and a heading within a page.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// A line of body text.
fn line(text: impl Into<String>) -> Row {
    Row::text(Span::body(text))
}

/// Items joined two spaces apart, `per` to a row, or "none" under the heading.
fn grouped(rows: &mut Vec<Row>, heading: &str, items: &[String], per: usize) {
    rows.push(Row::text(note(heading)));
    if items.is_empty() {
        rows.push(line("none"));
    }
    for chunk in items.chunks(per.max(1)) {
        rows.push(line(chunk.join("  ")));
    }
}

fn stats(data: &Data, m: &MemberView) -> Vec<Row> {
    let mut rows = vec![
        line(format!(
            "HP {}/{}  SP {}/{}  AC {}  Proficiency {}",
            m.hp,
            m.hp_max,
            m.spell_points,
            m.spell_points_max,
            m.ac,
            labels::signed(m.proficiency)
        )),
        line(format!(
            "XP {} / {}",
            m.xp,
            m.next_xp
                .map_or_else(|| "max".to_owned(), |n| n.to_string())
        )),
    ];
    let mut dice = format!("Hit dice {}/{} d{}", m.hit_dice_left, m.hit_dice, m.hit_die);
    if m.ready {
        dice.push_str("  Ready to train");
    }
    if m.spell_picks > 0 {
        dice.push_str(&format!("  Spell picks {}", m.spell_picks));
    }
    rows.push(line(dice));
    let scores: Vec<String> = Ability::ALL
        .iter()
        .map(|a| {
            format!(
                "{} {} {}",
                a.short(),
                m.scores[a.index()],
                labels::signed(m.modifiers[a.index()])
            )
        })
        .collect();
    for chunk in scores.chunks(3) {
        rows.push(line(chunk.join("  ")));
    }
    let saves: Vec<String> = m
        .saves
        .iter()
        .map(|(a, bonus)| format!("{} {}", a.short(), labels::signed(*bonus)))
        .collect();
    rows.push(line(if saves.is_empty() {
        "Saves  none".to_owned()
    } else {
        format!("Saves  {}", saves.join("  "))
    }));
    let skills: Vec<String> = m
        .skills
        .iter()
        .map(|(s, bonus)| format!("{} {}", labels::skill(*s), labels::signed(*bonus)))
        .collect();
    grouped(&mut rows, "Skills", &skills, 3);
    let conditions: Vec<&str> = m
        .conditions
        .iter()
        .map(|c| labels::condition(data, c))
        .collect();
    rows.push(line(if conditions.is_empty() {
        "Conditions  none".to_owned()
    } else {
        format!("Conditions  {}", conditions.join(", "))
    }));
    rows.push(line(format!(
        "{}  {}  {} years",
        labels::background(data, &m.background),
        labels::alignment(m.alignment),
        m.age_years
    )));
    rows
}

/// An effect's time left: "10 min", or "next turn" for one that ends at the bearer's turn.
fn left(effect: &EffectView) -> String {
    effect
        .minutes_left
        .map_or_else(|| "next turn".to_owned(), |m| format!("{m} min"))
}

fn magic(data: &Data, game: Game, m: &MemberView) -> Vec<Row> {
    let mut rows = vec![line(m.casting.map_or_else(
        || "No spellcasting".to_owned(),
        |a| {
            format!(
                "Casting {}  Points {}/{}",
                labels::ability(a),
                m.spell_points,
                m.spell_points_max
            )
        },
    ))];
    rows.push(Row::text(note("Spells")));
    if m.known_spells.is_empty() {
        rows.push(line("none"));
    }
    for k in &m.known_spells {
        let cost = if k.cost == 0 {
            "cantrip".to_owned()
        } else {
            format!("{} pt", k.cost)
        };
        rows.push(Row {
            tint: None,
            spans: vec![Span::body(labels::spell(data, &k.spell))],
            controls: Vec::new(),
            after: vec![note(format!("{cost}  {}", labels::pay(k.pay)))],
        });
    }
    rows.push(Row::text(note("Effects")));
    let effects: Vec<&EffectView> = m.effects.iter().chain(&game.views.party.effects).collect();
    if effects.is_empty() {
        rows.push(line("none"));
    }
    for e in effects {
        rows.push(Row {
            tint: None,
            spans: vec![Span::body(labels::spell(data, &e.spell))],
            controls: Vec::new(),
            after: vec![note(left(e))],
        });
    }
    rows
}

fn gear(data: &Data, m: &MemberView) -> Vec<Row> {
    let mut rows = vec![Row::text(note("Worn and wielded"))];
    for slot in EquipSlot::ALL {
        let item = m
            .worn
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or("-", |(_, id)| labels::item(data, id));
        rows.push(line(format!("{}  {item}", labels::slot(slot))));
    }
    let carried: Vec<String> = m
        .equipment
        .iter()
        .map(|i| labels::count_label(labels::item(data, &i.item), i.count))
        .collect();
    grouped(&mut rows, "Carried", &carried, 2);
    rows
}

/// The sheet as it stands.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    let Some(m) = owner(game, forms) else {
        return super::empty(ScreenId::Sheet);
    };
    let members = &game.views.party.members;
    let at = members
        .iter()
        .position(|x| x.member == m.member)
        .unwrap_or(0);
    let mut rows = vec![
        Row {
            tint: None,
            spans: Vec::new(),
            controls: vec![
                owner_choice(game, m.member, "Member"),
                Control::button(ControlId::Close, "Close"),
            ],
            after: vec![note(format!(
                "{} {} {}  member {} of {}",
                labels::race(data, &m.race),
                labels::class(data, &m.class),
                m.level,
                at + 1,
                members.len()
            ))],
        },
        Row::controls(vec![
            Control {
                id: ControlId::Page,
                label: "Page".to_owned(),
                kind: Kind::Choice {
                    options: Page::ALL.iter().map(|p| p.label().to_owned()).collect(),
                    at: forms.page.at(),
                },
                why_not: None,
            },
            Control {
                why_not: game
                    .views
                    .combat
                    .is_some()
                    .then(|| "Not in a fight".to_owned()),
                ..Control::button(ControlId::Tactics, "Tactics")
            },
        ]),
    ];
    rows.extend(match forms.page {
        Page::Stats => stats(data, m),
        Page::Magic => magic(data, game, m),
        Page::Gear => gear(data, m),
    });
    Screen {
        id: ScreenId::Sheet,
        title: Span::title("Sheet"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// The member or the page steps; Tactics opens over the sheet; Close closes.
pub fn apply(game: Game, forms: &mut Forms, control: ControlId, payload: &Payload) -> Effect {
    match control {
        ControlId::Owner => {
            step_owner(game, forms, payload);
            Effect::None
        }
        ControlId::Page => {
            let n = Page::ALL.len();
            let at = forms.page.at();
            forms.page = Page::ALL[match payload {
                Payload::Left => (at + n - 1) % n,
                _ => (at + 1) % n,
            }];
            Effect::None
        }
        ControlId::Tactics => Effect::Open(ScreenId::Tactics),
        ControlId::Close => Effect::Back,
        _ => Effect::None,
    }
}
