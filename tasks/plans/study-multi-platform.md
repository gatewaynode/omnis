# Study: multi-platform

Status: **study, nothing built** (W0.5, owner 2026-10-21). Read-only: what the documents say, what the code
constrains, the cost per platform now and at M12, proposed PRD rows, and questions for the owner. Dates are
against 2026-10-21. Its siblings (accounts, multiplayer) take D29 and D30; this study proposes D31.

## 1. What the documents say today

- PRD D5: "Desktop single-player (macOS, Linux, Windows); co-op later"; the rejected alternative was "Add WASM".
- PRD §2: "Explicit non-consumers for v1: mobile players, browser players, multiplayer groups." PRD §5: "Browser
  or mobile builds" is a non-goal. PRD §14, last bullet: "browser builds ... Owner to state which are rejected
  outright and which are deferred." PRD §13 Later: "Additional platforms"; since 2026-10-21 "further platforms
  are under study ... each enters this document as a decision row".
- PRD D9: Bevy latest stable, exempt from N-1; D20: the owner's 5120x1440 ultrawide first, 16:9 720p to 4K
  supported; §11.1: cross-platform determinism through integers, "all randomness from seeded, counted generators".
- ARCHITECTURE §11 tier 3: replay fingerprints "run on macOS and Linux in CI"; A5: integers "cross-platform
  determinism without doubt". §14: "CI (GitHub Actions, macOS and Linux; Linux deferred by owner 2026-09-12 until
  needed) ... replay fingerprints compared across the two runners"; "Windows builds in CI once Phase 1 is
  playable"; no `.cargo/config.toml`, the platform's default linker. §16 Phase 5: packaging.
- tasks/TODO.md W5 M12: "packaging with the platform's settings directory ... Windows and Linux CI".
- presentation-PRD §9: Bevy features `2d`, `png`, `bevy_pbr`, `ui`; no `3d` group. presentation-ARCHITECTURE §3:
  fonts compiled in with `include_bytes!`; §10.3: "Offscreen is not headless": `--screenshot` needs a GPU adapter,
  the tests under `MinimalPlugins` need none; §15.4: `.omnis/` is the development path, the platform's config
  directory arrives with packaging (M12).
- tasks/BACKLOG.md: `webgl2` enabled transitively through `default_platform` (found 2026-10-20, "presumed inert");
  `ttf-parser` 0.25.1 RUSTSEC-2026-0192 reached only on Linux through winit's Wayland decorations; "whether
  GitHub's macOS runners expose a Metal device for offscreen capture is unverified" (answered in §2 below).
- tasks/plans/bevy-0.20-migration.md: 0.20.0 published 2026-10-08, not before 2026-11-07.
- Owner rules: keyboard and mouse first, other controls stretch goals (2026-10-03); no function-key bindings
  (macOS); a display decision is measured on the ultrawide (agreements.md).

## 2. What the code constrains and already gives

- **The lint is the simulation's portability guarantee** (`scripts/lint-sim.sh`): no `f32`/`f64`, no hashed
  collections, no `std::time`/`std::thread`/`std::net`, no file I/O outside `omnis-data`, seven crates `#![no_std]`,
  rhai `no_float` + `only_i64`, no Bevy edge in the resolved graph. Integer arithmetic is identical on every Rust
  target, so a fingerprint equal on two platforms is a test of the toolchain and the host, not of the rules.
- **Platform-specific lines in the tree: none in production code.** Every `cfg(unix)` creates a symlink in a test:
  `crates/omnis-data/tests/files.rs:74,78,79,104,108`, `crates/omnis-data/tests/bad_packs.rs:303,307`,
  `crates/omnis-vector/tests/game_io.rs:30,32`, `crates/omnis-vector/tests/settings.rs:159,165`. On Windows those
  tests are compiled out (symlink creation needs a privilege there); the refusals they prove go untested, not wrong.
- **Symlink refusal is portable**: `crates/omnis-data/src/ron_io.rs:52-55` uses `std::fs::symlink_metadata`, which
  reports Windows reparse points too. `list_under` (ron_io.rs:138-178) never follows a link. The path rule
  (`crates/omnis-data/src/limits.rs:97-111`) admits ASCII and `/` only and refuses `Component::Prefix`, so a
  Windows drive letter in content is refused and `/` paths join correctly through `Path::join` on every platform.
