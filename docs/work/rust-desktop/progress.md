# Progress

## 2026-10-03

- P1 done. Stage 6 reviewed every surface for critique, accessibility and
  copy (298 findings, all applied); the contrast script grew to cover
  focus rings, badges, hover states, labels on fills and kind colours, and
  passes. The user signed off DESIGN.md at gate 6 ("lock it in for now").
  B31–B34 record Frost as data, how attention is presented, where glass
  may go, and the readings of the principles.

- P1 gate 5, wave C passed: Notification Centre and widgets, Mission
  Control and the window surfaces, Files, Monitor, Keychain and Cleaner,
  drawn on the wave C board. A colour-vision check failed stage 4's graph
  colours; a new four pass it, and Cleaner's ten kind colours were checked
  the same way. Stage 5 is complete.

- P1 gate 5, wave B passed: specs for banners, Focus, Control Centre's
  modules, the lock and prompts, Spotlight, capture, Settings and the app
  conventions, reconciled and drawn on the wave B board. The lock adds its
  own wash so its text passes; prompts are one glass frame over a scrim
  with an armed moment; sliders on glass fill with the text colour.
  About seventy tokens added; every glyph the specs name is vendored.

- P1 gate 5, wave A passed: specs for the island, menus, window chrome,
  the switcher and the wallpaper, drawn on the wave A board with a shared
  scene kit. The strip and the revealed bar are both 48 so their centres
  align; colour on the island sits in solid wells; menus are glass in the
  shell, raised in windows, solid over glass. About forty tokens added;
  the contrast check covers the island. DESIGN.md has its first surfaces.

- P1 gate 4 passed: Frost made into the system. The token file grew roles
  for every surface, eight accents, the glass range, macOS's text styles
  in Inter, the concentric corner rule, five springs, a terminal palette
  and the icon group; a contrast script checks every pair in both
  appearances and under Increase Contrast. Phosphor, Papirus and Bibata
  chosen. Research found Isle must serve the Settings portal (B13) and
  that Qt under the GTK platform theme bypasses Isle's file panels (P14).
  DESIGN.md's first draft written.

- P1 gate 2, batch 3 passed: Notification Centre unfolding from the pill
  with the date, month and agenda; the widgets dashboard as one canvas
  per screen on Ctrl+Option+Cmd+W; Mission Control with a minimised tray
  and a short slide between desktops; Files without tags or Connect to
  Server, undo as a stack; Monitor with a GPU view; Keychain's reveal
  gated once on opening. Stage 2 is complete; the intent docs follow.

- P1 gate 3: the user chose Frost. Three directions (Frost, Slate, Dusk)
  were drawn on the harness, now able to switch directions, each from its
  own `tokens.kdl` through a shared kit of components. Batch 3 of the
  structure sheets ran beside it and is on its own board.

- P1 gate 2, batch 2 passed with every recommendation accepted: banners
  top right under the island's strip; Game Mode wins over a manual Focus;
  Control Centre a four-column grid of sixteen modules, media only in Now
  Playing, Game Mode a row of the Focus page; Cmd+Tab sets a prompt
  aside; Spotlight without an assistant, Quick Look on Cmd+Y; one capture
  thumbnail with Markup in Quick Look; the greeter in the user's look;
  System Settings in 28 panes with Game Mode folded into Focus and
  Displays. The batch 1 sheets and the intent docs follow.

- P1 gate 2, batch 1 passed. The first board (menu bar always shown, Dock
  at the bottom) was redrawn when the user preferred v1's minimal desktop:
  revision 1 keeps v1's pill with the clock and active glyphs, opens
  Control Centre from it, brings the app's menus down on demand, and drops
  the Dock for Spotlight's Apps grid and the switcher. Every
  recommendation accepted; standard menu items keep Apple's title case.
  B18 and B19 revised; shell, compositor, gaming, apps and platform intent
  docs, DESIGN, ARCHITECTURE, README and the roadmap (P6 island and menus,
  P7 switcher and launching) updated.
- P1 stage 2 started: the structure template, the harness
  (`p1/harness/`, served by `serve.py`, `p1-boards` in launch.json), and
  batch 1's sheets for window chrome, the menu bar, the island, the Dock,
  the switcher and the wallpaper, with the copy and competition sheets.

