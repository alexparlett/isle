# Revision 1: the island at rest, menus on demand, no Dock

2026-10-03, during gate 2's first batch. The user looked at batch 1's
board and preferred v1's minimal desktop: the island and no Dock, with
macOS's feel kept through behaviour rather than a permanent menu bar and
Dock. This note directed the rewrite of batch 1. Approved at gate 2 on
2026-10-03 and folded into the sheets, the brief, B18 and B19.

## What changes

- **At rest the screen is the wallpaper and the island.** No menu bar
  and no Dock. Windows keep a strip at the top clear for the island, as
  v1 did (its top gap was 48 px); the bottom edge belongs to windows.
- **The island at rest is v1's pill**: the clock, and glyphs only when
  they say something (Wi-Fi only on Wi-Fi, Bluetooth only when a device is
  connected, volume only when muted, VPN, restart owed, shortcuts
  inhibited, a configuration error, Focus when on). No desktop dots. The
  status tier from batch 1 becomes the rule for these glyphs: Isle's own
  show only when active; a glyph can be set to always or never.
- **Live activities, events, OSDs and privacy marks** stay as
  `island.md` has them; the compact forms sit beside or replace the
  clock and glyphs.
- **Hover opens Control Centre from the island**, as v1's panel opened,
  laid out the macOS way: live activities on top (the batch 1 card's
  content), then Control Centre's modules, the tray (other programs'
  status items), and a footer. A click on the clock opens Notification
  Centre. A click elsewhere on the pill pins Control Centre. v1's hover
  guards are requirements.
- **App menus on demand.** The front app's menu bar, as batch 1 designed
  it (system menu, app menu, the app's menus, Window, Help with search),
  appears over the top of the screen when the pointer pushes the top edge
  outside the island's zone, or on Ctrl+F2. It is macOS's auto-hidden
  menu bar: it slides down over windows, carries the app menus on the
  left and nothing on the right, and leaves when the pointer leaves or a
  menu closes. Isle still serves every menu protocol (B18's plumbing);
  menu items are also searchable from Spotlight.
- **No Dock.** Launching is Spotlight's Apps grid (as macOS 26 replaced
  Launchpad). Switching is Cmd+Tab, Cmd+` and Mission Control.
  Minimised windows are in App Exposé and marked in the switcher.
  Badges, progress and attention move to the switcher's cards, with
  attention also as an island event. Running in the background after quit
  shows in the switcher. Minimise shrinks the window toward the island.

## What stays

Everything in batch 1 not named above: window chrome, the zoom menu and
zones, the island's queue and privacy rules, the switcher's model, the
wallpaper and the widgets dashboard, the copy rules, the competition
sheet's layers, focus order and keys.
