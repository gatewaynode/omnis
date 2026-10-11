//! Party creation (parity P3, presentation-ARCHITECTURE §6.7): the screen offers what the
//! engine's creation view allows, shows the running cost and the picks, says why Add or Begin
//! cannot be used in the engine's order and the old client's words, and adds the draft the form
//! describes. Model tests, no window; the app's run through it is in `screens.rs`.

use crate::common;

use common::app::config;
use omnis_sim::omnis_data::Skill;
use omnis_sim::{Command, Settings};
use omnis_vector::screen::create_party::CreateForm;
use omnis_vector::screen::{self, Control, ControlId, Effect, Forms, Payload, Refused, ScreenId};
use omnis_vector::shell::screen::ctx;
use omnis_vector::shell::session::{Packs, Session};
use omnis_vector::sim::game_settings;

/// A new game with no party yet.
fn fresh() -> Session {
    let packs = Packs::load(&config()).expect("the shipped packs load");
    Session::new_game(&packs, 7, game_settings(Settings::default())).expect("a game")
}

fn control(session: &Session, forms: &Forms, id: ControlId) -> Control {
    screen::content(ScreenId::CreateParty, &ctx(session), forms)
        .control(id)
        .unwrap_or_else(|| panic!("{id:?} on the screen"))
        .clone()
}

fn why(session: &Session, forms: &Forms, id: ControlId) -> Option<String> {
    control(session, forms, id).why_not
}

fn use_(
    session: &Session,
    forms: &mut Forms,
    id: ControlId,
    payload: Payload,
) -> Result<Effect, Refused> {
    screen::apply(ScreenId::CreateParty, &ctx(session), forms, id, &payload, 0)
}

/// Every line of text on the screen.
fn lines(session: &Session, forms: &Forms) -> Vec<String> {
    let shown = screen::content(ScreenId::CreateParty, &ctx(session), forms);
    let mut out = vec![shown.title.text];
    for panel in &shown.panels {
        for row in &panel.rows {
            out.extend(row.spans.iter().chain(&row.after).map(|s| s.text.clone()));
        }
    }
    out
}

fn skill(s: Skill) -> ControlId {
    let at = Skill::ALL.iter().position(|k| *k == s).unwrap();
    ControlId::Skill(u8::try_from(at).unwrap())
}

/// Step choice `id` until its option reads `want`.
fn choose(session: &Session, forms: &mut Forms, id: ControlId, want: &str) {
    for _ in 0..12 {
        if let screen::Kind::Choice { options, at } = control(session, forms, id).kind
            && options[at] == want
        {
            return;
        }
        use_(session, forms, id, Payload::Right).unwrap();
    }
    panic!("{id:?} never reads {want}");
}

/// Add the form's draft through the session, as the router does.
fn add(session: &mut Session, forms: &mut Forms) {
    let effect = use_(session, forms, ControlId::AddMember, Payload::Press).unwrap();
    let Effect::Command { command, close } = effect else {
        panic!("Add sends a command, got {effect:?}");
    };
    assert!(!close, "the screen stays for the next member");
    assert!(session.order(command.clone()), "the rules accept the draft");
    forms.accepted(&command);
}

#[test]
fn the_screen_offers_what_the_creation_view_allows() {
    let session = fresh();
    let forms = Forms::default();
    let race = control(&session, &forms, ControlId::Race);
    let screen::Kind::Choice { options, at } = race.kind else {
        panic!("a choice")
    };
    assert_eq!(
        options,
        [
            "Dwarf (hill dwarf)",
            "Elf (high elf)",
            "Halfling (lightfoot)",
            "Human"
        ]
    );
    assert_eq!(at, 0);
    let alignment = control(&session, &forms, ControlId::Alignment);
    let screen::Kind::Choice { options, .. } = alignment.kind else {
        panic!("a choice")
    };
    assert_eq!(options.len(), 9);
    assert_eq!(options[0], "Lawful Good");
    let text = lines(&session, &forms);
    assert!(
        text.contains(&"CREATE YOUR PARTY   0 of 6 members".to_owned()),
        "{text:?}"
    );
    assert!(
        text.contains(&"0 points spent of 27".to_owned()),
        "{text:?}"
    );
    assert!(text.contains(&"No one yet".to_owned()), "{text:?}");
    // The dwarf adds 2 to Constitution: the bonus beside the score.
    assert!(text.contains(&"+2".to_owned()), "{text:?}");
    let str_ = control(&session, &forms, ControlId::Score(0));
    assert_eq!(str_.label, "STR");
    assert_eq!(
        str_.kind,
        screen::Kind::Number {
            value: 8,
            min: 8,
            max: 15
        }
    );
}

#[test]
fn a_score_steps_and_slides_inside_the_range_and_its_cost_shows() {
    let session = fresh();
    let mut forms = Forms::default();
    let s = ControlId::Score(0);
    use_(&session, &mut forms, s, Payload::Left).unwrap();
    assert_eq!(
        forms.create.score(0, &session.views.creation),
        8,
        "held at 8"
    );
    assert_eq!(
        use_(&session, &mut forms, s, Payload::Set(16)),
        Err(Refused::OutOfRange)
    );
    use_(&session, &mut forms, s, Payload::Set(15)).unwrap();
    use_(&session, &mut forms, s, Payload::Right).unwrap();
    assert_eq!(
        forms.create.score(0, &session.views.creation),
        15,
        "held at 15"
    );
    assert!(lines(&session, &forms).contains(&"cost 9".to_owned()));
    // The standard array costs the whole budget.
    for (i, n) in [15, 14, 13, 12, 10, 8].into_iter().enumerate() {
        let id = ControlId::Score(u8::try_from(i).unwrap());
        use_(&session, &mut forms, id, Payload::Set(n)).unwrap();
    }
    assert!(lines(&session, &forms).contains(&"27 points spent of 27".to_owned()));
}

