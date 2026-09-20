We actually need to get closer to the SRD, rather than drift away.

Reactions will require some clever work in a purely grid based system when some of the SRD rules are based around minature
proximity rules, but I think it is doable.  Bonus actions should be allowed, and my personal call on this is we will override the
SRD that more than one bonus action is possible.  We'll define rules such as a class progression for additional actions with
items, spells and magic being routes to additional actions as well.  I think this will add more flavor to the grid/turn system and
help create that old MM2 feel of massive melee with heroes that actually can find massive hordes.

Brainstorming:
```
What I think we need is a "reactionary-tactics" field per player character that let's the player define reactions from all available reactions, spell, equipment, ability and such.  Reactionary tactics should have trigger states associated with them, like spell cast, attacked, party member attacked, part member wounded, party member dying, enemy flees, enemy casts spell, and such.  Reactionary tactics can have multiple triggers, but only one is ever evaluated and only one is the default.  Players should have a tactics screen similar to the character screen that let's them compose tactics.  Also in combat players should be able to toggle a flag at anytime that turns reactions off for a character, like when they need to conserve mana.
```
```
The tactics screen should be for more than just reactionary-tactics, it should also provide a way to create more complex combat tactics so that it can store different approaches to different combat situations.  The tactics runbooks, should have default triggers like "goblin shaman present" or "mostly rats" (maybe a more decomposed and qunatitative set of criteria here like "rats > 50%" or "golbin shaman >= 1"), based on the encounter combinations the system may present.  The system provides the options for tactic trigger criteria and the player decides from them, possibly composable.
```
:Brainstorming

Your note is in docs/background/mechanics_and_rules.md. It stays unstaged, like the rest of that directory. I recorded the correction (drift closes toward the SRD) in tasks/LESSONS.md and in memory. I have not touched PRD.md or ARCHITECTURE.md. What follows is only a proposal.

The three ideas are one system

1. More actions per turn. Each member gets a turn budget instead of one action.
2. More picks per round. Six members with several actions each, against dozens of monsters, means many more menu picks every round.
3. Tactics as the answer. Reactions resolve from what the player declared beforehand. Runbooks pick routine turns.

The cost I raised for bonus actions (more picks) is the problem your tactics screen solves. I would describe them together in the PRD.

1. Action budget

The SRD already grants extra actions in this shape:
- Action Surge (fighter 2).
- Extra Attack (fighter, paladin and ranger at 5).
- Haste.
- Thief's Reflexes (rogue 17).

Your override generalizes that pattern. Proposed form:
- Each turn has a budget of actions, bonus_actions and reactions.
- Three rule slots compute it: turn.actions, turn.bonus_actions and turn.reactions.
- The inputs are class, level, equipped items and active effects. The SRD default is 1/1/1, and the numbers live in the pack.
- Spells, items and features gain a cost field: action, bonus or reaction. healing_word becomes a bonus action again and stops being a worse cure_wounds.

2. Reactions without a grid

A reaction has a trigger and a response. Most SRD triggers survive if proximity maps onto rows and stacks:

