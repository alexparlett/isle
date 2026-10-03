# Brainstorm

Evidence: `docs/research/rust-desktop/` — `isle-v1-inventory.md` (what v1
does), `smithay-and-gaming.md` (compositor and gaming stack),
`ui-and-services.md` (UI rendering, toolkits, services), `shell-ui-spike.md`
(Freya against Slint, measured), `macos-model.md` (global menu, Cmd
translation, macOS behaviours against v1).

## Locked

Every locked decision, with its reasoning and the alternatives rejected,
is in `docs/DECISIONS.md`, the one authoritative copy. Index:

- B1 · Own compositor on Smithay, not a fork of an existing compositor.
- B2 · v1 parity, re-planned (revised).
- B3 · Fresh visual design.
- B4 · Docs as a feature-plan program.
- B5 · Smithay's built-in X11 window manager for Xwayland.
- B6 · Our own Smithay fork, pinned, carrying the async page flip PR.
- B7 · Freya, through the `alexparlett/freya` fork, draws the shell and the apps.
- B8 · Stacking with snapping, layout as a per-workspace module.
- B9 · One event loop, blocking work on a service worker.
- B10 · The compositor owns settings; KDL on disk; tokens reload live.
- B11 · Services are shared crates on zbus; audio through pipewire-pulse.
- B12 · The desktop takes the system's prompt and tray roles, and locks itself.
- B13 · Our own portal backend; the shell draws the picker.
- B14 · Built-in widgets, extended by data providers.
- B15 · The name stays Isle.
- B16 · Built in this repository on `feature/rust-desktop`.
- B17 · North star: a macOS-style desktop on Arch (CachyOS).
- B18 · The island at rest, the global menu on demand (revised).
- B19 · No Dock (revised).
- B20 · App-centric windows.
- B21 · Cmd everywhere.
- B22 · No console mode of Isle's own.
- B23 · HDR is in scope, after Isle replaces v1.
- B24 · How attention is presented is the design process's call.
- B25 · Window chrome.
- B26 · Where things run.
- B27 · Settings in practice.
- B28 · Roles and protocols, completed.
- B29 · Delivery.
- B30 · Keys and games, completed.
- B31 · Frost, as data.
- B32 · Attention, presented.
- B33 · Glass, and where it may go.
- B34 · Readings of the principles.

## Open

None. The 21 points the intent drafts raised were answered as proposed and
locked as B24–B30.
