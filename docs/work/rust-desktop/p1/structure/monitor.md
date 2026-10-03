# Monitor

Batch 3. Answers to: `apps.md` For the design process (Monitor: how
resources are switched) and the presentation side of `apps.md` Monitor
1–10 and the Parity map's Monitor rows ("tabs are P1's"); `apps.md`
Conventions 4, 24, 25; `conventions.md` (window anatomy, toolbar order,
menus), `prompts.md` (Force Quit, which this sheet does not repeat); B17,
B22.

## Purpose

Activity Monitor for Isle: one table of every process, seen by the
resource the user is asking about, with the system's totals and a short
history under it, and a way to quit, stop or inspect any process (`apps.md`
Monitor). It is where a gamer finds what is eating the GPU or the disk
while a game stutters, and where a hung helper with no window is ended. It
is not a dashboard of gauges (the widgets dashboard's System widget), not
a tuning tool (LACT and CoolerControl, which it opens), and not the
desktop's Force Quit panel, which ends apps rather than processes
(`prompts.md`).

## Content

### The window

```
┌─────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  All Processes     [CPU|Memory|Energy|Disk|Network|GPU]   ✕ … 🔍 │  toolbar
├─────────────────────────────────────────────────────────────────────────┤
│ Process name           % CPU   CPU time   Threads    PID   User         │  header
│ ▣ Firefox               12.4    1:02:11        84   2210   alex         │
│ ▣ Cyberpunk 2077         9.8      14:03        61   4412   alex         │  table
│   kworker/3:1            0.3       0:12         1     88   root         │
│ …                                                                       │
├─────────────────────────────────────────────────────────────────────────┤
│ System  4.1%   ╭────── CPU load ──────╮   Threads   2,310               │  footer
│ User   18.2%   │▁▂▂▃▅▃▂▁▂▃▄▆▅▃▂▂▁▁▂▃│   Processes   412               │
│ Idle   77.7%   ╰──────────────────────╯   Package  61 °C · fan 980 rpm  │
└─────────────────────────────────────────────────────────────────────────┘
```

No sidebar and no inspector (`conventions.md`, Window anatomy table).

- **Toolbar.** Leading: the lights; the title, which is the shown filter
  ("All Processes", "My Processes"), Activity Monitor's subtitle made the
  title because the title names the subject (`conventions.md`). Centre:
  the **resource switch**, a segmented control of six: CPU, Memory,
  Energy, Disk, Network, GPU (For the user 1). Trailing: Quit Process (✕),
  the one primary action; More (…) with Inspect Process, Stop, Continue,
  Priority, Send Signal…; the search field, which filters by name, PID or
  user.
- **The table.** One row per process: its icon and name first, then the
  resource's columns. A windowed process takes the app's name and icon
  from the compositor's window list (`apps.md` Monitor 2); any other
  shows its command name and no icon. Clicking a header sorts by it;
  View › Columns adds or removes columns. Each resource remembers its
  columns, widths and sort (`conventions.md`, What an app remembers).
- **The footer.** Part of the content, under the table at a fixed height:
  the system's figures for the shown resource, one history graph, and the
  sensors that belong to that resource. It is not a status bar.

### Resources

| Resource | Columns (default sort first) | Footer |
|---|---|---|
| CPU | % CPU, CPU time, Threads, PID, User | System, User, Idle; CPU load graph; Threads, Processes; package temperature and CPU fan |
| Memory | Memory, Threads, PID, User | Memory pressure graph (from the kernel's pressure figures); Physical memory, Memory used (apps, kernel, cached files), Swap used, Compressed (zram) |
| Energy | Energy impact, % CPU, % GPU, Preventing sleep, PID, User | Power graph (CPU package from RAPL, each GPU's board power); total watts; battery on a machine with one |
| Disk | Bytes written, Bytes read, PID, User | Reads in and writes out per second, data read and written per second; graph of either (Activity Monitor's choice); each drive's temperature |
| Network | Bytes received, Bytes sent, PID, User | Packets in and out, data received and sent, per second; graph |
| GPU | % GPU, GPU memory, GPU (shown only with more than one), PID, User | One block per GPU, named: utilisation graph, memory used of total, temperature, power, fan, clocks; "Open LACT" when LACT is installed (`apps.md` Monitor 10) |

- **Preventing sleep** names what a process holds from logind ("Sleep",
  "Idle", "Shutdown"); a process holding none shows nothing (`apps.md`
  Monitor 5).
- **Energy impact** is one relative number per process, as Activity
  Monitor's; how it weighs CPU, GPU and wake-ups is the packet's.
- **Sensors** are not a resource: they have no per-process figures, so
  each sits in the footer of the resource it belongs to, and Window ›
  Sensors opens a window listing every hwmon and GPU reading by device,
  with each value's low and high since the window opened, and "Open
  CoolerControl" as its toolbar's one action when installed. This replaces
  v1's Sensors tab and follows Activity Monitor's extra windows (CPU
  History, GPU History).

### Filters

View › All Processes, All Processes Hierarchically, My Processes, System
Processes, Windowed Processes (`apps.md` Monitor 2). Hierarchically shows
each process under its parent, collapsed below the top level, with its
descendants' totals beside its own (inventory §5). The filter is
remembered; the first-run filter is All Processes, as Activity Monitor.

### The table under a moving sort

Rows re-sort at each sample, as Activity Monitor's do, except while the
pointer is over the table or a context menu or alert is open: then values
update in place and no row moves, so a click or Quit lands on the process
the user aimed at. The order catches up when the pointer leaves. A
selected row stays selected as it moves and is kept in view while the
keyboard holds the selection.

### Sampling

- **Update Frequency** in View: Very Often (1 s), Often (2 s), Normally
  (5 s). Default Often (2 s), as v1: Activity Monitor's 5 s misses the
  spikes a game's stutter is made of.
- **Only while shown** (`apps.md` Conventions 24, Monitor 8): while the
  window is minimised, hidden, on a desktop not shown, or wholly covered
  (the compositor stops its frame callbacks), nothing is sampled. On
  return, the graphs show a gap for the time unseen rather than a line
  drawn across it, and rate columns show no value until the second sample.
- An inspect window and the Sensors window sample on the same terms; the
  main window hidden does not stop an open inspect window that is shown.

### Quit, Force Quit and the rest

- **Quit Process** (✕, ⌥⌘Q, the context menu) asks in an alert on the
  window: "Quit “{process}”?" with Force Quit, Cancel and Quit (default),
  as Activity Monitor. Quit sends TERM; Force Quit sends KILL and is never
  the default (`window-chrome.md`). Several selected: "Quit {n}
  processes?". A process that has not exited after TERM stays listed;
  asking again offers the same three buttons.
- **Stop** and **Continue** (STOP and CONT), **Priority** ▸ (Very low, Low,
  Normal, High, Very high), **Send Signal to Process…** (a sheet with a
  pop-up of signals) are in the Process menu and the context menu; Stop and
  Continue show whichever applies. A stopped process is marked "Stopped" in
  its row.
- **Another user's or root's process**, or raising a priority, asks once
  in the desktop's polkit dialog (`apps.md` Conventions 25, Monitor 7);
  refused, the row is unchanged and nothing else is said.
- **The compositor and the session** (Isle, the user's systemd manager)
  can be inspected but not quit from here: their Quit is disabled, since
  ending them ends the session; Log Out is the system menu's.

### Inspect

⌘I, a double-click or Inspect Process opens a window per process
(`conventions.md`, the table), titled "{process} ({PID})", placed by the
compositor; inspecting the same process again raises it.

- Toolbar centre: **General** and **Open Files and Ports**; trailing:
  Quit Process, then search on the second tab.
- **General**: parent process (a link that inspects it), user, started,
  state, priority, % CPU, memory, GPU memory, threads, the command line
  (selectable, copied whole with Copy), the executable, the working
  folder, and the systemd unit it runs in (`apps.md` Monitor 9).
- **Open Files and Ports**: open files with their paths, then sockets
  with protocol and addresses; ⌘F filters.
- When the process ends, the window says "This process has ended.", stops
  sampling and keeps the last figures until closed.

## States

| State | What shows |
|---|---|
| Rest | The table sorted by the resource's first column, the footer live |
| Loading | The frame and the table's names at once; rate columns and graphs empty until the second sample; no spinner |
| No results | "No results for “{query}”" in the table's place, the footer still live |
| A figure not available | Its column shows no value and the footer says why in one line: "Per-process network traffic isn't available.", "GPU figures aren't available for {GPU}." (`apps.md` Monitor Open). A sensor not found is left out, not shown as zero |
| A refused action | The polkit dialog said no: nothing changes |
| Process ended while selected | Its row goes; the selection clears; its inspect window says so |
| Many processes | The table scrolls under the toolbar; the footer stays |
| Hidden or covered | Nothing sampled; a gap in the graphs on return |
| Narrow window | The resource switch folds into » before the title truncates (`conventions.md`); columns past the window's edge scroll sideways; the footer drops its sensors, then its right-hand figures |
| Second screen | No difference |
| Game in front | An ordinary window, typically on the second screen or behind; it samples only while shown, so a game in front of it costs nothing |
| Locked | Nothing shows; covered, it stops sampling |
| Focus on | No change |
| Reduce Motion | Graphs step on each sample instead of scrolling; rows re-sort without moving |
| Reduce Transparency | Toolbar and footer solid |
| Increase Contrast | Table grid lines, the footer's edge and graph outlines |
| Largest text size | Rows and footer grow; the footer's figures wrap to two columns before any is cut |

## Opening and closing

Opened from Spotlight, the Apps grid or its desktop entry. It opens as any
Isle app (`conventions.md`): first complete frame, then the compositor's
open motion; the table has focus.
⌘W or the close light closes it and ends the process (`apps.md`
Conventions 4); inspect windows and the Sensors window close with it.
Resource changes swap the table and footer in place with no transition.

## Keyboard

| Key | Action |
|---|---|
| ⌘1 to ⌘6 | CPU, Memory, Energy, Disk, Network, GPU |
| ↑ ↓, type to select | Move in the table |
| ← → | Collapse or expand a process in the hierarchical view |
| ⌘I, Return | Inspect the selected process |
| ⌥⌘Q | Quit Process (asks) |
| ⌘F | Search |
| Menu key, ⇧F10 | The row's context menu: Quit, Force Quit, Stop or Continue, Priority, Send Signal, Inspect, Copy (name, PID and command line) |
| Escape | Clear the search; in an alert, Cancel |

Force Quit is the alert's button, reached with Tab, never a chord. The
standard map is `conventions.md`'s.

## Pointer

- Click selects, ⌘-click and ⇧-click extend; double-click inspects;
  secondary click opens the context menu.
- Header click sorts, again reverses; header drag reorders columns.
- Hover over a graph shows the value at that sample; nothing opens on
  hover.
- While the pointer is over the table, rows hold their order (above).

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| ⌘1 to ⌘6 | `conventions.md`'s ⌘1 to ⌘4 for an app's views | Monitor's resources; the convention extends to six here, as an app's command table may add |
| ⌥⌘Q | Nothing on the desktop | Monitor: Quit Process, as Activity Monitor |
| Ending things | The desktop's Force Quit panel (⌥⌘Esc, `prompts.md`) | Both, separate: Force Quit ends an app and every process it owns; Monitor ends one process. Neither opens the other |
| Sampling cost | A game in front | The game: Monitor samples only while it is shown, so a covered or minimised Monitor costs nothing |
| Sleep inhibitors | Control Centre | Monitor's Energy is where they show; Control Centre has no keep-awake switch (`control-centre.md`) |

## Answers

- **How resources are switched** (proposal: five tabs, CPU, Memory,
  Energy, Disk, Network). **Changed**: a segmented control in the
  toolbar's centre, as Activity Monitor and `conventions.md` have it, with
  a sixth segment, GPU (For the user 1), and ⌘1 to ⌘6.
- **Sensors** (`apps.md` Monitor 4; v1's Sensors tab). **Changed**: each
  resource's footer carries its own sensors, and Window › Sensors lists
  them all, because a tab without processes breaks the one-table model.
- **GPU figures for every GPU** (`apps.md` Monitor 3). **New**: the GPU
  resource with a GPU column when there are two, and a footer block per
  GPU.
- **Quit and Force Quit** (`apps.md` Monitor 7). **Adopted** as Activity
  Monitor: one alert offering both, Quit the default.
- **Sampling only while shown** (`apps.md` Monitor 8). **Adopted**, with
  covered counted as not shown and a gap drawn for the time unseen.
- **LACT and CoolerControl** (`apps.md` Monitor 10). **New**: each opens
  from where its subject is shown, LACT from the GPU footer, CoolerControl
  from the Sensors window.
- **A table that moves under the pointer.** **New**: rows hold their
  order while the pointer is over the table, so Quit never lands on the
  wrong process.

## Copy

| Where | String |
|---|---|
| Title (filters) | "All Processes", "All Processes, Hierarchically", "My Processes", "System Processes", "Windowed Processes" |
| Resource switch | "CPU", "Memory", "Energy", "Disk", "Network", "GPU" |
| Columns | "Process name", "% CPU", "CPU time", "Threads", "PID", "User", "Memory", "Energy impact", "% GPU", "Preventing sleep", "Bytes written", "Bytes read", "Bytes received", "Bytes sent", "GPU memory", "GPU" |
| Footer labels | "System", "User", "Idle", "Threads", "Processes", "Memory pressure", "Physical memory", "Memory used", "Cached files", "Swap used", "Compressed", "Power", "Reads in/s", "Writes out/s", "Data read/s", "Data written/s", "Packets in/s", "Packets out/s", "Data received/s", "Data sent/s" |
| Process menu | "Quit Process", "Inspect Process", "Stop", "Continue", "Priority", "Very low", "Low", "Normal", "High", "Very high", "Send Signal to Process…" |
| View | "Columns", "Update Frequency", "Very Often (1 s)", "Often (2 s)", "Normally (5 s)" |
| Window | "Sensors" |
| Quit alert | "Quit “{process}”?", "Quit {n} processes?"; "Force Quit", "Cancel", "Quit" |
| Row state | "Stopped" |
| Inspect | "{process} ({PID})", "General", "Open Files and Ports", "This process has ended." |
| Unavailable | "Per-process network traffic isn't available.", "GPU figures aren't available for {GPU}." |
| Launchers | "Open LACT", "Open CoolerControl" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **A GPU resource.** (a) Six resources, GPU after Network, with per
   process % GPU and GPU memory and a footer per GPU; (b) Activity
   Monitor's five, with % GPU as a CPU column and a GPU History window.
   **Recommended: (a).** `apps.md` Monitor 3 asks for per-process GPU
   memory on both GPUs (the RTX 5070 Ti and the iGPU), which a column and
   a history window cannot hold, and on a gaming machine the GPU is the
   resource asked about most (`design-references.md` §15, GNOME
   Resources). It is a departure from Activity Monitor, listed here.
