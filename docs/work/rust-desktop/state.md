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
  which can run beside stage 3 (directions). P2 is
  planned by its own run; P3 waits for the Freya fork at rc.8.
