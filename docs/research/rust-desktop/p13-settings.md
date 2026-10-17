# P13: Settings' measures, the VM, and the host run

## Headless

`cargo test -p isle-compositor --features test-hooks --test settings_bars`
against `target/debug/isle-settings`, on a headless Isle with a private
system bus (no NetworkManager, BlueZ or power-profiles-daemon):

| `apps.md` bar | Check | Result |
|---|---|---|
| 3 Nothing when closed | Cmd+W, then the process | gone, well within 5 s |
| 4 Responsive under failure | View › Sound with no daemon | the command taken at once |
| 5 Menus | the bar's menus | its own: System Settings, Edit, View with every pane and Show Changed Settings, Window |
| 6 Cmd | Ctrl+F, then Super+F | Ctrl's not taken; Super's runs Find |
| 7 Containment | `kill -9`, then relaunch | the shell answers; a new window opens |
| 8 Live settings | `appearance.mode` edited by hand | the window light within a second |

Bars 1 and 2 are the host's. The panes' own headless tests
(`isle-system-settings`, 38) drive every pane on its defaults and against
state the test gives.

## Memory, headless

One sample each, debug builds, Settings idle 30 s on a headless Isle,
`smaps_rollup` (`settings_bars::idle_memory`, ignored, run by name):

| Build | Renderer | PSS | Pss_Anon | Pss_File | RSS |
|---|---|---|---|---|---|
| default (no GPU crates) | software | 59.5 MB | 11.7 MB | 45.1 MB | 74.6 MB |
| `--features gpu` | software | 69.7 MB | 19.4 MB | 47.7 MB | 114.5 MB |
| `--features gpu` | GL | 151.6 MB | 46.5 MB | 105.1 MB | 216.4 MB |

Against bar 2 (80 MB PSS) only the software paths pass, as
`settings-2026.md` §8 found for the gallery. VRAM was not read headless.
The GPU build is the only one that can show the sidebar's blur: softbuffer
presents XRGB, so a software window is opaque (B98).

## The VM

In the guest (llvmpipe, 2560 by 1080), the workspace's debug build from
`/repo`, Isle's session on tty1, Settings opened by `isle settings open`:

- **Against the real daemons.** Network listed Ethernet, Connected, from
  NetworkManager; Sound listed PipeWire's output and input with their
  volumes and balance; Energy showed power-profiles-daemon's three modes;
  Displays drew the QEMU monitor to scale with the island's mark, its
  modes (75 and 50 Hz), Brightness refused for want of DDC; Wi-Fi said "No
  Wi-Fi device is connected." and Bluetooth "No Bluetooth adapter is
  connected." (the guest has neither); General › About read the guest's
  `os-release` (Arch Linux), the kernel, Isle's version and commit, QEMU's machine,
  the CPU, Virtio GPU and 8 GB.
- **A display change, live and reverted.** Refresh rate 50 Hz relit the
  connector at 50 Hz and raised "Keep these display settings?" counting
  down; left alone, the compositor put 75 Hz back at its deadline, the
  alert closed, and nothing was written to `settings.kdl`.
- **A write.** Natural scrolling for the Virtio tablet wrote
  `pointer "QEMU Virtio Tablet" { natural-scroll #true }`.
- **Found and fixed: panes froze on their daemons' state.** The copy of a
  daemon's state into the UI ran in the scope of the pane that first asked
  for it, so after that pane closed the next showed what it had last seen:
  Keyboard and Mouse stayed empty after the guest's input devices arrived.
  It now runs for the app's life (`spawn_forever`). With it, Keyboard
  listed the Virtio and AT keyboards as PC (automatic) and Mouse the
  tablet and the VMMouse, each with its group.
- **Found and fixed: an empty Keyboards group** drew its title alone; it
  says "No keyboard is connected."

## The host run, the user's

Not done. What it settles:

1. Bar 1: Settings' first complete frame within 300 ms of launch, warm.
2. Bar 2: PSS and VRAM (`nvidia-smi pmon -c 1 -s m`) after 30 s idle, for
   the release build with and without `--features gpu` (the latter with
   `FREYA_RENDERER=gl`), which settles B26's renderer.
3. A real mode change on each screen: reverting by itself, and kept on
   Keep, with NVIDIA's live modeset (the plan's first risk).
4. Cmd chords in Settings on the Mac keyboard and on the PC keyboard.
5. The sidebar's blur in the GPU build.
