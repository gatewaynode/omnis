//! The base pack loads, and the SRD figures it was transcribed from come back through the API.

mod common;

use omnis_core::{Dice, Pcg32, StreamName};
use omnis_data::omnis_expr::Value;
use omnis_data::{
    Ability, ArmorKind, BuffOn, DamageType, Effect, Fidelity, Geometry, ItemKind, Reach, Skill,
    SpellEffect, UseEffect, load_packs,
};
use std::path::PathBuf;

fn base_pack() -> PathBuf {
    common::test_pack().parent().unwrap().join("base")
}

#[test]
fn the_base_pack_loads_with_the_srd_subset() {
    let data = load_packs(&[&base_pack()]).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(data.packs[0].id, "base");
    assert_eq!(data.packs[0].license, "CC-BY-4.0");
    assert!(
        data.packs[0].attribution[0]
            .2
            .contains("System Reference Document 5.1")
    );
    assert!(
        data.maps.is_empty() && data.entry.is_none(),
        "content only, no maps"
    );
    assert_eq!(
        (
            data.races.len(),
            data.classes.len(),
            data.backgrounds.len(),
            data.items.len(),
            data.conditions.len(),
            data.spells.len(),
            data.monsters.len(),
            data.rules.slot_names().count(),
        ),
        (4, 4, 3, 24, 16, 11, 3, 19)
    );

    let dwarf = &data.races[&data.registry.races.get("base:race:dwarf").unwrap()];
    assert_eq!(dwarf.ability_bonuses[&Ability::Constitution], 2);
    assert_eq!(dwarf.ability_bonuses[&Ability::Wisdom], 1);
    assert_eq!(dwarf.speed, 25);
    assert!(
        dwarf
            .features
            .iter()
            .any(|f| f.effect == Effect::HitPointsPerLevel(1))
    );
    assert_eq!(data.label("en", &dwarf.name), "Dwarf (hill dwarf)");
    let human = &data.races[&data.registry.races.get("base:race:human").unwrap()];
    assert!(Ability::ALL.iter().all(|a| human.ability_bonuses[a] == 1));

    let wizard = &data.classes[&data.registry.classes.get("base:class:wizard").unwrap()];
    let casting = wizard.casting.as_ref().unwrap();
    assert_eq!(
        (wizard.hit_die, casting.ability, casting.list.len()),
        (6, Ability::Intelligence, 6)
    );
    assert!(
        casting
            .list
            .iter()
            .all(|s| data.registry.spells.get(s).is_some())
    );
    let fighter = &data.classes[&data.registry.classes.get("base:class:fighter").unwrap()];
    assert_eq!(
        fighter.saving_throws,
        [Ability::Strength, Ability::Constitution]
    );
    assert!(fighter.casting.is_none());
    let chain = data.registry.items.get("base:item:chain_mail").unwrap();
    assert!(
        fighter
            .starting_equipment
            .iter()
            .any(|(id, n)| id == "base:item:chain_mail" && *n == 1)
    );
    assert!(matches!(
        data.items[&chain].kind,
        ItemKind::Armor {
            kind: ArmorKind::Heavy,
            base_ac: 16,
            strength: 13,
            ..
        }
    ));
    let rogue = &data.classes[&data.registry.classes.get("base:class:rogue").unwrap()];
    assert_eq!(rogue.skills.choose, 4);
    assert!(rogue.skills.from.contains(&Skill::Stealth));

    let acolyte = &data.backgrounds[&data
        .registry
        .backgrounds
        .get("base:background:acolyte")
        .unwrap()];
    assert_eq!(acolyte.skills, [Skill::Insight, Skill::Religion]);
    assert_eq!(acolyte.gold, 15);
    assert!(
        acolyte
            .equipment
            .iter()
            .any(|(id, n)| id == "base:item:potion_of_healing" && *n == 1),
        "the acolyte carries a potion until shops exist"
    );
    let explorer = &data.backgrounds[&data
        .registry
        .backgrounds
        .get("base:background:explorer")
        .unwrap()];
    assert_eq!(explorer.skills, [Skill::Perception, Skill::Survival]);
    assert_eq!(explorer.equipment[0].0, "base:item:spyglass");
    let warden = &data.backgrounds[&data
        .registry
        .backgrounds
        .get("base:background:warden")
        .unwrap()];
    assert_eq!(warden.skills, [Skill::Arcana, Skill::Investigation]);

    let missile = &data.spells[&data
        .registry
        .spells
        .get("base:spell:magic_missile")
        .unwrap()];
    assert_eq!((missile.level, missile.point_cost()), (1, 1));
    let bolt = &data.spells[&data.registry.spells.get("base:spell:fire_bolt").unwrap()];
    assert_eq!(bolt.point_cost(), 0, "cantrips are free");
    assert_eq!(
        bolt.effect,
        Some(SpellEffect::Attack {
            dice: Dice::new(1, 10),
            damage_type: DamageType::Fire
        })
    );
    assert_eq!(
        missile.effect,
        Some(SpellEffect::AutoHit {
            dice: Dice {
                count: 3,
                sides: 4,
                modifier: 3
            },
            damage_type: DamageType::Force
        })
    );
    let spell = |name: &str| {
        &data.spells[&data
            .registry
            .spells
            .get(&format!("base:spell:{name}"))
            .unwrap()]
    };
    assert_eq!(spell("burning_hands").reach, Reach::Stack);
    assert!(matches!(
        spell("bless").effect,
        Some(SpellEffect::Buff {
            targets: 3,
            consumed: false,
            minutes: 10,
            ..
        })
    ));
    assert!(matches!(
        &spell("guidance").effect,
        Some(SpellEffect::Buff { on, consumed: true, .. }) if on == &[BuffOn::AbilityChecks]
    ));
    assert!(matches!(
        spell("shield").effect,
        Some(SpellEffect::Reaction { armor_bonus: 5 })
    ));
    assert!(matches!(
        spell("light").effect,
        Some(SpellEffect::Light {
            depth: 8,
            minutes: 60
        })
    ));
    assert!(
        data.spells.values().all(|s| s.effect.is_some()),
        "every base spell is castable"
    );
    let item = |name: &str| {
        &data.items[&data
            .registry
            .items
            .get(&format!("base:item:{name}"))
            .unwrap()]
    };
    let glass = item("spyglass").sense().unwrap();
    assert_eq!(glass.geometry, Geometry::Ray { range: 16 });
    assert_eq!(glass.fidelity, Fidelity::Structure);
    assert_eq!(glass.check, Some(Skill::Perception));
    assert!(!item("spyglass").consumable);
    let potion = item("potion_of_healing");
    assert_eq!(
        potion.use_effect,
        Some(UseEffect::Heal {
            dice: Dice {
                count: 2,
                sides: 4,
                modifier: 2
            }
        })
    );
    assert!(potion.consumable);
    assert_eq!(data.label("en", &potion.name), "Potion of healing");
    let goblin = &data.monsters[&data.registry.monsters.get("base:monster:goblin").unwrap()];
    assert_eq!((goblin.ac, goblin.xp, goblin.challenge), (15, 50, (1, 4)));
    assert_eq!(data.label("en", &goblin.attacks[0].name), "Scimitar");
}

