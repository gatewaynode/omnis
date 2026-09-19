//! Casting arithmetic against the base pack: the casting modifier and save DC, spell attacks,
//! monster saves, damage after a save, healing, cantrip scaling, the component threshold,
//! concentration, and an attack roll that carries a buff die.

use omnis_core::{CharacterId, Dice, Pcg32, StreamName};
use omnis_data::{Ability, Alignment, Data, Skill, load_packs};
use omnis_rules::{
    AttackBonus, Character, Draft, RollMode, attack_roll_with, cantrip_dice, cast_modifier,
    casting_ability, concentration_dc, create, heal_roll, kept_d20, monster_save, needs_components,
    rejudge, save_dc, saved_damage, spell_attack,
};
use std::path::PathBuf;

fn data() -> Data {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/base");
    load_packs(&[&base]).unwrap_or_else(|r| panic!("{r}"))
}

fn stream() -> StreamName {
    StreamName::new("combat")
}

fn member(data: &Data, race: &str, class: &str, scores: [u8; 6], skills: &[Skill]) -> Character {
    let draft = Draft {
        name: "Test".to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::NeutralGood,
        scores,
        skills: skills.to_vec(),
    };
    let mut rng = Pcg32::for_stream(1, &StreamName::new("party"));
    create(&draft, data, CharacterId(0), 0, &mut rng).unwrap_or_else(|e| panic!("{e}"))
}

fn wizard(data: &Data) -> Character {
    member(
        data,
        "elf",
        "wizard",
        [8, 14, 13, 15, 12, 10],
        &[Skill::Arcana, Skill::History],
    )
}

/// A generator whose next kept d20 shows `face`, found by scanning seeds.
fn rng_showing(face: u32) -> Pcg32 {
    (0..10_000u64)
        .map(|seed| Pcg32::for_stream(seed, &stream()))
        .find(|rng| {
            let mut probe = *rng;
            kept_d20(RollMode::Normal, &mut probe, &stream()).unwrap().1 == face
        })
        .expect("some seed shows the face")
}

#[test]
fn casters_have_a_modifier_and_a_save_dc() {
    let data = data();
    let wizard = wizard(&data);
    assert_eq!(casting_ability(&wizard, &data), Some(Ability::Intelligence));
    assert_eq!(cast_modifier(&wizard, &data), 3, "Int 16");
    let mut rng = Pcg32::for_stream(1, &stream());
    assert_eq!(save_dc(&wizard, &data, &mut rng, &stream()).unwrap(), 13);
    let fighter = member(
        &data,
        "human",
        "fighter",
        [15, 14, 13, 12, 10, 8],
        &[Skill::Athletics, Skill::Perception],
    );
    assert_eq!(casting_ability(&fighter, &data), None);
    assert_eq!(cast_modifier(&fighter, &data), 0);
    assert_eq!(rng.draws(), 0);
}

