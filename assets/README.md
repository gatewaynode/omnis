# Assets

Raw art sets as downloaded, one folder per set, each with its licence text. Game packs under
`packs/` reference sprites here through the pack asset search path (pack `assets/` first, this
folder second). When the base pack is assembled, the slices it uses are copied into
`packs/base/assets/` so a pack stays portable.

| Folder | Source | Licence | Committed | Base | Contents |
|---|---|---|---|---|---|
| `openrtp-tiles/` | finalbossblues.itch.io/openrtp-tiles (OpenRTP, 2022-05-24) | CC0, no credit required (`LICENSE.txt`) | yes | 16×16, RM2K/3 chipset layout, 480×256 sheets | world, exterior, interior, dungeon, ship tilesets; top-down, no characters or monsters |
| `private/time-fantasy-icons/` | finalbossblues.itch.io/icons (2020-08-13, paid) | GameDevMarket pro licence: commercial use and edits allowed, no credit required, **raw redistribution prohibited** | **no** (gitignored) | 16, 24, 32 px | 1023 item, equipment, skill, status, and UI icons in full-colour and limited-colour variants |

Rules:
- Anything under `private/` is never committed and never copied into a pack that ships. The
  loader substitutes the magenta placeholder when a referenced private asset is absent, so a
  clean clone still runs.
- Every new set gets a row here and its licence file kept verbatim in its folder. CC0 and
  CC-BY sets may be committed; CC-BY sets also get a line in `ATTRIBUTION.md`.
