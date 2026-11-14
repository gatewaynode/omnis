//! Members in feet (owner, 2026-11-14; `tasks/plans/combat-in-feet.md` step 4): a member's
//! weapon has a reach and a range, the party's line advances as one, a touch is a reach of five
//! feet, a spell's range in tiles is counted in the map's feet, and a ranged attack with a foe
//! upon the attacker has disadvantage.

use crate::common::{self, data, party_of, world};
use crate::targets::{fight, listed};
use omnis_data::{Data, Disposition, EquipSlot, ItemKind, Range, Targeting};
use omnis_rules::RollMode;
use omnis_sim::combat::{Strike, strike_for};
use omnis_sim::{
    ActorRef, Chosen, CombatCommand, CombatState, Command, EncounterSource, EncounterState, Event,
    Mode, Pay, Rejection, Settings, Stack, Surprise, Target, World, apply, combat, combat_view,
};

const FIGHTER: usize = 0;
const CLERIC: usize = 1;
const WIZARD: usize = 2;
/// Wren, a cleric in the back row.
const BACK_CLERIC: usize = 5;

/// The six in a fight with giant rats standing at `feet`, on the turn of the member in `slot`,
/// the round's advance unused.
fn duel(data: &Data, slot: usize, feet: &[u16]) -> World {
    let mut world = world(data);
    party_of(&mut world, data, 6);
    duel_in(world, data, slot, feet)
}

fn duel_in(mut world: World, data: &Data, slot: usize, feet: &[u16]) -> World {
    // The clerics learn a touch that strikes (the drafts know none).
    let wounds = data
        .registry
        .spells
        .get("base:spell:inflict_wounds")
        .unwrap();
    for member in &mut world.party.members {
        member.hp = 500;
        member.hp_max = 500;
        if !member.known_spells.is_empty() && !member.known_spells.contains(&wounds) {
            member.known_spells.push(wounds);
        }
    }
    let rats: Vec<(&str, u8)> = feet.iter().map(|_| ("giant_rat", 2)).collect();
    fight(&mut world, data, &rats, slot);
    for (stack, feet) in feet.iter().enumerate() {
        common::stand_back(&mut world, stack, *feet);
    }
    let state = state_mut(&mut world);
    state.advanced = 0;
    world
}

