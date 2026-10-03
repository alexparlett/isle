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
  Artifacts. Next: P1 stage 1 (research and the interview), and P2
  planned by its own run; P3 waits for the Freya fork at rc.8.