┌────────────────────┬──────────────────────────────┬─────────────────────────────────────────────────────────┬─────────────────┐
│    SRD reaction    │         SRD trigger          │                    Gridless mapping                     │  Your trigger   │
│                    │                              │                                                         │      name       │
├────────────────────┼──────────────────────────────┼─────────────────────────────────────────────────────────┼─────────────────┤
│ Opportunity attack │ Creature leaves your reach   │ A front-row stack flees, or a front-row member runs or  │ enemy flees     │
│                    │                              │ exchanges away from an engaged stack                    │                 │
├────────────────────┼──────────────────────────────┼─────────────────────────────────────────────────────────┼─────────────────┤
│ Shield             │ You are hit, or targeted by  │ Unchanged                                               │ attacked        │
│                    │ magic missile                │                                                         │                 │
├────────────────────┼──────────────────────────────┼─────────────────────────────────────────────────────────┼─────────────────┤
│ Hellish rebuke     │ You take damage              │ Unchanged                                               │ attacked /      │
│                    │                                                              │ wounded         │
├────────────────────┼──────────────────────────────┼─────────────────────────────────────────────────────────┼─────────────────┤
│ Counterspell       │ Creature casts within 60 ft   this needs monster             │ enemy casts     │
│                    │                              │ spellcasting                                            │ spell           │
├────────────────────┼──────────────────────────────────────────────────────────────┼─────────────────┤
│ Uncanny Dodge      │ Hit by an attacker you can   │ Unchanged                                               │ attacked        │
│ (rogue 5)          │ see                                                          │                 │
├────────────────────┼──────────────────────────────┼─────────────────────────────────────────────────────────┼─────────────────┤
│ Protection style   │ Ally within 5 ft is attacked                                 │ party member    │
│                    │                              │                                                         │ attacked        │
├────────────────────┼──────────────────────────────────────────────────────────────┼─────────────────┤
│ Ready (an action)  │ Player-defined trigger and   │ Your reactionary tactic, declared ahead of time         │ any             │
│                    │ response                                                     │                 │
├────────────────────┼──────────────────────────────┼─────────────────────────────────────────────────────────┼─────────────────┤
│ Monster Parry      │ Hit by a melee attack        ual                             │ —               │
└────────────────────┴──────────────────────────────┴─────────────────────────────────────────────────────────┴─────────────────┘

- Opportunity attacks come back. PRD §8.2 currently says they are dropped. Under this mapping they return through "enemy flees", so
  that row would change toward the SRD.
- Closest to the SRD: the SRD's Ready action is already a player-declared trigger and response. A reactionary-tactics field amounts
  to a standing Ready plus a declared reaction, whico the SRD.
- No interrupt in a menu fight: a declared rule also removes the need to stop the fight and ask the player.
- Shield today: try_shield already works this way, i. The auto_cast list would become the first tacticrule when the save format changes.
- Shield deviation becomes a choice: the "fires onlyle would be a player-chosen condition: "when hit" or"when it would change the outcome".

Proposed shape:
- A Character.tactics field holds a reactions_on flaules. Each rule has a trigger, an optional conditionand a response.
- The trigger list is a closed set that the simulati The player composes rules from it and never writesscript. That keeps replays deterministic and adds no attack surface.
- Members are evaluated in marching order, then by rthat matches and can be afforded spends the reaction.
- The in-fight toggle flips reactions_on and costs nothing.

3. Runbooks

- Shape: a runbook is an ordered list of pairs. Each pair is a criterion over the encounter and the action policy to use when it
  holds.
- Criteria: they are integer predicates the system offers and the player combines with and/or. Examples: count of a monster ≥ n, a
  monster's share > p%, a monster tag such as beast rty HP below p%.
- Both directions: there are two ways to build the chooser:

┌────────────────────┬──────────────────────────────────┬───────────────────────────────────┐
│                    │ Runbook chosen in the simulatn the app     │
├────────────────────┼──────────────────────────────────┼───────────────────────────────────┤
│ Replays            │ Depend on the stored runbook s, unaffected │
├────────────────────┼──────────────────────────────────┼───────────────────────────────────┤
│ Headless, MCP, CLI │ Work                                       │
├────────────────────┼──────────────────────────────────┼───────────────────────────────────┤
│ Save format        │ Runbooks stored in the World               │
└────────────────────┴──────────────────────────────────┴───────────────────────────────────┘

