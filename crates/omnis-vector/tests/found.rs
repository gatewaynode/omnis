//! What an object gives is taken through the loot window (owner, 2026-11-13): the engine lays
//! it on the ground and the window opens on it at once, so a found item is not left behind
//! for want of looking. On the yard fixture: the loose stone at (0, 2) hides a gem.

use crate::common;
use crate::loot::yard_app;

use bevy::prelude::*;
use common::app::{press_ui, tap, top};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Verb;
use omnis_sim::{Command, ItemCommand};
use omnis_vector::screen::loot::LootAct;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::ViewState;
use omnis_vector::shell::controls::Control;
use omnis_vector::shell::loot::PickUp;
use omnis_vector::shell::screen::Pressable;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Session;

const GEM: &str = "base:item:gem";

fn take_the_stone() -> Command {
    Command::Act {
        object: "stone_1".into(),
        verb: Verb::Take,
        option: 0,
    }
}

fn order(app: &mut App, command: Command) {
    app.world_mut().resource_mut::<Session>().order(command);
    app.update();
    app.update();
}

fn carried(app: &App, item: &str) -> u16 {
    let s = app.world().resource::<Session>();
    s.views
        .party
        .members
        .iter()
        .flat_map(|m| &m.equipment)
        .filter(|i| i.item == item)
        .map(|i| i.count)
        .sum()
}

fn on_the_ground(app: &App) -> Vec<(String, u16)> {
    let s = app.world().resource::<Session>();
    let ground = &s.views.objects.ground;
    ground.iter().map(|i| (i.item.clone(), i.count)).collect()
}

fn pick_up_shown(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Visibility, Has<Control>), With<PickUp>>();
    let (visibility, live) = q.single(world).expect("the button");
    *visibility != Visibility::Hidden && live
}

#[test]
fn a_found_item_opens_the_loot_window_and_is_taken_or_left_as_a_heap() {
    let mut app = yard_app(0, 2, Facing::East);
    assert_eq!(top(&app), None);
    order(&mut app, take_the_stone());
    assert_eq!(top(&app), Some(ScreenId::Loot), "opened by the find");
    let text = screen_text(app.world());
    for word in ["On the ground", "Gem", "Take everything"] {
        assert!(text.contains(word), "{word}: {text}");
    }
    assert_eq!(on_the_ground(&app), [(GEM.to_owned(), 1)]);
    assert_eq!(carried(&app, GEM), 0, "nothing was taken for the party");

    // Closed without taking: it opens no second time, the gem lies as a pile, and the
    // button offers it.
    tap(&mut app, KeyCode::Escape);
    app.update();
    app.update();
    assert_eq!(top(&app), None, "closed, and not opened again");
    assert_eq!(on_the_ground(&app), [(GEM.to_owned(), 1)]);
    assert!(pick_up_shown(&mut app));
    press_ui(&mut app, PickUp);
    assert_eq!(top(&app), Some(ScreenId::Loot));
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Everything)));
    app.update();
    assert_eq!(top(&app), None, "emptied: the window closed itself");
    assert_eq!(carried(&app, GEM), 1);
    assert_eq!(app.world().resource::<Session>().binder.refusals, 0);
}

#[test]
fn only_a_find_opens_the_window_not_a_drop_or_a_pick_up() {
    let mut app = yard_app(1, 2, Facing::East);
    let member = app.world().resource::<Session>().views.party.members[0].member;
    order(
        &mut app,
        Command::Item(ItemCommand::PickUp {
            member,
            item: "base:item:arrows".into(),
            count: 2,
        }),
    );
    assert_eq!(top(&app), None, "a pick-up is not a find");
    order(
        &mut app,
        Command::Item(ItemCommand::Drop {
            member,
            item: "base:item:arrows".into(),
            count: 1,
        }),
    );
    assert_eq!(top(&app), None, "nor is a drop");
}

#[test]
fn a_find_made_from_the_actions_menu_opens_the_window_in_its_place() {
    let mut app = yard_app(0, 2, Facing::East);
    // The stone has one verb, so the actions menu takes it directly and closes.
    tap(&mut app, KeyCode::Space);
    assert_eq!(top(&app), Some(ScreenId::Actions));
    let text = screen_text(app.world());
    assert!(text.contains("\"1  Use\""), "{text}");
    press_ui(&mut app, Pressable(ControlId::Action(0)));
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Loot), "the find's window");
    assert!(screen_text(app.world()).contains("Gem"));
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Everything)));
    app.update();
    assert_eq!(carried(&app, GEM), 1);
    assert_eq!(top(&app), None, "back on the view");
}

