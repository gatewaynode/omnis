//! Town services (parity P4): the service view the shell reads, the words each offer is shown
//! with, and the HUD lines for what the party does inside. Real packs, the fixed party, the
//! test pack's town.

use crate::common;
use common::ctx;

use bevy::input::ButtonState;
use bevy::input::touch::TouchPhase;
use bevy::prelude::*;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_rules::Gains;
use omnis_sim::{Command, Event, Mode, Rejection, ServiceCommand};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::screen::nav::{Escape, Nav};
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, Screen, ScreenId};
use omnis_vector::shell::screen::{Pressable, Screens, Scrolls, ctx as shell_ctx};
use omnis_vector::shell::session::Session;
use omnis_vector::shell::text;
use omnis_vector::sim::{SimWorld, Views};
use omnis_vector::style::Role;

/// The fixed party inside the service whose site tile is `(x, y)` in the town.
fn inside(data: &Data, (x, y): (u16, u16)) -> SimWorld {
    let (mut world, _) = common::town_session(data);
    let town = common::map(data, "test:map:town");
    common::place(&mut world, town, x, y, Facing::North);
    world
        .apply(data, Command::Interact)
        .expect("a site tile opens its service");
    world
}

const SMITH: (u16, u16) = (1, 3);
const TEMPLE: (u16, u16) = (4, 1);
const TRAINER: (u16, u16) = (7, 1);
const TAVERN: (u16, u16) = (4, 3);

/// Every offer's row label, caption, note and reason, in the view's order.
fn rows(data: &Data, views: &Views) -> Vec<(String, &'static str, String, Option<String>)> {
    let service = views.service.as_ref().expect("inside a service");
    service
        .offers
        .iter()
        .map(|o| {
            (
                labels::offer_label(data, &views.party, o),
                labels::offer_caption(&o.command),
                labels::offer_note(o),
                o.refusal.as_ref().map(labels::service_reason),
            )
        })
        .collect()
}

#[test]
fn the_views_hold_the_service_inside_and_none_outside() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    assert_eq!(common::views(&world, &data).service, None);
    let views = common::views(&inside(&data, SMITH), &data);
    let service = views.service.expect("the smith's view");
    assert_eq!(service.service, "base:service:smith");
    assert_eq!(
        service.offers.last().map(|o| &o.command),
        Some(&ServiceCommand::Leave)
    );
}

/// A purchase the party cannot pay keeps its price beside the reason, and a sale names the
/// stores' count (the other client's `service_panel` words).
#[test]
fn the_smith_s_rows_name_the_item_its_price_and_why_not() {
    let data = common::data();
    let mut world = inside(&data, SMITH);
    world.party.gold = 0;
    let views = common::views(&world, &data);
    let dagger = views.service.as_ref().unwrap().offers[0].clone();
    let (label, caption, note, why) = rows(&data, &views).remove(0);
    assert_eq!((label.as_str(), caption), ("Dagger", "Buy"));
    assert_eq!(note, labels::coins(dagger.price.expect("a price")));
    assert_eq!(why.as_deref(), Some("not enough money"));

    world.party.gold = 10_000;
    for _ in 0..2 {
        let buy = ServiceCommand::Buy {
            item: "base:item:dagger".to_owned(),
            count: 1,
        };
        world.apply(&data, Command::Service(buy)).expect("bought");
    }
    let views = common::views(&world, &data);
    let held = views
        .party
        .inventory
        .iter()
        .find(|i| i.item == "base:item:dagger")
        .map_or(0, |i| i.count);
    assert!(held >= 2, "two daggers bought");
    let sell = rows(&data, &views)
        .into_iter()
        .find(|(label, caption, ..)| *caption == "Sell" && label.starts_with("Dagger"))
        .expect("the daggers can be sold");
    assert_eq!(sell.0, format!("Dagger x{held}"));
    assert!(sell.2.starts_with("pays "), "{}", sell.2);
    assert_eq!(sell.3, None);
}

