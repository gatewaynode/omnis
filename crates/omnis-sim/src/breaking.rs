//! Things that break (PRD §8.3; `tasks/plans/combat-in-feet.md` step 8): an object whose kind
//! has a durability is struck from the road by a spell that may strike objects or by a weapon,
//! loses hit points, and at none is broken. A broken container spills what it held, less the
//! fragile items a destructive kind of damage destroys.

use crate::combat::Roller;
use crate::command::Rejection;
use crate::event::{Event, ItemPlace};
use crate::names::id_of;
use crate::objects;
use crate::targets::map::{self, Spot};
use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, Dice};
use omnis_data::placement::ResolvedPlacement;
use omnis_data::{DamageType, Data, ObjectKind, SpellEffect};
use omnis_rules::{
    AttackBonus, AttackRoll, Defenses, RollMode, RuleError, Weapon, attack_bonus, attack_roll_with,
    damage_roll, flags, spell_attack,
};

/// How well a thing stands damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Make {
    /// Armor class.
    pub armor_class: u8,
    /// Hit points whole.
    pub hit_points: u16,
    /// Damage from one blow under this does nothing.
    pub threshold: u8,
}

/// A kind's make, when it can be damaged at all.
pub(crate) fn make(kind: &ObjectKind) -> Option<Make> {
    let a = &kind.attributes;
    let d = a.durability.as_ref()?;
    Some(Make {
        armor_class: d.armor_class(a.material)?,
        hit_points: d.hit_points(a.size)?,
        threshold: d.threshold,
    })
}

/// An object a blow may be aimed at: it stands where the party knows of it, its kind can be
/// damaged, and it is not broken already.
pub(crate) struct Thing<'d> {
    pub placement: &'d ResolvedPlacement,
    pub kind: &'d ObjectKind,
    pub make: Make,
    pub spot: Spot,
}

/// The object `id` as a blow's target, or why it is none.
pub(crate) fn thing<'d>(world: &World, data: &'d Data, id: &str) -> Result<Thing<'d>, Rejection> {
    let none = || Rejection::NoSuchObject {
        object: String::from(id),
    };
    let (placement, spot) = map::object(world, data, id).ok_or_else(none)?;
    let kind = data.objects.get(&placement.kind).ok_or_else(none)?;
    let make = make(kind).ok_or(Rejection::CannotBeDamaged)?;
    if objects::broken(world, world.position.map, placement) {
        return Err(Rejection::Broken);
    }
    Ok(Thing {
        placement,
        kind,
        make,
        spot,
    })
}

