# P10: Spotlight's measures, and what the VM found

## Measures

`cargo test --release -p isle-compositor --features test-hooks --test bench
spotlight_cost -- --ignored --nocapture`, headless on the 5070 Ti: a 3440 by
1440 output over the frost wallpaper, a window open, the host's 97 desktop
entries with its icon themes, a home of 5,000 files. Draws are timed with
`glFinish` after each; typing is a key every 120 ms. Four runs:

| | Runs |
|---|---|
| Cmd+Space to the first drawn frame, as the test sees it | 28–63 ms the first time, then 2–4 ms; one later open of five took 52 ms |
| Fast sources per keystroke (48 searches) | p50 0.15 ms, p95 0.29–0.31 ms, max 0.32 ms |
| The list, typing (48 draws) | p50 0.40–0.45 ms, p95 0.57–0.73 ms, max 2.8–3.0 ms (25 ms once, the first draw with icons) |
| The Apps view, arrows (80 draws) | p50 0.68–0.77 ms, p95 0.93–1.20 ms, max 2.0–2.6 ms |
| The emoji grid, scrolling (110 draws) | p50 0.55–0.65 ms, p95 1.70–1.91 ms, max 10.7–30 ms |
| Closed, 10 s | nothing drew |
| RSS at rest, open, the Apps view, the emoji grid | 153, 160–167, 173–176, 190–193 MB; 255 MB once, in the first run after the change below |
| VRAM at rest, open, the emoji grid | 92, 61–63, 77–79 MiB |

- **The fast sources answer in a third of a millisecond**: apps, games,
  actions, windows, menu items and Settings rows are matched in the
  keystroke's own frame, far inside the 50 ms bar.
- **Every Spotlight draw is under 2 ms at p95.** The emoji grid's worst
  frames, 10 to 30 ms, are the first draws of glyphs the colour font has not
  drawn before; scrolling back over them is quick.
- **The first open takes up to 63 ms**, building the sources and loading the
  icons; later opens draw within a frame. Keys typed meanwhile reach the
  model at once, so none is lost.
- **Opening Spotlight costs about 10 MB, the Apps view's icons another 10,
  the emoji grid's glyphs 16.** The shell stays inside 250 MB RSS and
  200 MiB VRAM. One run read 255 MB with the emoji grid open and did not
  come back in three more.
- **Found on the way: tests scanned every font.** Since 10.1 gave each test
  compositor a HOME of its own, fontconfig found no cache and read the
  system's fonts afresh, about 53 MB and a burst of CPU per compositor; the
  P9 bench read 216 MB at rest where it had read 147. The tests now keep the
  user's cache folder; the same bench reads 151 MB. The suite's
  load-sensitive timing failures in 10.9 date from the same change.
- **The file index** on the host's own home: 83,960 entries in 7,022 folders,
  built in 165 ms, about 20 MB, a name search in about 2 ms (10.7).

## The VM

In the guest (llvmpipe, 2560 by 1080), the PC keyboard's Alt as Cmd:

- **Cmd+Space, "kit", Return** opened kitty with Papirus's icons in the list
  and kitty as the top hit; the same again raised it: one window.
- **The Apps view** listed the guest's 36 apps with their icons, every app
  alphabetically under All apps, the category filter in the bar.
- **Ctrl+Cmd+Space, "partying", Return** pasted 🥳 at kitty's prompt.
- **A copy made with `wl-copy`** showed in the Clipboard scope under Recent
  and was kept in `~/.local/state/isle/clipboard/`; it is attributed to
  wl-copy's own surface, which has focus as it copies.
- **A file written over ssh** was found by name in the Files scope at once.
- **`isle search`** over ssh answered apps, a sum and emoji.
- **xterm** started from Spotlight and took focus. An emoji did not reach it:
  xterm has no paste chord for the clipboard by default, so a pick pasted
  as P10 pastes everything does nothing there. The X11 paste path is checked
  headless against the X11 test client.

Not checked: a contents search and a drag out in the guest (both headless),
the host's real game libraries in the grid, and the Launchpad key, which
needs the host's Keychron.