fn state(world: &World) -> &CombatState {
    match &world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

fn state_mut(world: &mut World) -> &mut CombatState {
    match &mut world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

fn distances(world: &World) -> Vec<u16> {
    let stacks = state(world).encounter.stacks.iter();
    stacks.map(|s| s.distance).collect()
}

/// The same fight with nobody carrying a ranged weapon.
fn unslung(mut world: World) -> World {
    for member in &mut world.party.members {
        member.equipped.remove(&EquipSlot::Ranged);
    }
    world
}

/// How the member in `slot` would attack `stack`: `(ranged, advance, hindered)`.
fn strike(
    world: &World,
    data: &Data,
    slot: usize,
    stack: u8,
) -> Result<(bool, u16, bool), Rejection> {
    let Strike {
        weapon,
        advance,
        hindered,
    } = strike_for(state(world), world, data, slot, stack)?;
    Ok((weapon.ranged, advance, hindered))
}

fn refused(world: &World, data: &Data, command: CombatCommand) -> Rejection {
    let mut copy = world.clone();
    let result = apply(&mut copy, data, Command::Combat(command));
    assert_eq!(&copy, world, "a refusal changes nothing");
    result.expect_err("refused")
}

fn advanced(events: &[Event]) -> Vec<u16> {
    let moves = events.iter().filter_map(|e| match e {
        Event::PartyAdvanced { feet } => Some(*feet),
        _ => None,
    });
    moves.collect()
}

/// A front-row member with a sword and a crossbow: the sword on an engaged stack; the bow on
/// anything further, hindered past its normal range or with a stack upon the shooter; nothing
/// past its long range.
#[test]
fn a_member_s_weapon_follows_the_distance() {
    let data = data();
    let at = |feet: &[u16]| duel(&data, FIGHTER, feet);
    assert_eq!(strike(&at(&[5]), &data, FIGHTER, 0), Ok((false, 0, false)));
    assert_eq!(strike(&at(&[15]), &data, FIGHTER, 0), Ok((true, 0, false)));
    assert_eq!(strike(&at(&[80]), &data, FIGHTER, 0), Ok((true, 0, false)));
    assert_eq!(
        strike(&at(&[85]), &data, FIGHTER, 0),
        Ok((true, 0, true)),
        "past the light crossbow's normal range"
    );
    assert_eq!(strike(&at(&[320]), &data, FIGHTER, 0), Ok((true, 0, true)));
    assert_eq!(
        strike(&at(&[325]), &data, FIGHTER, 0),
        Err(Rejection::BeyondRange)
    );
    assert_eq!(
        strike(&at(&[5, 15]), &data, FIGHTER, 1),
        Ok((true, 0, true)),
        "a stack is upon the shooter"
    );
    // The back row stands five feet further off: no sword reaches, and no foe is upon them.
    let world = duel(&data, BACK_CLERIC, &[5, 15]);
    assert_eq!(strike(&world, &data, BACK_CLERIC, 0), Ok((true, 0, false)));
    assert_eq!(strike(&world, &data, BACK_CLERIC, 1), Ok((true, 0, false)));
    let world = unslung(world);
    assert_eq!(
        strike(&world, &data, BACK_CLERIC, 0),
        Err(Rejection::NeedsRangedWeapon)
    );
}

/// The attack itself rolls as the distance has it: a shot past normal range at disadvantage.
#[test]
fn a_long_shot_rolls_with_disadvantage() {
    let data = data();
    let mode = |feet: u16| {
        let mut world = duel(&data, FIGHTER, &[feet]);
        let attack = Command::Combat(CombatCommand::Attack { stack: 0 });
        let events = apply(&mut world, &data, attack).unwrap();
        let roll = events.iter().find_map(|e| match e {
            Event::AttackResolved {
                attacker: ActorRef::Member(_),
                roll,
                ..
            } => Some(roll.mode),
            _ => None,
        });
        roll.expect("an attack")
    };
    assert_eq!(mode(15), RollMode::Normal);
    assert_eq!(mode(85), RollMode::Disadvantage);
}

/// A weapon with a reach of ten feet strikes a stack ten feet off without moving, and from the
/// back row a stack that is engaged.
#[test]
fn a_long_weapon_reaches_further() {
    let mut data = data();
    for name in ["longsword", "mace"] {
        let id = data
            .registry
            .items
            .get(&format!("base:item:{name}"))
            .unwrap();
        let ItemKind::Weapon { reach, .. } = &mut data.items.get_mut(&id).unwrap().kind else {
            panic!("a weapon");
        };
        *reach = 10;
    }
    let world = unslung(duel(&data, FIGHTER, &[10, 15]));
    assert_eq!(strike(&world, &data, FIGHTER, 0), Ok((false, 0, false)));
    assert_eq!(
        strike(&world, &data, FIGHTER, 1),
        Ok((false, 5, false)),
        "five feet short: the line advances"
    );
    let world = unslung(duel(&data, BACK_CLERIC, &[5, 10]));
    assert_eq!(strike(&world, &data, BACK_CLERIC, 0), Ok((false, 0, false)));
    assert_eq!(
        strike(&world, &data, BACK_CLERIC, 1),
        Err(Rejection::NeedsRangedWeapon),
        "the back row does not lead an advance"
    );
}

/// Without a bow a front-row member's attack advances the line when the round's pace covers
/// the gap and no stack stands in the way; every stack is that much nearer.
#[test]
fn a_melee_attack_advances_the_line_when_it_can() {
    let data = data();
    // Durin the dwarf stands in front: the line's pace is his 25 feet.
    let world = unslung(duel(&data, FIGHTER, &[15, 40]));
    assert_eq!(combat_view(&world, &data).unwrap().advance, 10);
    assert_eq!(strike(&world, &data, FIGHTER, 0), Ok((false, 10, false)));
    assert_eq!(
        strike(&world, &data, FIGHTER, 1),
        Err(Rejection::OutOfReach { stack: 1 }),
        "the nearer stack stops the line first"
    );
    let mut after = world.clone();
    let attack = Command::Combat(CombatCommand::Attack { stack: 0 });
    let events = apply(&mut after, &data, attack).unwrap();
    assert_eq!(advanced(&events), [10]);
    let moved = events
        .iter()
        .position(|e| matches!(e, Event::PartyAdvanced { .. }))
        .unwrap();
    let swung = events
        .iter()
        .position(|e| matches!(e, Event::AttackResolved { .. }))
        .unwrap();
    assert!(moved < swung, "the advance comes first");
    if matches!(after.mode, Mode::Combat(_))
        && !events.iter().any(|e| matches!(e, Event::StackMoved { .. }))
    {
        assert_eq!(distances(&after), [5, 30]);
    }
    // Thirty feet off is more than the pace: out of reach this round.
    let world = unslung(duel(&data, FIGHTER, &[35]));
    assert_eq!(
        strike(&world, &data, FIGHTER, 0),
        Err(Rejection::OutOfReach { stack: 0 })
    );
    // With a stack engaged the line does not move.
    let world = unslung(duel(&data, FIGHTER, &[5, 15]));
    assert_eq!(combat_view(&world, &data).unwrap().advance, 0);
    assert_eq!(
        strike(&world, &data, FIGHTER, 1),
        Err(Rejection::OutOfReach { stack: 1 })
    );
    // What the line has advanced this round is spent.
    let mut world = unslung(duel(&data, FIGHTER, &[15]));
    state_mut(&mut world).advanced = 20;
    assert_eq!(
        strike(&world, &data, FIGHTER, 0),
        Err(Rejection::OutOfReach { stack: 0 })
    );
}

/// Advance moves the line on the nearest stack: free while the round's pace lasts, then a
/// dash, which is the action; refused once a stack is engaged. A new round restores the pace.
#[test]
fn advance_moves_the_line_and_then_costs_the_action() {
    let data = data();
    let mut world = duel(&data, FIGHTER, &[40, 60]);
    assert_eq!(combat_view(&world, &data).unwrap().advance, 25);
    let advance = || Command::Combat(CombatCommand::Advance);
    let events = apply(&mut world, &data, advance()).unwrap();
    assert_eq!(advanced(&events), [25]);
    assert_eq!(distances(&world), [15, 35]);
    assert_eq!(
        state(&world).budget.actions,
        1,
        "within the pace it is free"
    );
    assert_eq!(state(&world).advanced, 25);
    assert_eq!(combat_view(&world, &data).unwrap().advance, 0);
    // The pace is used up: the next advance is a dash, ten feet to the nearest stack.
    let mut spent = world.clone();
    state_mut(&mut spent).budget.actions = 0;
    assert_eq!(
        refused(&spent, &data, CombatCommand::Advance),
        Rejection::NoActionLeft
    );
    let events = apply(&mut world, &data, advance()).unwrap();
    assert_eq!(advanced(&events), [10]);
    // A dash of a whole pace when the gap is wider.
    let mut world = duel(&data, FIGHTER, &[100]);
    apply(&mut world, &data, advance()).unwrap();
    let events = apply(&mut world, &data, advance()).unwrap();
    assert_eq!(advanced(&events), [25], "one more pace");
    // Engaged, there is nowhere to advance to.
    let world = duel(&data, FIGHTER, &[5, 40]);
    assert_eq!(
        refused(&world, &data, CombatCommand::Advance),
        Rejection::AlreadyEngaged
    );
    // The round's end restores the pace.
    let mut world = duel(&data, FIGHTER, &[300]);
    apply(&mut world, &data, advance()).unwrap();
    assert_eq!(state(&world).advanced, 25);
    let round = state(&world).round;
    for _ in 0..40 {
        if state(&world).round > round {
            break;
        }
        let end = Command::Combat(CombatCommand::EndTurn);
        apply(&mut world, &data, end).unwrap();
    }
    assert!(state(&world).round > round);
    assert_eq!(state(&world).advanced, 0);
}

/// The line's pace is its slowest standing front-row member's speed.
#[test]
fn the_line_moves_at_the_pace_of_its_slowest_front_member() {
    let data = data();
    let mut world = duel(&data, FIGHTER, &[300]);
    assert_eq!(combat_view(&world, &data).unwrap().advance, 25, "the dwarf");
    world.party.members[CLERIC].hp = 0;
    assert_eq!(
        combat_view(&world, &data).unwrap().advance,
        30,
        "humans and an elf"
    );
}

fn cast(spell: &str, stack: u8) -> CombatCommand {
    CombatCommand::Cast {
        spell: format!("base:spell:{spell}"),
        target: Target::Stack(stack),
        pay: Pay::Action,
    }
}

/// A touch at a stack is a reach of five feet from the front row: the line advances to it when
/// it can, and the back row cannot touch a stack at all.
#[test]
fn a_touch_advances_the_line_or_is_out_of_reach() {
    let data = data();
    let mut world = duel(&data, CLERIC, &[15, 40]);
    assert_eq!(
        refused(&world, &data, cast("inflict_wounds", 1)),
        Rejection::OutOfReach { stack: 1 }
    );
    let events = apply(
        &mut world,
        &data,
        Command::Combat(cast("inflict_wounds", 0)),
    )
    .unwrap();
    assert_eq!(advanced(&events), [10]);
    let paid = events
        .iter()
        .position(|e| matches!(e, Event::SpellCast { .. }))
        .unwrap();
    let moved = events
        .iter()
        .position(|e| matches!(e, Event::PartyAdvanced { .. }))
        .unwrap();
    assert!(paid < moved);
    // Engaged already, nothing moves.
    let mut world = duel(&data, CLERIC, &[5]);
    let events = apply(
        &mut world,
        &data,
        Command::Combat(cast("inflict_wounds", 0)),
    )
    .unwrap();
    assert!(advanced(&events).is_empty());
    // The back row stands behind the line.
    let world = duel(&data, BACK_CLERIC, &[5]);
    assert_eq!(
        refused(&world, &data, cast("inflict_wounds", 0)),
        Rejection::OutOfReach { stack: 0 }
    );
}

/// A spell's range in tiles is that many of the map's tiles in feet: fire bolt's 24 tiles are
/// 120 feet where a tile is five, and twice that where a tile is ten.
#[test]
fn a_spell_s_range_is_counted_in_the_map_s_feet() {
    let mut data = data();
    let world = duel(&data, WIZARD, &[120, 125]);
    let rows = listed(
        &world,
        &data,
        WIZARD,
        &Chosen::Spell {
            spell: "base:spell:fire_bolt".into(),
            pay: Pay::Action,
        },
    );
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].distance, rows[0].refusal.as_ref()), (120, None));
    assert_eq!(
        (rows[1].distance, rows[1].refusal.as_ref()),
        (125, Some(&Rejection::BeyondRange))
    );
    // A spell that reaches as far as sight reaches any stack of the fight.
    let bolt = data.registry.spells.get("base:spell:fire_bolt").unwrap();
    data.spells.get_mut(&bolt).unwrap().targeting = Some(Targeting::of(Range::Sight));
    let world = duel(&data, WIZARD, &[2000]);
    assert_eq!(
        listed(&world, &data, WIZARD, &Chosen::Attack).len(),
        1,
        "the list and the command agree"
    );
    let mut copy = world.clone();
    apply(&mut copy, &data, Command::Combat(cast("fire_bolt", 0))).expect("sight reaches");
    data.spells.get_mut(&bolt).unwrap().targeting = Some(Targeting::of(Range::Tiles(24)));
    let map = world.position.map;
    data.maps.get_mut(&map).unwrap().def.tile_feet = Some(10);
    let world = duel(&data, WIZARD, &[240, 245]);
    let mut copy = world.clone();
    apply(&mut copy, &data, Command::Combat(cast("fire_bolt", 0))).expect("24 tiles of 10 ft");
    assert_eq!(
        refused(&world, &data, cast("fire_bolt", 1)),
        Rejection::BeyondRange
    );
}