/// What strikes a thing.
pub(crate) enum Blow<'a> {
    /// A spell: its effect, with a cantrip's dice already scaled.
    Spell(&'a SpellEffect),
    /// A weapon; `hindered` past its normal range.
    Weapon { weapon: &'a Weapon, hindered: bool },
}

/// Damage that leaves a thing's fragile contents destroyed when it breaks the thing.
const fn shatters(kind: DamageType) -> bool {
    matches!(
        kind,
        DamageType::Acid
            | DamageType::Bludgeoning
            | DamageType::Cold
            | DamageType::Fire
            | DamageType::Force
            | DamageType::Lightning
            | DamageType::Thunder
    )
}

/// A blow as rolled: the roll to hit when one was made, and the damage dice.
struct Landed {
    attack: Option<AttackRoll>,
    kind: DamageType,
    rolls: Vec<omnis_core::RollTrace>,
    raw: i64,
}

fn roll(
    world: &World,
    data: &Data,
    own: usize,
    make: Make,
    blow: &Blow,
    roller: &mut Roller,
) -> Result<Landed, RuleError> {
    let member = &world.party.members[own];
    let ac = i64::from(make.armor_class);
    let hampered = flags(&member.conditions, data).own_attacks_disadvantage;
    let none = Defenses::default();
    let rng = &mut roller.rng;
    let stream = &roller.stream;
    let dealt = |dice: Option<Dice>, bonus: i64, crit: bool, rng: &mut _| {
        damage_roll(data, dice, bonus, crit, none, rng, stream)
    };
    let (attack, kind, damage) = match blow {
        Blow::Weapon { weapon, hindered } => {
            let (modifier, proficiency) = attack_bonus(member, data, weapon)?;
            let with = AttackBonus {
                modifier,
                proficiency,
                extra: None,
            };
            let mode = RollMode::combine(false, *hindered || hampered);
            let attack = attack_roll_with(data, with, ac, mode, rng, stream)?;
            let damage = attack
                .hit
                .then(|| dealt(weapon.damage, modifier, attack.crit, rng))
                .transpose()?;
            (Some(attack), weapon.damage_type, damage)
        }
        Blow::Spell(SpellEffect::Attack { dice, damage_type }) => {
            let mode = RollMode::combine(false, hampered);
            let attack = spell_attack(member, data, ac, mode, None, rng, stream)?;
            let damage = attack
                .hit
                .then(|| dealt(Some(*dice), 0, attack.crit, rng))
                .transpose()?;
            (Some(attack), *damage_type, damage)
        }
        Blow::Spell(SpellEffect::AutoHit { dice, damage_type }) => {
            (None, *damage_type, Some(dealt(Some(*dice), 0, false, rng)?))
        }
        // A thing makes no save: a spell that says it harms objects harms it in full.
        Blow::Spell(SpellEffect::Save {
            dice, damage_type, ..
        }) => (None, *damage_type, Some(dealt(Some(*dice), 0, false, rng)?)),
        Blow::Spell(_) => return Err(RuleError::new("strike", "the spell harms nothing")),
    };
    let (rolls, raw) = damage.map_or((Vec::new(), 0), |d| (d.rolls, d.raw));
    Ok(Landed {
        attack,
        kind,
        rolls,
        raw,
    })
}

/// Land a blow of the member in slot `own` on a thing: the roll, the damage after what a thing
/// shrugs off (poison and psychic, and anything under its threshold), and its breaking when no
/// hit points are left.
pub(crate) fn strike(
    world: &mut World,
    data: &Data,
    own: usize,
    thing: &Thing,
    blow: &Blow,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let by: CharacterId = world.party.members[own].id;
    let map = world.position.map;
    let p = thing.placement;
    let landed = roll(world, data, own, thing.make, blow, roller)?;
    let immune = matches!(landed.kind, DamageType::Poison | DamageType::Psychic);
    let amount = u16::try_from(landed.raw).unwrap_or(u16::MAX);
    let amount = if immune || amount < u16::from(thing.make.threshold) {
        0
    } else {
        amount
    };
    let before = objects::saved(world, map, &p.id).map_or(0, |s| s.damage);
    let damage = before.saturating_add(amount).min(thing.make.hit_points);
    objects::edit(world, map, &p.id, |s| s.damage = damage);
    events.push(Event::ObjectDamaged {
        object: p.id.clone(),
        by,
        attack: landed.attack,
        kind: landed.kind,
        rolls: landed.rolls,
        raw: landed.raw,
        amount,
        left: thing.make.hit_points - damage,
    });
    if damage >= thing.make.hit_points {
        shatter(world, data, thing, landed.kind, events);
    }
    Ok(())
}

/// The thing breaks: it shows its kind's broken state when it names one, and what it held is
/// laid on the ground, less the fragile items the damage destroyed.
fn shatter(
    world: &mut World,
    data: &Data,
    thing: &Thing,
    kind: DamageType,
    events: &mut Vec<Event>,
) {
    let map = world.position.map;
    let p = thing.placement;
    let held = Vec::from(objects::contents(world, p));
    let state = thing.kind.attributes.durability.as_ref();
    let state = state.and_then(|d| d.broken_state.clone());
    let emptied = !p.contents.is_empty();
    objects::edit(world, map, &p.id, |s| {
        s.broken = true;
        if state.is_some() {
            s.state = state;
        }
        s.contents = emptied.then(Vec::new);
    });
    events.push(Event::ObjectBroken {
        object: p.id.clone(),
    });
    for (item, count) in held {
        let fragile = data.items.get(&item).is_some_and(|def| def.fragile);
        if fragile && shatters(kind) {
            events.push(Event::ItemDestroyed {
                item: id_of(&data.registry.items, item),
                count,
                from: ItemPlace::Object(p.id.clone()),
            });
        } else {
            crate::ground::found(world, data, p, item, count, events);
        }
    }
}
