//! The views the engine's API adds in M8 step 8 (ARCHITECTURE.md §4.3, §4.9): `query::here`,
//! the sheet's numbers and timed effects on `party.get`, each stack's refusal and the bribe on
//! `combat.get`, a feature's choices, an offer's subject, and `cast_view`. Each is checked
//! against the command it describes, and looking changes nothing. Party: Brenna (human
//! fighter), Durin (dwarf cleric), Ilvara (elf wizard), Pip (halfling rogue).

use crate::common;

use common::{act, data, encounter, inside, new_world, party_of, world};
use omnis_data::{Ability, Data, Disposition, EquipSlot, ServiceKind, Skill};
use omnis_sim::omnis_core::{Facing, MapId, Position};
use omnis_sim::omnis_rules::Expiry;
use omnis_sim::query::{door_open, flags, here, map_text};
use omnis_sim::{
    ActorRef, ChoiceKind, CombatCommand, Command, EncounterChoice, Event, Mode, PARTY, Rejection,
    SaveRule, ServiceCommand, Settings, Surprise, Target, World, apply, cast_view, combat,
    combat_view, ops, party_view, service_view,
};

const BRENNA: usize = 0;
const DURIN: usize = 1;
const ILVARA: usize = 2;
const PIP: usize = 3;

fn fight(world: &mut World, data: &Data, stacks: &[(&str, u8)]) {
    let here = world.position;
    let monsters = encounter(data, stacks, Disposition::Hostile, here);
    combat::start(world, data, monsters, Surprise::None, &mut Vec::new()).unwrap();
}

/// Dodge whole turns until the member in `slot` acts.
fn until_turn_of(world: &mut World, data: &Data, slot: usize) {
    for _ in 0..200 {
        let id = world.party.members[slot].id;
        let current = combat_view(world, data).and_then(|v| v.current);
        if current == Some(ActorRef::Member(id)) {
            return;
        }
        act(world, data, Command::Combat(CombatCommand::Dodge)).unwrap();
    }
    panic!("the turn never came");
}

#[test]
fn here_is_the_status_without_the_fingerprint_and_no_view_changes_the_world() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let before = world.fingerprint().unwrap();
    let place = here(&world, &data);
    let status = ops::status(&world, &data).unwrap();
    assert_eq!(
        (place.mode, place.turn, &place.position, &place.service),
        (status.mode, status.turn, &status.position, &status.service)
    );
    assert_eq!(place.date, status.date);
    assert_eq!(place.age, status.clock.elapsed);
    assert_eq!(
        (place.may_save, place.seed, place.settings),
        (status.may_save, status.seed, status.settings)
    );
    assert_eq!(place.seed, world.seed);
    assert!(
        place.may_save,
        "the default save rule allows a save anywhere"
    );
    let strict = Settings {
        save_rule: SaveRule::InnOnly,
        ..Settings::default()
    };
    let outside = new_world(&data, 1, strict);
    assert!(
        !here(&outside, &data).may_save,
        "only at an inn under InnOnly"
    );
    assert_eq!(here(&outside, &data).settings, strict);
    assert!(flags(&world, &data).iter().all(|(_, value)| *value == 0));

    let _ = (
        party_view(&world, &data),
        cast_view(&world, &data),
        combat_view(&world, &data),
        service_view(&world, &data),
    );
    assert_eq!(
        world.fingerprint().unwrap(),
        before,
        "looking draws nothing"
    );
}

