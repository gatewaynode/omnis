# Attribution

Omnis code is licensed under MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Original game
content is CC-BY-4.0. Third-party material and its terms are listed here and rendered on the
in-game credits screen from each pack's `attribution` field (ARCHITECTURE.md §6.3).

## System Reference Document 5.1

This work includes material taken from the System Reference Document 5.1 ("SRD 5.1") by Wizards
of the Coast LLC and available at https://dnd.wizards.com/resources/systems-reference-document.
The SRD 5.1 is licensed under the Creative Commons Attribution 4.0 International License
available at https://creativecommons.org/licenses/by/4.0/legalcode.

The full document is kept under `docs/dnd.srd.5.1/`.

## Art

- **OpenRTP tiles** by finalbossblues (https://finalbossblues.itch.io/openrtp-tiles), CC0. No
  credit is required; the author asks that the files be shared by link. Kept under
  `assets/openrtp-tiles/`.

Art under `assets/private/` is licensed for use in the game but not for redistribution, is never
committed, and never ships inside a pack (`assets/README.md`).

## Fonts

- Inter (`assets/fonts/inter/`): Copyright (c) 2016 The Inter Project Authors
  (https://github.com/rsms/inter), SIL Open Font License 1.1, text in `LICENSE.txt` beside the files.
- Alegreya Sans (`assets/fonts/alegreya-sans/`): Copyright 2013 The Alegreya Sans Project Authors
  (https://github.com/huertatipografica/Alegreya-Sans), SIL Open Font License 1.1, text in `OFL.txt`
  beside the files.
- Fira Sans and Fira Mono are embedded by `bevy_feathers` (SIL Open Font License 1.1).

All three are candidates of the Feathers experiment and are compiled only into builds with the
`feathers` feature; the shipped build carries none of them until the owner picks one.

## Might and Magic

Might and Magic is a trademark of its current rights holder. Omnis reproduces none of its text,
art, maps, or names; the C64 manual under `docs/` is a design reference only (PRD.md §11.2).

## Dependencies with attribution requirements

- `smartstring` (via `rhai`, from M3): MPL-2.0. Source of the unmodified crate is available at
  https://crates.io/crates/smartstring.
