# Design

How Isle looks and moves. Not written yet: the visual design is being made
afresh (B3) by the design-language packet, P1 in
`docs/work/rust-desktop/roadmap.md`, which produces this document and the
first `tokens.kdl`. Isle v1's design is on `main` for reference only.

Until then, what holds (the design process can revisit how any of it
looks, not that it exists):

- The north star is a macOS-style desktop (B17): a menu bar with the
  focused app's menus and the island where a notch would be (B18), a Dock
  (B19), app-centric switching (B20), Mac shortcuts (B21).
- What the island shows, where banners appear and where widgets live are
  for the design process to decide (B24), which lists the brainstorm's
  proposals as starting points.
- Isle's apps draw a unified toolbar with the traffic lights; other apps
  get Mac-style title bars drawn by the compositor (B25).
- Design tokens are data in `tokens.kdl` and reload live (B10); every
  surface and app reads them, and installed apps are themed from them.
- Open and close transitions are animated by the compositor, not the
  toolkit (B7).

What each surface must do is in `docs/design/`
(`shell.md`, `apps.md`); this document will own their look.