#[test]
fn the_sheet_s_numbers_are_the_srd_s() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let view = party_view(&world, &data);
    let brenna = &view.members[BRENNA];
    // Fighter: Strength and Constitution saves; a human's +1 makes Str 16 (+3) and Con 14
    // (+2); proficiency +2.
    assert_eq!(brenna.proficiency, 2);
    assert_eq!(
        brenna.saves,
        [(Ability::Strength, 5), (Ability::Constitution, 4)]
    );
    // Her two (Athletics on Str 16, +3; Perception on Wis 11, +0) and the acolyte's (Insight on
    // Wis, +0; Religion on Int 13, +1), each plus proficiency, in SRD order.
    assert_eq!(
        brenna.skills,
        [
            (Skill::Athletics, 5),
            (Skill::Insight, 2),
            (Skill::Perception, 2),
            (Skill::Religion, 3)
        ]
    );
    assert_eq!((brenna.hit_die, brenna.casting), (10, None));
    // Each modifier is the rule's for its score: Str 16 +3 ... Cha 9 -1.
    assert_eq!(
        brenna.modifiers,
        brenna.scores.map(omnis_sim::omnis_rules::modifier)
    );
    assert_eq!(brenna.modifiers[0], 3);
    assert_eq!(brenna.next_xp, Some(300), "SRD: level 2 at 300 experience");
    assert_eq!(brenna.background, "base:background:acolyte");
    assert_eq!(view.members[ILVARA].casting, Some(Ability::Intelligence));
    assert_eq!(view.members[DURIN].casting, Some(Ability::Wisdom));

    let calendar = data.calendar();
    let year = i64::from(calendar.minutes_per_day) * i64::from(calendar.days_per_year);
    let young = brenna.age_years;
    world.clocks.get_mut(&PARTY).unwrap().elapsed += year;
    assert_eq!(
        party_view(&world, &data).members[BRENNA].age_years,
        young + 1,
        "a year lived is a year older"
    );
}

#[test]
fn bless_is_cast_by_its_id_and_the_row_carries_that_id() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let durin = world.party.members[DURIN].id;
    let brenna = world.party.members[BRENNA].id;
    let bless = "base:spell:bless";
    let row = cast_view(&world, &data)
        .into_iter()
        .find(|r| r.caster == durin && r.spell == bless)
        .expect("Durin's bless, listed by its id");
    assert_eq!(row.refusal, None);

    let cast = |spell: &str| Command::Cast {
        caster: durin,
        spell: spell.to_owned(),
        target: Target::Member(brenna),
    };
    for unknown in ["base:spell:nope", "base:spell:magic_missile"] {
        let before = world.clone();
        assert_eq!(
            apply(&mut world, &data, cast(unknown)),
            Err(Rejection::UnknownSpell {
                spell: unknown.to_owned()
            }),
            "no pack defines it, or Durin does not know it"
        );
        assert_eq!(world, before, "a refusal changes nothing");
    }
    let events = apply(&mut world, &data, cast(&row.spell)).unwrap();
    let casts: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            omnis_sim::Event::SpellCast { caster, spell, .. } => Some((*caster, spell.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        casts,
        [(durin, bless)],
        "the cast names its caster and spell"
    );
    let applied: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            omnis_sim::Event::EffectApplied { spell, caster, .. } => {
                Some((spell.as_str(), *caster))
            }
            _ => None,
        })
        .collect();
    assert!(!applied.is_empty(), "bless settles on someone: {events:?}");
    assert!(
        applied.iter().all(|a| *a == (bless, durin)),
        "every effect names the spell by its id: {applied:?}"
    );
    assert!(
        party_view(&world, &data).members[BRENNA]
            .effects
            .iter()
            .any(|e| e.spell == bless),
        "Brenna is blessed"
    );
}

/// A row's cost is what the cast charges: the `spell_points.cost` slot, not the spell's level,
/// so a pack (or `rules.set`) that prices spells its own way is shown that price (parity P5).
#[test]
fn a_cast_row_costs_what_the_cost_slot_charges() {
    let mut data = data();
    data.rules
        .set_slot("spell_points.cost", "spell_level * 2")
        .unwrap();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let durin = world.party.members[DURIN].id;
    let bless = |world: &World| {
        cast_view(world, &data)
            .into_iter()
            .find(|r| r.caster == durin && r.spell == "base:spell:bless")
            .expect("Durin's Bless")
    };
    assert_eq!(bless(&world).cost, 2, "a level 1 spell at twice its level");
    world.party.members[DURIN].spell_points = 1;
    assert_eq!(
        bless(&world).refusal,
        Some(Rejection::NotEnoughPoints { need: 2, have: 1 })
    );
}

/// The fight's spell rows cost what the cast charges too: the `spell_points.cost` slot, not the
/// spell's level (parity P6; the road rows were fixed in P5).
#[test]
fn a_fight_spell_row_costs_what_the_cost_slot_charges() {
    let mut data = data();
    data.rules
        .set_slot("spell_points.cost", "spell_level * 2")
        .unwrap();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    fight(&mut world, &data, &[("goblin", 1)]);
    until_turn_of(&mut world, &data, DURIN);
    let view = combat_view(&world, &data).expect("in a fight");
    let bless = view
        .spells
        .iter()
        .find(|s| s.spell == "base:spell:bless")
        .expect("Durin's Bless");
    assert_eq!(bless.cost, 2, "a level 1 spell at twice its level");
}

