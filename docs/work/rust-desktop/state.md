# State

- Branch: `feature/rust-desktop`. v1's docs, canvas and screenshots are
  removed here and remain on `main`.
- Decisions: B1–B30 locked, the full text in `docs/DECISIONS.md` (B2
  revised to v1 parity; B17 the macOS north star). The brainstorm is
  complete.
- Shell UI spike done (`spike-shell-ui.md`; results in
  `docs/research/rust-desktop/shell-ui-spike.md`). Its code in
  `spikes/shell-ui/` is throwaway.
- Intent docs: `docs/design/` (compositor, shell, gaming, apps, platform),
  drafted, with B22–B30 folded in.
- Roadmap: `roadmap.md`, P1–P21 in milestones M1 (runs in the VM), M2
  (replaces v1 on the host) and parity, then P22 (HDR, B23) after
  cut-over.
- Spine: `docs/ARCHITECTURE.md` (planned), `docs/DECISIONS.md`,
  `docs/DESIGN.md` (placeholder until P1). README and CLAUDE.md point at
  them.
- Blocking P3 onwards: the user is merging upstream Freya v0.5.0-rc.8
  into `alexparlett/freya` (fork at rc.4).
- Coverage audit done: every decision reaches its intent docs,
  ARCHITECTURE and the roadmap, and the docs agree with DECISIONS.
- Presentation is separated from capability: each intent doc ends with a
  "For the design process" list that P1 answers (B24 rewritten). B18's
  arrangement stays a decision.
- The planning docs are committed.
- P1 planned in `p1-design.md`: six stages with a user gate each;
  mock-ups in HTML on a shared harness that reads `tokens.kdl`, moving to
  Freya once P3's preview mode exists; boards also published as private
  Artifacts.
- P1 stage 1: the three research notes are written
  (`design-macos.md`, `design-references.md`, `design-v1-lessons.md`),
  the interview is done, and `p1/brief.md` passed gate 1.
- P1 stage 2, batch 1 passed gate 2 (`p1/structure/`, board
  `p1/structure-1.html`, published as a private Artifact) after revision 1:
  at rest only the wallpaper and v1's pill; menus on demand; no Dock. B18
  and B19 are revised and the intent docs, DESIGN and the roadmap follow.
- P1 stage 2, batch 2 passed gate 2 (board `p1/structure-2.html`):
  notifications, Focus, Control Centre, lock and greeter, prompts,
  Spotlight with the Apps grid, capture, app conventions and Settings.
  Every recommendation accepted; the intent docs follow. Next: batch 3
  (parity surfaces: Notification Centre and widgets, Mission Control and
  the window model's rest, Files, Monitor, Keychain, Cleaner, the
  dropdown terminal, PiP, the on-screen keyboard, clipboard and emoji),
  which can run beside stage 3 (directions).
- P1 stage 3 passed gate 3: direction A, Frost (board
  `p1/directions-1.html`, tokens in `p1/directions/`). Batch 3 passed
  gate 2 (`p1/structure-3.html`); every recommendation accepted, the
  dashboard on Ctrl+Option+Cmd+W, no Connect to Server. Stage 2 is
  complete.
- P1 stage 4 passed gate 4 (board `p1/system-1.html`): `p1/system/`
  holds `tokens.kdl`, its `schema.md` for P3, `contrast.py` (all pass),
  `components.css`, `icons.md` (Phosphor, Papirus, Bibata) and
  `theming.md`. `docs/DESIGN.md` has its first draft. B13 now includes
  the Settings portal. - P1 stage 5, wave A passed gate 5 (board `p1/surfaces-a.html`, specs
  in `p1/surfaces/`, drawing kit `p1/surfaces/scene.js`): the island and
  its panels, menus, window chrome, the switcher, the wallpaper.
- Wave B passed gate 5 (board `p1/surfaces-b.html`): banners, Focus,
  Control Centre's modules, the lock and prompts, Spotlight, capture,
  Settings and the app conventions.
- Wave C passed gate 5 (board `p1/surfaces-c.html`); stage 5 is
  complete. The graph colours were replaced after a colour-vision check
  (`p1/system/data-colours.md`). Next: stage 6, the review, and gate 6,
  signing off DESIGN.md. P2 is
  planned by its own run; P3 waits for the Freya fork at rc.8.