#[test]
fn picks_follow_the_class_and_what_the_race_and_background_grant() {
    let session = fresh();
    let mut forms = Forms::default();
    choose(&session, &mut forms, ControlId::Class, "Rogue");
    choose(&session, &mut forms, ControlId::Background, "Acolyte");
    assert!(lines(&session, &forms).contains(&"Skills (pick 4)".to_owned()));
    // The acolyte grants Insight, on the rogue's list: dim, with where it comes from.
    assert_eq!(
        why(&session, &forms, skill(Skill::Insight)).as_deref(),
        Some("from the background")
    );
    assert_eq!(
        use_(&session, &mut forms, skill(Skill::Insight), Payload::Press),
        Err(Refused::Dim("from the background".to_owned()))
    );
    for s in [Skill::Stealth, Skill::Acrobatics, Skill::Deception] {
        use_(&session, &mut forms, skill(s), Payload::Press).unwrap();
    }
    assert_eq!(
        control(&session, &forms, skill(Skill::Stealth)).label,
        "[x] Stealth"
    );
    use_(
        &session,
        &mut forms,
        skill(Skill::Perception),
        Payload::Press,
    )
    .unwrap();
    // The fourth pick fills the count: the rest are dim with the count.
    assert_eq!(
        why(&session, &forms, skill(Skill::Athletics)).as_deref(),
        Some("This class picks 4 skills")
    );
    // A pick taken back frees the count.
    use_(
        &session,
        &mut forms,
        skill(Skill::Perception),
        Payload::Press,
    )
    .unwrap();
    assert!(why(&session, &forms, skill(Skill::Athletics)).is_none());
    // Another class clears the picks.
    choose(&session, &mut forms, ControlId::Class, "Fighter");
    assert!(forms.create.picks.is_empty());
}

#[test]
fn a_background_that_grants_a_pick_takes_it_back() {
    let session = fresh();
    let mut forms = Forms::default();
    choose(&session, &mut forms, ControlId::Class, "Rogue");
    let view = &session.views.creation;
    let other = view
        .backgrounds
        .iter()
        .position(|b| !b.grants.contains(&Skill::Insight))
        .expect("a background without Insight");
    forms.create.background = other;
    use_(&session, &mut forms, skill(Skill::Insight), Payload::Press).unwrap();
    assert_eq!(forms.create.picks, vec![Skill::Insight]);
    choose(&session, &mut forms, ControlId::Background, "Acolyte");
    assert!(forms.create.picks.is_empty(), "the acolyte grants it now");
}

#[test]
fn add_says_why_not_in_the_engine_s_order() {
    let mut session = fresh();
    let mut forms = Forms::default();
    let add_why = |session: &Session, forms: &Forms| why(session, forms, ControlId::AddMember);
    assert_eq!(
        add_why(&session, &forms).as_deref(),
        Some("Give the character a name")
    );
    use_(
        &session,
        &mut forms,
        ControlId::Name,
        Payload::Text("Ash".into()),
    )
    .unwrap();
    for i in 0..3 {
        let id = ControlId::Score(i);
        use_(&session, &mut forms, id, Payload::Set(15)).unwrap();
    }
    use_(&session, &mut forms, ControlId::Score(3), Payload::Set(9)).unwrap();
    assert_eq!(
        add_why(&session, &forms).as_deref(),
        Some("28 points spent of 27")
    );
    use_(&session, &mut forms, ControlId::Score(3), Payload::Set(8)).unwrap();
    // The dwarf cleric picks 2.
    assert_eq!(add_why(&session, &forms).as_deref(), Some("Pick 2 skills"));
    use_(&session, &mut forms, skill(Skill::History), Payload::Press).unwrap();
    use_(&session, &mut forms, skill(Skill::Medicine), Payload::Press).unwrap();
    assert_eq!(add_why(&session, &forms), None);
    assert_eq!(
        why(&session, &forms, ControlId::Begin).as_deref(),
        Some("Add at least one member")
    );

    let draft = forms.create.draft(&session.views.creation);
    add(&mut session, &mut forms);
    assert_eq!(
        forms.create,
        CreateForm::default(),
        "the next member starts blank"
    );
    let ash = &session.views.party.members[0];
    assert_eq!(ash.name, "Ash");
    // The dwarf's +2 Constitution is on the sheet; the screen bought 15.
    assert_eq!(ash.scores[2], 17);
    assert!(why(&session, &forms, ControlId::Begin).is_none());
    assert!(lines(&session, &forms).contains(&"Ash".to_owned()));
    assert!(lines(&session, &forms).contains(&"CREATE YOUR PARTY   1 of 6 members".to_owned()));

    // Five more fill the party; the seventh is refused.
    for n in 0..5 {
        forms.create.name = format!("Ash {n}");
        forms.create.picks = draft.skills.clone();
        add(&mut session, &mut forms);
    }
    forms.create.name = "Late".into();
    forms.create.picks = draft.skills.clone();
    assert_eq!(
        add_why(&session, &forms).as_deref(),
        Some("The party is full")
    );
}

#[test]
fn begin_plays_and_back_leaves_for_the_form() {
    let session = fresh();
    let mut forms = Forms::default();
    assert_eq!(
        use_(&session, &mut forms, ControlId::Back, Payload::Press),
        Ok(Effect::ToNewGame)
    );
    assert_eq!(
        use_(&session, &mut forms, ControlId::Begin, Payload::Press),
        Err(Refused::Dim("Add at least one member".to_owned()))
    );
    // `accepted` starts the draft over only for a new member.
    forms.create.name = "Kept".into();
    forms.accepted(&Command::Interact);
    assert_eq!(forms.create.name, "Kept");
}
