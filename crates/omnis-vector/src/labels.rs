//! Words for what the engine names (presentation-ARCHITECTURE §4.3). Views and events name
//! definitions by string id and text by key (protocol 2); this module is the one place that
//! turns them into words from the packs' text. No rule is read here: only labels.
//!
//! An id no pack defines reads as the id itself, so a stale or modded name shows what it is
//! rather than vanishing; a text key no pack defines reads as the key (`Data::label`), except a
//! `MessageKey`, which falls back to this client's own wording so a key never reaches the player.

use omnis_sim::api::{MemberView, PartyView};
use omnis_sim::objects::Refusal;
use omnis_sim::omnis_core::money::{Coins, gp_floor};
use omnis_sim::omnis_core::{Calendar, CharacterId, Level};
use omnis_sim::omnis_data::{
    Ability, Alignment, Cost, Data, Disposition, EquipSlot, MapData, Skill, Verb,
};
use omnis_sim::omnis_rules::{CreationError, TacticsFault};
use omnis_sim::{CheckKind, MessageKey, OfferView, Rejection, SaveRule, ServiceCommand};

/// The display text of a text key a view or event carries (`ItemView.name`, `SpellView.name`).
#[must_use]
pub fn text<'d>(data: &'d Data, key: &'d str) -> &'d str {
    data.label("en", key)
}

/// A level of a map as the player reads it (M8.5 step 1b), with its number: `Roof (2)`. The
/// map's own level is named by the map (`Test Tower (0)`). `None` for a map of one level, where
/// a level says nothing.
#[must_use]
pub fn level(data: &Data, map: &MapData, level: Level) -> Option<String> {
    if map.levels.len() < 2 {
        return None;
    }
    let key = map.level(level).map_or("", |l| l.name.as_str());
    let name = if key.is_empty() {
        data.registry
            .text
            .name(map.name)
            .map_or("", |k| text(data, k))
    } else {
        text(data, key)
    };
    Some(format!("{name} ({level})"))
}

