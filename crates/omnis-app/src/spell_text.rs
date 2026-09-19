//! Spell events as text: casts, healing, effects settling and ending, concentration, and the
//! auto-cast switch. Sits beside `combat_text.rs`, which pairs attacks with their damage and
//! renders checks; this file renders what casting adds. Bevy-free.

use crate::text::{Line, Names, trace_math};
use omnis_sim::{EffectEnd, EffectTarget, Event};

/// One line for a spell event, or `None` for events this file does not render.
#[must_use]
pub fn spell_line(event: &Event, names: &Names) -> Option<Line> {
    Some(match event {
        Event::SpellCast {
            caster,
            spell,
            points,
            components_consumed,
        } => {
            let who = names.member(*caster);
            let what = names.spell(*spell);
            let cost = match points {
                0 => "free".to_owned(),
                1 => "1 pt".to_owned(),
                n => format!("{n} pt"),
            };
            let parts = if components_consumed.is_empty() {
                String::new()
            } else {
                format!(", {} component(s)", components_consumed.len())
            };
            Line::same(format!("{who} casts {what} ({cost}{parts})"))
        }
        Event::Healed {
            target,
            rolls,
            amount,
            hp,
        } => {
            let who = names.member(*target);
            let math = rolls.iter().map(trace_math).collect::<Vec<_>>().join(" ");
            let outcome = format!("{who} regains {amount} hp, now {hp}");
            if math.is_empty() {
                Line::same(outcome)
            } else {
                Line::new(format!("{outcome} ({math})"), outcome)
            }
        }
        Event::EffectApplied { target, spell, .. } => {
            let what = names.spell(*spell);
            Line::same(match target {
                EffectTarget::Member(id) => format!("{what} settles on {}", names.member(*id)),
                EffectTarget::Party => format!("{what} lights the party's way"),
            })
        }
        Event::EffectEnded { target, spell, why } => {
            let what = names.spell(*spell);
            let whom = match target {
                EffectTarget::Member(id) => names.member(*id).to_owned(),
                EffectTarget::Party => "the party".to_owned(),
            };
            Line::same(match why {
                EffectEnd::Expired => format!("{what} fades from {whom}"),
                EffectEnd::Consumed => format!("{what} is spent on {whom}"),
                EffectEnd::Concentration => format!("{what} leaves {whom}"),
                EffectEnd::TurnBegan => format!("{what} lapses on {whom}"),
                EffectEnd::FightOver => format!("{what} ends with the fight for {whom}"),
            })
        }
        Event::Concentration { caster, spell, .. } => Line::same(format!(
            "{} lets {} go",
            names.member(*caster),
            names.spell(*spell)
        )),
        Event::AutoCast { member, spell, on } => Line::same(format!(
            "{} will {}cast {} on their own",
            names.member(*member),
            if *on { "" } else { "no longer " },
            names.spell(*spell)
        )),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_text::batch_lines;
    use omnis_sim::omnis_core::{CharacterId, SpellId};
    use omnis_sim::omnis_data::load_packs;
    use omnis_sim::{Command, PartyCommand, Settings, World};
    use std::path::PathBuf;

    fn names() -> (Names, SpellId, CharacterId) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")])
            .unwrap_or_else(|r| panic!("{r}"));
        let mut world = World::new(&data, 1, Settings::default()).unwrap();
        let draft = omnis_sim::omnis_rules::Draft {
            name: "Durin".to_owned(),
            race: "base:race:dwarf".to_owned(),
            class: "base:class:cleric".to_owned(),
            background: "base:background:acolyte".to_owned(),
            alignment: omnis_sim::omnis_data::Alignment::LawfulGood,
            scores: [10, 8, 14, 10, 15, 8],
            skills: vec![
                omnis_sim::omnis_data::Skill::Medicine,
                omnis_sim::omnis_data::Skill::History,
            ],
        };
        omnis_sim::apply(
            &mut world,
            &data,
            Command::Party(PartyCommand::Create(draft)),
        )
        .unwrap();
        let bless = data.registry.spells.get("base:spell:bless").unwrap();
        (Names::new(&world, &data), bless, world.party.members[0].id)
    }

    #[test]
    fn spell_events_read_as_english() {
        let (names, bless, durin) = names();
        let cast = Event::SpellCast {
            caster: durin,
            spell: bless,
            points: 1,
            components_consumed: vec![],
        };
        assert_eq!(
            spell_line(&cast, &names).unwrap().long,
            "Durin casts Bless (1 pt)"
        );
        let settled = Event::EffectApplied {
            target: EffectTarget::Member(durin),
            spell: bless,
            caster: durin,
        };
        assert_eq!(
            spell_line(&settled, &names).unwrap().short,
            "Bless settles on Durin"
        );
        let faded = Event::EffectEnded {
            target: EffectTarget::Party,
            spell: bless,
            why: EffectEnd::Expired,
        };
        assert_eq!(
            spell_line(&faded, &names).unwrap().long,
            "Bless fades from the party"
        );
        let let_go = Event::Concentration {
            caster: durin,
            spell: bless,
            ended: true,
        };
        assert_eq!(
            spell_line(&let_go, &names).unwrap().long,
            "Durin lets Bless go"
        );
        let auto = Event::AutoCast {
            member: durin,
            spell: bless,
            on: false,
        };
        assert_eq!(
            spell_line(&auto, &names).unwrap().long,
            "Durin will no longer cast Bless on their own"
        );
        let healed = Event::Healed {
            target: durin,
            rolls: vec![],
            amount: 7,
            hp: 12,
        };
        assert_eq!(
            spell_line(&healed, &names).unwrap().long,
            "Durin regains 7 hp, now 12"
        );
        assert!(spell_line(&Event::PartyChanged, &names).is_none());
        let lines = batch_lines(&[cast, settled], &names);
        assert_eq!(lines.len(), 2, "the batch renders spell events too");
    }
}
