# TODO

## PRD (2026-09-11)
- [x] Review skeleton files and reference docs
- [x] Resolve the eight foundational decisions with the owner (PRD §6)
- [x] Extract MM2 mechanical structure from the C64 manual (PRD §8.1)
- [x] Verify Bevy 0.19 facts (PRD §11.1)
- [x] Draft PRD v0.1
- [x] Owner review of PRD v0.1; rules inverted to SRD spine (D1), decisions D10–D19 recorded
- [x] Resolve §14 open questions (three remain, none blocking)
- [x] Derive ARCHITECTURE.md from the PRD (v0.2, owner-reviewed 2026-09-12)
- [ ] Fix README.md typos and align it with the PRD vision paragraph

## Architecture (2026-09-11)
- [x] Decide MCP approach: own minimal MCP, stdio bridge binary to a game localhost socket
- [x] Decide rules expression: tiny in-house language first, revised to Rhai in review (A4)
- [x] Research: MCP spec and transports; Bevy 0.19 headless, assets, UI, structure; dependency versions
- [x] Decide editor UI toolkit with owner: bevy_egui for the editor, bevy_ui for the game
- [x] Write ARCHITECTURE.md v0.1
- [x] Owner review round 1: A4 changed to Rhai, A11 confirmed; §5 rewritten as Rhai host; §4.4 subjective time; A14 RNG seed strategy proposed
- [x] rhai pinned at 1.26.1 by owner exception (re-audit 2026-10-10)
- [x] A14 seed strategy approved
- [x] ARCHITECTURE.md v0.2 reviewed by owner
- [ ] Plan Phase 0 tasks (after compact)
- [ ] Sentrux review once code exists

## Phase 0 — Foundation (not started; see PRD §13)
