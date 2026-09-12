//! The base pack loads, and the SRD figures it was transcribed from come back through the API.

mod common;

use omnis_core::{Pcg32, StreamName};
use omnis_data::omnis_expr::Value;
use omnis_data::{Ability, ArmorKind, Effect, ItemKind, Skill, load_packs};
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
        (4, 4, 1, 21, 15, 11, 3, 4)
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

    let missile = &data.spells[&data
        .registry
        .spells
        .get("base:spell:magic_missile")
        .unwrap()];
    assert_eq!((missile.level, missile.point_cost()), (1, 1));
    let bolt = &data.spells[&data.registry.spells.get("base:spell:fire_bolt").unwrap()];
    assert_eq!(bolt.point_cost(), 0, "cantrips are free");
    let goblin = &data.monsters[&data.registry.monsters.get("base:monster:goblin").unwrap()];
    assert_eq!((goblin.ac, goblin.xp, goblin.challenge), (15, 50, (1, 4)));
    assert_eq!(data.label("en", &goblin.attacks[0].name), "Scimitar");
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
    assert_eq!(rng.draws(), 0, "no base formula rolls dice");
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