/// The roll mode of the acting member's spell attack at `stack`.
fn bolt_mode(world: &World, data: &Data, spell: &str, stack: u8) -> RollMode {
    let mut copy = world.clone();
    let events = apply(&mut copy, data, Command::Combat(cast(spell, stack))).unwrap();
    let roll = events.iter().find_map(|e| match e {
        Event::AttackResolved {
            attacker: ActorRef::Member(_),
            roll,
            ..
        } => Some(roll.mode),
        _ => None,
    });
    roll.expect("a spell attack")
}

/// A ranged spell attack has disadvantage with a stack upon a front-row caster; a touch spell's
/// attack has none, and the back row is not pressed.
#[test]
fn a_ranged_spell_attack_is_hindered_with_a_foe_upon_the_caster() {
    let data = data();
    let pressed = duel(&data, WIZARD, &[5, 30]);
    assert_eq!(
        bolt_mode(&pressed, &data, "fire_bolt", 1),
        RollMode::Disadvantage
    );
    assert_eq!(
        bolt_mode(&pressed, &data, "fire_bolt", 0),
        RollMode::Disadvantage
    );
    let free = duel(&data, WIZARD, &[10, 30]);
    assert_eq!(bolt_mode(&free, &data, "fire_bolt", 1), RollMode::Normal);
    let touching = duel(&data, CLERIC, &[5]);
    assert_eq!(
        bolt_mode(&touching, &data, "inflict_wounds", 0),
        RollMode::Normal
    );
    // Ilvara behind the line: the same stack upon the party is not upon her.
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    world.party.members.swap(WIZARD, 3);
    let behind = duel_in(world, &data, 3, &[5, 30]);
    assert_eq!(bolt_mode(&behind, &data, "fire_bolt", 1), RollMode::Normal);
}