/// Fight whatever the party has met to its end with the fight's own commands: attack the
/// first stack with anyone alive in it, and end the turn when that is refused.
fn fight_it_out(app: &mut App) {
    for _ in 0..400 {
        let command = {
            let s = app.world().resource::<Session>();
            match &s.views.combat {
                None => return,
                Some(c) if c.phase == omnis_sim::api::ModeKind::Encounter => {
                    Command::Encounter(omnis_sim::EncounterChoice::Attack)
                }
                Some(c) => {
                    let stack = c
                        .stacks
                        .iter()
                        .find(|s| !s.hps.is_empty())
                        .expect("something to hit")
                        .stack;
                    Command::Combat(omnis_sim::CombatCommand::Attack { stack })
                }
            }
        };
        let mut session = app.world_mut().resource_mut::<Session>();
        if !session.order(command) {
            session.order(Command::Combat(omnis_sim::CombatCommand::EndTurn));
        }
    }
    panic!("the fight did not end");
}

fn view_state(app: &App) -> ViewState {
    *app.world().resource::<State<ViewState>>().get()
}

#[test]
fn the_spoils_of_a_won_fight_open_the_window_once_the_view_is_the_map_s() {
    // South of the pack rats' cell (5, 1), stepping into them.
    let mut app = yard_app(5, 2, Facing::North);
    order(
        &mut app,
        Command::Step(omnis_sim::omnis_core::Direction::Forward),
    );
    assert!(
        app.world().resource::<Session>().views.combat.is_some(),
        "the rats are met"
    );
    app.update();
    assert_eq!(view_state(&app), ViewState::Fight);
    fight_it_out(&mut app);
    {
        let s = app.world().resource::<Session>();
        assert!(s.views.exploring() && !s.views.fallen(), "won");
        assert!(
            s.lines.iter().any(|l| l.contains("on the ground: ")),
            "this seed's rats dropped something: {:?}",
            s.lines
        );
    }
    let spoils = on_the_ground(&app);
    assert!(!spoils.is_empty());
    // The frame the fight ends its screen is still up: nothing opens over it.
    app.update();
    assert_eq!(view_state(&app), ViewState::Fight);
    assert_eq!(top(&app), None, "not over the fight's screen");
    app.update();
    assert_eq!(view_state(&app), ViewState::Explore);
    assert_eq!(top(&app), Some(ScreenId::Loot), "the spoils' window");
    // Drawn the frame after it opens.
    app.update();
    let text = screen_text(app.world());
    assert!(text.contains("On the ground"), "{text}");
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Everything)));
    app.update();
    assert_eq!(on_the_ground(&app), []);
    assert_eq!(top(&app), None);
    for (item, count) in spoils {
        assert_eq!(carried(&app, &item), count, "{item}");
    }
}

#[test]
fn only_a_victory_that_left_something_is_a_find() {
    use omnis_sim::event::{CombatOutcome, Loot};
    use omnis_vector::shell::session::finds;
    let ended = |outcome, loot: Vec<Loot>| omnis_sim::Event::CombatEnded {
        outcome,
        xp: 10,
        gold: 5,
        fallen: Vec::new(),
        loot,
    };
    let gem = || {
        vec![Loot {
            item: GEM.into(),
            count: 1,
        }]
    };
    assert!(finds(&ended(CombatOutcome::Victory, gem())));
    assert!(
        !finds(&ended(CombatOutcome::Victory, Vec::new())),
        "nothing dropped"
    );
    assert!(!finds(&ended(CombatOutcome::Fled, gem())), "left behind");
    assert!(!finds(&ended(CombatOutcome::Defeat, gem())));
}

#[test]
fn a_won_fight_that_left_nothing_opens_nothing() {
    // The test dungeon's giant rats carry no loot.
    let mut app = common::app::met_with(1);
    fight_it_out(&mut app);
    {
        let s = app.world().resource::<Session>();
        assert!(s.views.exploring() && !s.views.fallen(), "won");
        assert!(
            s.lines.iter().any(|l| l.contains("Victory!")),
            "{:?}",
            s.lines
        );
    }
    assert_eq!(on_the_ground(&app), []);
    for _ in 0..3 {
        app.update();
        assert_eq!(top(&app), None);
    }
    assert_eq!(view_state(&app), ViewState::Explore);
}

/// Force the yard's chest until it gives (Athletics against 15), from the cell south of it.
fn force_the_chest(app: &mut App) {
    for _ in 0..80 {
        order(
            app,
            Command::Act {
                object: "chest_1".into(),
                verb: Verb::Open,
                option: 0,
            },
        );
        if chest(app).is_some() {
            return;
        }
        assert_eq!(top(app), None, "a failed try opens nothing");
    }
    panic!("the chest never gave");
}

/// What the chest shows of its contents: `None` while shut.
fn chest(app: &App) -> Option<Vec<(String, u16)>> {
    let s = app.world().resource::<Session>();
    let objects = &s.views.objects.objects;
    let chest = objects.iter().find(|o| o.object == "chest_1")?;
    let contents = chest.contents.as_ref()?;
    Some(contents.iter().map(|i| (i.item.clone(), i.count)).collect())
}

fn act(app: &mut App, object: &str, verb: Verb) {
    order(
        app,
        Command::Act {
            object: object.into(),
            verb,
            option: 0,
        },
    );
}

