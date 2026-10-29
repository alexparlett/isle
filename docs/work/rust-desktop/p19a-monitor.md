# P19a · Monitor

Planned 2026-10-29. Reviewed by the user the same day: every
recommendation taken.

P19 is three packets at the user's choice (2026-10-29): P19a Monitor,
P19b Cleaner, P19c Keychain, each planned, walked and closed before the
next starts, as P15 and P15b taught. This is P19a.

`apps.md` Monitor 1–10 says what Monitor gives: Activity Monitor's one
process table seen by resource, with the GPU and sensors a gaming machine
needs, Quit and Force Quit, inspection, and sampling only while shown. P1
answered every presentation question: `p1/structure/monitor.md` (binding)
and `p1/surfaces/monitor.md` with its Wave C board, and its new tokens are
already in `tokens.kdl`. What is open is how it is built and `apps.md`'s
two points: per-process network without a capability-granted nethogs, and
GPU figures for both vendors.

Today `isle-sysinfo` samples the system's CPU, memory, network, two
temperatures and one GPU for the dashboard's System widget (B130). Nothing
reads processes, disks, fans, power, or a second GPU. `Windows1` carries
no pid. winit ignores xdg-shell's `suspended` state, so no Isle app knows
when it is not shown. `isle-ui` has no table and no graph. The `monitor`
shortcut has waited for P19 since P5, and the System widget's click since
P17. `docs/research/rust-desktop/apps-2026.md` §0–5 has the evidence: the
host's two GPUs (the RTX 5070 Ti on NVIDIA's open module, with no DRM
fdinfo and no hwmon, and the Granite Ridge iGPU on amdgpu); RAPL readable
by root only; the iGPU's PPT reading as world-readable package power;
TCP's per-socket counters open to every user through sock_diag, and UDP's
absent.

What P19a must get right is cost and aim. A process table of five hundred
rows resampled every two seconds beside a game must cost the game nothing,
and nothing at all when Monitor is not seen. And Quit must land on the
process the user aimed at: not a row that moved under the pointer, and not
a pid the kernel has since given to something else.

## What it answers to

