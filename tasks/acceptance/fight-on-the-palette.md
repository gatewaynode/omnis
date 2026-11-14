# The fight screen and the picture window on the palette: acceptance

Built 2026-11-13 (`tasks/plans/fight-on-the-palette.md`). Not tried by you yet.

## What changed, line by line

| On the fight screen | Before | Now |
|---|---|---|
| A standing party member | bright green | the palette's **party** ink |
| A standing monster | the same green | the palette's **hostile** ink |
| A figure that is down, and its slash | dim green | its side's ink at a quarter |
| A dead figure, and its cross | red | its side's ink at an eighth |
| The brackets round whoever acts | amber | the acting side's ink, full |
| A target's frame | cyan | the **interface** ink, bright |
| The hovered target's frame | white | white |
| A health bar's frame and fill | two greens | the **interface** ink, dim and bright |
| The picture window's drawing | green | the **hostile** ink, on the panels' shade |
| Title, menu, roll log, labels, party column, status line | the fixed green | the biosphere's **interface** ink; dim text and panel backgrounds are that ink darkened; caution yellow and alert red unchanged |

## What your own maps will show
A map with no biosphere, or one whose biosphere does not write these inks, uses the defaults:
party figures teal-green, monsters orange-red, the picture window's rat orange-red, and the
text and panels exactly as before. The picture window's ground is now the panels' dark green
(it was a shade darker).

On the default palette the party ink is the interface's hue, twice as bright. So your members,
the brackets round the one acting, the health bars and the target frames are all greens there,
told apart by brightness and shape. The monsters are the only other hue.

## Try it
1. The default palette: `just run` and a fight in your own world, or `just qa test:map:dungeon`
   (the fixture dungeon, which names no biosphere; two giant rats stand at (3, 8)).
   - The two sides are different colours.
   - On a member's turn the brackets are the party's colour; on a monster's turn the monsters'.
   - Choose Attack: the targets are framed; the one under the pointer turns white.
   - Let a member fall: the figure dims and takes its slash.
2. A biosphere that writes every ink: `just qa` (the fixture yard, biosphere ember). Walk east
   along the row north of the hedge to the two pack rats at (5, 1).
   - Party white, rats pink, the rat in the picture window pink.
   - The title, menu, roll log, labels and party column are a warm peach, not green.
   - When the fight ends the HUD is green again (that is this plan's limit; see decision 4).

## Decisions for you
1. **Amber and cyan are gone.** Whoever acts is marked by brackets in their side's colour, and a
   target by a frame in the interface's colour. Enough, or should the acting mark and the target
   frame have hues of their own again (which would need two more roles in a biosphere)?
2. **Dead is no longer red.** A dead figure is its side's colour, very dim, with its cross.
   Keep, or draw the dead in the danger ink?
3. **Party and interface are one hue on the default palette** (see above). Leave, or move the
   default party ink away from the interface's green?
4. **The interface outside a fight** stays green on every map until the HUD's plan, so on a
   biosphere map the colour changes as a fight starts and ends. Do the HUD's plan next to close
   that, or the minimap first?
5. **Icons** are still drawn in the fixed green. None stands on the fight screen today (a test
   holds that); they join the theme with the HUD's plan.

## What I saw myself
Tests only: the fight's colours are read from the nodes and the picture's pixels in a headless
app. I started the game to take a picture through its dev socket; the socket did not answer, so
I have not seen the fight screen. The brightness factors (a quarter, an eighth, the bars) are
therefore untuned by eye.