- **Fonts compiled in**: `crates/omnis-vector/src/shell/fonts.rs:47,51` (`include_bytes!`, Inter and Alegreya
  Sans). No font file is looked up on the host; nothing in text rendering is platform-specific.
- **Settings and save paths are the working directory**: `main.rs:118` reads settings under `Path::new(".")`;
  `shell/screen.rs:228-230` `root()` is `"."`; `shell/session.rs:68-74` names `.omnis/quick.ron`,
  `.omnis/settings.ron`, `.omnis/vector-session.ron`; worlds under `.omnis/worlds/<id>/`. Nothing reads `HOME`,
  `XDG_*` or `APPDATA` (grep: zero hits). A shipped build on any platform needs one root-resolution function (§5).
- **The dev socket** (`shell/socket.rs:24,30,32,115`): `std::net::TcpListener` on `127.0.0.1:0`, address file
  `.omnis/dev.addr`, feature `devtools`, compiled out with `--no-default-features`. Portable on the three desktops;
  impossible on web (no TCP); the MCP bridge (`omnis-mcp`) is desktop-only by nature.
- **The offscreen capture** (`main.rs:66-89`, `shell/capture.rs:148`): `DefaultPlugins` less `WinitPlugin`,
  `ScheduleRunnerPlugin`, a `Bgra8UnormSrgb` render target; needs a wgpu adapter (Metal here, Vulkan on Linux,
  DX12 on Windows). **GitHub's macOS runners do expose Metal**: Bevy's own `.github/workflows/example-run.yml` job
  `run-examples-macos-metal` runs on `"macos-14"` and uploads screenshots; its Linux job runs on `ubuntu-latest`
  under `xvfb-run` with Mesa's Vulkan driver (kisak PPA); Windows on `windows-latest` with `WGPU_BACKEND=dx12`.
  This closes BACKLOG's open question: CI can take offscreen captures on all three, with the Linux one on a
  software rasterizer (slow, not a performance reading).
- **Entropy** (`shell/screen.rs:280-281`): `std::time::SystemTime::now()` in the client, outside the lint's reach;
  fine on desktops; on `wasm32-unknown-unknown` it panics ("time not implemented").
- **The window** (`main.rs:91-111`): `BorderlessFullscreen(Current)` by default, 1600x900 windowed, AutoVsync
  or not. Keys are physical `KeyCode` (`shell/movement.rs`, `combat.rs`, `screen.rs`), so a layout change does
  not move WASD; no function keys. No `#![windows_subsystem]`, so a Windows build opens a console beside the
  window; `eprintln!` notes (`main.rs:119`) go to it.
- **Lock pins reached per platform** (`Cargo.lock`): winit 0.30.13; wgpu 29.0.4; x11rb 0.13.2, wayland-client
  0.31.15, sctk-adwaita 0.10.1 -> owned_ttf_parser/ttf-parser 0.25.1 (Linux only); objc2 0.5.2 and 0.6.4 (macOS);
  windows-sys 0.52.0/0.59.0/0.60.2/0.61.2 (Windows; on the duplicates allow list already). Bevy 0.19.1's
  `default_platform` carries `wayland`, `x11`, `webgl2`; `webgpu`, `android-native-activity`,
  `android-game-activity` are opt-in (docs.rs feature table).
