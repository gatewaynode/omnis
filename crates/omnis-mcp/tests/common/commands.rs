//! Every `Command` variant, nested variants included, as a chain of exhaustive matches: a new
//! variant fails the build here until it has a successor arm. The schema proof holds each
//! instance against the hand-written schema; the vocabulary test reads every key they carry.

use omnis_cli::omnis_sim::omnis_core::CharacterId;
use omnis_cli::omnis_sim::omnis_core::{Direction, Facing, Rotation};
use omnis_cli::omnis_sim::omnis_data::{Ability, Alignment, EquipSlot, Skill};
use omnis_cli::omnis_sim::omnis_rules::{
    ActionRef, Cmp, Criteria, CriteriaSet, Draft, Named, Predicate, Trigger, Who,
};
use omnis_cli::omnis_sim::rest::HitDiceSpend;
use omnis_cli::omnis_sim::tactics::TacticsCommand;
use omnis_cli::omnis_sim::{
    CombatCommand, Command, DevCommand, EncounterChoice, FeatureChoice, ItemCommand, PartyCommand,
    Pay, RestCommand, ServiceCommand, Target,
};

pub fn draft(alignment: Alignment) -> Draft {
    Draft {
        name: "Wren".into(),
        race: "base:race:human".into(),
        class: "base:class:cleric".into(),
        background: "base:background:acolyte".into(),
        alignment,
        scores: [15, 14, 13, 12, 10, 8],
        skills: Skill::ALL.to_vec(),
    }
}

/// The item after `current` in a type's `ALL` list.
fn after<T: Copy + PartialEq>(all: &[T], current: T) -> Option<T> {
    let at = all.iter().position(|one| *one == current)?;
    all.get(at + 1).copied()
}

fn next_party(command: &PartyCommand) -> Option<PartyCommand> {
    Some(match command {
        PartyCommand::Create(made) => match after(&Alignment::ALL, made.alignment) {
            Some(alignment) => PartyCommand::Create(draft(alignment)),
            None => PartyCommand::Reorder {
                order: vec![CharacterId(1), CharacterId(0)],
            },
        },
        PartyCommand::Reorder { .. } => PartyCommand::Tactics(TacticsCommand::SetReactions {
            member: CharacterId(0),
            on: false,
        }),
        PartyCommand::Tactics(command) => PartyCommand::Tactics(next_tactics(command)?),
    })
}

/// The switch, then a declared reaction for every trigger, cycling the four actions and the
/// two shapes of `when`, then a removal.
fn next_tactics(command: &TacticsCommand) -> Option<TacticsCommand> {
    let put = |i: usize| {
        let action = match i % 4 {
            0 => ActionRef::<Named>::Spell("base:spell:shield".into()),
            1 => ActionRef::Attack,
            2 => ActionRef::Item("base:item:potion_of_healing".into()),
            _ => ActionRef::Feature("base:text:class.fighter.second_wind".into()),
        };
        let when = if i.is_multiple_of(2) {
            Criteria::Always
        } else {
            Criteria::All(vec![Criteria::Is(Predicate::Hp {
                who: Who::Subject,
                cmp: Cmp::Lt,
                percent: 50,
            })])
        };
        TacticsCommand::PutReaction {
            member: CharacterId(1),
            entry: (i > 0).then(|| u8::try_from(i - 1).unwrap()),
            set: CriteriaSet {
                name: format!("Set {i}"),
                action,
                trigger: Trigger::ALL[i],
                when,
            },
        }
    };
    Some(match command {
        TacticsCommand::SetReactions { .. } => put(0),
        TacticsCommand::PutReaction { set, .. } => {
            let i = Trigger::ALL.iter().position(|t| *t == set.trigger)? + 1;
            if i < Trigger::ALL.len() {
                put(i)
            } else {
                TacticsCommand::RemoveReaction {
                    member: CharacterId(1),
                    entry: 0,
                }
            }
        }
        TacticsCommand::RemoveReaction { .. } => return None,
    })
}