/// Each member's known spells carry what a cast charges, from the cost slot, and what it spends
/// of the turn, from the definition (parity P6, the sheet's Magic page; ARCHITECTURE §4.4).
#[test]
fn a_known_spell_costs_what_a_cast_charges_and_pays_its_definition_s_cost() {
    let mut data = data();
    data.rules
        .set_slot("spell_points.cost", "spell_level * 2")
        .unwrap();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let shield = data.registry.spells.get("base:spell:shield").unwrap();
    if !world.party.members[ILVARA].known_spells.contains(&shield) {
        world.party.members[ILVARA].known_spells.push(shield);
    }
    let view = party_view(&world, &data);
    for (member, shown) in world.party.members.iter().zip(&view.members) {
        let ids: Vec<&str> = shown
            .known_spells
            .iter()
            .map(|k| k.spell.as_str())
            .collect();
        let spells: Vec<&str> = shown.spells.iter().map(String::as_str).collect();
        assert_eq!(
            ids, spells,
            "{}: one row per known spell, in order",
            member.name
        );
        for row in &shown.known_spells {
            let def = data
                .spells
                .get(&data.registry.spells.get(&row.spell).unwrap())
                .unwrap();
            let want = def.points.unwrap_or(u32::from(def.level) * 2);
            assert_eq!(row.cost, want, "{}", row.spell);
            assert_eq!(row.pay, def.cost, "{}", row.spell);
            assert_eq!(row.level, def.level, "{}", row.spell);
        }
    }
    assert!(
        view.members[DURIN].known_spells.iter().any(|k| k.cost == 0),
        "a cantrip is free"
    );
    let shield = view.members[ILVARA]
        .known_spells
        .iter()
        .find(|k| k.spell == "base:spell:shield")
        .expect("Ilvara knows Shield");
    assert_eq!(
        shield.pay,
        omnis_data::Cost::Reaction,
        "Shield answers an attack"
    );

    // The price shown is the price paid.
    let durin = world.party.members[DURIN].id;
    let bless = view.members[DURIN]
        .known_spells
        .iter()
        .find(|k| k.spell == "base:spell:bless")
        .expect("Durin knows Bless")
        .cost;
    let before = world.party.members[DURIN].spell_points;
    let command = Command::Cast {
        caster: durin,
        spell: "base:spell:bless".to_owned(),
        target: Target::Member(durin),
    };
    apply(&mut world, &data, command).unwrap();
    assert_eq!(before - world.party.members[DURIN].spell_points, bless);
}

#[test]
fn a_cast_row_s_refusal_is_the_command_s_and_its_effect_shows_its_time_left() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let rows = cast_view(&world, &data);
    assert!(
        !rows.is_empty(),
        "the cleric and the wizard cast outside a fight"
    );
    let rngs = world.rngs.clone();
    for row in &rows {
        let mut after = world.clone();
        let command = Command::Cast {
            caster: row.caster,
            spell: row.spell.clone(),
            target: Target::Member(row.caster),
        };
        let result = apply(&mut after, &data, command);
        assert_eq!(result.as_ref().err(), row.refusal.as_ref(), "{row:?}");
    }
    assert_eq!(world.rngs, rngs, "the rows draw nothing");

    let mut spent = world.clone();
    spent.party.members[DURIN].spell_points = 0;
    let broke = cast_view(&spent, &data);
    for row in broke
        .iter()
        .filter(|r| r.caster == spent.party.members[DURIN].id && r.cost > 0)
    {
        assert!(row.refusal.is_some(), "no points: {row:?}");
    }

    let buff = rows
        .iter()
        .find(|r| {
            r.refusal.is_none()
                && r.cost > 0
                && apply(
                    &mut world.clone(),
                    &data,
                    Command::Cast {
                        caster: r.caster,
                        spell: r.spell.clone(),
                        target: Target::Member(r.caster),
                    },
                )
                .is_ok_and(|events| {
                    events
                        .iter()
                        .any(|e| matches!(e, omnis_sim::Event::EffectApplied { .. }))
                })
        })
        .expect("a buff castable outside a fight");
    apply(
        &mut world,
        &data,
        Command::Cast {
            caster: buff.caster,
            spell: buff.spell.clone(),
            target: Target::Member(buff.caster),
        },
    )
    .unwrap();
    let now = world.party_clock().elapsed;
    let view = party_view(&world, &data);
    let shown: Vec<_> = view
        .members
        .iter()
        .flat_map(|m| &m.effects)
        .chain(&view.effects)
        .filter(|e| e.spell == buff.spell)
        .collect();
    let held: Vec<_> = world
        .party
        .members
        .iter()
        .flat_map(|m| &m.effects)
        .chain(&world.party.effects)
        .collect();
    assert_eq!(shown.len(), held.len(), "{shown:?}");
    for (view, effect) in shown.iter().zip(&held) {
        assert_eq!(view.caster, buff.caster);
        let left = match effect.until {
            Expiry::Minute(m) => Some(m - now),
            Expiry::NextTurn => None,
        };
        assert_eq!(view.minutes_left, left);
    }
}

