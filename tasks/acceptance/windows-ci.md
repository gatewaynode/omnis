# Acceptance: Windows CI, advisory (W1's close, PRD D31)

Built 2026-11-08 at the owner's order ("Windows CI first, advisory"), in two commits after the entrances and
exits enhancement closed W1. Owner's choices while planning: the `windows_subsystem` attribute now (not deferred
to M12's packaging); a failed start's reason kept in a file, no `unsafe` (an `AttachConsole` call was declined;
the client library forbids unsafe code).

## What was built
- **Commit 1, pretty RON ends its lines with LF on every platform.** `ron_io::to_string` sets ron's
  `new_line` to `\n`; ron's default is CRLF on Windows, and the pack hash, the world id and the world fingerprint
  are taken over that text, so a pack sealed or a fingerprint computed on Windows would have differed from
  macOS's. Found by the sweep below, not by a run. `round_trip.rs` asserts no `\r`; on macOS the default already
  held, so the assertion bites on the Windows job, with the golden replays themselves.
- **Commit 2, the job.** `.github/workflows/ci.yml`: a third job `windows` (`windows-latest`, `continue-on-error`,
  the Linux job's steps without the apt step, every step under `shell: bash` since PowerShell runs no `.sh` and
  reads only a block's last exit code), and `workflow_dispatch` so the workflow runs by hand on a branch.
  `.gitattributes` with `* text=auto eol=lf`: git on Windows checks out CRLF by default, and the seals, the golden
  fingerprints, the frozen world folders' bases and `scripts/duplicates.allow` are hashed or compared as text
  (shown on a scratch clone with the packs converted to CRLF: `validate` and `replay` both report "the content
  changed since version ... was sealed" for `packs/base` and the fixture). The owner's checkout is untouched by
  the line (`git status` clean; the three CRLF licence texts under `assets/` stay as they are). `main.rs`:
  `#![windows_subsystem = "windows"]`, and the lines the client says before a window is up go through `said`
  and `fail`, which also write `.omnis/last-error.txt` (`shell/failure.rs::record`). Test: the binary run on a
  pack that is not there exits non-zero, says the pack's name on stderr and leaves it in the file.
- Type-checked from the Mac for `x86_64-pc-windows-msvc` as far as a Mac can, the W0.6 precedent: the Bevy-free
  crates (`omnis-core` to `omnis-sim`, `omnis-cli`, `omnis-mcp`, `omnis-icon`, all targets). The client cannot
  be: `blake3` in Bevy's asset tree builds MSVC assembly and the Mac has no `ml64.exe` (the runner has MSVC).
  `shell/failure.rs` is std-only and compiled on macOS; the attribute is checked by the first run. Not run: the
  job itself.

## Steps (the owner, on GitHub)
1. Push the branch. In the Actions tab choose CI, Run workflow, the branch `W1-authoring-tools` (or open the PR).
2. Read the `windows (advisory, never blocks)` job. Expected green: toolchain, fmt, the three scripts, clippy
   three times, `cargo test --workspace`, both replays with their macOS fingerprints (`0370ab07b628f193`,
   `c81c08bde3a070e9`), pack validation. Expected time: the first run builds Bevy cold, likely 30 to 45 minutes;
   the cache shortens the next.
3. A red step is a Windows fault: paste its log lines here and it gets a fix in its own commit with a test.
   Advisory means the required macOS check and the merge do not wait on it meanwhile.
4. `macOS` and `linux` stay as they were; the `.gitattributes` must change nothing in either.

## Known and expected
- On Windows a terminal that runs `omnis` sees no stderr (the GUI subsystem); `.omnis/last-error.txt` holds the
  last line said. The capture's `--timing` report is lost there too (captures stay on the owner's Mac, W2).
- The five `#[cfg(unix)]` symlink tests are skipped on Windows: symlink refusal is proven on macOS and Linux.
- Whether the Windows job blocks is the owner's call later, as the Linux job's is.
