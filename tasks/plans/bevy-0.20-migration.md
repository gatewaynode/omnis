# Bevy 0.19.1 → 0.20.0 migration plan

Status: **planned, not started** (written 2026-10-10 on `gui-3d-experiment`). The source is the official
[0.19 → 0.20 migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/), checked line by
line against this branch's tree at `17dbc24`. The audit is static: 0.20 was not downloaded or compiled.

## When

- **Not before 2026-11-07.** `bevy` 0.20.0 was published on 2026-10-08, and CLAUDE.md forbids
  packages under 30 days old.
- **N−1 does not hold Bevy back:** PRD D9 exempts Bevy from the N−1 rule (latest stable). The
  30-day rule still applies, and so do pinning and lockfile checksums.
- **After B7 and the merge.** The `omnis-app` lines below are this branch's copy, which predates the
  mechanics work (M7c's tactics panel and more). Re-run the `omnis-app` audit on the merged tree
  before starting. Rebasing across an engine upgrade is avoided.

## Size

- 2 crates use Bevy (`omnis-vector`, `omnis-app`). The simulation crates, `omnis-cli` and
  `omnis-mcp` do not (confirmed by grep).
- **Must change to build:** 2 manifest lines and 36 `.rs` lines in 9 files, all in `omnis-vector`.
  `omnis-app` needs no code edit, only runtime checks.
- **Effort:** about 1 day with a temporary `#[allow(deprecated)]` on the button code, or 2–3 days
  with the move to picking done properly (recommended; it lines up with the art update).

## Steps (one commit each; the gate and Sentrux before each)

1. **Baseline captures on 0.19.1**, kept in the scratchpad, not committed:
   - `omnis-vector`: the 3D view, and the fight screen via the fight capture recipe, at 1600×900
     and 5120×1440.
   - `omnis-app`: the devtools canvas screenshot on a fixed seed and script.
2. **Manifests** (§1), then `cargo update -p bevy` to regenerate `Cargo.lock`. Check the new
   checksums and the transitive crates' ages.
3. **Compile and settle the open questions** (§4). Expect errors and deprecation warnings only in
   `omnis-vector`'s button code. Clippy runs with `-D warnings`.
4. **`omnis-vector` tonemapping** (§2.1).
5. **`omnis-vector` buttons to `ui_widgets::Button` + `Hovered`/`Pressed`** (§2.2, §2.3),
   including the test helper `set`. Mutation-check the press paths again: click, digit, Esc and
   hover.
6. **Runtime checks** (§3) against the baseline captures. Fix or record each difference.
7. **Docs** that name 0.19:
   - `ARCHITECTURE.md:400,406,413,424,510,520`: the verified-against line, 0.19 facts, the
     `bevy_egui` pin, the dependency rule, and the version table.
   - `PRD.md:78,289`: D9's "0.19.x" and the engine facts.
   - `alt-PRD.md:167` and `alt-ARCHITECTURE.md:58`: the pinned version and the feature list read from
     the 0.19.1 manifest.

## 1. Manifests

| file:line | now | change | confidence |
|---|---|---|---|
| `Cargo.toml:40` | `bevy = { version = "=0.19.1", default-features = false }` | `"=0.20.0"`. Shared by both crates. | certain |
| `Cargo.toml:48` | `bevy_egui = "=0.41.1"` | 0.41 targets Bevy 0.19. Re-pin it to the stable `bevy_egui` release built for 0.20, once that release is 30 days old (only `0.43.0-rc.1` exists on 2026-10-10). No crate uses it yet and it is absent from `Cargo.lock`, so it does not block the build. Do not delete it: ARCH A11 reserves it for the editor. | stale: certain; version: not yet released |
| `Cargo.toml:50` | `png = "=0.18.1"` (omnis-cli; comment: tracks Bevy's image stack) | Re-pin to whatever 0.20's image stack resolves. | verify after resolving |
| `crates/omnis-vector/Cargo.toml:19` | `features = ["2d", "png", "bevy_pbr", "ui"]` | May need the feature that brings `ui_widgets` and `picking::hover::Hovered`. The guide does not name it. | verify at compile |
| `crates/omnis-app/Cargo.toml:20,26` | `dev` features; `["2d", "png"]` | None renamed by the guide. | verify at compile |

The guide says crates without default features that use curves must enable `bevy/bevy_curve`. We use
no curve API, so no change.

## 2. `omnis-vector` code

All paths are under `crates/omnis-vector/`.

### 2.1 Tonemapping (guide: "Tonemapping::None is now a full passthrough")

`None` no longer applies `ColorGrading` or `DebandDither` and no longer clamps negative channels.
Bevy also warns when `None` is combined with dither. `Linear` keeps 0.19's behaviour.

| file:line | now | change | confidence |
|---|---|---|---|
| `src/shell/render.rs:73` | `Tonemapping::None,` (Camera3d + Hdr) | `Tonemapping::Linear,` | certain |
| `src/shell/combat.rs:377` | `Tonemapping::None,` (Camera2d + Hdr) | `Tonemapping::Linear,`, or delete the line (Camera2d now defaults to Linear) | certain |
| `src/shell/render.rs:12`, `src/shell/combat.rs:19` | `use bevy::core_pipeline::tonemapping::Tonemapping;` | Optional: it moved to `bevy_render::view`, and the old path still re-exports. Likely `bevy::render::view::Tonemapping`. | old path: certain; new path: verify |

### 2.2 Buttons in `src/` (guide: "ui::widgets::Button and ui::Interaction are deprecated")

The replacements are `ui_widgets::Button`, `picking::hover::Hovered` and `ui::Pressed`. The guide
points to the updated `button.rs` example for usage. It does not say how to detect a press edge, so
`Added<Pressed>` in place of `Changed<Interaction>` + `Pressed` is a guess to confirm against that
example.

| file:line | now | change | confidence |
|---|---|---|---|
| `src/shell/controls.rs:112` | `entity.insert(Button);` | the new `Button` (likely `bevy::ui_widgets::Button`) | deprecation certain; path verify |
| `src/shell/controls.rs:177,184` | `Query<(Ref<Interaction>, &Action)>`; `*interaction != Interaction::Pressed` | Held actions act while `Pressed` is present. One-shot actions (line 197 uses `is_changed()`) act on the press edge. | deprecation certain; shape verify |
| `src/shell/controls.rs:219,222–224` | `shade`: `Changed<Interaction>`, `Interaction::{None,Hovered,Pressed}` | Shade from `Hovered` + `Pressed`, and re-shade when `Pressed` is removed. | deprecation certain; shape verify |
| `src/shell/panel.rs:125,132` | `Query<(&Interaction, &Order), Changed<Interaction>>`; `== Interaction::Pressed` | press edge on `Pressed` | deprecation certain; shape verify |
| `src/shell/combat.rs:158,168` | `Query<(&Interaction, &Choose), Changed<Interaction>>`; `== Interaction::Pressed` | press edge on `Pressed` | deprecation certain; shape verify |
| `src/shell/render.rs:101,114` | `Query<&Interaction>`; `.any(\|i\| *i != Interaction::None)` (pointer over a button pauses mouse look) | `Hovered` or `Pressed` | deprecation certain; shape verify |
| `src/shell/controls.rs:2,94` | doc comments naming `Interaction` and `Button` | wording | docs only |

### 2.3 Buttons in `tests/`

The helper `tests/common/app.rs::set` writes `Interaction` the way picking does. It becomes a helper
that inserts or removes `Pressed` (and sets `Hovered`), and every caller follows its new signature.

| file:line | now | change |
|---|---|---|
| `tests/common/app.rs:103,106,133,137` | `interaction: Interaction`; `query::<(&C, &mut Interaction)>()`; `Interaction::{Pressed,None}` | rewrite `set` on `Pressed`/`Hovered` |
| `tests/common/app.rs:99` | doc comment | wording |
| `tests/shell.rs:51,58,70,84,88,89` | `set(.., Interaction::Pressed/None)` | new `set` signature |
| `tests/fight.rs:16` | `query_filtered::<&Order, With<Button>>()` | the new `Button` |
| `tests/fight.rs:21,40,44` | `Interaction::Pressed/None` | new `set` signature |
| `tests/actions.rs:123,132,135,146,150` | `Interaction::Pressed/None` | new `set` signature |
| `tests/combat.rs:38` | `query_filtered::<&Choose, With<Button>>()` | the new `Button` |
| `tests/combat.rs:43` | `set(app, &Choose(act), Interaction::Pressed)` | new `set` signature |

**Risk:** in 0.19 the old `Button` requires `Interaction`, and `set` asserts that the marked button
has one. If the new `Button` does not bring `Hovered`/`Pressed` with it, `controls::button` must
insert them explicitly.

## 3. Runtime and visual checks (no compile error, possible difference)

### `omnis-vector`
- **Tonemapping:** compare the 3D view's line glow and bloom, and the fight screen's, against the
  baseline. Banding in faded lines is the likely place for a difference. Check the log for the
  None-with-dither warning (there should be none after §2.1).
- **UI rendering is now retained** (re-extracted only for changed nodes). Confirm each of these
  still updates on screen:
  - the minimap image mutated in place (`src/shell/minimap.rs:99–102`);
  - `Display::None`/`Flex` toggles (`src/shell/combat.rs:120–128` `give_way`; `src/shell/panel.rs:195,198`);
  - despawn-and-rebuild of children (`src/shell/combat.rs:225`, `src/shell/panel.rs:191`);
  - hover shades on `BackgroundColor` (`src/shell/controls.rs:221`).
- **Text:** unaffected. Every `TextFont` sets `FontSize::Px` (`controls.rs:118`, `panel.rs:161`,
  `hud.rs:32,100`, `combat.rs:336`), so the new default `Rem(1.)` never applies.

### `omnis-app` (the Sprite backend moved to `Mesh2d` + `SpriteMaterial`)
No line changes: `Sprite` only gains `alpha_mode` (default `Blend`, as before), and there are no
`Sprite { .. }` literals. Check:
- **Same-Z draw order** (the guide says it may differ). `viewport.rs:74–79` gives each op
  `z0 + i*0.001` on layer bases 0.5 / 1.0 / 2.0 / 10.0 / 20.0. Ties occur only past 1000 ops in a
  layer. Confirm the maximum op counts of `plan::viewport` and `plan::automap_window`.
- **Transparent overlay:** `ui.rs:185–191`, at Z 50 over the viewport.
- **Images resized at runtime:** `ui.rs:217–227` and `pixel.rs:134–143`, with `custom_size: None`.
  The quad must follow the new size. Resize across size classes, including the ultrawide wing.
- **Image swapped after spawn:** `assets.rs:80–87` (the magenta placeholder for a missing pack image).
- **RenderLayers:** the viewport and UI on `PIXEL_LAYER` (`ui.rs:189`), the canvas on `WINDOW_LAYER`
  (`pixel.rs:82`). Nothing should be doubled or drawn unscaled.
- **Nearest sampling** (`main.rs:160` `ImagePlugin::default_nearest()`) still crisp under the
  sprite material.
- **Camera2d now defaults to `Linear`** (`pixel.rs:66,83`; no `Hdr`, so the guide says no
  change). Pixel-diff one canvas screenshot against the baseline.
- Sprite call sites, for reference: `viewport.rs:80,85,90–95`, `ui.rs:186`, `pixel.rs:82`,
  `assets.rs:85–86`.

## 4. Open questions to settle at the first compile
- The facade paths of `ui_widgets::Button`, `picking::hover::Hovered` and `ui::Pressed`, and the
  Cargo feature that provides them.
- Whether the new `Button` brings `Hovered`/`Pressed` (or picking) as required components.
- The press-edge idiom in 0.20's `button.rs` example (§2.2).
- Whether `bevy::render::view::Tonemapping` is the facade path (optional move).
- Whether the `2d` feature group still resolves without curves, and which `png` version 0.20 pulls.

## 5. Guide sections checked and found not to apply
These need no change: BSN syntax; the observer bundle generic (`omnis-app/src/socket.rs:414`
`On<ScreenshotCaptured>` has none); WESL shaders; ScreenSpaceTransmission; flat pointer events;
FontSource; `MeshAabb`; the `bevy_shape` and `bevy_curve` splits; `Val::Em`/`Rem` (no `resolve`
calls; `Node` now requires `EmSize`, which is automatic); OIT; exclusive systems unified;
EditableText/TextInput; `CalculatedClip`; TextReader; ReflectFromPtr (both); `meta_transform`;
Escape in text input (our Escape reads `ButtonInput<KeyCode>`/`KeyboardInput`); `MeshTag`;
CustomAttributes; `Font::from_bytes`; DownsampleShaders; TextScroll; `resolve_font_source`;
UnpreparedBindGroup; WorldQuery defaults; macOS activation (`focused` stays `true`:
`omnis-vector/src/main.rs:58`, `omnis-app/src/main.rs:161–174`); `with_luminance`;
RenderDebugOverlay (off by default); FilteredResources; weak ordering of built-in sets (our
`.chain()`s are our own `Update` systems); `Entity::PLACEHOLDER` (ours are input-message window
fields, not on the guide's list); boxed error variants; ComponentId constants; extraction generic
over worlds; `Name` from `&str`; `Ptr::as_ptr`; the `SpriteMaterial` rename; `define_label!`;
the Feathers cursor module; `MainEntityHashMap`; FeathersNumberInput; octahedral shader utils;
`FromType`; `AssetId::invalid`; depth/stencil attachments; ScheduleBuildSettings;
FeathersColorPlane; `FocusCause::Auto`; Atmosphere; CompressedImageSaver (not enabled);
`to_dynamic`; WgpuWrapper; `NextState::set_if_neq` (we call `set`); TypeId maps; QueryManyIter;
`Access::reads_and_writes`; compositing space; `DeferredWorld::query`; `SettingsGroup`;
BorderRadius; ShaderBuffer; render-world window data; contextual theming; BRP `schedule.graph`;
RenderAppChannels; `ComponentInfo::id`; `SortedCamera::hdr` (both `omnis-vector` cameras are
`Hdr` and only one is active at a time; `omnis-app`'s cameras target different render targets).