fn next_combat(command: &CombatCommand) -> Option<CombatCommand> {
    let spell = || String::from("base:spell:magic_missile");
    let item = || String::from("base:item:potion_of_healing");
    Some(match command {
        CombatCommand::Attack { .. } => CombatCommand::Cast {
            spell: spell(),
            target: Target::Stack(0),
            pay: Pay::Action,
        },
        CombatCommand::Cast { target, .. } => match target {
            Target::Stack(_) => CombatCommand::Cast {
                spell: spell(),
                target: Target::Member(CharacterId(1)),
                pay: Pay::BonusAction,
            },
            Target::Member(_) => CombatCommand::Use {
                item: item(),
                receiver: Some(CharacterId(4)),
            },
        },
        CombatCommand::Use {
            receiver: Some(_), ..
        } => CombatCommand::Use {
            item: item(),
            receiver: None,
        },
        CombatCommand::Use { receiver: None, .. } => CombatCommand::Dodge,
        CombatCommand::Dodge => CombatCommand::Exchange {
            with: CharacterId(5),
        },
        CombatCommand::Exchange { .. } => CombatCommand::Run,
        CombatCommand::Run => CombatCommand::Feature {
            feature: "base:text:class.fighter.second_wind".into(),
            choice: FeatureChoice::None,
        },
        CombatCommand::Feature { choice, .. } => match choice {
            FeatureChoice::None => CombatCommand::Feature {
                feature: "base:text:class.rogue.cunning_action".into(),
                choice: FeatureChoice::Exchange {
                    with: CharacterId(2),
                },
            },
            FeatureChoice::Exchange { .. } => CombatCommand::Feature {
                feature: "base:text:class.rogue.cunning_action".into(),
                choice: FeatureChoice::Hide,
            },
            FeatureChoice::Hide => CombatCommand::EndTurn,
        },
        CombatCommand::EndTurn => return None,
    })
}

fn next_slot(slot: EquipSlot) -> Option<EquipSlot> {
    match slot {
        EquipSlot::MainHand => Some(EquipSlot::OffHand),
        EquipSlot::OffHand => Some(EquipSlot::Ranged),
        EquipSlot::Ranged => Some(EquipSlot::Body),
        EquipSlot::Body => None,
    }
}

fn next_item(command: &ItemCommand) -> Option<ItemCommand> {
    let (member, count) = (CharacterId(0), 2);
    let item = String::from("base:item:dagger");
    Some(match command {
        ItemCommand::Equip { .. } => ItemCommand::Unequip {
            member,
            slot: EquipSlot::MainHand,
        },
        ItemCommand::Unequip { slot, .. } => match next_slot(*slot) {
            Some(slot) => ItemCommand::Unequip { member, slot },
            None => ItemCommand::Give {
                giver: CharacterId(0),
                receiver: CharacterId(1),
                item,
                count,
            },
        },
        ItemCommand::Give { .. } => ItemCommand::Stow {
            member,
            item,
            count,
        },
        ItemCommand::Stow { .. } => ItemCommand::Take {
            member,
            item,
            count,
        },
        ItemCommand::Take { .. } => ItemCommand::Use {
            member,
            item,
            receiver: Some(CharacterId(1)),
        },
        ItemCommand::Use {
            receiver: Some(_), ..
        } => ItemCommand::Use {
            member,
            item,
            receiver: None,
        },
        ItemCommand::Use { receiver: None, .. } => return None,
    })
}

fn next_facing(facing: Facing) -> Option<Facing> {
    match facing {
        Facing::North => Some(Facing::East),
        Facing::East => Some(Facing::South),
        Facing::South => Some(Facing::West),
        Facing::West => None,
    }
}

fn teleport(facing: Facing) -> DevCommand {
    DevCommand::Teleport {
        map: "test:map:dungeon".into(),
        x: 3,
        y: 4,
        facing,
    }
}

fn set_score(ability: Ability) -> DevCommand {
    DevCommand::SetScore {
        member: CharacterId(0),
        ability,
        score: 18,
    }
}

fn give(member: Option<CharacterId>) -> DevCommand {
    DevCommand::GiveItem {
        member,
        item: "base:item:gem".into(),
        count: 2,
    }
}

fn next_dev(command: &DevCommand) -> Option<DevCommand> {
    let member = CharacterId(0);
    Some(match command {
        DevCommand::GiveItem {
            member: Some(_), ..
        } => give(None),
        DevCommand::GiveItem { member: None, .. } => DevCommand::SetHp { member, hp: -1 },
        DevCommand::SetHp { .. } => DevCommand::SetSpellPoints { member, points: 4 },
        DevCommand::SetSpellPoints { .. } => DevCommand::SetGold { gold: 50 },
        DevCommand::SetGold { .. } => DevCommand::SetFood { food: 7 },
        DevCommand::SetFood { .. } => DevCommand::SetXp { member, xp: 300 },
        DevCommand::SetXp { .. } => set_score(Ability::ALL[0]),
        DevCommand::SetScore { ability, .. } => match after(&Ability::ALL, *ability) {
            Some(ability) => set_score(ability),
            None => DevCommand::SetCondition {
                member,
                condition: "base:condition:poisoned".into(),
                applied: true,
            },
        },
        DevCommand::SetCondition { .. } => DevCommand::SetFlag {
            flag: "test:flag:door".into(),
            value: -3,
        },
        DevCommand::SetFlag { .. } => teleport(Facing::North),
        DevCommand::Teleport { facing, .. } => match next_facing(*facing) {
            Some(facing) => teleport(facing),
            None => DevCommand::SetMonsterHp {
                stack: 0,
                index: 1,
                hp: 0,
            },
        },
        DevCommand::SetMonsterHp { .. } => DevCommand::KillStack { stack: 1 },
        DevCommand::KillStack { .. } => DevCommand::Reconcile {
            region: "test:region:town".into(),
        },
        DevCommand::Reconcile { .. } => return None,
    })
}

