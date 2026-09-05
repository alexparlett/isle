# Shell design

The design language for a self-built desktop shell on Hyprland, and the
reasoning behind each decision. Written before the code so the code has
something to be accountable to.

## Brief

Minimal, clean, Apple-standard visuals. A GUI we own, sitting over the standard
Linux tools rather than replacing them. Shaped around two workloads: gaming, and
agentic coding on a 3440×1440 ultrawide.

## Principles

**1. Nothing on screen that isn't earning its place.**
The desktop's resting state is a wallpaper and a thin status strip. Everything
else appears on demand and leaves when it's done. This is the single principle
the rest derive from.

**2. Colour is information, never decoration.**
Surfaces are neutral. Colour appears only where it carries meaning: an agent
needs you, a temperature is high, a recording is live. A shell where seven
widgets are seven colours has taught you to ignore colour. Ours has a mostly
grey bar so that one amber dot is unmissable.

**3. Depth over borders.**
Apple separates surfaces with blur, shadow and translucency rather than lines.
Every panel is a floating material over the wallpaper. Borders are a last
resort, and 1px when used.

**4. Motion explains, it doesn't perform.**
Every animation answers "where did this come from and where did it go".
Spring physics, not linear easing — things settle, they don't stop dead.
Nothing animates longer than 250ms. Reduced-motion disables all of it.

**5. The compositor owns windows; the shell owns everything else.**
Hyprland handles tiling, focus and rules. The shell never tries to manage
windows. The line stays clean.

**6. We own the interface, not the implementation.**
Audio is `wpctl`. Network is `nmcli`. Clipboard is `cliphist`. Capture is
`grim`/`slurp`/`wf-recorder`. We build the surface people touch and delegate the
work to tools that already do it properly. This is the difference between a
shell and a reimplementation of PipeWire.

## Visual language

### Geometry

A 4pt base unit; everything is a multiple. Apple's 8pt grid halved, because a
3440px-wide screen at 1× has room for finer rhythm than a laptop.

| Token | Value | Used for |
|---|---|---|
| `space.1` – `space.6` | 4, 8, 12, 16, 24, 32 | all padding and gaps |
| `radius.sm` | 8 | chips, buttons |
| `radius.md` | 16 | cards, popovers |
| `radius.lg` | 24 | panels, the dashboard |
| `radius.full` | 999 | pills, the island |

Corners use **continuous curvature** (squircles) rather than circular arcs —
this is one of the most recognisable parts of Apple's visual language and
Hyprland exposes it directly as `rounding_power`. QML gets it via a shader or a
rounded-rect shape with the same curve.

### Type

**Inter** as the SF Pro substitute — it was drawn for the same job and is the
closest freely licensed match. **JetBrains Mono** for anything numeric or
monospaced.

| Role | Size | Weight | Use |
|---|---|---|---|
| `display` | 34 | 600 | lock clock |
| `title` | 20 | 600 | panel headings |
| `body` | 14 | 400 | everything |
| `label` | 12 | 500 | widget labels |
| `caption` | 11 | 400 | secondary text |
| `mono` | 12 | 400 | numbers, paths, agent state |

Numbers use tabular figures so a changing clock or percentage doesn't shift its
neighbours. Text is never centred except in the lock screen.

### Colour

Neutral surfaces, one accent, semantic status colours. Deliberately **not** a
seven-hue palette — see principle 2.

| Token | Purpose |
|---|---|
| `bg` | wallpaper shows through; surfaces are translucent over it |
| `surface` / `surface.raised` / `surface.sunken` | three depths, no more |
| `text` / `text.secondary` / `text.tertiary` | three weights of emphasis |
| `accent` | the one colour, used for focus and selection |
| `success` `warning` `danger` `info` | status only |

Two palette modes, both valid:

- **Fixed** — a hand-chosen neutral set. Predictable, always legible.
- **Adaptive** — Matugen derives the palette from the wallpaper.

Adaptive is the more impressive demo and the more common failure: a wallpaper
with low contrast produces an unreadable shell. If we ship it, contrast ratios
are clamped to WCAG AA after generation, not before.