- Decisions: B2, B9, B11, B17, B19, B20, B25, B26 (software rendering), B27,
  B94–B98 (`isle-app`, the lights, Settings' writes), B112 (the window list),
  B130 (the shared sampler); D13 (memory measured on the host).
- Intent: `apps.md` Monitor 1–10, its v1 lessons and open points,
  Conventions 1–25, bars 1–8 and 11, the Parity map's Monitor rows;
  `platform.md` (polkit, logind); `compositor.md` (the window list, idle
  holds).
- Design, binding: `p1/structure/monitor.md`, `p1/surfaces/monitor.md` and
  its board, `structure/conventions.md` and `surfaces/conventions.md`
  (tables, segmented controls, what an app remembers),
  `surfaces/window-chrome.md` (alerts, sheets), `structure/prompts.md` and
  `surfaces/prompts.md` (Force Quit, the polkit frame),
  `system/data-colours.md` (series).
- Evidence: `apps-2026.md` §0–5 (new), `isle-v1-inventory.md` §5 Monitor and
  §6 Polling, v1's `shell/windows/monitor/`, `services/Monitor.qml` and
  `scripts/sysinfo.py`.

## Choices this plan makes

### Scope

1. **One packet; the Sensors window and the launchers cuttable** (19a.12).
   The footers' sensors stay either way. Everything else is P1's sheet.
2. **No gate.** P1 drew every surface, state and motion, and its tokens
   are in. Two lines P1 did not write are copy, added to
   `p1/structure/monitor.md` at 19a.0: "UDP traffic isn't counted per
   process." (choice 9) and "Open files are shown only for your own
   processes." (choice 22).

### Sampling

3. **`isle-sysinfo` grows; the widget's `Sample` stays as it is** (B130).
   New modules beside it: `procs`, `memory`, `disks`, `gpus`, `net`,
   `sensors`, `power`, each reading a root passed in, so tests run on
   fixtures, as `Sampler::at` does today. A Monitor sample asks for what
   its resource shows plus the footer's figures; the System widget's
   sampler asks for nothing new and pays nothing new. Hand-rolled on
   `/proc` and sysfs, as today: neither `procfs` nor `sysinfo` reads
   fdinfo, NVML or sock_diag, and both read more than is shown.
4. **Processes from `/proc`.** Every pid's `stat`, `status` and `cmdline`,
   which every user can read; `io` and `fd` only for the user's own,
   which need ptrace-read. A process is keyed by pid and start time, so a
   reused pid starts afresh and never inherits another's rates. The name
   is the executable's base name, or `comm` in brackets for a kernel
   thread; a process holding a window takes the app's (choice 15).
5. **Memory as Activity Monitor's footprint.** A process's Memory is
   `RssAnon + RssShmem + VmSwap` from `status`, cheap for every process;
   Inspect adds PSS from `smaps_rollup` for its one process. The footer:
   Physical memory, Memory used (total less available) split into apps,
   kernel and cached files in its figure rows, Swap used, Compressed from
   zram's `mm_stat`, and Memory pressure from `/proc/pressure/memory`'s
   `some avg10`.
6. **Disk.** Per process, `io`'s read and written bytes, for the user's
   own processes; other users' rows show no Disk figures. The footer from
   `/proc/diskstats`, whole disks only; each NVMe drive's temperature from
   its hwmon, matched by device.

### GPUs

7. **One `Gpu` per device, two backends.**
   - **NVIDIA through NVML,** loaded at run time as today: utilisation,
     memory, temperature, fan, power, clocks; per process, memory from
     the running graphics and compute processes, and utilisation from the
     process samples since the last sample. NVIDIA's open module exposes
     no DRM fdinfo and no hwmon on the host.
   - **amdgpu and Intel through DRM fdinfo and sysfs:** per process,
     engine time and memory from `/proc/<pid>/fdinfo`, counted once per
     `drm-client-id`, for the user's own processes; system-wide,
     `gpu_busy_percent`, VRAM and GTT, and hwmon's temperature, clocks
     and power.
   - Each GPU is named short, by trimming the vendor ("RTX 5070 Ti",
     "Radeon 610M"). A process on both GPUs shows the busier one's
     figures, and the GPU column names that GPU.
   - A GPU with neither backend gets "GPU figures aren't available for
     {gpu}." in the footer, as P1 has it; the VM's virtio GPU is one.

### Network

8. **Per process through sock_diag, TCP only** (`apps.md` Monitor Open).
   - **The dump.** One `NETLINK_SOCK_DIAG` dump a sample, with
     `INET_DIAG_INFO` and the cgroup id. Any user may read every TCP
     socket's byte and segment counters this way, so no capability and
     no nethogs are needed.
   - **Who a socket belongs to.** A socket is the user's process's when
     its inode is among that process's fds. Another user's socket counts
     for the only process in its cgroup, or else only in the totals.
   - **What the columns mean.** Bytes received and sent, and packets as
     segments, over the lives of the process's sockets. A socket that
     closes between two samples loses its last interval.
   - **The footer** is the system's from `/proc/net/dev`, loopback left
     out, with every protocol in it.
9. **UDP is said, not hidden.** Games and QUIC travel on UDP, which the
   kernel counts per socket nowhere. The Network footer carries "UDP traffic
   isn't counted per process." in P1's line for an unavailable figure, so a
   game's empty Bytes column is not read as nothing.
   - **Rejected:** a privileged BPF helper counting every protocol per
     cgroup, which is a system service with `CAP_BPF` and a nightly
     toolchain with `bpf-linker`; and v1's nethogs, which sniffs every
     packet under game load and needs capabilities set on it.
   - **The BPF helper waits** for a packet of its own after cut-over
     (Not in P19a).

### Sensors, power and energy

10. **Sensors from hwmon, read by hand, and NVML.**
    - Every hwmon device's temperatures, fans, voltages, currents, power
      and clocks, labelled by the chip's own labels.
    - A short table for chips whose labels are generic: `nct6687`'s
      unlabelled fans become "Fan 1" to "Fan 16", and `k10temp`'s Tctl
      becomes "CPU package".
    - The CPU footer's package temperature is `k10temp`'s Tctl or
      `coretemp`'s Package. Its CPU fan is a fan labelled CPU, or else
      the first fan that has turned since Monitor opened.
    - Fans that have read 0 since opening are left out of the footers.
      The Sensors window lists them last.
11. **CPU package power, where it can be read without root.**
    - RAPL's `energy_uj` when readable, which on the host it is not.
    - Else the iGPU's PPT, which on the host's APU is the socket's power,
      not the iGPU's alone: the probe (2026-10-29) put it within 4% of
      RAPL at idle and 2% under a game. The host's driver reports it in
      milliwatts, not hwmon's microwatts, so a PPT reading that would be
      under a watt is taken as milliwatts, with a test for each scale.
    - Else no CPU row, and the Power graph is the GPUs'.
    - Each GPU's board power comes from NVML, or from amdgpu's PPT on a
      discrete card. A battery comes from UPower (`isle-power`) where
      there is one.
    - Isle never loosens `energy_uj`'s permissions: CVE-2020-8694 is why
      they are root's.
12. **Energy impact as Activity Monitor's: a relative number,** the same
    on every machine and independent of what power can be read:
    - 1 for every 10 ms of CPU a second;
    - plus 1 for every percent of a GPU's time;
    - plus 1 for every 100 context switches a second.
    A game at 150% CPU and 99% GPU reads about 250; an idle helper under 1.
    The weights go into the decision record, and the host tunes them.
13. **Preventing sleep.**
    - **From logind:** `ListInhibitors`, block mode only. A delay lock
      prevents nothing; Isle's own before-sleep lock is one.
    - **Holds Isle takes for a client:** the portal's Inhibit, the
      ScreenSaver interface and idle-inhibit surfaces, which logind would
      show under the compositor's pid. The compositor lists them with the
      client's pid on `Shell1.Holds()` (choice 16).
    - **What a row shows:** suspend becomes "Sleep", idle "Idle", and
      shutdown or logout "Shut down".

### What the compositor gives

14. **"Shown" reaches the app.**
    - **When a window counts as not shown.** The compositor marks a window
      suspended when it is parked, which it does today: minimised, hidden,
      or on a desktop not shown. It also marks it suspended when it is
      wholly covered by opaque windows on every output it touches, and
      while the session is locked. Coverage is worked out when the layout
      changes, not each frame.
    - **How the app learns it.** winit drops the state, so
      `isle_titlebar_v1` version 3 sends `shown` beside it, and
      `isle_app::use_shown()` exposes it.
    - **What Monitor does with it.** Its sampler runs only while one of
      its windows is shown. Other apps may follow; none has to.
15. **A pid on every window.** `Windows1.List`'s `WindowInfo` gains `pid`:
    the Wayland client's, or `_NET_WM_PID` for an X11 window, as
    `WindowRecord` already holds it. A process holding a window takes
    that app's name and icon from the app index (`apps.md` Monitor 2). Its
    other processes keep their own names, as Activity Monitor's helpers
    do. Windowed Processes is the processes holding a window.
16. **`Shell1.Holds()`** lists the compositor's idle and sleep holds with
    the client pid and kind (choice 13), with a change signal.
17. **The ties that waited.**
    - The `monitor` shortcut (P5, `packet: Some("P19")`) gets its action,
      launching Monitor as `Action::Files` launches Files, and Spotlight's
      actions list it.
    - The System widget's body opens Monitor (P17 choice 3), on CPU, or on
      GPU from a GPU series.

### Acting on processes

18. **Signals through pidfds.** Quit, Force Quit, Stop, Continue and Send
    Signal open a pidfd, check the process's start time against the
    row's, and signal through `pidfd_send_signal`. A pid the kernel has
    reused is never signalled.
19. **Another user's process goes through polkit, by a helper.**
    `/usr/lib/isle/isle-process-helper`, run by pkexec, takes one pid,
    its start time and one signal or nice level. It does the same pidfd
    check and refuses pid 1, the compositor, the session's manager, and
    Monitor's own parents. Isle ships its own polkit policy for it,
    `io.github.alexparlett.Isle.Monitor.policy`, with two actions
    (signal, priority) at `auth_admin_keep` and readable messages; pkexec's
    default is a bare `auth_admin` naming the binary. The prompt is Isle's
    own agent's (P9).
20. **Priority: nice, and the app's share when the process leads one.**
    - **The nice values:** Very low 19, Low 10, Normal 0, High −5,
      Very high −10.
    - **Asking.** A raise is tried directly first, since the host's
      `RLIMIT_NICE` allows −11. polkit asks only when the kernel refuses.
    - **Why nice is not enough.** The host's `cpu` controller is on in
      `user.slice`, so nice orders threads only within the process's own
      cgroup. A lowered compile would still get its scope's equal share
      against a game.
    - **The app's share.** When the process leads an app's scope
      (`app-*.scope`'s first process), the step also sets the scope's
      `CPUWeight` through the user manager: 10, 50, 100, 200, 500. This
      needs no privilege and is undone when the scope ends.
21. **What cannot be quit.** The compositor, `isle-session` and the user's
    systemd manager can be inspected but not quit: Quit Process is
    disabled for them, as P1 has it.

### Inspect

22. **Inspect reads one process.**
    - **For any process:** `stat`, `status`, `cmdline`, the cgroup's
      unit, `exe` and `cwd` where readable, and PSS.
    - **For the user's own:** Open Files and Ports from `fd`, joined to
      sock_diag's TCP, UDP and Unix dumps by inode. A Unix socket names
      its peer's process where that is known.
    - **For another user's process,** the tab shows "Open files are shown
      only for your own processes." Isle does not escalate to read
      another user's files.

### The window

23. **A shared table in `isle-ui`.** `isle_ui::table`: sortable, resizable
    and reorderable columns; leading and trailing alignment, with figures
    in the mono face; selection with ⌘ and ⇧; type-select; a held order
    while the pointer is over it, a key is fresh, or a menu is open;
    disclosure for the hierarchy; rows drawn only as they are visible;
    re-sort and leaving motion; the accessibility tree's rows. It is built
    from Files' list view's lessons. Files does not move onto it in P19a,
    because its list view carries rename, drag and spring-loading that a
    shared table need not. Cleaner and Keychain use it.
24. **A shared graph in `isle-ui`.** `isle_ui::graph`: series, stacked and
    mirrored, with gaps, the stepped glide, a scale in 1, 2 and 5 steps,
    and the hover line. It is focusable, with its summary and arrow keys,
    as P1's Accessibility asks. The dashboard's graphs stay the shell's.
25. **`isle-monitor` on `isle_app::run_with`:** one main window, an
    Inspect window per process (inspecting again raises it), and the
    Sensors window. Closing the main window closes the rest and ends the
    process (Conventions 4). The command table holds ⌘1 to ⌘6, ⌥⌘Q and
    the Process and View menus as P1's Keyboard has them.
26. **What Monitor remembers is state, not settings.** The resource, the
    filter, each resource's columns, widths and sort, the Disk and Network
    graph choice, and Update Frequency are kept in Monitor's `isle-state`
    domain, as Files keeps its views (`conventions.md`, What an app
    remembers). Monitor has no Settings window, as Activity Monitor
    keeps these in its View menu.
27. **LACT and CoolerControl** show their buttons when their desktop
    entries are in the app index, and open through `isle-apps`. Monitor
    reads nothing from either: NVML and hwmon give the figures v1 asked
    LACT's socket for.

### Proof

28. **Fixtures for every reader.** `isle-fakes` gains a `/proc` and
    `/sys` tree builder: processes with their files, hwmon chips, DRM
    fdinfo, an amdgpu card's sysfs, diskstats and zram. sock_diag is
    tested against the test's own loopback sockets. The fake logind
    already has inhibitors.
29. **`examples/sample.rs` grows a `--monitor` form** that prints a whole
    Monitor sample as JSON, with its cost, for the host's checks.
30. **Bars** beside `apps.md`'s 1–8:
    - **Sample cost:** a full sample with every figure and five hundred
      processes takes under 15 ms at p95 on the host's worker.
    - **Running cost:** shown at Often, Monitor uses under 2% of one core
      over a minute, sampling and drawing.
    - **Not shown costs nothing:** while minimised, covered, on another
      desktop or locked, Monitor's process takes no CPU time over 30 s.

## The workspace

```
crates/
  isle-sysinfo/           procs, memory, disks, gpus, net, sensors, power
  isle-monitor/           the app: window, resources, footers, inspect,
                          sensors, acting (new)
  isle-process-helper/    the pkexec helper (new)
  isle-ui/src/
    table.rs              the shared table (new)
    graph.rs              the shared graph (new)
  isle-app/               use_shown, isle_titlebar_v1 v3
  isle-compositor/src/
    wm/                   suspended when covered or locked
    titlebar.rs           shown
    dbus.rs               WindowInfo.pid, Shell1.Holds
    idle.rs               holds with client pids
    binds.rs, dashboard.rs  the shortcut, System's click
  isle-ipc/               pid, Holds
  isle-fakes/             the /proc and /sys builder
data/
  apps/                   Monitor's desktop entry and service file
  polkit/                 io.github.alexparlett.Isle.Monitor.policy
  spotlight/actions.kdl   Open Monitor
pkg/arch/PKGBUILD         the app, the helper, the policy
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 19a.0 · The note and the record

`apps-2026.md` (written with this plan); the roadmap's P19a, P19b and P19c
entries; `apps.md`'s open points answered by choices 7–9; the two lines
into `p1/structure/monitor.md` (choice 2). The PPT probe (choice 11) is
done (2026-10-29, `apps-2026.md` §3): PPT tracks RAPL, in milliwatts.

### 19a.1 · Shown

Suspended when wholly covered or locked; `isle_titlebar_v1` version 3's
`shown`; `isle_app::use_shown()`. Compositor tests: a window covered by
another, minimised, on a hidden desktop and under the lock reads not
shown, and shown again on return.

### 19a.2 · Processes, memory and disks

`procs`, `memory` and `disks` on fixtures (choices 4–6): rates across two
samples, a reused pid, a kernel thread, another user's process with no
`io`, zram, pressure.

### 19a.3 · GPUs

The two backends (choice 7). The fdinfo backend runs on fixtures copied
from the host's iGPU, with duplicated client ids. NVML runs on the host,
behind a test that skips without it.

### 19a.4 · Network

sock_diag, the inode join and the cgroup fallback (choice 8), tested on
the test's own sockets; the footer from `/proc/net/dev`.

### 19a.5 · Sensors, power, energy and sleep

hwmon with the label table, the CPU fan, power by choice 11, Energy impact
(choice 12), logind's inhibitors.

### 19a.6 · The compositor's side

`WindowInfo.pid`, `Shell1.Holds()`, the shortcut's action and the System
widget's click (choices 15–17), with compositor tests.

### 19a.7 · The table

`isle_ui::table` (choice 23) with its own tests: sort, held order, type-select,
disclosure, the rows drawn as they are visible, a re-sort by keys,
accessibility rows.

### 19a.8 · The graph

`isle_ui::graph` (choice 24): stacked, mirrored, gaps, the scale's steps,
hover and the focused summary.

### 19a.9 · The window

`isle-monitor`: the toolbar, the six resources and their footers, filters,
the hierarchy with totals, search, columns and what Monitor remembers,
sampling while shown, the unavailable lines; the desktop entry, service
file and package.

### 19a.10 · Acting

The Quit Process alert, Stop and Continue, Priority with `CPUWeight`, Send
Signal…, through pidfds; the helper and its policy; Quit disabled for the
session's own. A test signals a child it made, and refuses one whose
start time has changed.

### 19a.11 · Inspect

General, Open Files and Ports, the ended form; raising an open Inspect.

### 19a.12 · Sensors and launchers (cuttable)

The Sensors window with low and high since opening; Open LACT and Open
CoolerControl.

### 19a.13 · Motion, states and accessibility

P1's Motion table under springs, Reduce Motion and Game Mode; every row
of States; Increase Contrast, the largest text size, focus, the narrow
window.

### 19a.14 · Measures and the walk

The bars on a release build (choice 30; `apps.md` 1–8 in the VM where they
do not need the host). Walked in the VM by real input, as
`verify-by-interaction` asks:
- every resource, each sort and a column drag;
- a busy row held while pointed at;
- Quit and Force Quit on a test process, Stop and Continue, Priority;
- root's process through the polkit dialog, refused and allowed;
- Inspect, and its process ending;
- covering, minimising and locking, with the gap on return;
- ⌘Q.
The journal is read after.

### 19a.15 · The host and close

The host run on the installed package:
- both GPUs with a game running and per-process % GPU;
- the PPT row, and the sensors on the real board;
- a game's TCP downloads in Network, and the UDP line;
- the running and hidden bars beside a game on the other screen.
Then the record:
- decisions recorded from B132;
- ARCHITECTURE gains How Monitor samples;
- `apps.md`'s open points closed;
- the roadmap and `state.md` follow;
- this file ends with its outcome.

## Reviewed

The user reviewed the plan on 2026-10-29 and took every recommendation.
The PPT probe ran the same day (choice 11).

## Not in P19a

- Per-process UDP, and network per app for every protocol: the BPF helper,
  a packet after cut-over.
- Activity Monitor's 12-hour power and App Nap columns; CPU History and GPU
  History windows; per-engine (encode, decode) GPU columns.
- Tuning: fan curves, GPU clocks and power limits, which are LACT's and
  CoolerControl's.
- Another user's open files.
- Files on the shared table.
- Containers and other namespaces' processes, shown as their host pids
  only.

## Depends on, and feeds

- Needs P4 and P16 (the window list, parking), P8 (`isle-power`), P9 (the
  polkit agent, idle holds, the lock), P13 (`isle-app`), P14a (the
  package), P17 (`isle-sysinfo`, the System widget).
- Feeds P19b, which takes the table and the pkexec helper pattern for the
  system's share, and P19c, which takes the table and the policy pattern for
  its reveal gate. Feeds the BPF packet and P20's audit.

## Done when

- `cargo test` passes headless with the readers' fixture tests, the table's,
  the graph's, the compositor's shown and pid tests, and the pidfd test.
- In the VM, Monitor is walked by real input through every row of the
  sheet's Keyboard, Pointer and States, and P1's board's scenes can be
  reproduced.
- On the host, both GPUs show per process, a game's figures are right
  against `nvidia-smi pmon`, the bars of choice 30 hold, and a hidden
  Monitor takes no CPU time.

## Risks

- **Covered windows in the compositor.** Working out coverage at each layout
  change is new, and an error leaves an app not drawing while seen. The
  tests cover partial cover, cover across two outputs, and a translucent
  window above, which never covers.
- **A table of five hundred rows in software.** Freya draws the whole
  window each frame (B26). Re-sort motion over many rows may cost more than
  bar 30 allows. Rows are drawn only as they are visible, and the
  measures decide whether the glide is cut to a jump beyond a row count.
- **NVML's per-process samples** are empty for a process idle since the
  last sample, and short bursts may be missed. They are shown as no figure,
  not as zero.
- **PPT as package power** may not hold on another APU, or under a
  discrete-only load. It agreed on the host at idle and under a game;
  elsewhere it is named "CPU package" only on an APU's amdgpu.
- **`CPUWeight` on an app's scope** is a step beyond Activity Monitor's
  nice. If it surprises on the host, Priority falls back to nice alone and
  the decision records why.