#[test]
fn the_temple_names_what_it_treats_and_the_trainer_the_next_level() {
    let data = common::data();
    let mut world = inside(&data, TEMPLE);
    world.party.members[0].hp = 3;
    let views = common::views(&world, &data);
    let brenna: Vec<_> = rows(&data, &views)
        .into_iter()
        .filter(|(label, ..)| label.starts_with("Brenna"))
        .collect();
    let max = views.party.members[0].hp_max;
    assert_eq!(brenna[0].0, format!("Brenna, 3 of {max} HP"));
    assert_eq!(brenna[0].1, "Heal");
    assert_eq!(
        (brenna[1].0.as_str(), brenna[1].3.as_deref()),
        ("Brenna, no conditions", Some("nothing to treat"))
    );
    assert_eq!(
        (brenna[2].0.as_str(), brenna[2].3.as_deref()),
        ("Brenna, alive", Some("not dead"))
    );

    let views = common::views(&inside(&data, TRAINER), &data);
    let (label, caption, _, why) = rows(&data, &views).remove(0);
    assert_eq!((label.as_str(), caption), ("Brenna, level 1 to 2", "Train"));
    assert_eq!(
        why.as_deref(),
        Some("0 of 300 XP"),
        "the row names the member"
    );
    assert_eq!(
        labels::service_reason(&Rejection::SpellTooHigh { level: 2, max: 1 }),
        "level 2 spell, up to level 1 now"
    );
}

#[test]
fn a_rest_refused_too_soon_says_how_long_to_wait() {
    assert_eq!(
        labels::service_reason(&Rejection::RestTooSoon { minutes: 65 }),
        "rested too recently (1h 05m)"
    );
}

/// The HUD's lines for a tavern visit, from the door to the price (the other client's
/// `a_night_at_the_inn_reads_from_the_door_to_the_price`).
#[test]
fn a_tavern_visit_reads_from_the_door_to_the_rumor() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    world.party.gold = 1537;
    let town = common::map(&data, "test:map:town");
    common::place(&mut world, town, TAVERN.0, TAVERN.1, Facing::North);
    let mut events = world.apply(&data, Command::Interact).unwrap();
    for command in [
        ServiceCommand::BuyFood { count: 3 },
        ServiceCommand::Rumor,
        ServiceCommand::Leave,
    ] {
        events.extend(world.apply(&data, Command::Service(command)).unwrap());
    }
    let names = Names::default();
    let lines: Vec<String> = events
        .iter()
        .filter_map(|e| text::describe(e, &names, &data))
        .collect();
    assert_eq!(
        lines,
        [
            "The party enters the Tavern",
            "Bought 3 food for 1 gp 5 sp 0 cp",
            "\"Talk from today: the rats below grow bolder every week.\"",
            "The party leaves the Tavern",
        ]
    );
}

#[test]
fn a_level_a_spell_and_the_bank_read_with_names_and_coins() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let mut names = Names::default();
    names.observe(&common::views(&world, &data).party);
    let (brenna, ilvara) = (common::id(&world, 0), common::id(&world, 2));
    let line = |event: Event| text::describe(&event, &names, &data).expect("a town line");
    assert_eq!(
        line(Event::LevelUp {
            member: brenna,
            level: 2,
            cost: 2000,
            gains: Gains {
                hp: 8,
                spell_points: 0,
                picks: 0,
                proficiency: 2,
                features: vec!["base:text:class.fighter.action_surge".to_owned()],
            },
        }),
        "Brenna reaches level 2: +8 HP, Action Surge"
    );
    let shatter = "base:spell:shatter".to_owned();
    assert_eq!(
        line(Event::SpellLearned {
            member: ilvara,
            spell: shatter.clone(),
            cost: 0,
        }),
        "Ilvara chooses Shatter"
    );
    assert_eq!(
        line(Event::SpellLearned {
            member: ilvara,
            spell: shatter,
            cost: 10_000,
        }),
        "Ilvara learns Shatter for 100 gp 0 sp 0 cp"
    );
    assert_eq!(
        line(Event::Banked {
            amount: 250,
            deposit: true,
        }),
        "Deposited 2 gp 5 sp 0 cp"
    );
    assert_eq!(
        line(Event::Banked {
            amount: 100,
            deposit: false,
        }),
        "Withdrew 1 gp 0 sp 0 cp"
    );
}

