# HDR modeset spike

Throwaway, for P22's 22.0 (`docs/work/rust-desktop/p22-hdr.md`, choice 2).
Results go in `docs/research/rust-desktop/hdr-2026.md` §3.4.

```
cargo build --release --offline
./target/release/hdr-modeset gl     # render targets; runs inside the session
./target/release/hdr-modeset run    # the modeset; from a free console
```

`gl` checks FP16 and 10-bit render targets on the NVIDIA render node.

`run` needs the card to itself, so it is run from a free console:
Ctrl+Option+F3 (Fn too on the Mac keyboard), log in, run it, and come back
with Ctrl+Option+F1 or F2. The session stays logged in, and gives the card
up while it is away.

The test pattern covers the console while it runs, so each question is
drawn at the top of the pattern. Each one is answered with `y` or `n` and
Enter. Anything else, or three minutes' silence, records no answer and
moves on. The questions are:

0. SDR bars are showing: Enter to start.
1. Does the monitor say it is in HDR?
2. Do the four right-hand grey bars (302 cd/m² and up) look alike?
3. Are the bottom-right three patches (BT.2020's) deeper than the
   bottom-left three (BT.709's)?
4. Back in SDR: is the white about as bright as HDR's 7th bar (203)?
5. A: SDR, screen off and on by `ACTIVE`. Is it lit?
6. B: HDR, screen off and on by `ACTIVE`. Is it lit, and in HDR?
7. C: HDR, the output disabled and relit by a modeset, the way Isle
   sleeps a screen. Is it lit, and in HDR?
8. Done, back in SDR: Enter to exit.

A `n` at 5, 6 or 7 disables the output and relights it in SDR. Then one
more question asks whether that brought the screen back (5r, 6r, 7r).
Ctrl+C returns the screen to SDR and exits. If the screen stays black
through all of that, switch back to the session with Ctrl+Option+F1 or
F2. The session relights it.

Every line is also appended to `hdr-modeset.log` in the current directory.

`--eight` runs the same steps on an 8-bit buffer, the plan's fallback.
`--mode 3440x1440@144` picks the mode.