- **CI today** (`.github/workflows/ci.yml`): one job on `macos-latest` (macOS 26, arm64 since the runner-images
  README's current mapping): fmt, lint-sim with self-test, duplicates, test-modules, clippy x3, `cargo test
  --workspace`, both golden replays through `omnis-cli`, pack validation. The comment names what Linux's return
  restores: the OS matrix, apt packages, fingerprint artifact upload, the compare job. No run time is recorded in
  the tree (`tasks/knowledge/verification.md` gives the owner's machine: the test step 549 s on a full rebuild;
  CI is reported by the owner, `gh` is not installed). GitHub minute multipliers: Linux 1x, Windows 2x, macOS 10x
  (GitHub billing docs); list rates $0.006, $0.010, $0.062 per minute (2026 runner pricing reference). Minutes are
  free on a public repository; the study does not know whether `gatewaynode/omnis` is public.

## 3. Per platform

### Linux (desktop)
- **Build today**: expected yes, unverified; no platform line in the tree; link needs `g++ pkg-config libx11-dev
  libasound2-dev libudev-dev libxkbcommon-x11-0` (Bevy's linux_dependencies.md) plus `libwayland-dev
  libxkbcommon-dev` for the `wayland` feature. Tests are headless (`MinimalPlugins`), so no X server is needed.
- **Wayland and X11**: both compiled in by `default_platform`; winit picks at run time. The ttf-parser advisory
  (RUSTSEC-2026-0192, published 2026-06-28) is informational: unmaintained, no patched version, successor skrifa.
  It reaches the Linux build through `sctk-adwaita` (client-side decorations on Wayland); dropping the `wayland`
  feature removes it and leaves XWayland; waiting for 0.20's winit is the other path. Not a vulnerability.
- **Cost now**: one job of about 15 lines (apt line, the same steps, upload `fingerprint` artifacts) plus a compare
  job (~10 lines, `diff` on two files); zero source changes expected. Minutes at 1x, likely cheaper per run than
  the macOS job. Cost deferred to M12: the same lines, plus whatever Linux-only breakage the W1-W4 code accrues
  unseen. Risk of waiting: ARCHITECTURE §11 tier 3 promises a two-platform fingerprint today and CI runs one;
  shader and winit differences in W2 (the mesh renderer, the look) land unverified on Vulkan.
- **Earliest phase**: now (W0).

### Windows
- **Build today**: unknown; nothing in the tree is Unix-only in production, so a clean build is likely. MSVC Build
  Tools are preinstalled on `windows-latest` (Windows Server 2025); no `.cargo/config.toml` so the MSVC linker
  (`link.exe`) is used; Bevy's setup notes offer `rust-lld.exe` as a faster option. Path separators: content
  paths are `/` by rule and are joined with `Path::join`; `.omnis/...` literals are joined the same way; fine.
- **The console-less binary**: `#![windows_subsystem = "windows"]` on `main.rs` (one line, `cfg_attr` on
  `not(debug_assertions)` so dev builds keep stderr); then the start-up notes need a log file or a notice on the
  title (a packaging item, §5). Function keys are not bound and keys are physical, so nothing in the keyboard
  layer changes. The symlink tests are compiled out there (above).
- **Cost now**: a job of the same shape at 2x minutes; `cargo test` on Windows is the slow part (link times).
  Cost deferred: the same job; the risk is DX12 shader divergence found after W2's renderer work. ARCHITECTURE §14
  already says "once Phase 1 is playable", and Phase 1 closes with W1 (editor v1).
- **Earliest phase**: W1's close, before W2's renderer steps (the existing sentence, kept).

### Web (WASM)
- **What the simulation already allows**: seven `no_std` crates and `omnis-expr` (rhai builds on wasm32) compile
  for `wasm32-unknown-unknown` as they stand; the lint's bans (no clock, threads, net, fs, floats) are exactly the
  wasm constraints. `omnis-data` is the one std crate: `load_packs(&[&Path])` (`loader.rs:158`) reads directories;
  a browser needs a reader over bytes (packs fetched or embedded), the same seam D28's `load_save(reader)` already
  has for saves.
- **What the client needs**: `webgl2` is already on (transitively); `webgpu` is a feature switch. Drop `devtools`
  (no TCP); replace `SystemTime` entropy with a host hook; saves to browser storage (IndexedDB) instead of
  `.omnis/`; single-threaded (`multi_threaded` off); Bevy's `wasm-release` profile and `wasm-opt`; a page and
  `wasm-bindgen`. The dev socket, MCP bridge and screenshots stay desktop.
- **Determinism in a browser**: the sim is integer-only, so a wasm fingerprint equals the desktop's; nothing in
  the promise depends on the host. A CI job could run the goldens under `wasm32-wasip1` on Linux (1x) as a fourth
  "platform" for the fingerprint, cheaply, without a browser.
- **Cost**: an `omnis-data` byte reader (~100 lines), a client host layer (~200 lines), a page, a feature set;
  WebGL2/WebGPU are "work in progress" on Bevy's own example tester. Cost deferred: none that compounds while
  `omnis-data` keeps the reader seam. Earliest phase: a demo at W3 (a seed for modders to see) at the soonest; a
  shipping target after W5.

### Mobile (iOS, Android)
- **What exists**: Bevy's example tester lists iOS and Android as supported; features `android-native-activity`
  and `android-game-activity`; `bevy_main` for the entry point; Xcode and the Apple Developer Program ($99/yr)
  for iOS, Android Studio and the NDK for Android; the same `omnis-data` reader seam as web (app bundles).