/// A monster that casts is held to its spells' ranges: a touch only when engaged, a range in
/// tiles as far as that many tiles in feet.
#[test]
fn a_casting_monster_s_spells_have_their_ranges() {
    let casts_from = |spell: &str, feet: u16| casts(&[spell], feet).len();
    assert!(casts_from("fire_bolt", 120) > 0);
    assert_eq!(casts_from("fire_bolt", 125), 0, "past 24 tiles of 5 ft");
    assert!(casts_from("thunderwave", 5) > 0, "a touch, engaged");
    assert_eq!(casts_from("thunderwave", 10), 0);
    // Of two spells, only the one that reaches is ever chosen.
    let both = casts(&["fire_bolt", "thunderwave"], 60);
    assert!(!both.is_empty());
    assert!(both.iter().all(|s| s == "base:spell:fire_bolt"), "{both:?}");
}

/// The spells Bob the Rat King, with only `spells` and no weapon, casts from `feet` off over a
/// few rounds.
fn casts(spells: &[&str], feet: u16) -> Vec<String> {
    {
        let mut data = data();
        let bob = data
            .registry
            .monsters
            .get("test:monster:bob_the_rat_king")
            .unwrap();
        let m = data.monsters.get_mut(&bob).unwrap();
        m.casting.as_mut().unwrap().spells =
            spells.iter().map(|s| format!("base:spell:{s}")).collect();
        m.attacks.clear();
        let mut world = common::new_world(&data, 7, Settings::default());
        party_of(&mut world, &data, 4);
        for member in &mut world.party.members {
            member.hp = 500;
        }
        let encounter = EncounterState {
            source: EncounterSource::Random,
            stacks: vec![Stack {
                monster: bob,
                initial: 1,
                hp: vec![60],
                spent: Vec::new(),
                distance: feet,
            }],
            disposition: Disposition::Hostile,
            retreat: world.position,
        };
        let mut events = Vec::new();
        combat::start(&mut world, &data, encounter, Surprise::None, &mut events).unwrap();
        for _ in 0..8 {
            let end = Command::Combat(CombatCommand::EndTurn);
            events.extend(apply(&mut world, &data, end).unwrap());
        }
        let cast = events.iter().filter_map(|e| match e {
            Event::MonsterCast { spell, .. } => Some(spell.clone()),
            _ => None,
        });
        cast.collect()
    }
}