#[test]
fn a_stack_s_refusal_is_the_attack_s_and_reach_follows_it() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    // Without her bow Brenna's sword reaches the front stacks only.
    world.party.members[BRENNA]
        .equipped
        .remove(&EquipSlot::Ranged);
    fight(
        &mut world,
        &data,
        &[("goblin", 2), ("goblin", 2), ("goblin", 2)],
    );
    until_turn_of(&mut world, &data, BRENNA);
    let view = combat_view(&world, &data).unwrap();
    assert_eq!(view.bribe, None, "no bribe once the fight is on");
    let mut refused = 0;
    for stack in &view.stacks {
        let mut after = world.clone();
        let result = apply(
            &mut after,
            &data,
            Command::Combat(CombatCommand::Attack { stack: stack.stack }),
        );
        assert_eq!(result.err(), stack.refusal, "{stack:?}");
        assert_eq!(stack.reachable, stack.refusal.is_none());
        refused += usize::from(stack.refusal.is_some());
    }
    assert!(refused > 0, "Brenna's sword cannot reach the back stack");
}

#[test]
fn the_bribe_shown_is_the_bribe_paid() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    world.party.gold = 1_000_000;
    let here = world.position;
    world.mode = Mode::Encounter(encounter(
        &data,
        &[("goblin", 3)],
        Disposition::Neutral,
        here,
    ));
    let bribe = combat_view(&world, &data)
        .unwrap()
        .bribe
        .expect("neutral goblins take a bribe");
    let gold = world.party.gold;
    apply(
        &mut world,
        &data,
        Command::Encounter(EncounterChoice::Bribe),
    )
    .unwrap();
    assert_eq!(gold - world.party.gold, bribe);
}

#[test]
fn cunning_action_offers_its_two_uses_and_second_wind_one() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    world.party.members[PIP].level = 2;
    fight(&mut world, &data, &[("goblin", 3)]);
    let view = combat_view(&world, &data).unwrap();
    let cunning = view.members[PIP]
        .features
        .iter()
        .find(|f| f.feature == "base:text:class.rogue.cunning_action")
        .unwrap();
    assert_eq!(cunning.choices, [ChoiceKind::Exchange, ChoiceKind::Hide]);
    assert!(
        view.members[BRENNA]
            .features
            .iter()
            .all(|f| f.choices == [ChoiceKind::Plain])
    );
}

#[test]
fn an_offer_names_the_item_or_spell_its_row_is() {
    let data = data();
    let mut world = inside(&data, "smith");
    world.party.gold = 1_000_000;
    let view = service_view(&world, &data).unwrap();
    assert_eq!(view.kind, ServiceKind::Smith);
    let def = data
        .services
        .values()
        .find(|d| d.id == view.service)
        .unwrap();
    let buys: Vec<_> = view
        .offers
        .iter()
        .filter(|o| matches!(o.command, ServiceCommand::Buy { .. }))
        .collect();
    for (row, offer) in buys.iter().enumerate() {
        assert_eq!(offer.subject.as_ref(), def.items.get(row), "{offer:?}");
    }
    apply(&mut world, &data, Command::Service(buys[0].command.clone())).unwrap();
    let view = service_view(&world, &data).unwrap();
    let sale = view
        .offers
        .iter()
        .find(|o| matches!(o.command, ServiceCommand::Sell { .. }))
        .unwrap();
    assert_eq!(
        sale.subject.as_ref(),
        def.items.first(),
        "sold what was bought"
    );
    assert_eq!(
        view.offers.last().unwrap().subject,
        None,
        "leaving names nothing"
    );
    let refused = view
        .offers
        .iter()
        .all(|o| o.refusal != Some(Rejection::WrongMode));
    assert!(refused);
}

