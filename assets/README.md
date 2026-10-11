# Assets

Raw art sets as downloaded, one folder per set, each with its licence text. Game packs under
`packs/` reference sprites here through the pack asset search path (pack `assets/` first, this
folder second). When the base pack is assembled, the slices it uses are copied into
`packs/base/assets/` so a pack stays portable.

| Folder | Source | Licence | Committed | Base | Contents |
|---|---|---|---|---|---|
| `openrtp-tiles/` | finalbossblues.itch.io/openrtp-tiles (OpenRTP, 2022-05-24) | CC0, no credit required (`LICENSE.txt`) | yes | 16×16, RM2K/3 chipset layout, 480×256 sheets | world, exterior, interior, dungeon, ship tilesets; top-down, no characters or monsters |
| `private/time-fantasy-icons/` | finalbossblues.itch.io/icons (2020-08-13, paid) | GameDevMarket pro licence: commercial use and edits allowed, no credit required, **raw redistribution prohibited** | **no** (gitignored) | 16, 24, 32 px | 1023 item, equipment, skill, status, and UI icons in full-colour and limited-colour variants |
| `fonts/inter/` | github.com/rsms/inter, release v4.1 (2024-11-16), `Inter-4.1.zip` sha256 `9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e`, `extras/ttf/` | SIL OFL 1.1 (`LICENSE.txt`) | yes | TrueType, static | `Inter-Regular.ttf` sha256 `40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82`, `Inter-Bold.ttf` sha256 `288316099b1e0a47a4716d159098005eef7c0066921f34e3200393dbdb01947f`; a candidate of the Feathers experiment, compiled into builds with the `feathers` feature only |
| `fonts/alegreya-sans/` | github.com/huertatipografica/Alegreya-Sans, tag v2.008 (2018-03-13), `fonts/ttf/` (the release has no archive; the files are the tag's) | SIL OFL 1.1 (`OFL.txt`) | yes | TrueType, static | `AlegreyaSans-Regular.ttf` sha256 `21918f51699a828eb81e142811019471aaabdafa60cc79444dd1e78c5d4f5fa0`, `AlegreyaSans-Bold.ttf` sha256 `de414196b1f9e302015af5769360fbe5bdd1881b83ddf2acc239a3dffb09a696`; a candidate of the Feathers experiment, as above |

Rules:
- Anything under `private/` is never committed and never copied into a pack that ships. The
  loader substitutes the magenta placeholder when a referenced private asset is absent, so a
  clean clone still runs.
- A font file is parsed by the text stack (`parley`, `swash`): fonts come only from the
  project's own upstream, from a release older than 30 days, with the hashes recorded here.
- Every new set gets a row here and its licence file kept verbatim in its folder. CC0 and
  CC-BY sets may be committed; CC-BY sets also get a line in `ATTRIBUTION.md`.


# SVG Importing

A script can turn simple SVGs into the 3D world's line format. It fits the open question in presentation-PRD §12.4, which names SVG import as one candidate format for authored set pieces. It would likely be a subcommand of omnis-cli, like tileset bake. It would read the SVG, turn the shapes into line segments, and write a RON line model into the pack for the loader to check.

A few conventions will make your files convert cleanly:
- Shapes: use outlines made of path, line, polyline, polygon, rect, circle and ellipse. Curves are fine; they get flattened into short lines.
- Text: convert it to paths. Skip filters, gradients, masks and embedded images; they have no line equivalent.
- Fills: a closed shape with a fill could become a dark surface that hides what's behind it, so a statue can block the wall behind it. Unfilled shapes stay pure lines.
- Size and ground: make the bottom of the canvas the ground. Let the width of the viewBox stand for one cell, or tell me the scale you'd prefer.
- Krita: export plain SVG. The .kra file in assets/ is your Krita source, and the converter would only read the SVG.

SVG is flat, so each file needs a rule for becoming 3D. There are three ways:
- Standing cutout: an upright flat shape. Good for signs, statues and flat silhouettes.
- Spun around a vertical axis: good for columns, towers and urns.
- Extruded to a depth: good for arches, walls and machines.

Tell me which kind each placeholder is, or name the files with a prefix such as spin- or cutout-, and the script can follow that.
