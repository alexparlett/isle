# P16 · Mission Control: measures, the VM and the host run

P16's proof (`p16-mission-control.md` choice 29). Headless and VM numbers
are debug builds unless said; the host's release build is the measure that
counts.

## Headless

Tests (`cargo test -p isle-compositor --features test-hooks`):

- `mission` (13): the tiles drawn where the model puts them, a click
  choosing, keys kept from windows, App Exposé and Tab between apps, the
  set-apart row, the tray and restoring from it, Space to look, W closing,
  a desktop removed by its badge, ⌃→ while open, a drop on +, automatic
  rearranging, the bottom-left corner, a bound mouse button, an app
  assigned to every desktop.
- `show_desktop` (2), `split` (4), `stage` (2); `menubar`'s Full screen tile
  enabled and disabled; the window suite unchanged through the switch's
  rework; `isle-shell`'s model (21 tests at both screen sizes).

Bar 5, `bench::mission_control_cost` (by hand, `--ignored`): twenty live
windows on a 3440×1440 headless output, Mission Control opened five times
while every window redraws thirty times.

| Build | `mission-back` draws, p50 / p95 / max | `mission-front` p50 / p95 | VRAM at rest / open / after |
|---|---|---|---|
| debug | 2.52 / 5.91 / 6.05 ms (48 draws) | 1.05 / 2.24 ms | 148 / 210 / 152 MiB |
| release | 0.74 / 1.70 / 2.72 ms (30 draws) | 0.10 / 0.16 ms | 148 / 210 / 152 MiB |

The release build meets bar 5's 2 ms p95 for both surfaces. The back
surface draws again as live miniatures and tray thumbnails refresh (a few
at a time, ten times a second at most), which is most of its draws. The
tiles cost no shell draw: they are the compositor's own pass. Opening holds
62 MiB more VRAM (the miniatures and thumbnails, the two surfaces' targets),
given back on closing. Whether a 144 Hz output drops a frame is the host's
to show.

Found by the suite: a closed shell instance with a box of its own cost the
output a stray frame (the probe's settle test caught it after Stage
Manager's column was registered); the column now covers its output as
Mission Control's surfaces do. The shell's behaviour itself is left as it
is; a surface with an offset box that is closed should cost nothing, and a
later packet that adds one will meet it.

## The VM

The guest (llvmpipe, 2560×1080, debug build) with kitty, GTK 4 Demo and
Firefox, driven through QMP and `isle` over ssh:

- ⌃↑ opened Mission Control: the three windows as tiles with their icons,
  the bar with Desktop 1's live miniature and +.
- ⌃↓ opened App Exposé on Firefox with its header; Escape closed it.
- ⌘F3 (Alt in the guest) moved the windows aside; ⌘F3 again put them back.
- `isle desktops split 1` sent kitty to the left half and offered GTK Demo
  and Firefox in the veiled right half; a click on Firefox paired them,
  edge to edge with the divider and its grip.
- `isle desktops stage 1` with Show recent apps on staged Firefox and put
  kitty and GTK Demo in the column; a click on kitty's entry swapped it in.
- Found and fixed: `isle desktops list` named a Split View desktop "full
  screen".

## The host run (the user's)

On the installed package, on the ultrawide:

1. ⌃↑ opens Mission Control on every screen; the windows travel to their
   tiles and back; ⌃↑ or Escape closes it.
2. Rest the pointer in the bottom-left corner: Mission Control opens after a
   quarter-second, and again there closes it; a quick pass through the
   corner does nothing.
3. In Mission Control, hover a tile (ring, title, close badge), click one to
   choose it, middle-click one to close it, press Space on a selection to
   look at it.
4. Drag a tile onto another desktop's miniature and onto +; drag a desktop
   along the bar; remove a desktop by its badge.
5. ⌃↓ in Firefox: App Exposé with its windows; Tab moves to the next app.
6. Minimise a window and hide an app: both in the tray; click to restore.
7. ⌘F3: windows to the edges with slivers; click a sliver to put them back.
8. Zoom button's menu › Left of screen: choose a window for the right half;
   drag the divider; double-click it; Exit Full Screen on one pane.
9. A full-screen game in front: ⌃↑ opens Mission Control without motion or
   blur; Escape returns to the game.
10. Mission Control's miniature's menu › Stage Manager on a spare desktop:
    one app on the stage, the others in the column (Show recent apps on) or
    at a push of the left edge; click an entry to swap.
11. Settings › Desktop › Shortcuts…: Back on a mouse with side buttons opens
    Mission Control, and the browser no longer goes back on it.