fn next_step(direction: Direction) -> Command {
    match direction {
        Direction::Forward => Command::Step(Direction::Back),
        Direction::Back => Command::Step(Direction::Left),
        Direction::Left => Command::Step(Direction::Right),
        Direction::Right => Command::Turn(Rotation::Left),
    }
}

fn next_turn(rotation: Rotation) -> Command {
    match rotation {
        Rotation::Left => Command::Turn(Rotation::Right),
        Rotation::Right => Command::Turn(Rotation::Around),
        Rotation::Around => Command::Interact,
    }
}

fn next_encounter(choice: EncounterChoice) -> Command {
    match choice {
        EncounterChoice::Attack => Command::Encounter(EncounterChoice::Bribe),
        EncounterChoice::Bribe => Command::Encounter(EncounterChoice::Hide),
        EncounterChoice::Hide => Command::Encounter(EncounterChoice::Run),
        EncounterChoice::Run => Command::Combat(CombatCommand::Attack { stack: 0 }),
    }
}

fn next_service(command: &ServiceCommand) -> Option<ServiceCommand> {
    let (member, count, amount) = (CharacterId(1), 3, 250);
    let item = || String::from("base:item:dagger");
    let spell = || String::from("base:spell:bless");
    Some(match command {
        ServiceCommand::Leave => ServiceCommand::Room,
        ServiceCommand::Room => ServiceCommand::Rumor,
        ServiceCommand::Rumor => ServiceCommand::BuyFood { count },
        ServiceCommand::BuyFood { .. } => ServiceCommand::Heal { member },
        ServiceCommand::Heal { .. } => ServiceCommand::Cure { member },
        ServiceCommand::Cure { .. } => ServiceCommand::Raise { member },
        ServiceCommand::Raise { .. } => ServiceCommand::Buy {
            item: item(),
            count,
        },
        ServiceCommand::Buy { .. } => ServiceCommand::Sell {
            item: item(),
            count,
        },
        ServiceCommand::Sell { .. } => ServiceCommand::Deposit { amount },
        ServiceCommand::Deposit { .. } => ServiceCommand::Withdraw { amount },
        ServiceCommand::Withdraw { .. } => ServiceCommand::Train { member },
        ServiceCommand::Train { .. } => ServiceCommand::Choose {
            member,
            spell: spell(),
        },
        ServiceCommand::Choose { .. } => ServiceCommand::Learn {
            member,
            spell: spell(),
        },
        ServiceCommand::Learn { .. } => return None,
    })
}

/// The instance after `command`, from `Step(Forward)` to the last `Dev` edit. Every match here
/// and in the helpers is exhaustive: a new variant gets an arm, and a link from its neighbour.
fn next(command: &Command) -> Option<Command> {
    let cast = |target| Command::Cast {
        caster: CharacterId(0),
        spell: "base:spell:cure_wounds".into(),
        target,
    };
    Some(match command {
        Command::Step(direction) => next_step(*direction),
        Command::Turn(rotation) => next_turn(*rotation),
        Command::Interact => Command::Party(PartyCommand::Create(draft(Alignment::ALL[0]))),
        Command::Party(party) => {
            next_party(party).map_or(Command::Encounter(EncounterChoice::Attack), Command::Party)
        }
        Command::Encounter(choice) => next_encounter(*choice),
        Command::Combat(combat) => {
            next_combat(combat).map_or(cast(Target::Stack(2)), Command::Combat)
        }
        Command::Cast { target, .. } => match target {
            Target::Stack(_) => cast(Target::Member(CharacterId(3))),
            Target::Member(_) => Command::Item(ItemCommand::Equip {
                member: CharacterId(0),
                item: "base:item:dagger".into(),
            }),
        },
        Command::Item(item) => {
            next_item(item).map_or(Command::Service(ServiceCommand::Leave), Command::Item)
        }
        Command::Service(service) => next_service(service).map_or(
            Command::Rest(RestCommand::Short {
                spend: vec![
                    HitDiceSpend {
                        member: CharacterId(0),
                        count: 1,
                    },
                    HitDiceSpend {
                        member: CharacterId(2),
                        count: 2,
                    },
                ],
            }),
            Command::Service,
        ),
        Command::Rest(RestCommand::Short { .. }) => Command::Rest(RestCommand::Long),
        Command::Rest(RestCommand::Long) => Command::Dev(give(Some(CharacterId(0)))),
        Command::Dev(dev) => return next_dev(dev).map(Command::Dev),
    })
}

/// Every variant once at least, from `Step(Forward)` along [`next`].
pub fn instances() -> Vec<Command> {
    let mut all = vec![Command::Step(Direction::Forward)];
    while let Some(following) = next(&all[all.len() - 1]) {
        all.push(following);
    }
    all
}
