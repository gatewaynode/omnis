# Resize a map from the editor

Built 2026-11-15 (`tasks/plans/editor-resize.md`). Not seen on screen by me: each "you will see" has
a test behind it, none has a picture.

## How to start

A Save reseals the pack on disk, so try it on a scratch copy of your world:
`cp -R packs/the_lower_kingdoms /tmp/omnis-scratch`, then start the editor with that as the last
pack. Open any map.

## What to try

1. **The button.** In the inspector (right column), the line with the map's id and size now ends
   with **Resize...**. It opens a window over the editor: "The map is 12 x 5." and four fields,
   North, East, South, West.
2. **Grow.** Type `3` into West. The window says "It becomes 15 x 5." and "Every cell moves 3 east
   and 0 south: a saved game's party on this map will stand where the old cell was." Press
   **Apply**. The window closes; the map is three cells wider on the left; everything that was on
   it stands where it stood relative to everything else; the old west wall is still there, now an
   inside wall. Open a gap in it or put a door, and draw your hallway.
3. **One step.** Undo (Cmd Z) puts the whole map back. The history list names the step "grew 3
   west".
4. **Cut, with a warning.** Resize... again, type `-2` into East, Apply. The window does not cut:
   it says "This cut destroys:" and lists what goes (drawn cells counted, each portal, encounter,
   site, object, pile and lore by name and cell, the start's cell, notes on those cells; eight
   lines, then "and N more"), with **Yes, resize** and **No**.
   - **No** returns to the four fields with `-2` still in East. Nothing changed.
   - **Yes, resize** makes the cut, closes the window and names the step "cut 2 east (10 cells, 2
     things lost)". Undo brings it all back.
5. **A cut of nothing asks nothing.** A strip whose cells have no ground and no walls of their
   own is cut at once. (Tested in the engine; I have not walked this through the window's eraser.)
6. **What cannot be applied.** A field that is not a number, all four empty, or a size under 1 or
   over 256 a side: Apply is dim and says why. Cancel closes the window and changes nothing.
7. **A cut portal.** Cut the side a portal stands on and say Yes: Save goes dim with an error
   count, and the inspector says which map's portal now links to nothing. Undo, or link it again.
8. **Notes.** Write a note on a cell, grow the west side, Save: the note is on the same cell at
   its new place. A note on a cell you cut is gone after the Save, and was counted in the warning.
9. **The QA yard.** `just qa`: the yard now has a gate in the east edge of its middle row; the long
   hall with the shamblers is behind it. It was made with this edit.

## Changes you did not ask for by name

- **A side that grows keeps its old outer wall.** Until now a grow to the east or south (through
  the API, the only way there was) silently opened that wall. You chose "keep it" in the plan.
- **New ground cells are never a stair or ladder terrain.** They are the first plain terrain of
  the map's list (they were always the first terrain, which the loader refused when it was a
  stair).
- **The API's `Resize` now behaves this way too**, and a new `Extend` takes the four sides. The API
  cuts without asking, as it always did; the question is the window's.

## Said plainly

- **Saved games are not rewritten.** Growing the west or north moves every cell, and a save keeps
  the party's place, the automap and each object's state by cell. A party saved on that map will
  stand that many cells off (the engine already moves one that lands off the map to the nearest
  cell it can stand on). The window says so before you apply.
- **Other maps are not rewritten.** A portal moves with its cell and its partner follows, since
  landings are worked out from where the portal stands. A portal that is cut leaves its partner
  erring until you relink or undo.
- **No outline is drawn on the plan** while you type; the "It becomes" line is the preview.
- **A cell cut and grown back in one sitting keeps its note** if you do not Save in between: notes
  move by the net shift at Save.
- The map's other fields (name, tileset, tile size in feet, what doors are made of) and New map
  still have no form.

## Decisions for you

1. **Typed fields, not steppers.** Each side is a text field. Say if you want +/- buttons as well.
2. **What counts as a loss.** Any cell with ground, a height, or a wall or door of its own, and
   anything placed. A wall shared with a cell that stays is not counted. Say if painted ground
   alone should not ask.
3. **An outline on the plan** as a follow-on, showing the new edge while the numbers change.
4. **Saves and a west or north grow**: leave as is (said in the window), or record the shift as a
   pack migration so saves follow. The second is a separate piece of work.
