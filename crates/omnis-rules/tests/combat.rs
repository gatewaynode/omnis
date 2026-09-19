//! Combat arithmetic against the base pack: roll modes, attack rolls at the natural 1 and 20,
//! damage with critical hits and defenses, weapons and proficiency, monster reads, death saves.

use omnis_core::{CharacterId, Dice, Pcg32, StreamName};
use omnis_data::{Ability, Alignment, DamageType, Data, Skill, load_packs};
use omnis_rules::{
    Character, DamageAdjust, DeathSaveResult, DeathSaves, Defenses, Draft, RollMode, attack_bonus,
    attack_roll, best_weapon, check, choose_target, create, damage_roll, death_save, flags,
    initiative, kept_d20, member_defenses, modifier_of, monster_defenses, monster_hit_points,
    passive, passive_perception, pick_attack, skill_bonus, weapons, wound_at_zero,
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

fn fighter(data: &Data) -> Character {
    member(
        data,
        "human",
        "fighter",
        [15, 14, 13, 12, 10, 8],
        &[Skill::Athletics, Skill::Perception],
    )
}

/// A generator whose next kept d20 under `mode` shows `face`, found by scanning seeds.
fn rng_showing(face: u32, mode: RollMode) -> Pcg32 {
    (0..10_000u64)
        .map(|seed| Pcg32::for_stream(seed, &stream()))
        .find(|rng| {
            let mut probe = *rng;
            kept_d20(mode, &mut probe, &stream()).unwrap().1 == face
        })
        .expect("some seed shows the face")
}

fn item(data: &Data, name: &str) -> omnis_core::ItemId {
    data.registry
        .items
        .get(&format!("base:item:{name}"))
        .unwrap_or_else(|| panic!("{name}"))
}

#[test]
fn advantage_and_disadvantage_roll_two_dice_and_keep_one() {
    let mut rng = Pcg32::for_stream(5, &stream());
    let (trace, face) = kept_d20(RollMode::Normal, &mut rng, &stream()).unwrap();
    assert_eq!(trace.rolls.len(), 1);
    assert_eq!(face, trace.rolls[0].value);
    let (trace, face) = kept_d20(RollMode::Advantage, &mut rng, &stream()).unwrap();
    assert_eq!(trace.rolls.len(), 2);
    assert_eq!(face, trace.rolls.iter().map(|r| r.value).max().unwrap());
    let (trace, face) = kept_d20(RollMode::Disadvantage, &mut rng, &stream()).unwrap();
    assert_eq!(face, trace.rolls.iter().map(|r| r.value).min().unwrap());
    assert_eq!(rng.draws(), 5, "one, two, two");
    assert_eq!(RollMode::combine(true, true), RollMode::Normal);
    assert_eq!(RollMode::combine(true, false), RollMode::Advantage);
    assert_eq!(RollMode::combine(false, true), RollMode::Disadvantage);
    let data = data();
    let fighter = fighter(&data);
    let mut rng = Pcg32::for_stream(5, &stream());
    let roll = check(
        &fighter,
        &data,
        Some(Skill::Athletics),
        Ability::Strength,
        RollMode::Advantage,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(roll.mode, RollMode::Advantage);
    assert_eq!(roll.total, i64::from(roll.face) + 3 + 2);
    assert_eq!(roll.trace.stream, stream());
}

#[test]
fn attack_rolls_follow_the_natural_one_and_twenty() {
    let data = data();
    let mut rng = rng_showing(20, RollMode::Normal);
    let hit = attack_roll(&data, -5, 0, 99, RollMode::Normal, &mut rng, &stream()).unwrap();
    assert!(hit.hit && hit.crit, "{hit:?}");
    assert_eq!((hit.roll.face, hit.roll.total, hit.ac), (20, 15, 99));
    let mut rng = rng_showing(1, RollMode::Normal);
    let miss = attack_roll(&data, 30, 6, 1, RollMode::Normal, &mut rng, &stream()).unwrap();
    assert!(!miss.hit && !miss.crit, "{miss:?}");
    for seed in 0..40 {
        let mut rng = Pcg32::for_stream(seed, &stream());
        let roll =
            attack_roll(&data, 4, 2, 16, RollMode::Disadvantage, &mut rng, &stream()).unwrap();
        assert_eq!(roll.roll.total, i64::from(roll.roll.face) + 6);
        let expected = roll.roll.face == 20 || (roll.roll.face != 1 && roll.roll.total >= 16);
        assert_eq!(roll.hit, expected, "{roll:?}");
        assert_eq!(roll.crit, roll.roll.face == 20);
        assert_eq!(roll.roll.trace.rolls.len(), 2);
    }
}

#[test]
fn damage_doubles_dice_on_a_critical() {
    let data = data();
    let mut rng = Pcg32::for_stream(9, &stream());
    let plain = damage_roll(
        &data,
        Some(Dice::new(1, 6).plus(2)),
        3,
        false,
        Defenses::default(),
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(plain.rolls.len(), 1);
    assert_eq!(plain.raw, i64::from(plain.rolls[0].total) + 5);
    assert_eq!(
        (plain.amount, plain.adjust),
        (plain.raw, DamageAdjust::None)
    );
    let crit = damage_roll(
        &data,
        Some(Dice::new(1, 6).plus(2)),
        3,
        true,
        Defenses::default(),
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(crit.rolls.len(), 2);
    assert_eq!(
        crit.raw,
        i64::from(crit.rolls[0].total) + i64::from(crit.rolls[1].total) + 5,
        "the modifiers are added once"
    );
    assert!(crit.rolls.iter().all(|r| r.dice.modifier == 0));
}

#[test]
fn defenses_adjust_damage_and_unarmed_hits_land() {
    let data = data();
    let mut rng = Pcg32::for_stream(9, &stream());
    let resist = Defenses {
        resist: true,
        ..Defenses::default()
    };
    let halved = damage_roll(
        &data,
        Some(Dice::new(1, 1)),
        6,
        false,
        resist,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(
        (halved.raw, halved.amount, halved.adjust),
        (7, 3, DamageAdjust::Resisted)
    );
    let vulnerable = Defenses {
        vulnerable: true,
        ..Defenses::default()
    };
    let doubled = damage_roll(
        &data,
        Some(Dice::new(1, 1)),
        6,
        false,
        vulnerable,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(
        (doubled.amount, doubled.adjust),
        (14, DamageAdjust::Vulnerable)
    );
    let immune = Defenses {
        immune: true,
        resist: true,
        vulnerable: true,
    };
    let nothing = damage_roll(
        &data,
        Some(Dice::new(1, 1)),
        6,
        false,
        immune,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!((nothing.amount, nothing.adjust), (0, DamageAdjust::Immune));
    let floor = damage_roll(
        &data,
        Some(Dice::new(1, 1)),
        -9,
        false,
        Defenses::default(),
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(floor.raw, 0, "never negative");
    let unarmed = damage_roll(
        &data,
        None,
        2,
        true,
        Defenses::default(),
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert!(unarmed.rolls.is_empty());
    assert_eq!(
        unarmed.raw, 3,
        "one point plus the bonus, no dice to double"
    );
}

#[test]
fn weapons_come_from_the_kit_with_class_proficiency() {
    let data = data();
    let fighter = fighter(&data);
    let carried = weapons(&fighter, &data);
    assert_eq!(carried.len(), 3, "longsword, light crossbow, unarmed");
    let mut sheathed = fighter.clone();
    sheathed.equipped.remove(&omnis_data::EquipSlot::MainHand);
    assert_eq!(
        weapons(&sheathed, &data).len(),
        2,
        "a sword in the pack is not swung"
    );
    let sword = best_weapon(&fighter, &data, false).unwrap();
    assert_eq!(sword.item, Some(item(&data, "longsword")));
    assert!(sword.proficient && !sword.ranged && sword.ability == Ability::Strength);
    assert_eq!(attack_bonus(&fighter, &data, &sword).unwrap(), (3, 2));
    let bow = best_weapon(&fighter, &data, true).unwrap();
    assert_eq!(bow.item, Some(item(&data, "light_crossbow")));
    assert!(bow.ranged && bow.ability == Ability::Dexterity);
    assert_eq!(attack_bonus(&fighter, &data, &bow).unwrap(), (2, 2));

    let wizard = member(
        &data,
        "elf",
        "wizard",
        [8, 14, 13, 15, 12, 10],
        &[Skill::Arcana, Skill::History],
    );
    let staff = best_weapon(&wizard, &data, false).unwrap();
    assert_eq!(staff.item, Some(item(&data, "quarterstaff")));
    assert!(staff.proficient, "listed by id on the class");
    assert_eq!(best_weapon(&wizard, &data, true), None);

    let rogue = member(
        &data,
        "halfling",
        "rogue",
        [8, 15, 12, 10, 13, 14],
        &[
            Skill::Stealth,
            Skill::Acrobatics,
            Skill::Deception,
            Skill::Perception,
        ],
    );
    let rapier = best_weapon(&rogue, &data, false).unwrap();
    assert_eq!(rapier.item, Some(item(&data, "rapier")));
    assert!(rapier.proficient, "martial, but on the rogue's list");
    assert_eq!(
        best_weapon(&rogue, &data, true).unwrap().item,
        Some(item(&data, "shortbow"))
    );

    let mut bare = fighter.clone();
    bare.equipped.clear();
    let fists = best_weapon(&bare, &data, false).unwrap();
    assert_eq!(fists.item, None);
    assert_eq!(fists.average_twice(), 2);
    assert_eq!(best_weapon(&bare, &data, true), None);
    let mut unskilled = fighter.clone();
    unskilled.class = wizard.class;
    let sword = best_weapon(&unskilled, &data, false).unwrap();
    assert!(!sword.proficient, "a wizard with a longsword");
    assert_eq!(attack_bonus(&unskilled, &data, &sword).unwrap(), (3, 0));
}

#[test]
fn conditions_and_races_shape_defenses_and_flags() {
    let data = data();
    let condition = |name: &str| {
        data.registry
            .conditions
            .get(&format!("base:condition:{name}"))
            .unwrap()
    };
    let dwarf = member(
        &data,
        "dwarf",
        "cleric",
        [10, 8, 14, 10, 15, 8],
        &[Skill::Medicine, Skill::History],
    );
    assert!(member_defenses(&dwarf, &data, DamageType::Poison).resist);
    assert!(!member_defenses(&dwarf, &data, DamageType::Fire).resist);
    let mut stone = dwarf.clone();
    stone.conditions.push(condition("petrified"));
    assert!(member_defenses(&stone, &data, DamageType::Fire).resist);
    let mut down = dwarf.clone();
    down.conditions.push(condition("unconscious"));
    let f = flags(&down.conditions, &data);
    assert!(f.incapacitated && f.attacks_against_advantage && f.melee_hits_crit && !f.resist_all);
    assert_eq!(flags(&[], &data), Default::default());
    assert_eq!(
        skill_bonus(&dwarf, &data, Skill::Medicine).unwrap(),
        5,
        "Wis 16 (+3) and proficient (+2)"
    );
    assert_eq!(passive(&dwarf, &data, Skill::Perception).unwrap(), 13);
    let fighter = fighter(&data);
    assert_eq!(passive(&fighter, &data, Skill::Perception).unwrap(), 12);
    assert_eq!(passive(&fighter, &data, Skill::Stealth).unwrap(), 12);
}

#[test]
fn monster_stat_blocks_are_read_as_the_srd_says() {
    let data = data();
    let monster = |name: &str| {
        &data.monsters[&data
            .registry
            .monsters
            .get(&format!("base:monster:{name}"))
            .unwrap()]
    };
    let goblin = monster("goblin");
    assert_eq!(modifier_of(goblin, Ability::Dexterity), 2);
    assert_eq!(passive_perception(goblin), 9);
    assert_eq!(pick_attack(goblin, false).map(|a| a.ranged), Some(false));
    assert_eq!(
        pick_attack(goblin, true).map(|a| a.name.as_str()),
        Some("base:text:monster.goblin.shortbow")
    );
    let rat = monster("giant_rat");
    assert_eq!(pick_attack(rat, true), None, "no bow on a rat");
    assert!(pick_attack(rat, false).is_some());
    let skeleton = monster("skeleton");
    assert!(monster_defenses(skeleton, DamageType::Bludgeoning).vulnerable);
    assert!(monster_defenses(skeleton, DamageType::Poison).immune);
    assert_eq!(
        monster_defenses(skeleton, DamageType::Slashing),
        Defenses::default()
    );
    let mut rng = Pcg32::for_stream(1, &stream());
    assert_eq!(
        monster_hit_points(goblin, &data, &mut rng, &stream()).unwrap(),
        7
    );
    assert_eq!(
        monster_hit_points(skeleton, &data, &mut rng, &stream()).unwrap(),
        13
    );
    assert_eq!(rng.draws(), 0, "the base formula takes the average");
    assert_eq!(choose_target(0, &mut rng), None);
    let picks: Vec<usize> = (0..20)
        .map(|_| choose_target(3, &mut rng).unwrap())
        .collect();
    assert!(picks.iter().all(|p| *p < 3));
    assert!(picks.contains(&0) && picks.contains(&2), "{picks:?}");
    assert_eq!(rng.draws(), 20, "one draw per pick");
    let (total, trace) = initiative(&data, 2, &mut rng, &stream()).unwrap();
    assert_eq!(total, i64::from(trace.rolls[0].value) + 2);
}

#[test]
fn death_saves_count_to_three_either_way() {
    let data = data();
    let mut saves = DeathSaves::default();
    let mut rng = rng_showing(1, RollMode::Normal);
    let (trace, result) = death_save(&data, &mut saves, &mut rng, &stream()).unwrap();
    assert_eq!(
        (trace.rolls[0].value, result),
        (1, DeathSaveResult::Failure)
    );
    assert_eq!(saves.failures, 2, "a natural one counts twice");
    assert_eq!(wound_at_zero(&mut saves, false), DeathSaveResult::Died);
    assert_eq!(saves.failures, 3);

    let mut saves = DeathSaves::default();
    let mut rng = rng_showing(20, RollMode::Normal);
    let (_, result) = death_save(&data, &mut saves, &mut rng, &stream()).unwrap();
    assert_eq!(result, DeathSaveResult::Revived);
    assert_eq!(saves, DeathSaves::default());

    let mut saves = DeathSaves {
        successes: 2,
        failures: 1,
        stable: false,
    };
    let mut rng = rng_showing(10, RollMode::Normal);
    let (_, result) = death_save(&data, &mut saves, &mut rng, &stream()).unwrap();
    assert_eq!(result, DeathSaveResult::Stable, "ten meets the DC");
    assert!(saves.stable && saves.successes == 0 && saves.failures == 0);
    assert_eq!(wound_at_zero(&mut saves, true), DeathSaveResult::Failure);
    assert!(
        !saves.stable && saves.failures == 2,
        "a critical hit counts twice"
    );

    let mut saves = DeathSaves::default();
    let mut rng = rng_showing(9, RollMode::Normal);
    let (_, result) = death_save(&data, &mut saves, &mut rng, &stream()).unwrap();
    assert_eq!((result, saves.failures), (DeathSaveResult::Failure, 1));
    let mut rng = rng_showing(15, RollMode::Normal);
    let (_, result) = death_save(&data, &mut saves, &mut rng, &stream()).unwrap();
    assert_eq!((result, saves.successes), (DeathSaveResult::Success, 1));
}