// The service screen (P4b): the screen model, then the shell.

const INN: (u16, u16) = (1, 1);
const GUILD: (u16, u16) = (10, 1);
const BANK: (u16, u16) = (7, 3);

/// Each panel's headings: the spans drawn in the title role.
fn headings(shown: &Screen) -> Vec<Vec<String>> {
    shown
        .panels
        .iter()
        .map(|p| {
            p.rows
                .iter()
                .flat_map(|r| &r.spans)
                .filter(|s| s.role == Role::Title)
                .map(|s| s.text.clone())
                .collect()
        })
        .collect()
}

#[test]
fn each_service_opens_with_its_money_leave_and_lists_in_the_other_client_s_order() {
    let data = common::data();
    // Each site and the headings of each of its panels.
    type Lists = &'static [&'static [&'static str]];
    let expected: [((u16, u16), Lists); 7] = [
        (INN, &[&["On offer"]]),
        (TEMPLE, &[&["On offer"], &["Spells"]]),
        (TRAINER, &[&["Levels"], &["Spell picks"]]),
        (GUILD, &[&["Spells"]]),
        (SMITH, &[&["For sale"], &["Your stores"]]),
        (TAVERN, &[&["On offer"]]),
        (BANK, &[&[]]),
    ];
    for (site, lists) in expected {
        let world = inside(&data, site);
        let views = common::views(&world, &data);
        let shown = screen::content(
            ScreenId::Service,
            &ctx(&data, &world, &views),
            &Forms::default(),
        );
        let view = views.service.as_ref().expect("inside");
        assert_eq!(shown.title.text, labels::service(&data, &view.service));
        assert_eq!(headings(&shown), lists, "{}", view.service);
        let top = &shown.panels[0].rows[0];
        assert_eq!(
            top.spans[0].text,
            format!(
                "Gold {}  Bank {}  Food {}",
                labels::coins(view.gold),
                labels::coins(view.bank),
                view.food
            )
        );
        assert_eq!(top.controls[0].id, ControlId::Leave);
    }
}

/// Every offer but leaving is a button that sends exactly the view's command, and the engine
/// takes every live one; a refused one is dim with the other client's reason (the other
/// client's `every_offer_sends_what_the_view_promised`).
#[test]
fn every_offer_sends_what_the_view_promised_and_nothing_it_refuses() {
    let data = common::data();
    let (mut live, mut dim) = (0, 0);
    for site in [INN, TEMPLE, TRAINER, GUILD, SMITH, TAVERN] {
        let mut world = inside(&data, site);
        world.party.gold = 1_000_000;
        world.party.members[0].hp = 1;
        let views = common::views(&world, &data);
        let ctx = ctx(&data, &world, &views);
        let shown = screen::content(ScreenId::Service, &ctx, &Forms::default());
        let offers = &views.service.as_ref().expect("inside").offers;
        let ids: Vec<ControlId> = shown
            .controls()
            .map(|c| c.id)
            .filter(|id| matches!(id, ControlId::Offer(_)))
            .collect();
        let all: Vec<ControlId> = (0..offers.len() - 1)
            .map(|i| ControlId::Offer(u16::try_from(i).unwrap()))
            .collect();
        let mut sorted = ids.clone();
        sorted.sort_by_key(|id| match id {
            ControlId::Offer(i) => *i,
            _ => u16::MAX,
        });
        assert_eq!(sorted, all, "every offer but leaving, once");
        for (i, offer) in offers.iter().enumerate().take(offers.len() - 1) {
            let id = ControlId::Offer(u16::try_from(i).unwrap());
            let control = shown.control(id).expect("the offer's button");
            assert_eq!(control.label, labels::offer_caption(&offer.command));
            let used = screen::apply(
                ScreenId::Service,
                &ctx,
                &mut Forms::default(),
                id,
                &Payload::Press,
                0,
            );
            match &offer.refusal {
                None => {
                    live += 1;
                    let command = Command::Service(offer.command.clone());
                    assert_eq!(
                        used,
                        Ok(Effect::Command {
                            command: command.clone(),
                            close: false
                        })
                    );
                    assert!(world.trial(&data, &command).is_ok(), "{command:?} is taken");
                }
                Some(refusal) => {
                    dim += 1;
                    let why = labels::service_reason(refusal);
                    assert_eq!(control.why_not.as_deref(), Some(why.as_str()));
                    assert_eq!(used, Err(Refused::Dim(why)));
                }
            }
        }
    }
    assert!(live > 10 && dim > 0, "{live} live, {dim} dim");
}

/// A purchase the party cannot pay is dim with its reason and keeps its price beside it; a
/// list with nothing in it says so.
#[test]
fn a_refused_purchase_keeps_its_price_and_an_empty_list_says_so() {
    let data = common::data();
    let mut world = inside(&data, SMITH);
    world.party.gold = 0;
    let views = common::views(&world, &data);
    let shown = screen::content(
        ScreenId::Service,
        &ctx(&data, &world, &views),
        &Forms::default(),
    );
    let first = &views.service.as_ref().unwrap().offers[0];
    let row = &shown.panels[0].rows[2];
    assert_eq!(row.controls[0].why_not.as_deref(), Some("not enough money"));
    assert_eq!(row.after[0].text, labels::coins(first.price.unwrap()));

    // Empty stores: nothing to sell.
    world.party.inventory.clear();
    let views = common::views(&world, &data);
    let shown = screen::content(
        ScreenId::Service,
        &ctx(&data, &world, &views),
        &Forms::default(),
    );
    let stores = &shown.panels[1].rows;
    assert_eq!(stores.len(), 2, "{stores:?}");
    assert_eq!(stores[1].spans[0].text, "Nothing to offer");
    assert_eq!(stores[1].spans[0].role, Role::Dim);
}

/// The bank: dim until an amount is typed, digits only and no more than its copper holds, the
/// amount sent in copper, and the money line following.
#[test]
fn the_bank_takes_whole_gold_sends_copper_and_says_why_not() {
    let data = common::data();
    let mut world = inside(&data, BANK);
    world.party.gold = 1000;
    let mut forms = Forms::default();
    let shown = |world: &SimWorld, forms: &Forms| {
        let views = common::views(world, &data);
        screen::content(ScreenId::Service, &ctx(&data, world, &views), forms)
    };
    let why = |s: &Screen, id| s.control(id).unwrap().why_not.clone();
    let s = shown(&world, &forms);
    assert_eq!(
        why(&s, ControlId::Deposit).as_deref(),
        Some("Type an amount")
    );
    assert_eq!(
        why(&s, ControlId::Withdraw).as_deref(),
        Some("Type an amount")
    );

    let type_ = |world: &SimWorld, forms: &mut Forms, text: &str| {
        let views = common::views(world, &data);
        screen::apply(
            ScreenId::Service,
            &ctx(&data, world, &views),
            forms,
            ControlId::Amount,
            &Payload::Text(text.to_owned()),
            0,
        )
    };
    assert_eq!(type_(&world, &mut forms, "5"), Ok(Effect::None));
    assert_eq!(type_(&world, &mut forms, "5g"), Err(Refused::NotDigits));
    assert_eq!(type_(&world, &mut forms, "-5"), Err(Refused::NotDigits));
    assert_eq!(
        type_(&world, &mut forms, "42949673"),
        Err(Refused::OutOfRange)
    );
    assert_eq!(
        type_(&world, &mut forms, "123456789"),
        Err(Refused::TooLong(8))
    );
    assert_eq!(forms.bank, "5", "a refused amount keeps the last");
    assert_eq!(type_(&world, &mut forms, "42949672"), Ok(Effect::None));
    assert_eq!(type_(&world, &mut forms, "5"), Ok(Effect::None));

    let s = shown(&world, &forms);
    assert_eq!(why(&s, ControlId::Deposit), None);
    assert_eq!(
        why(&s, ControlId::Withdraw),
        Some(format!("the bank holds {}", labels::coins(0)))
    );
    let press = |world: &SimWorld, forms: &mut Forms, id| {
        let views = common::views(world, &data);
        screen::apply(
            ScreenId::Service,
            &ctx(&data, world, &views),
            forms,
            id,
            &Payload::Press,
            0,
        )
    };
    let deposit = Command::Service(ServiceCommand::Deposit { amount: 500 });
    assert_eq!(
        press(&world, &mut forms, ControlId::Deposit),
        Ok(Effect::Command {
            command: deposit.clone(),
            close: false
        })
    );
    world.apply(&data, deposit).expect("deposited");
    let s = shown(&world, &forms);
    assert_eq!(
        s.panels[0].rows[0].spans[0].text,
        format!(
            "Gold {0}  Bank {0}  Food {1}",
            labels::coins(500),
            world.party.food
        )
    );
    assert_eq!(
        press(&world, &mut forms, ControlId::Withdraw),
        Ok(Effect::Command {
            command: Command::Service(ServiceCommand::Withdraw { amount: 500 }),
            close: false
        })
    );
    type_(&world, &mut forms, "6").unwrap();
    let s = shown(&world, &forms);
    assert_eq!(
        why(&s, ControlId::Deposit).as_deref(),
        Some("not enough money")
    );
    assert_eq!(
        why(&s, ControlId::Withdraw),
        Some(format!("the bank holds {}", labels::coins(500)))
    );
}

/// A new top screen clears the outcome; entering a service clears the bank's amount, and a
/// pause over it does not.
#[test]
fn the_outcome_clears_when_the_screen_changes_and_the_amount_on_entering() {
    let service = Some(ScreenId::Service);
    let mut forms = Forms::default();
    forms.shown(None, None);
    forms.bank = "7".to_owned();
    forms.message = Some(screen::Span::body("Bought"));
    forms.shown(service, service);
    assert_eq!((forms.bank.as_str(), forms.message.clone()), ("", None));
    forms.bank = "7".to_owned();
    forms.message = Some(screen::Span::body("Deposited"));
    forms.shown(service, service);
    assert!(forms.message.is_some(), "the same screen keeps it");
    forms.shown(Some(ScreenId::Pause), service);
    forms.shown(service, service);
    assert_eq!(forms.bank, "7", "a pause over the service keeps the amount");
    assert_eq!(forms.message, None);
}

#[test]
fn escape_in_a_service_leaves_rather_than_pausing() {
    let data = common::data();
    let world = inside(&data, INN);
    let views = common::views(&world, &data);
    let mut nav = Nav::default();
    assert_eq!(nav.escape(Some(&views), true), Escape::LeaveService);
    assert_eq!(
        nav.top(Some(&views)),
        Some(ScreenId::Service),
        "nothing opened"
    );
    nav.open(ScreenId::Pause, Some(&views));
    assert_eq!(
        nav.escape(Some(&views), true),
        Escape::Closed,
        "the pause closes first"
    );
}

// The shell.

/// The shell's top screen.
fn top(app: &App) -> Option<ScreenId> {
    let session = app.world().resource::<Session>();
    app.world()
        .resource::<Screens>()
        .nav
        .top(Some(&session.views))
}

/// The shell in a game, the party inside the service at `site` with `gold` copper.
fn app_inside(site: (u16, u16), gold: u32) -> App {
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let town = common::map(&session.data, "test:map:town");
        common::place(&mut session.world, town, site.0, site.1, Facing::North);
        session.world.party.gold = gold;
        session.refresh();
        assert!(session.order(Command::Interact), "inside");
    }
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Service));
    app
}