- Recommendation: put it in between.
  - A pure chooser function (world, data, member → Cis-sim. Any front end calls it, and the command logstill records what was chosen.
  - Runbooks are saved with the party.
  - Reactions have no such choice. They resolve in the middle of a command, so they must live in the simulation.

Where it goes in the documents

PRD
- §7.3: rewrite the action-economy bullet as the budions.
- New §7.9 "Tactics": what the player composes, the screen, the in-fight toggle and runbooks.
- §8.2: two new rows, each with its crawl-loop reaso
  - "Turn budget beyond one bonus action": heroes against hordes.
  - "Declared reactions": a menu fight has no interrows.
- §8.3: the action-economy row is restated. A new "Reactions and proximity" row carries the mapping table above. The note that
  opportunity attacks are dropped is revisited.
- §6: new decisions for the budget, reactionary tactics and runbooks.
  - They are numbered after the duplicate-D20 drift
  - D1's rationale gains "closer to the SRD is the default direction".
- §12: R3 gains nova turns (spell points times severisk is added: fights that play themselves. Themitigation for both is the seed measurement harness.
- §14: the open questions below.

ARCHITECTURE (rolled into the v0.3 bump)
- §4.2:
  - PartyCommand::Tactics replaces AutoCast.
  - A turn takes several commands until the budget is spent or an EndTurn arrives.
  - A new Event::Reaction is added.
- §4.5: the closed trigger list, the evaluation order, and the save schema 5 migration from auto_cast.
- §5: the three budget slots.
- §6: the cost field on spells, items and features, and class features gaining mechanics.
- §8.1: a TacticsPlugin.
- §17: a new decision row.

Entry point
- The tool pad's six buttons and the pause overlay's
- A fourth page on the character sheet, "Tactics", fits what you described ("similar to the character screen") and needs no seventh
  button.
- The reactions toggle goes on the fight's action row.

TODO sequencing (recommendation)
- M7: budget and reactions land here. Progression mag Action reachable, so M7 needs bonus actions anyway.
- M7b: the tactics screen with reactionary rules. Deferred-milestone numbering is untouched.
- M10: runbooks, recorded as a horizon. Monster tagsm M9 are what make their criteria meaningful.

Questions that change the write-up

1. Trigger matching. You wrote "multiple triggers, but only one is ever evaluated and only one is the default". I read it as: several rules per character, at most one fires per trigger event, and one rule is marked as the fallback. Is that right? Or does each rule have several triggers with only one active at a time?
2. Reaction count. Reactions stay at one per round as in the SRD. Can the budget raise that count the way it raises bonus actions?
3. Bonus-action spells. The SRD limits a turn with a bonus-action spell to a cantrip as its action. Keep that limit, or drop it as part of your override? With spell points, dropping it is the larger balance change. I would measure both before choosing.
4. Runbook scope. Do runbooks pick whole turns, or do they only propose and wait for a confirm? A third option is a per-member auto flag.
5. Monsters and hirelings. Do monster stacks and hirelings get tactics of their own? Monster Parry and hireling presets argue yes. I would record it as a horizon.

1 --> Let me try to clarify.  Every action that may have a trigger can have multiple sets of criteria for that trigger.  There is always only one default set of criteria per runbook that is evaluated in combat.  And there is always only one default runbook.  Different runbooks can be created where the default trigger criteria can be changed for any given action.  Previously constructed criteria are available in runbook creation for each action.
2 --> Reaction count becomes fluid, more than one reaction is allowed and with progression inevitable.  Extend from the SRD.
3 --> Bonus-action spells also extends from the SRD to potentially more than one spell per round, but in a way that is limited by the spell itself.  The spell itself defines if it can be used as bonus action, this largely depends on the speed at which a spell can be cast or if it can be prepared in advance.  But it is always explicitly set in the spell definition if it can be used as a bonus action, so three system fields for spells: <boolean: bonus_action_available>, <boolean: preparation_available>, <boolean: preparation_required_for_bonus_action>.
4 --> This should be a per-member auto flag.  There is a place for fully automated combat in the game, to remove tedium, and then only switch to player driven combat when needed or desired.
5 --> Yes.  Monsters and hirelings get their own collections of runbooks.  The details of how to distribute and progress this is a system, environment, game master set of features for later development.