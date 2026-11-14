//! Casting outside a fight: healing, buffs, light, and mage hand, on the `cast` stream, for
//! `cast_minutes` of the party's clock.

use crate::apply::advance;
use crate::combat::cast::{self, Aim, CastPlan, Target};
use crate::combat::{Roller, state};
use crate::command::Rejection;
use crate::event::Event;
use crate::party;
use crate::utility::{self, Hand};
use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, Pcg32};
use omnis_data::{Data, Spell, SpellEffect, Utility};
use omnis_rules::{EffectKind, heal_roll};

/// Minutes a cast costs when the rules do not say.
const DEFAULT_CAST_MINUTES: u32 = 1;

/// A cast outside a fight that passed every check: who, what, and on whom.
pub(crate) struct Checked<'a> {
    own: usize,
    def: &'a Spell,
    effect: SpellEffect,
    plan: CastPlan,
    /// What a hand will do, when the spell is one.
    hand: Option<Hand<'a>>,
    /// The standing group an opening strike is sent at, when the cast is one.
    strike: Option<String>,
}

/// Every check of an explore-mode cast, before anything is paid: a caster who can act, a
/// spell they know that is castable here and affordable, and a target its effect and its
/// `targeting` reach. `rng` is the `cast` stream or a copy of it (a pack's cost may roll).
pub(crate) fn validate<'a>(
    world: &World,
    data: &'a Data,
    caster: CharacterId,
    spell: &str,
    target: Target,
    rng: &mut Pcg32,
) -> Result<Checked<'a>, Rejection> {
    let own = usize::from(world.party.slot_of(caster)?);
    let member = &world.party.members[own];
    if state::is_dead(member, data) {
        return Err(Rejection::MemberDead { member: caster });
    }
    if member.is_down() {
        return Err(Rejection::MemberDown { member: caster });
    }
    let spell = cast::spell_id(data, spell)?;
    let (id, def, cost) = cast::check(world, data, own, spell, false, rng)?;
    let effect = def.effect.clone().ok_or_else(|| Rejection::NotCastable {
        spell: cast::spell_name(data, id),
    })?;
    // Mending is data only until repair is built (step 10 of `tasks/plans/combat-in-feet.md`).
    if let SpellEffect::Utility(Utility::Mend { .. }) = effect {
        return Err(Rejection::NotCastable {
            spell: cast::spell_name(data, id),
        });
    }
    if let SpellEffect::Heal { .. } | SpellEffect::Buff { .. } = effect {
        let Target::Member(id) = target else {
            return Err(Rejection::WrongTarget);
        };
        let member = &world.party.members[usize::from(world.party.slot_of(id)?)];
        if state::is_dead(member, data) {
            return Err(Rejection::MemberDead { member: id });
        }
        crate::targets::in_range(world, data, None, own, def.targeting(), &target)?;
    }
    let hand = match effect {
        SpellEffect::Utility(Utility::OpenDoor { .. }) => {
            Some(utility::check(world, data, own, def.targeting(), &target)?)
        }
        _ => None,
    };
    // A harmful spell on the road is an opening strike, at a group the party sees.
    let strike = match &target {
        Target::Being { being } if effect.strikes() => {
            crate::targets::in_range(world, data, None, own, def.targeting(), &target)?;
            Some(being.clone())
        }
        _ if effect.strikes() => return Err(Rejection::WrongTarget),
        _ => None,
    };
    let aim = match target {
        Target::Member(id) => Aim::Member(usize::from(world.party.slot_of(id)?)),
        Target::Stack(stack) => Aim::Stack(stack),
        _ if hand.is_some() || strike.is_some() => Aim::Party,
        _ => return Err(Rejection::WrongTarget),
    };
    let plan = CastPlan {
        own,
        spell: id,
        cost,
        target: aim,
    };
    Ok(Checked {
        own,
        def,
        effect,
        plan,
        hand,
        strike,
    })
}

/// Apply an explore-mode cast: validation on a copy of the `cast` stream, then payment,
/// the effect, and the minutes.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    caster: CharacterId,
    spell: &str,
    target: Target,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let mut roller = Roller::take_stream(world, "cast");
    let Checked {
        own,
        def,
        effect,
        plan,
        hand,
        strike,
    } = validate(world, data, caster, spell, target, &mut roller.rng)?;
    cast::pay(world, data, &plan, events).map_err(Rejection::Rule)?;
    if let Some(being) = strike {
        // The opening strike: the group is met where the party stands and the spell lands in
        // the fight it starts, which keeps the time from here.
        roller.store(world);
        let retreat = omnis_core::Position {
            facing: world.position.facing.opposite(),
            ..world.position
        };
        let met = crate::encounter::struck(world, data, &being, retreat, events);
        if let Some(state) = met.map_err(Rejection::Rule)? {
            crate::combat::start_struck(world, data, state, &plan, events)
                .map_err(Rejection::Rule)?;
        }
        return Ok(());
    }
    match (effect, plan.target) {
        (SpellEffect::Heal { dice, add_mod }, Aim::Member(slot)) => {
            let caster = &world.party.members[own];
            let heal = heal_roll(caster, data, dice, add_mod, &mut roller.rng, &roller.stream)
                .map_err(Rejection::Rule)?;
            party::heal(world, data, slot, heal.rolls, heal.amount, events);
        }
        (SpellEffect::Buff { .. }, Aim::Member(slot)) => {
            cast::cast_buff(world, data, &plan, slot, events);
        }
        (SpellEffect::Light { depth, minutes }, _) => {
            let light = EffectKind::Light { depth };
            cast::cast_lasting(world, data, &plan, def, light, minutes, events);
        }
        (SpellEffect::Utility(Utility::OpenDoor { .. }), _) => {
            if let Some(hand) = &hand {
                utility::carry_out(world, data, hand, events)?;
            }
        }
        (
            SpellEffect::Utility(Utility::Reveal {
                range_tiles,
                minutes,
            }),
            _,
        ) => {
            let reveal = EffectKind::Reveal { range: range_tiles };
            cast::cast_lasting(world, data, &plan, def, reveal, minutes, events);
        }
        _ => {}
    }
    roller.store(world);
    let minutes = data
        .rules
        .value("cast_minutes")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(DEFAULT_CAST_MINUTES);
    advance(world, data, minutes, events);
    Ok(())
}