/// The saving throws a cast at stack 0 asks of the monsters: one for each individual caught.
fn caught(data: &Data, monster: &str, count: u8, spell: &str) -> usize {
    let mut world = world(data);
    party_of(&mut world, data, 6);
    fight(&mut world, data, &[(monster, count)], WIZARD);
    common::stand_back(&mut world, 0, 5);
    let events = apply(&mut world, data, Command::Combat(cast(spell, 0))).unwrap();
    let saves = events.iter().filter(|e| {
        matches!(
            e,
            Event::Check {
                actor: ActorRef::Monster { .. },
                ..
            }
        )
    });
    saves.count()
}

/// An area catches as many of a stack as fit its squares (owner, 2026-11-14: one human a
/// square, two kobolds, three rats): burning hands, six squares, catches eighteen giant rats or
/// twelve goblins, and never more than stand there. A spell that takes the whole stack still
/// does.
#[test]
fn an_area_catches_what_fits_its_squares() {
    let mut data = data();
    assert_eq!(caught(&data, "giant_rat", 20, "burning_hands"), 18);
    assert_eq!(caught(&data, "goblin", 20, "burning_hands"), 12);
    assert_eq!(caught(&data, "giant_rat", 4, "burning_hands"), 4);
    let rat = data
        .registry
        .monsters
        .get("base:monster:giant_rat")
        .unwrap();
    data.monsters.get_mut(&rat).unwrap().per_square = Some(1);
    assert_eq!(caught(&data, "giant_rat", 20, "burning_hands"), 6);
    let hands = data
        .registry
        .spells
        .get("base:spell:burning_hands")
        .unwrap();
    data.spells.get_mut(&hands).unwrap().reach = omnis_data::Reach::Stack;
    assert_eq!(caught(&data, "giant_rat", 20, "burning_hands"), 20);
}

