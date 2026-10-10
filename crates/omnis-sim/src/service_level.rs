//! The trainer and the spell sellers (PRD §8.2, M7b): a level for a fee, the spells a level
//! owes chosen one at a time for free, and spells bought at a guild or a temple. The rules are
//! `omnis_rules::level`'s; this file turns them into deals and refusals. Every check runs
//! before anything changes, and a level is worked out on a copy of the member.

use crate::combat::Roller;
use crate::command::Rejection;
use crate::service::{Deal, price};
use crate::world::World;
use alloc::boxed::Box;
use alloc::string::String;
use omnis_core::SpellId;
use omnis_data::{Data, ServiceDef};
use omnis_rules::{SpellRefusal, level_up, may_learn, next_threshold, ready};

/// The member's next level at `trainer.cost` for the new level.
pub(crate) fn train(
    world: &World,
    data: &Data,
    index: usize,
    roller: &mut Roller,
) -> Result<Deal, Rejection> {
    let who = &world.party.members[index];
    let member = who.id;
    let Some(needed) = next_threshold(who, data).map_err(Rejection::Rule)? else {
        return Err(Rejection::MaxLevel { member });
    };
    if !ready(who, data).map_err(Rejection::Rule)? {
        return Err(Rejection::NotReady {
            member,
            xp: who.xp,
            needed,
        });
    }
    let level = [("level", i64::from(who.level) + 1)];
    let cost = price(data, "trainer.cost", &level, roller)?;
    let mut after = who.clone();
    let gains = level_up(&mut after, data, &mut roller.rng).map_err(Rejection::Rule)?;
    Ok(Deal::Train {
        index,
        cost,
        after: Box::new(after),
        gains,
    })
}

/// A pick owed by a level: `spell`, on the member's class list.
pub(crate) fn choose(
    world: &World,
    data: &Data,
    index: usize,
    spell: &str,
) -> Result<Deal, Rejection> {
    let who = &world.party.members[index];
    if who.spell_picks == 0 {
        return Err(Rejection::NoPicks { member: who.id });
    }
    let spell = data
        .classes
        .get(&who.class)
        .and_then(|c| c.casting.as_ref())
        .and_then(|c| on_list(data, &c.list, spell))
        .ok_or_else(|| no_such(spell))?;
    learnable(world, data, index, spell)?;
    Ok(Deal::Spell {
        index,
        spell,
        cost: 0,
        pick: true,
    })
}

/// A spell bought at `spell.learn_cost` for its level: `spell`, among the service's spells.
pub(crate) fn learn(
    world: &World,
    data: &Data,
    def: &ServiceDef,
    index: usize,
    spell: &str,
    roller: &mut Roller,
) -> Result<Deal, Rejection> {
    let spell = on_list(data, &def.spells, spell).ok_or_else(|| no_such(spell))?;
    learnable(world, data, index, spell)?;
    let level = data.spells.get(&spell).map_or(0, |s| s.level);
    let cost = price(
        data,
        "spell.learn_cost",
        &[("spell_level", i64::from(level))],
        roller,
    )?;
    Ok(Deal::Spell {
        index,
        spell,
        cost,
        pick: false,
    })
}

/// The spell `spell` names, when `list` holds it.
fn on_list(data: &Data, list: &[String], spell: &str) -> Option<SpellId> {
    list.iter()
        .find(|id| *id == spell)
        .and_then(|id| data.registry.spells.get(id))
}

/// The refusal for a spell not on the list the command names.
fn no_such(spell: &str) -> Rejection {
    Rejection::NoSuchSpell {
        spell: String::from(spell),
    }
}

/// Refused unless the spell may go onto the member's list.
fn learnable(world: &World, data: &Data, index: usize, spell: SpellId) -> Result<(), Rejection> {
    let who = &world.party.members[index];
    let refusal = may_learn(who, data, spell).map_err(Rejection::Rule)?;
    let member = who.id;
    match refusal {
        None => Ok(()),
        Some(SpellRefusal::NotOnList) => Err(Rejection::NotOnList { member }),
        Some(SpellRefusal::Cantrip) => Err(Rejection::CantripNotLearned),
        Some(SpellRefusal::TooHigh { level, max }) => Err(Rejection::SpellTooHigh { level, max }),
        Some(SpellRefusal::Known) => Err(Rejection::AlreadyKnown { member }),
    }
}