- **Without touch** (the owner's stretch goal) a mobile Omnis is a keyboard-and-mouse game on a tablet with a
  keyboard; the interface is laid out for 1280x720 and up (`settings::fits`), so phones are out on size alone.
- **Cost**: two toolchains, two stores, a touch layer, tablet layouts. Deferred cost: none. Earliest phase: none;
  it enters only after touch becomes a goal.

### Steam Deck
- A Linux x86_64 target with a controller and a 1280x800 display: the Linux build runs on it (or the Windows
  build under Proton); 1280x800 holds the 1280x720 layout at 100% scale. Steam Input can map a keyboard and mouse
  onto the pad, so it is playable without a gamepad layer, not "Deck Verified". Controllers are stretch goals, so
  no code until then; Steam's Linux page: Vulkan preferred, the Steam Linux Runtime chosen per branch, SDL2 backs
  Steam Input. Earliest phase: W5 with the Linux package; verification after a gamepad layer, if ever.

## 4. Comparison

| Platform | Build today | CI now | Cost to add | Cost to defer | Recommended phase |
|---|---|---|---|---|---|
| macOS | yes | yes (10x minutes) | 0 | 0 | now (as is) |
| Linux | unverified, likely | no | ~25 workflow lines, apt line, 1x minutes, 0 source | the same, plus unseen breakage from W1-W4; §11 tier 3 unmet meanwhile | now (W0) |
| Windows | unknown, likely | no | a job at 2x minutes, 1 line (`windows_subsystem`), a log for notes | the same; DX12 differences found after W2 | W1's close |
| Web | no (omnis-data reads dirs; SystemTime; TCP) | no | ~300 lines, a page, a feature set; a wasip1 golden job is cheap | none that compounds if the reader seam is kept | demo W3 at soonest; ship after W5 |
| Mobile | no | no | two toolchains, stores, a touch layer, tablet layouts | none | none until touch is a goal |
| Steam Deck | as Linux | as Linux | 0 beyond Linux; a gamepad layer for verification | none | W5 with the Linux package |

## 5. Packaging (M12)

- **Per desktop**: macOS: a `.app` bundle, Developer ID signing and notarization (Steam requires 64-bit and
  notarized since 2019-10-14, and no `app-sandbox` entitlement; itch accepts unnotarized builds, Gatekeeper then
  warns); $99/yr. Windows: a zip or an installer, the console-less binary, optional Authenticode (SmartScreen
  warns without it). Linux: a tarball with the binary beside `packs/`; Steam Linux Runtime chosen per branch;
  Vulkan preferred. itch's `butler` pushes one channel per platform; Steam takes depots per platform.
- **The packs folder**: today `packs/base` and `packs/test` are paths given on the command line
  (`--pack DIR`, `omnis-mcp --root`); a package needs a default pack root beside the executable (macOS: inside the
  bundle's `Resources`), resolved once at start.
- **The settings directory** (presentation-ARCHITECTURE §15.4, left to M12): the three conventions are macOS
  `~/Library/Application Support/Omnis`, Linux `$XDG_CONFIG_HOME` or `~/.config/omnis` for settings and
  `$XDG_DATA_HOME` or `~/.local/share/omnis` for saves and worlds, Windows `%APPDATA%\Omnis`. An own
  implementation is `std::env::var` on `HOME`/`USERPROFILE`, `XDG_*`, `APPDATA` with `cfg(target_os)` per
  convention: a few dozen lines, one `cfg` per platform, testable with set variables. The `dirs` crate is 7.0.0
  (2026-09-05, 46 days old; N-1 is 6.0.0 of 2025-01-12) and pulls `dirs-sys` and, on Windows, a `windows-sys`
  that may duplicate again; the self-supporting rule (CLAUDE.md) and the size of the job both say write it.
  Keep `.omnis/` under the working directory as the development path and when a `--root` is given.

## 6. Proposed PRD rows (blanks are the owner's)

- **D31 | Platforms and CI** | Decision: desktop macOS, Linux and Windows are the v1 targets (D5 stands); CI runs
  macOS and Linux from W0 with the replay fingerprint compared across them, and Windows from W1's close (Phase 1
  playable); web is deferred to ____ (recommended: a demo at W3 at the soonest, a target after W5) and the
  `omnis-data` reader seam is kept open for it; mobile is ____ (recommended: deferred without a milestone, entering
  only after touch is a goal); Steam Deck ships as the Linux build at M12, verified only after a gamepad layer.
  Packaging and the settings directory at M12 with an own path helper, no `dirs` crate. | Alternatives: all CI at
  M12 (cheapest in minutes, one-platform fingerprint until then); Windows now (2x minutes before any Windows
  code exists); reject web outright. | Rationale: the lint makes the simulation portable by construction, the
  client is Bevy's reach (Linux/Vulkan and macOS/Metal supported on Bevy's own tester, Windows/DX12 and web
  "work in progress"); a Linux runner is the cheapest second platform for the determinism test §11 promises.