- P1 stage 1: research notes on macOS 27, other desktops and v1's
  lessons written; the interview took four rounds (both appearances dark
  first, frosted glass with a slider, Mac density, one picked accent,
  restrained motion, Dock at the bottom, the island a dark pill opening
  on hover, a full-screen widget dashboard kept). The brief passed
  gate 1 ("agreed for now").

- P1 planned (`p1-design.md`). The user chose HTML mock-ups first, then
  Freya from P3's preview mode on, and working files in the repo with
  each gate's board also published as an Artifact.

## 2026-10-02

- The user pointed out that presentation (the island's priority order and
  the like) belongs to a real design process, and that coding agents are
  no longer a feature. B24 rewritten to hand presentation of attention
  surfaces to P1; P1 rewritten as a six-stage design process; coding-agent
  references removed from DECISIONS; the intent docs are being split into
  capability rules and a "For the design process" list.

- B30 refined: the Alt/Super swap is for PC-layout keyboards and is
  suspended in game windows; Mac-layout keyboards need none.

- B24–B30 folded into the intent docs, ARCHITECTURE, DESIGN and the
  roadmap. A coverage audit checked both directions and fixed what
  disagreed (Game Mode wording, the island as the home of casts and
  activities, calendars as notifications, apps' Modes rows, roadmap
  briefs; P22 moved after cut-over). Nothing committed yet.

- The user accepted all 21 proposed answers; locked as B24–B30.

- B23: HDR in scope as P22 after M2. The 21 open points from the intent
  drafts are listed in `brainstorm.md` for the user.

- Intent docs drafted for all five areas. The user dropped Isle's own
  console mode (B22); gaming, shell, compositor and platform docs and the
  roadmap updated.

- The user set the north star (macOS-style on Arch) and parity scope:
  B2 revised; B17–B21 locked (menu bar with global menu and the island in
  the notch, Dock, app-centric windows, Cmd everywhere).

- Round 4 locked B13 (own portal backend), B14 (built-in widgets plus
  providers), B15 (Isle) and B16 (this repo and branch). Brainstorm
  complete.

- Round 3 locked B10 (compositor-owned settings, KDL, live tokens), B11
  (shared zbus service crates, pulse audio) and B12 (prompt, tray and
  lock roles).

- Round 2 locked B8 (stacking with snapping, layout per workspace) and
  B9 (one loop, service worker, panic boundaries, supervisor, watchdog).

- Locked B7: Freya through the user's fork for shell and apps. Round 2 of
  the spike (a 3440×1440 launcher) decided it; final numbers rerun one at
  a time on a quiet host. The user updates the fork to rc.8.

- Spike results in `docs/research/rust-desktop/shell-ui-spike.md`: both
  variants run the island at about +0.1–0.6 ms over the empty base and
  idle correctly; Freya wins text (colour emoji, font features) and GPU
  scaling, Slint wins the iteration loop (live reload 80–96 ms, state
  kept), maturity and VRAM. Large full-output surfaces untested. The base
  now paces by frame callbacks; the host itself paces new windows at
  30 fps (es2gears shows the same).

- Spike base built in `spikes/shell-ui/base`: Smithay 118e34f, winit
  backend, island shape sprung and drawn by GLES shaders, content clipped to
  it. Empty-content floor on the host: morph frame 0.87 ms p50, 1 idle
  frame in 10 s, RSS 145 MB / PSS 81 MB, 38 MiB VRAM. Freya and Slint
  variants handed to two agents in parallel.

- Recon: inventory of v1, Smithay and gaming research, UI and services
  research, in `docs/research/rust-desktop/`.
- Round 1 locked B1–B6. Shell UI rendering compared in depth (Freya on
  Skia, own scene graph, CPU-drawn); the user chose to spike Freya against
  Slint.
- Otto checked at d308f56: the compositor draws dock, exposé, switcher,
  OSDs and window chrome; bar, islands, launcher and lock are otto-kit
  clients, styled by the compositor through `otto-surface-style-unstable-v1`.