/// A monster's area catches a member a square: two squares, two members of the front row.
#[test]
fn a_monster_s_area_catches_a_member_a_square() {
    let saves = |squares: u8| {
        let mut data = data();
        let wave = data.registry.spells.get("base:spell:thunderwave").unwrap();
        data.spells.get_mut(&wave).unwrap().reach = omnis_data::Reach::Area { squares };
        let bob = data
            .registry
            .monsters
            .get("test:monster:bob_the_rat_king")
            .unwrap();
        let m = data.monsters.get_mut(&bob).unwrap();
        m.casting.as_mut().unwrap().spells = vec!["base:spell:thunderwave".into()];
        m.attacks.clear();
        let mut world = common::new_world(&data, 7, Settings::default());
        party_of(&mut world, &data, 6);
        for member in &mut world.party.members {
            member.hp = 500;
        }
        let encounter = EncounterState {
            source: EncounterSource::Random,
            stacks: vec![Stack {
                monster: bob,
                initial: 1,
                hp: vec![60],
                spent: Vec::new(),
                distance: 5,
            }],
            disposition: Disposition::Hostile,
            retreat: world.position,
        };
        let mut events = Vec::new();
        combat::start(&mut world, &data, encounter, Surprise::None, &mut events).unwrap();
        for _ in 0..8 {
            let end = Command::Combat(CombatCommand::EndTurn);
            events.extend(apply(&mut world, &data, end).unwrap());
        }
        // The saves each cast asked for.
        let mut per_cast: Vec<usize> = Vec::new();
        for event in &events {
            match event {
                Event::MonsterCast { .. } => per_cast.push(0),
                Event::Check {
                    actor: ActorRef::Member(_),
                    kind: omnis_sim::CheckKind::Save(_),
                    ..
                } => *per_cast.last_mut().expect("a cast before a save") += 1,
                _ => {}
            }
        }
        assert!(!per_cast.is_empty(), "Bob cast");
        per_cast
    };
    assert!(saves(2).iter().all(|n| *n == 2), "{:?}", saves(2));
    assert!(
        saves(9).iter().all(|n| *n == 3),
        "the front row: {:?}",
        saves(9)
    );
}