/// Walking in opens the panel at once, and Leave leaves at once: no question either way.
#[test]
fn walking_into_the_guild_opens_its_panel_and_leave_steps_back_out() {
    use omnis_sim::omnis_core::{Direction, Rotation};
    // The game's start: the street at (10, 2) facing West, the guild's door to the north.
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.order(Command::Turn(Rotation::Right));
        session.order(Command::Interact);
        session.order(Command::Step(Direction::Forward));
        assert!(matches!(session.world.mode, Mode::Town(_)), "in the guild");
    }
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Service));
    {
        let session = app.world().resource::<Session>();
        let shown = screen::content(ScreenId::Service, &shell_ctx(session), &Forms::default());
        assert_eq!(headings(&shown), [["Spells"]]);
    }
    common::app::set(&mut app, &Pressable(ControlId::Leave), Interaction::Pressed);
    app.update();
    let session = app.world().resource::<Session>();
    assert!(
        matches!(session.world.mode, Mode::Explore),
        "back on the street"
    );
    assert_eq!(top(&app), None, "the panel closed");
}

#[test]
fn escape_in_the_shell_leaves_the_service() {
    let mut app = app_inside(TAVERN, 0);
    common::app::key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    app.update();
    let session = app.world().resource::<Session>();
    assert!(matches!(session.world.mode, Mode::Explore), "left");
    assert_eq!(top(&app), None, "not paused");
    assert_eq!(
        session.lines.last().map(String::as_str),
        Some("The party leaves the Tavern")
    );
}