#[test]
fn monsters_and_conditions_carry_the_combat_fields() {
    let data = load_packs(&[&base_pack()]).unwrap_or_else(|r| panic!("{r}"));
    let goblin = &data.monsters[&data.registry.monsters.get("base:monster:goblin").unwrap()];
    assert!(!goblin.attacks[0].ranged && goblin.attacks[1].ranged);
    assert_eq!(goblin.gold, Some(Dice::new(2, 4)));
    let skeleton = &data.monsters[&data.registry.monsters.get("base:monster:skeleton").unwrap()];
    assert_eq!(skeleton.immunities, [DamageType::Poison]);
    assert_eq!(skeleton.vulnerabilities, [DamageType::Bludgeoning]);
    assert!(skeleton.resistances.is_empty());
    let condition = |name: &str| {
        &data.conditions[&data
            .registry
            .conditions
            .get(&format!("base:condition:{name}"))
            .unwrap()]
    };
    let unconscious = condition("unconscious");
    assert!(
        unconscious.incapacitated
            && unconscious.auto_fail_str_dex_saves
            && unconscious.attacks_against_advantage
            && unconscious.melee_hits_crit
            && !unconscious.own_attacks_disadvantage
            && !unconscious.resist_all
    );
    assert!(condition("petrified").resist_all);
    assert!(condition("poisoned").own_attacks_disadvantage);
    let dead = condition("dead");
    assert!(dead.incapacitated && !dead.melee_hits_crit);
    assert_eq!(data.label("en", &dead.name), "Dead");
    let blinded = condition("charmed");
    assert!(!blinded.incapacitated && !blinded.attacks_against_advantage);
}