- **§2**, last line as it should read: "Explicit non-consumers for v1: mobile players, browser players,
  multiplayer groups. Browser players are deferred (D31, §13 Later); mobile players are ____ (deferred without a
  milestone / rejected outright)."
- **§5**: "Browser or mobile builds (v1; browser deferred by D31, mobile ____)."
- **§13 Later**: "Additional platforms: a browser build after W5 on the `omnis-data` reader seam (D31); mobile only
  after touch controls become a goal; Steam Deck as the Linux build."
- **§14**, last bullet: resolved for platforms by D31; language-model content remains.
- **ARCHITECTURE §14 CI sentence** as it should read: "CI (GitHub Actions): macOS and Linux (Linux restored
  2026-10-__, D31) run fmt, clippy with `-D warnings`, the simulation lint, `cargo test --workspace`, both golden
  replays, and pack validation; each uploads its replay fingerprints and a compare job fails on any difference
  (§11 tier 3). Windows joins at the close of W1 (Phase 1 playable) with `WGPU_BACKEND=dx12` for captures."

## 7. Questions for the owner

1. Linux CI now, or at M12 as TODO W5 says? Recommended: now; it is the §11 tier 3 promise, 1x minutes, no
   source change.
2. Windows CI at W1's close (the ARCHITECTURE sentence) or now? Recommended: W1's close; nothing Windows-only
   exists yet and the minutes are 2x.
3. Is the repository public (minutes free) or private (a 2,000-minute monthly allowance at the multipliers)?
   Decides whether a Windows job's cost is real. Recommended: state it in ARCHITECTURE §14.
4. Web: deferred or rejected? Recommended: deferred to after W5 with a demo possible at W3; keep the reader seam.
5. Mobile: deferred without a milestone, or rejected outright? Recommended: deferred without a milestone, worded
   "enters only after touch controls become a goal"; agreements.md keeps rejections to owner-stated ones.
6. The `wayland` feature: keep (advisory noted, informational) or drop until 0.20? Recommended: keep; note the
   advisory in ARCHITECTURE §13; review at the 0.20 move.
7. The settings directory: an own helper (recommended) or the `dirs` crate (7.0.0, 46 days old, passes the age
   rule, fails the self-supporting rule)?
8. Should the offscreen capture join CI now that macOS runners are known to expose Metal (Bevy's own job)?
   Recommended: not yet; a capture is acceptance on the owner's machine until the look settles at W2.

## 8. Recommendation

Add the Linux job now: it is the cheapest runner GitHub has (1x against macOS's 10x), the tree has no Unix-only
production line so no source change is expected, and the project's determinism test is written as a two-platform
fingerprint comparison that CI has not run since 2026-09-12. The honest cost is about 35 workflow lines, one
apt line, and a second full compile per push (Linux compiles are not faster than the owner's Mac Studio, so a
run is minutes, not seconds, and the owner reads CI by report). Windows should join when W1 closes, as
ARCHITECTURE §14 already says, which is also before W2's renderer work makes DX12 differences likely; it costs 2x
minutes and one `windows_subsystem` line. Web stays a horizon, deferred rather than rejected, because the lint
has already done most of the porting work and only `omnis-data`'s directory reader, the entropy call and the dev
socket stand in the way; a `wasm32-wasip1` golden job on Linux would prove the fingerprint on a fourth target for
almost nothing if the owner wants it. Mobile has no value until touch is a goal; the Steam Deck is the Linux
build. Packaging stays at M12 with an own path helper.

Sources (read 2026-10-21): bevyengine/bevy `.github/workflows/example-run.yml`; example-runs.bevy.org/about;
bevy.org setup page and `docs/linux_dependencies.md`; docs.rs bevy 0.19.1 features; bevy.org/news/bevy-0-20;
actions/runner-images README; GitHub billing docs and the 2026 runner pricing reference; rustsec.org
RUSTSEC-2026-0192; crates.io API for `dirs`; Steamworks platform and Linux pages; itch devlogs on notarization.