/// Buying at the smith: the gold drops, the panel reports the bought line, the HUD has it too,
/// and the report goes when the panel does.
#[test]
fn a_purchase_reports_on_the_panel_and_the_report_goes_with_it() {
    let mut app = app_inside(SMITH, 100_000);
    app.update();
    common::app::set(
        &mut app,
        &Pressable(ControlId::Offer(0)),
        Interaction::Pressed,
    );
    app.update();
    let line = {
        let session = app.world().resource::<Session>();
        let line = session.lines.last().cloned().expect("a line");
        assert!(line.starts_with("Bought 1 Dagger for "), "{line}");
        assert!(session.world.party.gold < 100_000, "paid");
        line
    };
    let message = app.world().resource::<Screens>().forms.message.clone();
    assert_eq!(message, Some(screen::Span::body(line)));
    app.update();
    assert!(
        app.world().resource::<Screens>().forms.message.is_some(),
        "the report stays while the panel does"
    );
    common::app::set(&mut app, &Pressable(ControlId::Leave), Interaction::Pressed);
    app.update();
    app.update();
    assert_eq!(top(&app), None);
    assert_eq!(app.world().resource::<Screens>().forms.message, None);
}

/// The wheel over a panel scrolls it; over none, nothing moves.
#[test]
fn the_wheel_scrolls_the_panel_under_the_cursor() {
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
    use bevy::ui::{RelativeCursorPosition, ScrollPosition};
    let mut app = app_inside(SMITH, 0);
    let wheel = |app: &mut App| {
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -2.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.update();
    };
    let at = |app: &mut App| -> Vec<f32> {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&ScrollPosition, With<Scrolls>>();
        q.iter(world).map(|p| p.0.y).collect()
    };
    assert_eq!(at(&mut app), [0.0, 0.0], "two panels at the top");
    wheel(&mut app);
    assert_eq!(at(&mut app), [0.0, 0.0], "the cursor is over neither");
    let first = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Scrolls>>();
        q.iter(world).next().unwrap()
    };
    app.world_mut()
        .entity_mut(first)
        .insert(RelativeCursorPosition {
            cursor_over: true,
            normalized: None,
        });
    wheel(&mut app);
    let moved = app.world().get::<ScrollPosition>(first).unwrap().0.y;
    assert!(moved > 0.0, "wheel down scrolls the panel down: {moved}");
    assert_eq!(at(&mut app).iter().filter(|y| **y > 0.0).count(), 1);
}