/// A number no pack names reads `#n` in the party view as it does in events and every other
/// view (protocol 2, `names::id_of`); it read `?`, a second spelling of the same thing.
#[test]
fn a_number_no_pack_names_reads_the_same_in_the_party_view_as_everywhere() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    world.party.members[0]
        .known_spells
        .push(omnis_core::SpellId(999));
    let view = omnis_sim::party_view(&world, &data);
    assert_eq!(
        view.members[0].spells.last().map(String::as_str),
        Some("#999")
    );
}

/// Whether `text` (a `map_text` layout) shows the door on `facing` of `(x, y)` open.
fn drawn_open(text: &str, x: u16, y: u16, facing: Facing) -> bool {
    let (row, col) = (2 * usize::from(y) + 1, 2 * usize::from(x) + 1);
    let (row, col) = match facing {
        Facing::North => (row - 1, col),
        Facing::South => (row + 1, col),
        Facing::West => (row, col - 1),
        Facing::East => (row, col + 1),
    };
    let line = text.lines().nth(row).expect("row");
    matches!(line.chars().nth(col), Some('_' | '\''))
}

#[test]
fn door_open_is_the_world_s_door_on_every_edge_of_every_map() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    // Open the first door Interact opens, standing on each door's tile in turn.
    let mut opened = None;
    'find: for (&map, map_data) in &data.maps {
        for y in 0..map_data.def.height {
            for x in 0..map_data.def.width {
                for facing in Facing::ALL {
                    if !map_data.cell(x, y).is_some_and(|c| c.doors.has(facing)) {
                        continue;
                    }
                    let mut trial = world.clone();
                    trial.position = Position { map, x, y, facing };
                    let events = apply(&mut trial, &data, Command::Interact).unwrap_or_default();
                    if events.iter().any(|e| matches!(e, Event::Door { .. })) {
                        world = trial;
                        opened = Some((map, x, y, facing));
                        break 'find;
                    }
                }
            }
        }
    }
    let (map, x, y, facing) = opened.expect("some door opens");
    assert!(
        door_open(&world, map, x, y, facing),
        "the opened door reads open"
    );
    let mut doors = 0;
    for (&map, map_data) in &data.maps {
        let text = map_text(&world, &data, map).expect("map text");
        for y in 0..map_data.def.height {
            for x in 0..map_data.def.width {
                for facing in Facing::ALL {
                    if map_data.cell(x, y).is_some_and(|c| c.doors.has(facing)) {
                        doors += 1;
                        assert_eq!(
                            door_open(&world, map, x, y, facing),
                            drawn_open(&text, x, y, facing),
                            "map {map:?} ({x}, {y}) {facing:?}"
                        );
                    }
                }
            }
        }
    }
    assert!(doors > 1, "the test pack has doors");
    assert!(
        !door_open(&world, MapId(u32::MAX), 0, 0, Facing::North),
        "an unknown map"
    );
}

#[test]
fn can_fight_is_up_alive_and_not_incapacitated() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let fit = |world: &World| -> Vec<bool> {
        party_view(world, &data)
            .members
            .iter()
            .map(|m| m.can_fight)
            .collect()
    };
    assert_eq!(fit(&world), [true, true, true, true]);
    world.party.members[BRENNA].hp = 0;
    let incapacitated = data
        .registry
        .conditions
        .get("base:condition:incapacitated")
        .expect("incapacitated");
    world.party.members[DURIN].conditions.push(incapacitated);
    let dead = data
        .registry
        .conditions
        .get("base:condition:dead")
        .expect("dead");
    world.party.members[ILVARA].conditions.push(dead);
    assert_eq!(fit(&world), [false, false, false, true]);
    let view = party_view(&world, &data);
    assert!(view.members[BRENNA].down && !view.members[DURIN].down && view.members[ILVARA].dead);
}
