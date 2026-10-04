//! The turn budget measured (M7c step 5): one clear of the test dungeon by parties of two, four
//! and six, played three ways: M7b's one command a turn; the whole budget (Action Surge on the
//! first round, Second Wind under half, Healing Word with the bonus action, Cunning Action's
//! Hide before the rogue's attack, then cast or attack with the action); and the whole budget
//! with Shield declared. The policy never exchanges rows nor runs, so no opportunity attack is
//! drawn; the column shows it.

use super::clear::{ONE_COMMAND, Play, clear_dungeon};
use super::common::data;
use super::{choose, hundredths, tenths};

/// Ten times the other tables' seeds: the plays differ by a few points, inside 300 seeds'
/// noise (about 2.4 points at a 21% wipe rate).
const SEEDS: u64 = 3000;
use omnis_data::{Data, FeatureEffect, SpellEffect};
use omnis_sim::omnis_rules::{combat_features, uses_left};
use omnis_sim::{
    ActorRef, CombatCommand, Command, FeatureChoice, Mode, Pay, Target, World, combat_view,
};

/// The whole budget for the member whose turn it is: free and bonus options first, then the
/// action, then the end of the turn.
pub(crate) fn budgeted(world: &World, data: &Data) -> Command {
    let Mode::Combat(state) = &world.mode else {
        unreachable!("a fight");
    };
    let view = combat_view(world, data).expect("a fight");
    let Some(ActorRef::Member(id)) = view.current else {
        unreachable!("the fight parks on a member");
    };
    let own = world.party.members.iter().position(|m| m.id == id).unwrap();
    let member = &world.party.members[own];
    let feature = |wanted: &dyn Fn(&FeatureEffect) -> bool| {
        combat_features(member, data)
            .iter()
            .position(|f| f.effect.as_ref().is_some_and(wanted) && uses_left(member, f) != Some(0))
            .map(|at| u8::try_from(at).unwrap())
    };
    let use_feature = |feature, choice| Command::Combat(CombatCommand::Feature { feature, choice });
    let budget = state.budget;
    if budget.actions > 0
        && view.round == 1
        && let Some(surge) = feature(&|e| matches!(e, FeatureEffect::ExtraAction))
    {
        return use_feature(surge, FeatureChoice::None);
    }
    if budget.bonus_actions > 0 {
        if member.hp * 2 < member.hp_max
            && let Some(wind) = feature(&|e| matches!(e, FeatureEffect::Heal { .. }))
        {
            return use_feature(wind, FeatureChoice::None);
        }
        let hurt = world
            .party
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.hp > 0 && m.hp * 2 < m.hp_max)
            .min_by_key(|(_, m)| m.hp)
            .map(|(i, _)| u8::try_from(i).unwrap());
        let word = view.spells.iter().find(|s| {
            let spell = &data.spells[&member.known_spells[usize::from(s.index)]];
            s.blocked.is_none()
                && spell.bonus_action_available
                && !spell.preparation_required_for_bonus_action
                && matches!(spell.effect, Some(SpellEffect::Heal { .. }))
                && !state
                    .spells_cast
                    .refuses(spell.level, omnis_data::Cost::BonusAction)
        });
        if let (Some(word), Some(hurt)) = (word, hurt) {
            return Command::Combat(CombatCommand::Cast {
                spell: word.index,
                target: Target::Member(hurt),
                pay: Pay::BonusAction,
            });
        }
        if budget.actions > 0
            && !state.hidden.contains(&id)
            && let Some(cunning) = feature(&|e| matches!(e, FeatureEffect::Cunning))
        {
            return use_feature(cunning, FeatureChoice::Hide);
        }
    }
    if budget.actions > 0 {
        return choose(world, data, state.spells_cast.bonus);
    }
    Command::Combat(CombatCommand::EndTurn)
}

#[test]
#[ignore = "the measurement harness; run with --nocapture to read the table"]
fn budget_over_seeds() {
    let data = data();
    let plays: [(&str, Play); 3] = [
        ("one", ONE_COMMAND),
        (
            "budget",
            Play {
                policy: budgeted,
                whole_turns: true,
                shield: false,
            },
        ),
        (
            "+shield",
            Play {
                policy: budgeted,
                whole_turns: true,
                shield: true,
            },
        ),
    ];
    println!(
        "One clear of the test dungeon by a new level-1 party over {SEEDS} seeds, three plays. \
         Per fight: rounds, shields fired, opportunity attacks, Second Wind, Action Surge, \
         Cunning Action and bonus-action spells."
    );
    println!(
        "{:>7} {:>7} {:>6} {:>7} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "members",
        "play",
        "wipe%",
        "rounds",
        "xp/mem",
        "level",
        "shield",
        "opp",
        "wind",
        "surge",
        "hide",
        "bonus"
    );
    for members in [2usize, 4, 6] {
        for (name, play) in plays {
            let (mut wipes, mut done, mut xp, mut levels) = (0u64, 0u64, 0u64, 0u64);
            let (mut fights, mut rounds, mut shields, mut opportunity) = (0u64, 0u64, 0u64, 0u64);
            let mut uses = [0u64; 4];
            for seed in 0..SEEDS {
                let (clear, world) = clear_dungeon(&data, seed, members, play);
                fights += u64::from(clear.fights);
                rounds += clear.rounds;
                shields += u64::from(clear.reactions);
                opportunity += u64::from(clear.opportunity);
                for (total, n) in uses.iter_mut().zip(clear.uses) {
                    *total += u64::from(n);
                }
                if clear.wiped {
                    wipes += 1;
                    continue;
                }
                done += 1;
                xp += world
                    .party
                    .members
                    .iter()
                    .map(|m| u64::from(m.xp))
                    .sum::<u64>();
                levels += world
                    .party
                    .members
                    .iter()
                    .map(|m| u64::from(m.level))
                    .sum::<u64>();
            }
            let (d, f) = (done.max(1), fights.max(1));
            let m = u64::try_from(members).unwrap();
            let per_fight = |n: u64| hundredths(n * 100 / f);
            println!(
                "{:>7} {:>7} {:>6} {:>7} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
                members,
                name,
                tenths(wipes * 1000 / SEEDS),
                per_fight(rounds),
                xp / (d * m),
                hundredths(levels * 100 / (d * m)),
                per_fight(shields),
                per_fight(opportunity),
                per_fight(uses[0]),
                per_fight(uses[1]),
                per_fight(uses[2]),
                per_fight(uses[3])
            );
        }
    }
}