### Material

Panels are `surface` at 85% opacity with a 24px background blur, one soft
shadow, and no border. The island is 92% — it sits over content more often, so
it needs more substance.

## Surfaces

Seven, and no more without deleting one.

### 1. Island

A pill, top centre. Centred because on a 3440px display the corners are a long
way from where you're looking.

- **Resting** — clock, and any active-state dots. Under 400px wide.
- **Hover** — expands to show media, volume, network, agents, battery.
- **Event** — briefly expands for a track change, a screenshot, an agent
  needing you, then collapses. This is the island's whole reason to exist:
  transient information without a notification.
- **Game mode** — hides entirely. A fullscreen game gets the whole screen.

### 2. Dashboard

Full-width overlay, invoked deliberately. Cards on a grid: time, weather,
media, system, notifications, **agents**, quick toggles. This is the one place
density is allowed.

### 3. Launcher

Centred, single input, results beneath. Apps, then commands, then calculator.
Not a grid of icons.

### 4. Control centre

Quick toggles and sliders — audio, network, Bluetooth, focus, power, wallpaper.
Each toggle is a switch with a long-press or right-click to reach the full tool.

### 5. Notifications

Toasts top-right, stacking to three. A centre in the dashboard. Critical
notifications persist; the rest expire.

### 6. Capture

Region, window, screen, record. Recording state lives in the island — a red dot
you cannot miss, which is the failure mode of every screen recorder.

### 7. Lock

The same visual language: wallpaper, blurred, clock, one input. Not a separate
aesthetic.

## What our use cases add

Nothing else on this list is unusual for a shell. These two are the reason to
build our own rather than install Odyssey.

### Agent surface

Claude Code and Codex sessions as a first-class citizen: how many are running,
what they're doing, and — the part that matters — which one is blocked on you.
Present in three places at three densities:

- Island: a dot. Amber when something is waiting.
- Control centre: one line per session.
- Dashboard: a card with project, state, elapsed, and a click to focus.

The state already exists as JSON written by the agents' own lifecycle hooks, so
the shell reads files and reacts to a signal. No polling.

### Game mode

Detected, not toggled. A fullscreen game means: island hidden, notifications
suppressed, and the compositor already handles tearing and direct scanout. The
shell's job is to get out of the way and prove it did — a single indicator on
return, not a settings page.

## Architecture

```
shell/
  shell.qml            entry point; declares surfaces per output
  theme/
    tokens.qml         the tables above, one source
    palette.qml        fixed or Matugen-derived, contrast-clamped
  services/            one per underlying tool, all the same shape
    Audio.qml            wpctl
    Network.qml          nmcli
    Bluetooth.qml        bluetoothctl
    Agents.qml           the hook state files
    Media.qml            playerctl / MPRIS
    Capture.qml          grim, slurp, wf-recorder
    Clipboard.qml        cliphist
    System.qml           sysfs, hyprctl
  surfaces/
    Island.qml  Dashboard.qml  Launcher.qml
    ControlCentre.qml  Notifications.qml  Capture.qml  Lock.qml
  components/          Card, Pill, Toggle, Slider, Icon…
```

**Services are the contract.** Each exposes properties and methods, polls or
watches as appropriate, and knows nothing about presentation. Each surface binds
to services and knows nothing about `wpctl`. That separation is what stops a
shell becoming a pile of shell-outs behind a skin, and it's what makes surfaces
testable with a fake service.

## Scope, honestly

Odyssey is one person working on this "pretty much nonstop", still calling it
WIP, and it has had months. This is not an evening's work, and the first version
will be worse than what we replace.

The order that keeps a working desktop throughout:

1. **Tokens and one component.** Prove the visual language on a single pill.
2. **Island, resting only.** Clock and dots. Waybar stays until this is better.
3. **Services**, one at a time, each with the island surfacing it.
4. **Dashboard**, then launcher, then control centre.
5. **Capture, notifications, lock** — the ones with working alternatives, last.

Waybar, rofi and swaync stay installed and configured until the replacement is
genuinely better. Nothing is deleted on the promise of a rewrite.