#[test]
fn the_base_rules_evaluate() {
    let data = load_packs(&[&base_pack()]).unwrap_or_else(|r| panic!("{r}"));
    let rules = &data.rules;
    assert_eq!(rules.value("point_budget"), Some(27));
    assert_eq!(rules.value("component_threshold"), Some(5));
    assert_eq!(rules.table("point_cost").unwrap(), [0, 1, 2, 3, 4, 5, 7, 9]);
    assert_eq!(rules.table("xp_thresholds").unwrap()[1], 300);
    assert_eq!(rules.table("xp_thresholds").unwrap()[19], 355_000);
    assert_eq!(rules.table("proficiency_bonus").unwrap()[4], 3);
    let stream = StreamName::new("party");
    let mut rng = Pcg32::for_stream(1, &stream);
    let eval = |slot: &str, inputs: &[(&str, Value)], rng: &mut Pcg32| {
        rules.eval(slot, inputs, rng, &stream).unwrap().value
    };
    // PRD §8.3, focused caster at level 5: 5 × 3 + 1 = 16.
    let pool = |level: i64, cast: i64, other: i64, half: bool, rng: &mut Pcg32| {
        eval(
            "spell_points.pool",
            &[
                ("level", Value::Int(level)),
                ("cast_mod", Value::Int(cast)),
                ("other_mental_mods", Value::Int(other)),
                ("half_caster", Value::Bool(half)),
            ],
            rng,
        )
    };
    assert_eq!(pool(5, 3, 1, false, &mut rng), Value::Int(16));
    assert_eq!(pool(1, 3, -2, false, &mut rng), Value::Int(1));
    assert_eq!(pool(5, 3, 1, true, &mut rng), Value::Int(7));
    assert_eq!(
        eval(
            "hit_points.first_level",
            &[
                ("hit_die", Value::Int(10)),
                ("con_mod", Value::Int(2)),
                ("per_level", Value::Int(1))
            ],
            &mut rng
        ),
        Value::Int(13)
    );
    assert_eq!(
        eval(
            "hit_points.per_level",
            &[
                ("hit_die", Value::Int(6)),
                ("con_mod", Value::Int(-3)),
                ("per_level", Value::Int(0))
            ],
            &mut rng
        ),
        Value::Int(1)
    );
    let int = |slot: &str, inputs: &[(&str, Value)], rng: &mut Pcg32| match eval(slot, inputs, rng)
    {
        Value::Int(n) => n,
        other => panic!("{slot}: {other:?}"),
    };
    let i = Value::Int;
    let b = Value::Bool;
    assert_eq!(
        int(
            "spell.save_dc",
            &[("cast_mod", i(3)), ("proficiency", i(2))],
            &mut rng
        ),
        13
    );
    let saved = |amount: i64, saved: bool, half: bool, rng: &mut Pcg32| {
        int(
            "spell.save_damage",
            &[
                ("amount", i(amount)),
                ("saved", b(saved)),
                ("half_on_save", b(half)),
            ],
            rng,
        )
    };
    assert_eq!(saved(10, false, true, &mut rng), 10);
    assert_eq!(saved(10, true, true, &mut rng), 5);
    assert_eq!(saved(10, true, false, &mut rng), 0);
    assert_eq!(
        int(
            "heal.total",
            &[("dice", i(4)), ("cast_mod", i(3)), ("add_mod", b(true))],
            &mut rng
        ),
        7
    );
    assert_eq!(
        int(
            "heal.total",
            &[("dice", i(4)), ("cast_mod", i(3)), ("add_mod", b(false))],
            &mut rng
        ),
        4
    );
    for (level, dice) in [(1, 1), (4, 1), (5, 2), (11, 3), (17, 4)] {
        assert_eq!(int("cantrip.dice", &[("level", i(level))], &mut rng), dice);
    }
    assert_eq!(int("concentration.dc", &[("damage", i(4))], &mut rng), 10);
    assert_eq!(int("concentration.dc", &[("damage", i(30))], &mut rng), 15);
    let dc = |distance: i64, visibility: i64, layer: i64, rng: &mut Pcg32| {
        int(
            "sense.dc",
            &[
                ("distance", i(distance)),
                ("visibility", i(visibility)),
                ("layer", i(layer)),
            ],
            rng,
        )
    };
    assert_eq!(dc(1, 12, 1, &mut rng), 10);
    assert_eq!(dc(16, 12, 1, &mut rng), 14);
    assert_eq!(dc(16, 6, 2, &mut rng), 22);
    assert_eq!(rules.value("cast_minutes"), Some(1));
    assert_eq!(rules.value("don_armor_minutes"), Some(5));
    assert_eq!(rules.value("use_item_minutes"), Some(1));
    assert_eq!(rng.draws(), 0, "no base formula rolls dice");
}