/// A class's name from its string id.
#[must_use]
pub fn class<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .classes
        .get(id)
        .and_then(|n| data.classes.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A race's name from its string id.
#[must_use]
pub fn race<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .races
        .get(id)
        .and_then(|n| data.races.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A background's name from its string id.
#[must_use]
pub fn background<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .backgrounds
        .get(id)
        .and_then(|n| data.backgrounds.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// An item's name from its string id.
#[must_use]
/// Items a monster dropped, by name: "2 Gems, Dagger".
pub fn loot(data: &Data, loot: &[omnis_sim::Loot]) -> String {
    let named: Vec<String> = loot
        .iter()
        .map(|l| match l.count {
            1 => item(data, &l.item).to_owned(),
            n => format!("{n} {}", item(data, &l.item)),
        })
        .collect();
    named.join(", ")
}

/// An item's name from its string id.
pub fn item<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .items
        .get(id)
        .and_then(|n| data.items.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A spell's name from its string id.
#[must_use]
pub fn spell<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .spells
        .get(id)
        .and_then(|n| data.spells.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A monster's name from its string id.
#[must_use]
pub fn monster<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .monsters
        .get(id)
        .and_then(|n| data.monsters.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A condition's name from its string id.
#[must_use]
pub fn condition<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .conditions
        .get(id)
        .and_then(|n| data.conditions.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A town service's name from its string id.
#[must_use]
pub fn service<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .services
        .get(id)
        .and_then(|n| data.services.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A map's name from its string id.
#[must_use]
pub fn map<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .maps
        .get(id)
        .and_then(|n| data.maps.get(&n))
        .map_or(id, |d| data.text("en", d.name))
}

/// What a `Message` event says. The packs' text wins; without it, this client's wording.
#[must_use]
pub fn message(data: &Data, key: MessageKey) -> &str {
    let found = data.label("en", key.text_key());
    if found != key.text_key() {
        return found;
    }
    match key {
        MessageKey::NothingHere => "There is nothing to do here",
    }
}

/// A purse or price in copper by coin, largest first: `15 gp 3 sp 7 cp` (as the retired client
/// shows town prices and purses).
#[must_use]
pub fn coins(cp: u32) -> String {
    Coins::of(cp).to_string()
}

/// Copper as whole gold rounded down, where a line names gold: a bribe, the victory's loot.
#[must_use]
pub fn gold(cp: u32) -> String {
    format!("{} gold", gp_floor(cp))
}

/// The save rule as the new-game and pause screens name it.
#[must_use]
pub const fn save_rule(rule: SaveRule) -> &'static str {
    match rule {
        SaveRule::Anywhere => "Anywhere",
        SaveRule::Relief => "Inns, items, and spells",
        SaveRule::InnOnly => "Inns only",
    }
}

/// A name the engine spells as one Rust word (`SleightOfHand`), as words: "Sleight of Hand".
fn spaced(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.char_indices() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out.split(' ')
        .enumerate()
        .map(|(i, w)| match w {
            "Of" | "The" if i > 0 => w.to_ascii_lowercase(),
            _ => w.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// An alignment: "Lawful Good".
#[must_use]
pub fn alignment(a: Alignment) -> String {
    spaced(&format!("{a:?}"))
}

/// An ability: "Wisdom".
#[must_use]
pub fn ability(a: Ability) -> String {
    format!("{a:?}")
}

/// A bonus with its sign: "+3", "-1", "+0".
#[must_use]
pub fn signed(n: i64) -> String {
    format!("{n:+}")
}

/// How monsters feel about the party: "hostile", "wary", "neutral", "friendly".
#[must_use]
pub const fn disposition(d: Disposition) -> &'static str {
    match d {
        Disposition::Hostile => "hostile",
        Disposition::Wary => "wary",
        Disposition::Neutral => "neutral",
        Disposition::Friendly => "friendly",
    }
}

/// What a spell spends of the turn: "action", "bonus action", "reaction", "free".
#[must_use]
pub const fn pay(cost: Cost) -> &'static str {
    match cost {
        Cost::Action => "action",
        Cost::BonusAction => "bonus action",
        Cost::Reaction => "reaction",
        Cost::Free => "free",
    }
}

/// A verb, as a button says it when the kind gives no label: "Examine", "Extinguish".
#[must_use]
pub fn verb(v: Verb) -> String {
    format!("{v:?}")
}

/// A check an object asks for: "Athletics DC 15", "Strength DC 12".
#[must_use]
pub fn check(kind: CheckKind, dc: i64) -> String {
    let name = match kind {
        CheckKind::Skill(s) => skill(s),
        CheckKind::Ability(a) => ability(a),
        other => spaced(&format!("{other:?}")),
    };
    format!("{name} DC {dc}")
}

/// Any refusal worded short: an object's by [`object_refusal`], the rest by [`reason`].
#[must_use]
pub fn refused(data: &Data, party: &PartyView, refusal: &Rejection) -> String {
    match refusal {
        Rejection::VerbRefused { reason } => object_refusal(data, reason),
        other => reason(party, other),
    }
}

/// Why an object will not do something now, in a few words for a dim button (M8.5 step 2b):
/// the item it needs by name. A flag is not named: it is the pack's bookkeeping, and the player
/// learns what opens the gate by finding it.
#[must_use]
pub fn object_refusal(data: &Data, refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotOffered => "not now".to_owned(),
        Refusal::Spent => "done already".to_owned(),
        Refusal::Lacks { item: id } => format!("needs {}", item(data, id)),
        Refusal::Flag { .. } => "something else must be done first".to_owned(),
        Refusal::School { school } => format!("needs someone who knows {school:?} magic"),
        Refusal::Rule { .. } => "not allowed now".to_owned(),
        Refusal::NoneCanAct => "nobody can act".to_owned(),
        Refusal::Closed => "it is shut".to_owned(),
        Refusal::Full => "it is full".to_owned(),
    }
}

/// A skill: "Sleight of Hand".
#[must_use]
pub fn skill(s: Skill) -> String {
    spaced(&format!("{s:?}"))
}

/// Why a draft would not be added, in the words party creation uses: the party full, a name
/// missing, points overspent, the wrong number of picks (`choose`); anything else in the
/// engine's own words.
#[must_use]
pub fn creation_refusal(rejection: &Rejection, picks: usize, choose: u8) -> String {
    match rejection {
        Rejection::PartyFull => "The party is full".to_owned(),
        Rejection::Character(CreationError::Name) => "Give the character a name".to_owned(),
        Rejection::Character(CreationError::Points { spent, budget }) => {
            format!("{spent} points spent of {budget}")
        }
        Rejection::Character(CreationError::Skills(_)) if picks != usize::from(choose) => {
            format!("Pick {choose} skills")
        }
        other => other.to_string(),
    }
}

/// How long ago something happened, as a rumor tells it: "today", "yesterday", "3 days ago",
/// "a month ago", "2 years ago" (the retired client's `ago_text`).
#[must_use]
pub fn ago(minutes: i64, calendar: Calendar) -> String {
    let days = calendar.day(minutes.max(0));
    let per_year = i64::from(calendar.days_per_year.max(1));
    let count = |n: i64, one: &str, many: &str| {
        if n == 1 {
            format!("a {one} ago")
        } else {
            format!("{n} {many} ago")
        }
    };
    match days {
        0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        d if d < 30 => format!("{d} days ago"),
        d if d < per_year => count(d / 30, "month", "months"),
        d => count(d / per_year, "year", "years"),
    }
}

/// One of a service's rumors, by its row in the service's file, told `ago` minutes after it
/// happened; "?" for a row or service no pack holds.
#[must_use]
pub fn rumor(data: &Data, service: &str, row: u16, ago_minutes: i64) -> String {
    let said = data
        .registry
        .services
        .get(service)
        .and_then(|n| data.services.get(&n))
        .and_then(|s| s.rumors.get(usize::from(row)))
        .map_or("?", |r| text(data, &r.text));
    let when = ago(ago_minutes, data.calendar());
    omnis_sim::omnis_data::text::fill(said, &[("ago", &when)])
}

/// An offer's row label in a service (the retired client's `service_panel::row_label`): "A
/// night's rest", "Longsword", "Dagger x2", "Brenna, 5 of 12 HP", "Brenna, level 1 to 2",
/// "Ilvara: Shatter". Names come from the party view and the packs.
#[must_use]
pub fn offer_label(data: &Data, party: &PartyView, offer: &OfferView) -> String {
    let member = |id: CharacterId| party.members.iter().find(|m| m.member == id);
    let named = |id: CharacterId, line: &dyn Fn(&MemberView) -> String| {
        member(id).map_or_else(|| "?".to_owned(), line)
    };
    match &offer.command {
        ServiceCommand::Room => "A night's rest".to_owned(),
        ServiceCommand::Rumor => "A rumor".to_owned(),
        ServiceCommand::BuyFood { count } => format!("Food for {count} day"),
        ServiceCommand::Buy { item, .. } => self::item(data, item).to_owned(),
        ServiceCommand::Sell { item, .. } => {
            let count = party
                .inventory
                .iter()
                .find(|i| i.item == *item)
                .map_or(0, |i| i.count);
            format!("{} x{count}", self::item(data, item))
        }
        ServiceCommand::Heal { member: id }
        | ServiceCommand::Cure { member: id }
        | ServiceCommand::Raise { member: id } => named(*id, &|m| treated_label(data, offer, m)),
        ServiceCommand::Train { member: id } => named(*id, &|m| {
            format!("{}, level {} to {}", m.name, m.level, m.level + 1)
        }),
        ServiceCommand::Choose { member: id, spell }
        | ServiceCommand::Learn { member: id, spell } => named(*id, &|m| {
            format!("{}: {}", m.name, self::spell(data, spell))
        }),
        ServiceCommand::Leave
        | ServiceCommand::Deposit { .. }
        | ServiceCommand::Withdraw { .. } => String::new(),
    }
}

/// A temple row names what it treats: hit points, conditions, or death.
fn treated_label(data: &Data, offer: &OfferView, m: &MemberView) -> String {
    let name = &m.name;
    match offer.command {
        ServiceCommand::Cure { .. } if m.conditions.is_empty() => format!("{name}, no conditions"),
        ServiceCommand::Cure { .. } => {
            let names: Vec<&str> = m.conditions.iter().map(|c| condition(data, c)).collect();
            format!("{name}: {}", names.join(", "))
        }
        ServiceCommand::Raise { .. }
            if matches!(offer.refusal, Some(Rejection::NotDead { .. })) =>
        {
            format!("{name}, alive")
        }
        ServiceCommand::Raise { .. } => format!("{name}, dead"),
        _ => format!("{name}, {} of {} HP", m.hp.max(0), m.hp_max),
    }
}

/// The button's caption for an offer.
#[must_use]
pub const fn offer_caption(command: &ServiceCommand) -> &'static str {
    match command {
        ServiceCommand::Room => "Take a room",
        ServiceCommand::Rumor => "Listen",
        ServiceCommand::BuyFood { .. } | ServiceCommand::Buy { .. } => "Buy",
        ServiceCommand::Sell { .. } => "Sell",
        ServiceCommand::Heal { .. } => "Heal",
        ServiceCommand::Cure { .. } => "Cure",
        ServiceCommand::Raise { .. } => "Raise",
        ServiceCommand::Deposit { .. } => "Deposit",
        ServiceCommand::Withdraw { .. } => "Withdraw",
        ServiceCommand::Train { .. } => "Train",
        ServiceCommand::Choose { .. } => "Choose",
        ServiceCommand::Learn { .. } => "Learn",
        ServiceCommand::Leave => "Leave",
    }
}

/// An offer's price ("free" at no cost) or what a sale pays. A refused purchase keeps its
/// price: the reason shows on its button.
#[must_use]
pub fn offer_note(offer: &OfferView) -> String {
    match (offer.price, offer.pays) {
        (Some(0), _) => "free".to_owned(),
        (Some(price), _) => coins(price),
        (None, Some(pays)) => format!("pays {}", coins(pays)),
        (None, None) => String::new(),
    }
}

/// A service's refusal in a few words, for a dim button (the retired client's
/// `service_panel::reason`); anything else in the engine's own words.
#[must_use]
pub fn service_reason(refusal: &Rejection) -> String {
    match refusal {
        Rejection::CannotAfford { .. } => "not enough money".to_owned(),
        Rejection::NothingToTreat { .. } => "nothing to treat".to_owned(),
        Rejection::NotDead { .. } => "not dead".to_owned(),
        Rejection::BankShort { bank, .. } => format!("the bank holds {}", coins(*bank)),
        // The row names the member; the engine's words name them by number.
        Rejection::NotReady { xp, needed, .. } => format!("{xp} of {needed} XP"),
        Rejection::SpellTooHigh { level, max } => {
            format!("level {level} spell, up to level {max} now")
        }
        Rejection::RestTooSoon { minutes } => format!(
            "rested too recently ({}h {:02}m)",
            minutes / 60,
            minutes % 60
        ),
        other => other.to_string(),
    }
}

/// An equipment slot's name (the retired client's `sheet_menu::slot_label`).
#[must_use]
pub const fn slot(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::MainHand => "Main hand",
        EquipSlot::OffHand => "Off hand",
        EquipSlot::Ranged => "Ranged",
        EquipSlot::Body => "Body",
    }
}

/// An item's name with its count when there is more than one: "Rations x5" (ASCII; the font
/// has no multiplication sign).
#[must_use]
pub fn count_label(name: &str, count: u16) -> String {
    if count > 1 {
        format!("{name} x{count}")
    } else {
        name.to_owned()
    }
}

/// A member's name in `party`, or "someone" for one not in it.
fn named(party: &PartyView, member: CharacterId) -> &str {
    party
        .members
        .iter()
        .find(|m| m.member == member)
        .map_or("someone", |m| m.name.as_str())
}

/// An item's, a road cast's, a rest's or a tactic's refusal in a few words, for a dim button (the retired client's
/// `blocked_note` and inventory messages): members by name where the engine's words give their
/// number; anything else in the engine's own words.
#[must_use]
pub fn reason(party: &PartyView, refusal: &Rejection) -> String {
    match refusal {
        Rejection::MemberDead { member } | Rejection::TargetDead { member } => {
            format!("{} is dead", named(party, *member))
        }
        Rejection::MemberDown { member } => format!("{} is down", named(party, *member)),
        Rejection::NotEnoughPoints { need, .. } => format!("need {need} pt"),
        Rejection::MissingComponents { .. } => "needs components".to_owned(),
        Rejection::NotCastable { .. } => "not here".to_owned(),
        Rejection::NotEquippable => "cannot be worn or wielded".to_owned(),
        Rejection::HandsFull => "no hand free for a shield".to_owned(),
        Rejection::NotUsable => "does nothing when used".to_owned(),
        Rejection::NotUsableHere => "not from a fight".to_owned(),
        // The fight's features (the engine's words name the feature by its key).
        Rejection::NoUsesLeft { .. } => "no uses left until a rest".to_owned(),
        // Rests (the retired client's camp notes).
        Rejection::RestTooSoon { minutes } => format!(
            "rested too recently ({}h {:02}m)",
            minutes / 60,
            minutes % 60
        ),
        Rejection::NoFood { need, have } => {
            format!("the rest eats {need} food; the stores hold {have}")
        }
        Rejection::NothingToTreat { member } => format!("{} is unhurt", named(party, *member)),
        Rejection::NoHitDice { member, .. } => {
            format!("{} has no hit dice left", named(party, *member))
        }
        // Tactics.
        Rejection::CannotReact => "cannot answer that".to_owned(),
        Rejection::NoSuchEntry { .. } => "no such entry".to_owned(),
        Rejection::Tactics(fault) => match fault {
            TacticsFault::Name => "the name does not fit",
            TacticsFault::TooDeep => "too many conditions",
            TacticsFault::TooMany => "no room for another",
            TacticsFault::Percent => "a percent over 100",
            TacticsFault::NoSuchSet | TacticsFault::NoDefault => "the tactics do not fit together",
        }
        .to_owned(),
        Rejection::UnknownId { id } => format!("not loaded: {id}"),
        other => other.to_string(),
    }
}