#[test]
fn a_spell_attack_adds_the_casting_modifier_and_proficiency() {
    let data = data();
    let wizard = wizard(&data);
    let mut rng = rng_showing(10);
    let hit = spell_attack(
        &wizard,
        &data,
        15,
        RollMode::Normal,
        None,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(
        (hit.roll.modifier, hit.roll.proficiency, hit.roll.total),
        (3, 2, 15)
    );
    assert!(hit.hit && !hit.crit, "10 + 5 reaches AC 15");
    let mut rng = rng_showing(9);
    let miss = spell_attack(
        &wizard,
        &data,
        15,
        RollMode::Normal,
        None,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert!(!miss.hit, "{miss:?}");
    assert_eq!(miss.roll.bonus, None);
}

#[test]
fn monsters_save_on_a_d20_plus_their_modifier() {
    let data = data();
    let goblin = &data.monsters[&data.registry.monsters.get("base:monster:goblin").unwrap()];
    let mut rng = rng_showing(11);
    let (roll, saved) = monster_save(goblin, Ability::Dexterity, 13, &mut rng, &stream()).unwrap();
    assert_eq!((roll.face, roll.modifier, roll.total), (11, 2, 13));
    assert!(saved, "13 meets DC 13");
    let mut rng = rng_showing(10);
    let (roll, saved) = monster_save(goblin, Ability::Dexterity, 13, &mut rng, &stream()).unwrap();
    assert_eq!(roll.total, 12);
    assert!(!saved);
    let mut rng = Pcg32::for_stream(1, &stream());
    assert_eq!(
        saved_damage(&data, 10, false, true, &mut rng, &stream()).unwrap(),
        10
    );
    assert_eq!(
        saved_damage(&data, 10, true, true, &mut rng, &stream()).unwrap(),
        5
    );
    assert_eq!(
        saved_damage(&data, 10, true, false, &mut rng, &stream()).unwrap(),
        0
    );
    assert_eq!(rng.draws(), 0, "the slots roll nothing");
}

#[test]
fn healing_adds_the_casting_modifier_when_asked() {
    let data = data();
    let cleric = member(
        &data,
        "dwarf",
        "cleric",
        [10, 8, 14, 10, 15, 8],
        &[Skill::Medicine, Skill::History],
    );
    assert_eq!(cast_modifier(&cleric, &data), 3, "Wis 16");
    for seed in 0..20 {
        let mut rng = Pcg32::for_stream(seed, &stream());
        let heal = heal_roll(&cleric, &data, Dice::new(1, 8), true, &mut rng, &stream()).unwrap();
        assert_eq!(heal.rolls.len(), 1);
        assert_eq!(heal.amount, i64::from(heal.rolls[0].total) + 3);
        assert!((4..=11).contains(&heal.amount));
        let plain = heal_roll(
            &cleric,
            &data,
            Dice {
                count: 2,
                sides: 4,
                modifier: 2,
            },
            false,
            &mut rng,
            &stream(),
        )
        .unwrap();
        assert_eq!(
            plain.amount,
            i64::from(plain.rolls[0].total) + 2,
            "the dice's own modifier counts once"
        );
    }
}

#[test]
fn cantrips_scale_by_level_and_components_start_at_the_threshold() {
    let data = data();
    let mut wizard = wizard(&data);
    let mut rng = Pcg32::for_stream(1, &stream());
    let bolt = Dice::new(1, 10);
    for (level, count) in [(1, 1), (4, 1), (5, 2), (11, 3), (17, 4), (20, 4)] {
        wizard.level = level;
        let dice = cantrip_dice(&wizard, &data, bolt, &mut rng, &stream()).unwrap();
        assert_eq!((dice.count, dice.sides), (count, 10), "level {level}");
    }
    let missile = &data.spells[&data
        .registry
        .spells
        .get("base:spell:magic_missile")
        .unwrap()];
    let fire_bolt = &data.spells[&data.registry.spells.get("base:spell:fire_bolt").unwrap()];
    assert!(
        !needs_components(missile, &data),
        "level 1 is under the threshold of 5"
    );
    let mut lowered = data.clone();
    lowered.rules.insert_value("component_threshold", 1);
    assert!(needs_components(missile, &lowered));
    assert!(
        !needs_components(fire_bolt, &lowered),
        "a cantrip stays free of components"
    );
    assert_eq!(concentration_dc(&data, 4, &mut rng, &stream()).unwrap(), 10);
    assert_eq!(
        concentration_dc(&data, 30, &mut rng, &stream()).unwrap(),
        15
    );
    assert_eq!(rng.draws(), 0);
}

#[test]
fn a_buff_die_joins_the_attack_and_a_rejudge_keeps_the_die() {
    let data = data();
    let mut rng = rng_showing(9);
    let extra = Dice::new(1, 4)
        .roll(&mut Pcg32::for_stream(3, &stream()), &stream())
        .unwrap();
    let bonus = AttackBonus {
        modifier: 5,
        proficiency: 0,
        extra: Some(extra.clone()),
    };
    let mut roll =
        attack_roll_with(&data, bonus, 15, RollMode::Normal, &mut rng, &stream()).unwrap();
    assert_eq!(roll.roll.total, 14 + i64::from(extra.total));
    assert_eq!(roll.roll.bonus.as_ref().map(|b| b.total), Some(extra.total));
    assert!(roll.hit, "9 + 5 + the die reaches 15");
    let before = rng.draws();
    rejudge(&data, &mut roll, 25, &mut rng, &stream()).unwrap();
    assert!(
        !roll.hit && roll.ac == 25 && roll.roll.face == 9,
        "{roll:?}"
    );
    rejudge(&data, &mut roll, 10, &mut rng, &stream()).unwrap();
    assert!(roll.hit);
    assert_eq!(rng.draws(), before, "a rejudge rolls nothing");
    let mut rng = rng_showing(20);
    let mut crit = attack_roll_with(
        &data,
        AttackBonus::default(),
        30,
        RollMode::Normal,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert!(crit.hit && crit.crit);
    rejudge(&data, &mut crit, 40, &mut rng, &stream()).unwrap();
    assert!(crit.hit && crit.crit, "a natural 20 stays a hit");
}