#[test]
fn the_combat_rules_evaluate() {
    let data = load_packs(&[&base_pack()]).unwrap_or_else(|r| panic!("{r}"));
    let rules = &data.rules;
    let stream = StreamName::new("combat");
    let mut rng = Pcg32::for_stream(1, &stream);
    let eval = |slot: &str, inputs: &[(&str, Value)], rng: &mut Pcg32| {
        rules.eval(slot, inputs, rng, &stream).unwrap().value
    };
    let attack = |die: i64, total: i64, ac: i64, rng: &mut Pcg32| {
        eval(
            "attack.hit",
            &[
                ("die", Value::Int(die)),
                ("total", Value::Int(total)),
                ("ac", Value::Int(ac)),
            ],
            rng,
        )
    };
    assert_eq!(attack(20, 21, 99, &mut rng), Value::Bool(true));
    assert_eq!(attack(1, 30, 1, &mut rng), Value::Bool(false));
    assert_eq!(attack(10, 16, 16, &mut rng), Value::Bool(true));
    assert_eq!(attack(10, 15, 16, &mut rng), Value::Bool(false));
    assert_eq!(
        eval("attack.crit", &[("die", Value::Int(20))], &mut rng),
        Value::Bool(true)
    );
    let adjusted = |amount: i64, resist: bool, vulnerable: bool, immune: bool, rng: &mut Pcg32| {
        eval(
            "damage.adjusted",
            &[
                ("amount", Value::Int(amount)),
                ("resist", Value::Bool(resist)),
                ("vulnerable", Value::Bool(vulnerable)),
                ("immune", Value::Bool(immune)),
            ],
            rng,
        )
    };
    assert_eq!(adjusted(7, true, false, false, &mut rng), Value::Int(3));
    assert_eq!(adjusted(7, false, true, false, &mut rng), Value::Int(14));
    assert_eq!(adjusted(7, true, true, true, &mut rng), Value::Int(0));
    assert_eq!(
        eval(
            "damage.total",
            &[("dice", Value::Int(1)), ("bonus", Value::Int(-3))],
            &mut rng
        ),
        Value::Int(0)
    );
    assert_eq!(
        eval(
            "monster.hit_points",
            &[
                ("count", Value::Int(2)),
                ("sides", Value::Int(8)),
                ("modifier", Value::Int(4))
            ],
            &mut rng
        ),
        Value::Int(13),
        "the SRD average of 2d8+4"
    );
    let bribe = |xp: i64, disposition: i64, rng: &mut Pcg32| {
        eval(
            "bribe.cost",
            &[
                ("xp", Value::Int(xp)),
                ("disposition", Value::Int(disposition)),
            ],
            rng,
        )
    };
    assert_eq!(bribe(200, 0, &mut rng), Value::Int(200));
    assert_eq!(bribe(200, 2, &mut rng), Value::Int(66));
    assert_eq!(bribe(200, 3, &mut rng), Value::Int(0));
    assert_eq!(
        bribe(1, 2, &mut rng),
        Value::Int(1),
        "never free unless friendly"
    );
    assert_eq!(
        eval(
            "encounter.random",
            &[("roll", Value::Int(4)), ("chance_percent", Value::Int(4))],
            &mut rng
        ),
        Value::Bool(true)
    );
    assert_eq!(rules.value("monster_front_stacks"), Some(2));
    assert_eq!(rules.value("death_save_dc"), Some(10));
    assert_eq!(rules.value("combat_round_minutes"), Some(1));
    assert_eq!(rules.table("run_dc").unwrap(), [15, 12, 10, 0]);
    assert_eq!(rng.draws(), 0, "no combat formula rolls dice");
}

#[test]
fn base_and_test_packs_load_together() {
    let data = load_packs(&[&base_pack(), &common::test_pack()]).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(
        data.packs.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
        ["base", "test"]
    );
    assert_eq!(data.maps.len(), 2);
    assert_eq!(data.races.len(), 4);
    assert_eq!(data.entry, data.registry.maps.get("test:map:meadow"));
}