/// The Actions menu's entries, as words.
fn actions(app: &App) -> Vec<String> {
    let s = app.world().resource::<Session>();
    omnis_vector::screen::actions::available(&s.world, &s.data)
        .into_iter()
        .map(|o| o.label)
        .collect()
}

const DAGGER: &str = "base:item:dagger";

#[test]
fn an_opened_container_opens_the_window_on_it_and_actions_lead_back() {
    let mut app = yard_app(5, 4, Facing::North);
    assert_eq!(chest(&app), None, "shut");
    assert!(!actions(&app).iter().any(|a| a.starts_with("Take from")));
    force_the_chest(&mut app);
    assert_eq!(chest(&app), Some(vec![(DAGGER.to_owned(), 1)]));
    assert_eq!(top(&app), Some(ScreenId::Loot), "opened with the chest");
    app.update();
    let text = screen_text(app.world());
    assert!(text.contains("\"Chest\""), "the chest's own window: {text}");
    assert!(!text.contains("On the ground"), "{text}");
    assert!(text.contains("Dagger"), "{text}");

    // Closed without taking: it does not come back by itself, and Actions lead back to it.
    tap(&mut app, KeyCode::Escape);
    app.update();
    app.update();
    assert_eq!(top(&app), None);
    let offered = actions(&app);
    let entry = offered
        .iter()
        .position(|a| a == "Take from Chest...")
        .unwrap_or_else(|| panic!("{offered:?}"));
    tap(&mut app, KeyCode::Space);
    assert_eq!(top(&app), Some(ScreenId::Actions));
    press_ui(
        &mut app,
        Pressable(ControlId::Action(u8::try_from(entry).unwrap())),
    );
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Loot));
    let text = screen_text(app.world());
    assert!(
        text.contains("\"Chest\"") && text.contains("Dagger"),
        "{text}"
    );
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Take(0))));
    app.update();
    app.update();
    assert_eq!(carried(&app, DAGGER), 1 + carried_at_start(), "taken out");
    assert_eq!(chest(&app), Some(vec![]), "open and empty");
    assert_ne!(top(&app), Some(ScreenId::Loot), "emptied: closed itself");
    assert!(
        !actions(&app).iter().any(|a| a.starts_with("Take from")),
        "nothing to take from an empty chest"
    );
    assert_eq!(app.world().resource::<Session>().binder.refusals, 0);
}

/// Daggers the yard party carries before taking any.
fn carried_at_start() -> u16 {
    let app = yard_app(5, 4, Facing::North);
    carried(&app, DAGGER)
}

#[test]
fn only_a_container_with_something_in_it_opens_the_window() {
    // A torch's new state is no container's.
    let mut app = yard_app(6, 3, Facing::South);
    order(&mut app, Command::Door);
    act(&mut app, "torch_1", Verb::Extinguish);
    assert_eq!(top(&app), None, "a torch put out");

    // The chest opened, emptied through the inventory's own command, shut and opened again.
    let mut app = yard_app(5, 4, Facing::North);
    force_the_chest(&mut app);
    tap(&mut app, KeyCode::Escape);
    app.update();
    let member = app.world().resource::<Session>().views.party.members[0].member;
    order(
        &mut app,
        Command::Item(ItemCommand::TakeOut {
            member,
            object: "chest_1".into(),
            item: DAGGER.into(),
            count: 1,
        }),
    );
    assert_eq!(top(&app), None, "a take is not an opening");
    act(&mut app, "chest_1", Verb::Close);
    assert_eq!(chest(&app), None);
    assert_eq!(top(&app), None, "shutting opens nothing");
    force_the_chest(&mut app);
    assert_eq!(chest(&app), Some(vec![]));
    app.update();
    assert_eq!(top(&app), None, "an empty chest opens nothing");
    assert_eq!(
        opened(&app, "chest_1"),
        None,
        "open and empty: no offer made"
    );
}

/// What a new state of `object` would offer now, by the session's own rule.
fn opened(app: &App, object: &str) -> Option<omnis_sim::ItemPlace> {
    let s = app.world().resource::<Session>();
    let event = omnis_sim::Event::ObjectState {
        object: object.into(),
        state: "any".into(),
    };
    omnis_vector::shell::session::opens(&event, &s.views)
}

#[test]
fn a_new_state_offers_its_object_only_when_it_shows_contents() {
    // The window closes itself on a place with nothing to show, so the offer is asked directly:
    // none is made for a torch, a shut chest or an object out of reach.
    let mut app = yard_app(6, 3, Facing::South);
    order(&mut app, Command::Door);
    assert_eq!(opened(&app, "torch_1"), None, "no container");
    assert_eq!(opened(&app, "chest_1"), None, "not in reach from here");
    let mut app = yard_app(5, 4, Facing::North);
    assert_eq!(opened(&app, "chest_1"), None, "shut");
    force_the_chest(&mut app);
    assert_eq!(
        opened(&app, "chest_1"),
        Some(omnis_sim::ItemPlace::Object("chest_1".into()))
    );
    assert_eq!(opened(&app, "nothing"), None);
}
