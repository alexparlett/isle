# HDR modeset spike

Throwaway, for P22's 22.0 (`docs/work/rust-desktop/p22-hdr.md`, choice 2).
Results go in `docs/research/rust-desktop/hdr-2026.md` §3.4.

```
cargo build --release --offline
./target/release/hdr-modeset gl     # render targets; runs inside the session
./target/release/hdr-modeset run --dpms
```

`gl` needs no console: it checks FP16 and 10-bit render targets on the
NVIDIA render node.

`run` needs the card to itself, so it is run from a free console:
Ctrl+Option+F3 (Fn too on the Mac keyboard), log in, run it, and come back
with Ctrl+Option+F1 or F2. The session stays logged in, and gives the card
up while it is away.

It runs the steps below, about 70 seconds in all:
1. **SDR for 5 s.** Note what the monitor's OSD says.
2. **Three cycles.** In each, HDR10 is on for 10 s and SDR for 5 s.
3. **The screen-off test** (`--dpms`). The screen goes off for 5 s and
   comes back in HDR.
4. **SDR again,** then it exits.

What it shows:
- **The top two thirds:** grey bars at 0, 0.5, 2, 10, 50, 100, 203, 250,
  302, 400, 1000 and 10000 cd/m².
  - In HDR, the bars from about 302 up should look alike, because that is
    the panel's peak.
  - The 203 bar should look like SDR's white.
- **The bottom third:** BT.709's red, green and blue, then BT.2020's, all
  at 203.
  - In HDR, BT.2020's three should look deeper than BT.709's.
  - In SDR, both sets look the same.

Ctrl+C returns the screen to SDR and exits.

**If the screen stays black,** wait a minute: the program sets SDR back at
the end of every run. If it does not come back, switch back to the session
with Ctrl+Option+F1 or F2. If the machine itself is hung, use
Alt+SysRq+REISUB.

Every line is also appended to `hdr-modeset.log` in the current directory.

`--eight` runs the same steps on an 8-bit buffer, the plan's fallback.
`--hold SECS`, `--cycles N` and `--mode 3440x1440@144` change the run.
