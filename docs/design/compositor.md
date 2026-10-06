# Compositor

Intent document for the rust-desktop program. Nothing here is built yet.

The compositor is Isle's own Smithay compositor: it owns every window, output
and input device, runs the X11 window manager for Xwayland, draws the
decorations, and hosts the shell's surfaces in its scene. This document says
how windows behave, how displays and input are handled, how games reach the
screen, how the session survives faults, and which wire protocols the
compositor serves. What the shell draws on top of these mechanics is in
`shell.md`; how any of it looks and moves is the design process's (P1),
whose open questions are listed under For the design process.

## Rests on

- B1: own compositor on Smithay; the layout model, config and render path are Isle's.
- B2: every v1 capability in this area exists here, re-designed; the parity map below is the check.
- B5: Smithay's in-tree X11 window manager; X11 policy is Isle's code.
- B6: a pinned Smithay fork carrying async page flip for tearing; VRR covers the gap.
- B7: shell surfaces are Freya trees composited by the compositor, which animates their open and close.
- B8: stacking with snapping, ultrawide zones, real minimise and maximise, PiP and dropdown in the model, layout behind a per-Space trait.
- B9: one calloop loop, blocking work on the service worker, panic boundaries, supervisor, watchdog.
- B10: the compositor owns settings and the state store (window places, session restore) and applies changes live.
- B12: the lock is the compositor's own; v1's locker-death lessons are requirements.
- B13: window previews are the compositor's own textures; capture only while visible.
- B17: where a choice is open, the answer is what a Mac user expects.
- B18: the compositor serves the Wayland and X11 halves of the global menu.
- B19: no Dock; the switcher, App Exposé and Mission Control show running apps and minimised windows, and the compositor supplies them.
- B20: app-centric windows: hide versus close, keep-running policy, Cmd+Tab and Cmd+`.
- B21: Cmd translation in the keyboard filter, per-device profiles, inhibit and games pass through.
- B22: no console mode and no pad bridge; Steam's Big Picture is a full-screen game like any other.
- B23: HDR is its own packet, after Isle replaces v1; nothing here serves colour management.
- B24: what Show Desktop reveals, widgets among it, is P1's; the compositor provides the move.
- B25: the compositor draws Mac-style server-side title bars in Freya; Stage Manager comes later, off by default.
- B26: the greeter is the compositor in a greeter mode under greetd, reusing the lock screen.
- B28: ext-session-lock is not served (the lock is Isle's); layer-shell is.
- B30: Cmd+Q quits keep-running apps too; Alt and Super swap on PC keyboards by default; the game escape set; VRR while a full-screen app is in front.
- B44: the seat sees only the keys Isle sends; Cmd and Option are withheld until the next key decides them; focus moves through one path.
- B45: the translation tables are a shipped data file with the user's over it.
- B46: a game is a flag on the window's record, from Steam's labels, GameMode and the user's lists.

## Features

### Stacking and window places

Windows float, overlap and open where they were last, as on macOS. Clicking a
window raises it; activating an app raises all its windows on the current
Space, keeping their order.

1. Every toplevel floats. Layout is a trait chosen per Space; the default module is stacking, and no other module is required by this document.
2. An app's main window opens at its remembered place in its first frame: the box is set before the first configure is answered, so it never appears elsewhere and moves.
3. A place is recorded only for an app's main window, which is its only titled toplevel, when a move or resize ends; the record is event-driven, not polled.
4. Untitled windows, a second titled window (a dialog), PiP, the dropdown terminal and windows recognised as games are never recorded or placed. While Game Mode is on, no places are applied.
5. A place that is off every output is never recorded and never applied; places are clamped whole onto the output they name, or onto the output with focus when that one is absent.
6. A window with no place opens at its requested size, or a default proportion of the work area, centred on the output with focus; a dialog opens centred over its parent; a modal dialog (xdg-dialog, X11 transient with modal state) keeps its parent from taking focus.
7. The work area of each output excludes a strip along its top kept clear for the island, and reaches the bottom edge. The menu bar reserves nothing: it slides over windows when revealed and moves none. Snapping and fill respect the work area, full-screen Spaces ignore it.
8. Open, close, minimise and restore are animated by the compositor; a window placed at map starts its open animation at its place, never sliding there.
9. A window can be kept on top and on every Space (v1's pin), from the window menu or a chord; focus moves by direction between visible windows with a chord.
10. Show Desktop (a chord, a hot corner or the spread gesture) moves every window on the output aside to show the desktop; the same route puts them back in their boxes.

**v1 lessons kept.** D40: a place belongs to the main window only; Steam's X11 menus took the main window's place for weeks. D91: placement happens at map, before the open animation, or the window visibly slides. D57: never record or restore an off-screen place; a KVM switch left windows at unreachable coordinates.

**Open for its packet.** How places are keyed when an app has several titled windows (xdg-toplevel-tag where clients send it). Whether places are keyed per output set (ultrawide alone versus with a second screen). Default size proportion and cascade rule.

**Settled in P4.** Places are kept per app and output identity, for the app's only titled window with no parent; a place for an absent output uses the app's latest, clamped onto the output with focus; xdg-toplevel-tag waits for Smithay. A window with no place opens at its own size, or at 50% of the work area's width by 70% of its height when it asks for more than the area, centred; a second window of the app cascades by the bar's height.

### Snapping and ultrawide zones

Drag a window to an edge or press a chord and it fills a zone, as macOS
tiling does, with zones designed for 3440×1440 and a preview while dragging.

1. Zones: left and right halves, left, centre and right thirds, two-thirds left and right, quarters at corners, a centred 60%, fill (the work area), and the free space beside an existing window.
2. A drag that ends with the pointer in an edge or corner region tiles the window to that region's zone, whatever started the drag: a modifier drag, a server-side title bar, a client's own title bar, or X11 `_NET_WM_MOVERESIZE`.
3. Dragging to the top edge fills; the bottom edge gives no zone, only its corners do. Holding Option while dragging shows the zone targets immediately, as on macOS.
4. A preview of the target zone is drawn by the compositor while the pointer is in a region; nothing in the drag path waits on another process.
5. Each zone has a keyboard chord and a place in the zoom button's menu; the chords do not use Fn.
6. Tiles keep margins when the "tiled windows have margins" setting is on; the window's decoration sits inside the tile.
7. The pre-tile box is remembered; dragging a tiled window away or the restore chord returns it to that size.
8. While dragging, a window's edges snap to other windows' edges and to the work area.

**v1 lessons kept.** Inventory §3.4, D91: the drop hook catches every drag regardless of how it began; tiling missed client-initiated drags until it did. macos-model.md §3: Fn is invisible on most keyboards, so tiling chords avoid it.

**Open for its packet.** Region sizes at the edges and corners. Which zones the "beside an existing window" rule offers when several windows border the gap. Multi-window arrangements (macOS's Shift and Option variants).

**Settled in P4.** Edge regions are 8 points in from an output's edge, corners 64 along each edge; an edge shared with another output gives no zone. Beside is the widest gap across the work area between the other windows, at least 200 points. Arrangements of several windows are not built.

### Apps, hide, quit and the keep-running policy

The desktop speaks in apps: Cmd+Tab switches apps, Cmd+` cycles the front
app's windows, Cmd+H hides an app, Cmd+Q quits it, Cmd+W closes a window
(B20, macos-model.md §3).

1. Each window belongs to an app, resolved from `xdg_toplevel.app_id` or `WM_CLASS` through desktop entries; `steam_app_<id>` resolves to the game; a gamescope window is named from its title; Isle's own apps each carry their own app id.
2. The compositor keeps a most-recently-used order of apps and of windows; Cmd+Tab orders by app recency, and Cmd+` walks the front app's windows in a stable order, not recency.
3. Activating an app (switcher, launcher, Mission Control) raises its most recent window and its other windows on that Space; if it has no window on the current Space, the view moves to a Space where it has one.
4. Launching an app that already has a window raises it, un-hiding it if hidden; a second instance is the app's own new-window chord.
5. Hide (Cmd+H) removes all of an app's windows from view on every Space; Hide Others (Cmd+Option+H) hides every other app; activating a hidden app shows its windows again in their order.
6. Cmd+W on an app's last window hides the app instead of closing the window when the app is on the keep-running list; for every other app it closes the window, and the app decides whether to exit.
7. Cmd+Q quits the app, keep-running apps included (B30): the app's own quit chord where the translation table names one, otherwise a close request to every toplevel of the app; the keep-running list never turns Cmd+Q into a hide, and an app still running once its windows are gone is sent the polite signal by PID.
8. The same close path serves Cmd+W, the title bar's close button, the switcher and Mission Control, so the keep-running policy holds whichever is used; Cmd+Q, Log out and Force Quit do not take it.
9. A window the compositor acts on is addressed by its own id; an action on a window that is already going away is a no-op, never a protocol error.
10. A client that misses a ping (`xdg_wm_base` or `_NET_WM_PING`) is marked not responding; Force Quit ends an app by PID, polite signal first.
11. Focus requests come through xdg-activation; a request with a valid token from recent user input focuses the window, any other marks the app as wanting attention.

**v1 lessons kept.** Inventory §3.6 keep-running: the per-app list is the only lever, because GTK and Qt apps exit on last-window close. D59: the switcher lists apps; the window cycle walks a stable order so a third window is reachable. D63: launching what is open raises it; focus returns from the closing shell surface first, then the raise applies, in the same frame rather than after a delay. D92, D75: one app id per app, set before map, or grouping and places fall apart. Inventory §3.6: close by id, never through a dying handle; that error killed v1's shell.

**Open for its packet.** The app-resolution heuristic for apps whose app id matches no entry, multi-process apps and Flatpak ids. The quit timeout before an app is shown as holding up log out. Whether hidden and other-Space windows are sent xdg-shell's `suspended` state or only throttled frame callbacks.

**Settled in P4.** An app id resolves by desktop id, `StartupWMClass`, case and last dotted part, either way round; an id nothing claims is its own app, and grouping is by app, never by PID. Parked and other-desktop windows get no frame callbacks and are sent `suspended`. Quit sends a close to every window and the polite signal to the app's PIDs once they are gone; log out's timeout is P14's. Pings are sent when the user acts on a window, not on a timer; X11's `_NET_WM_PING` waits.

### Minimise and maximise

Minimise shrinks a window toward the island; maximise fills the work area.
Both are real states, including when an X11 client asks for them.

1. Minimise (Cmd+M, the title bar button, or a client request through `xdg_toplevel.set_minimized` or X11 `WM_CHANGE_STATE` iconic) removes the window from its Space, shrinking it toward the island; it is reached from App Exposé, Mission Control, the switcher and the Window menu, and restoring returns it to its Space and box.
2. Maximise (`set_maximized`, `_NET_WM_STATE_MAXIMIZED_*`, the zoom button with Option, or the title-bar double-click action) fills the work area and remembers the previous box; the client is told it is maximised.
3. The title-bar double-click action is a setting: fill, minimise or nothing, as on macOS.
4. Minimised and hidden windows keep their live texture for the switcher, App Exposé and Mission Control, drawn only while a surface shows them.
5. A client minimise request is never answered with another state.

**v1 lessons kept.** Inventory §3.4: Hyprland answered an X11 minimise request with a sticky maximise toggle; the request is consumed and handled as minimise.

**Open for its packet.** Whether minimise and hide share one parked state with two presentations, as v1's `special:hidden` did.

**Settled in P4.** Two flags, not one parked state (B40), so a window minimised inside a hidden app comes back minimised.

### Spaces and full-screen Spaces

Each display has its own row of Spaces; a full-screen app gets a Space of its
own beside the one it came from, as on macOS.

1. Spaces are per output ("displays have separate Spaces"); each output has at least one; Spaces are added and removed from Mission Control and persist in the state store.
2. Ctrl+←/→ moves between Spaces and Ctrl+1…9 jumps to one, taken from the physical Ctrl key only; a window moves to a Space by Mission Control drag, a chord or the window menu.
3. Space switches are animated by the compositor as a short slide in the switch's direction with the two Spaces crossfading, and can be driven by a gesture that tracks the fingers. The wallpaper, the island and the top layer stay still; only that output's windows move, and nothing bounces at either end of the row.
4. "Automatically rearrange Spaces based on most recent use" is a setting, off by default; an app can be assigned to one Space or to all, and its new windows open there.
5. A window that becomes full screen (`set_fullscreen`, `_NET_WM_STATE_FULLSCREEN`, the zoom button, Ctrl+Cmd+F) moves to a new full-screen Space to the right of its origin, and the view follows only if the window has focus.
6. A window that covers its whole output without decoration (a game's borderless fullscreen) is given a full-screen Space the same way, without forcing the fullscreen state on it.
7. When fullscreen ends, or the window closes, the Space is removed, the window returns to its origin Space and box, and the view returns with it; a window already alone on its Space is left where it is.
8. In a full-screen Space the island and the menu bar are hidden and revealed by the pointer at the top edge.

**v1 lessons kept.** D48: a fullscreen window on a shared Space covered windows that alt-tab then "focused"; borderless fullscreen is detected by size and not forced into fullscreen, which would make the client's resize back to windowed be ignored. Inventory §3.6: v1's special workspaces (`hidden`, `terminal`, `steam`) become states of the model: hidden, the dropdown, and a full-screen Space.

**Open for its packet.** Whether Space count is fixed (1…9) or dynamic with chords for the first nine. What a removed output's Spaces become.

**Settled in P4.** Dynamic, up to sixteen per output; Ctrl+1…9 past the end adds desktops up to the number until Mission Control (P16) adds them. A removed output's windows join the current desktop of the output they move to and return to their own; its row is kept for its return.

### Split View

Two windows share a full-screen Space with a divider between them, as macOS
Split View.

1. Split View is entered from the zoom button's menu ("tile to the left or right of the screen"), by chord, or by dragging a window onto a full-screen Space in Mission Control.
2. The other half is chosen from the origin Space's other windows, offered in the free half; a window whose minimum width does not fit the half is not offered, and cancelling returns the first window to its box and removes the Space.
3. The divider moves both windows; it stops at either window's minimum size.
4. Leaving full screen for one window returns it to its origin Space; the other stays in its full-screen Space alone.
5. Two panes only: no third column on an ultrawide, where the zones divide a desktop in thirds.

### Stage Manager

macOS Stage Manager (a strip of recent apps beside one front app) is a
layout module behind B8's per-Space trait, not a change to stacking. It comes
later than the rest of this document and is off by default (B25).

1. Stage Manager is a layout module chosen per Space and off by default; the recent apps are shown by a shell surface (`shell.md`) fed with the compositor's app and window list and live textures, in a column at the leading edge of the work area that stays hidden until the pointer presses that edge unless the user shows it.
2. With it on, the front app's windows have the work area and other apps' windows are parked, each app reachable in one step; activating an app swaps it in. A set keeps the user's arrangement; it is centred only the first time it is staged.

### Picture-in-picture

A video's picture-in-picture window floats above everything, on every Space,
in a corner, as on macOS.

1. A PiP window is recognised by title ("Picture-in-Picture", "Picture in picture") or by an `xdg-toplevel-tag`, and is kept on top and on every Space, including full-screen Spaces. While a full-screen game is in front it is hidden, still playing, and comes back in its corner when the game leaves the front.
2. It keeps its aspect ratio when resized, snaps to the nearest corner of the work area when dropped, and opens in the corner it last used.
3. It is not listed in the switcher or in Mission Control's spread.
4. It has no title bar or lights: the compositor gives it the system radius, a shadow and a resize band, and the player draws its own controls.

**v1 lessons kept.** Inventory §3.3 `pip`: a rule's own move was ignored, so v1 moved each PiP window after it mapped; placement here is at map.

**Open for its packet.** Detection for players other than Firefox and Chromium (mpv, VLC).

### Dropdown terminal

A terminal appears on a chord, over every Space, and goes again on the
same chord.

1. One dropdown terminal exists per session, launched on first use under an app id of Isle's own and kept running when hidden.
2. The chord toggles it on the output with focus; it is above windows, below shell overlays, and on every Space. It slides down from behind the island's strip, which stays above it.
3. Its size is a setting; it is excluded from places, the switcher, Mission Control and session restore.
4. It hides when another window takes focus, a setting on by default; a shell surface taking focus for a moment does not hide it.

**v1 lessons kept.** Inventory §3.10: v1 sized it by dispatch after it mapped; size is set at map here.

**Open for its packet.** Which terminal, and whether the dropdown follows the terminal setting or a fixed one.

### Server-side decorations

Apps that accept server-side decorations get a Mac-style title bar drawn by
the compositor in Freya (B25). Isle's own apps draw their own unified
toolbar with the same traffic lights. Its look and arrangement are P1's.

1. The compositor prefers server-side mode in xdg-decoration and KDE server-decoration, and draws a title bar with close, minimise and zoom and the title, as a Freya tree composited with the window like any shell surface.
2. No bar is drawn over a client that draws its own: xdg-decoration client mode, KDE server-decoration "none", X11 Motif hints with decorations off, or `_GTK_FRAME_EXTENTS`. This is decided again whenever the client's mode, fullscreen state or hints change.
3. Close takes the same path as Cmd+W (keep-running included); minimise and zoom act as in this document; zoom also offers the zones, full screen and Split View.
4. Dragging the bar moves the window with snapping; double-clicking runs the title-bar action; right-click opens the window menu (move to Space or display, keep on top, close).
5. Every decorated window resizes from its edges.
6. Title bars can be turned off; clients that negotiate decorations are then told to draw their own, and X11 windows get none.

**v1 lessons kept.** Inventory §3.5, D48: a client that turns borderless loses its bar on the next state change, or a game wore a title bar.

**Open for its packet.** Which `gtk_shell1` version to serve, and whether GTK 4 then hands title-bar clicks to the compositor (`titlebar_gesture`, macos-model.md OPEN).

**Settled in P4, in part.** Server side is preferred over both protocols; a bar is a Freya tree per window (B41). `gtk_shell1` is not served yet; the window menu it would carry is P6's.

**Settled in P6.** `gtk_shell1` is served at version 5 with `titlebar_gesture`: GTK 4 hands its header bar's double, right and middle clicks to Isle, which runs the title-bar action, opens the window menu and lowers the window (B50).

### Xwayland window manager

Steam, most games and some apps are X11 clients. Isle runs Xwayland under
Smithay's in-tree X11 WM and owns every X11 policy (B5).

1. Xwayland starts with the session, so Steam and games find `DISPLAY` set; if it dies it is restarted and the session continues.
2. Override-redirect windows (menus, tooltips, drag icons) are never placed, focused, decorated, animated, shadowed or blurred, and are drawn at their own coordinates.
3. An untitled X11 window is treated as a menu or overlay, including one that turns override-redirect after mapping.
4. An X11 window with no class, no title and no fullscreen state takes no focus, so drag-and-drop sources work.
5. `_NET_WM_MOVERESIZE` starts the compositor's own move or resize, so Chromium and GTK windows move by their own controls.
6. Client requests for minimise, maximise, fullscreen and activation (`WM_CHANGE_STATE`, `_NET_WM_STATE`, `_NET_ACTIVE_WINDOW`) are honoured as in this document; Motif hints and `_GTK_FRAME_EXTENTS` decide decoration and geometry; transient windows open over their parent.
7. Nothing is blurred beneath an X11 window, whose own translucent shadows would smear.
8. X11 and Wayland share one global coordinate space across outputs.
9. `_NET_WM_SYNC_REQUEST` paces interactive resize; `_NET_WM_PING` feeds "not responding"; XDND works both ways.
10. The WM reads `_GTK_*` and `_KDE_NET_WM_APPMENU_*` properties and maps `RegisterWindow` ids to windows for the menu bar (macos-model.md §1.3); Xwayland's tearing-control and explicit sync reach the gaming path (smithay-and-gaming.md §2.3).

**v1 lessons kept.** D40: Steam's menus are X11 popups of the client's class; placing or noting them misrendered Steam for a month. Inventory §3.3 `x11-popup`, `x11-untitled`, `fix-xwayland-drags`, `x11-no-blur`: each rule is now WM policy. Inventory §3.4: without the `_NET_WM_MOVERESIZE` handler, X11 Chromium and GTK windows cannot be moved by their controls. smithay-and-gaming.md §1.4: Steam context menus closing at once under xwayland-satellite is the failure B5 avoids; it is a test.

**Open for its packet.** X11 apps on fractional-scale outputs (scaled and blurry, or native and small). Whether Xwayland starts at session start or on first connection with `DISPLAY` reserved.

**Settled in P4.** X11 clients draw at scale 1 and are scaled on a fractional output. Xwayland starts with the compositor, its display reported to the supervisor and kept across restarts (B42). Untitled windows are left alone like override-redirect ones.

### Outputs

Displays come and go, through hotplug and a KVM, without losing windows or
leaving a screen dark.

1. Outputs are identified by EDID make, model and serial, and their mode, scale, position, transform, VRR and enabled state are restored by that identity.
2. Changes apply live from settings (B10); the last enabled output cannot be disabled; a mode change reverts unless confirmed within a timeout.
3. Scale is per output, integer or fractional, through wp-fractional-scale and viewporter.
4. Windows on an output that goes are moved to a remaining output, clamped whole, keeping their Space, and go back when it returns; a floating window found outside every output is moved onto the nearest one.
5. When no output is lit, the session continues headless and re-probes connectors every few seconds until one returns, then lights it with its Spaces.
6. Redetect, on a chord and in settings, wakes every output, releases it and probes connectors again, as one step that cannot stop half way.
7. The render GPU is the boot VGA device by default, with a setting to override; an output on the other GPU is scanned out from there.
8. Night light is a colour transform the compositor applies per output, and is off in Game Mode.
9. Zoom (accessibility) magnifies the output around the pointer in quarter steps to 2× and halves to 4×.

**v1 lessons kept.** D56: a KVM that holds EDID leaves a connector up with a CRTC and is never re-probed; releasing outputs recovers it, a renderer reload does not. D76: an output asleep is not released, so redetect wakes first; the stranded state must be read from what is lit, not a stale list; a KVM hands the screen back minutes later. D57: windows are never left off the outputs. smithay-and-gaming.md §0: the AMD iGPU enumerates first on the host, so "first render node" is wrong. Inventory §13: the `renderHWCursorBuffer` crash on layout change; cursor planes are reassigned safely on every output change.

**Settled in part by P2.** Whether the scanner sees a KVM return without a hotplug event no longer matters: while nothing is lit the connectors are probed every three seconds, and again on every session resume (checked headless and in the VM; the KVM itself is P14's to check). Open: night light through `GAMMA_LUT` or a shader.

### Idle, DPMS and sleep

The screen dims, locks and sleeps on the user's timeouts, never while a
video plays or a game runs.

1. Idle is measured from the last input on any device; the policy dims, then locks, then turns outputs off, each at its own timeout, and a timeout of zero turns that step off.
2. Changing a timeout takes effect from the current idle time; it never fires at once and never stops firing.
3. Idle is inhibited by idle-inhibit from a visible surface, by `org.freedesktop.ScreenSaver` inhibitors and by Game Mode; clients get ext-idle-notify.
4. Input wakes outputs; the DPMS state is applied again whenever the set of outputs changes.
5. logind's Lock signal locks; the session locks before sleep when the setting asks.

**v1 lessons kept.** D18: v1's idle monitor never fired after a timeout change and fired at once with zero, which locked at startup. D76: an output returning through a KVM saw no idle-to-active edge and stayed off.

**Open for its packet.** Whether `org.freedesktop.ScreenSaver` is served here or by the service worker on the compositor's behalf.

### Pointer, touchpad and gestures

Touchpad gestures drive Mission Control, App Exposé and Spaces as on macOS;
every gesture also has a key and a mouse route, since the host is a desktop.

1. Each libinput device has acceleration profile (flat by default), speed, natural scroll, left-handed, tap-to-click and scroll method, applied live and restored by device identity.
2. Focus follows click: a click on an inactive window focuses and raises it and is delivered; scrolling goes to the window under the pointer without focusing it; focus never warps the pointer.
3. A modifier drag moves a window and a modifier right-drag resizes it; the chord is in the shortcut set. Middle-click paste is off by default.
4. The pointer is drawn on the cursor plane, so captures leave it out unless asked; the compositor falls back to drawing it when the plane is unavailable.
5. Hot corners: a pointer dwelling in an output's corner, with an optional modifier, triggers the action set for it (the actions are the shell's); a pointer pushing the top edge reveals the island inside its zone and the menu bar outside it, by a dwell instead over a game, as v1's game-mode hot zone did. Nothing is revealed from the bottom edge.
6. Three fingers up open Mission Control and down open App Exposé; three or four fingers sideways move between Spaces (the count is a setting); thumb and three fingers spread shows the desktop.
7. A gesture tracks the fingers one to one and commits by distance or velocity at release, otherwise springs back; gestures the desktop does not use reach the focused client through pointer-gestures.

**v1 lessons kept.** D27: hardware cursors keep the pointer out of screenshots; on NVIDIA this needs the open driver. Inventory §3.1: v1's three-finger gestures were discrete triggers for Mission Control only.

**Open for its packet.** Hot-corner detection across outputs whose corners meet. Edge-start swipes (Notification Centre) need finger positions libinput's gesture events do not carry (macos-model.md OPEN).

### Keyboards and per-device profiles

1. Layouts (the languages typed), variants, XKB options, repeat rate and delay and Num Lock at start are settings applied live.
2. Each keyboard has a profile, Mac or PC, guessed from the device name (Apple, Keychron, Magic Keyboard) and overridable, stored by device identity and applied the moment the device appears.
3. A PC-profile keyboard swaps Alt and Super, so Cmd sits beside the space bar; the swap is a per-device setting, on by default, and applies first, so bindings and Cmd translation see the swapped keys. It is suspended while a game window has focus, so a game gets the keys as labelled. A Mac-profile keyboard needs no swap: its Cmd key is Super and its Option key is Alt everywhere, games included (B30).
4. A Mac-profile keyboard gets the Macintosh XKB variant and its model (ISO or ANSI) on that device only; the layouts list stays plain.
5. Chords written as symbols are resolved through XKB: against the device's own keymap for matching, and the target keymap for anything emitted; a symbol with no unshifted key drops the rule.
6. A recording grab lets Settings capture a chord or detect a keyboard: while it runs, desktop chords are suspended and Escape ends it.
7. Caps Lock state and the active layout are exposed to the shell.

**v1 lessons kept.** D58: the remap config rendered before devices were known matched a stand-in name, which silently disabled it; the compositor sees each device directly, so there is no stand-in. D32: the Mac variant goes on the device, or @ moves for every keyboard. D61: where a symbol sits is asked of the layout (AZERTY, Dvorak, every Mac variant moves grave).

**Open for its packet.** Smithay keeps one XKB state per seat; how keyboards with different keymaps share a seat without sending a new keymap on every device switch.

**Settled in P5.** Keyboards with the same configuration share a compiled keymap, and the seat takes the typing keyboard's keymap at its first key, only while no key is held, so a switch happens when the other keyboard types, not per key. The PC swap is of the left Alt and Super only; right Alt stays AltGr. Option types a Mac keyboard's third-level symbol from either side, and is Alt for a key without one.

### Cmd translation

Super acts as Cmd everywhere: Cmd+C copies in every app, translated in the
compositor's keyboard filter into the focused app's own shortcut (B21,
macos-model.md §2).

1. The filter runs ahead of bindings and is keyed by the source device's profile, the focused window's app (app id, or `WM_CLASS` from the WM, plus title for dialog rules), and whether translation is on.
2. Cmd+key becomes Ctrl+key by default; per-app tables written for Isle in KDL list where an app differs (terminals, browsers, IDEs, file managers, dialogs without Ctrl+W).
3. Ctrl stays Ctrl; Option is Alt for shortcuts and Level 3 for characters through the Mac variant, as the device profile says.
4. A Super press is withheld until the next key decides it: a desktop chord, a translated chord, or a lone tap.
5. A translated chord is sent as synthetic modifier key events plus a matching `modifiers` event, so Wayland clients and Xwayland see one consistent state; Alt is never sent for a chord that does not need it, so menus do not open on it.
6. When focus moves while modifiers are held, the old client receives the releases before it leaves and the new client is entered with the state its own table implies.
7. A single translated key repeats through client-side repeat; a chord translated to a sequence is one-shot.
8. Translation is off in game windows, for the keep-it-raw list (VMs and remote desktops), and while keyboard-shortcuts-inhibit is active.
9. Keys Isle injects itself are a separate source and are never translated.
10. The table's inverse gives the shell the Mac chord to show beside a menu item.

**v1 lessons kept.** D58, D60: Cmd chords belong to the app in front; shell actions move to chords apps do not need, and every chord is derived from its action rather than written by hand. D59: a remap is verified by measurement; here, a headless test feeds keys through the filter and asserts what the client receives.

**Open for its packet.** Whether synthetic modifier key events are needed for Xwayland beyond the `modifiers` event. text-input-v3 as a "text field focused" signal, so Cmd+← is Home in a field and Back elsewhere. kitty's Shift+Insert. All three are macos-model.md OPEN.

**Settled in P5.** The seat is fed only the keys Isle sends, modifier keys included, and X11 state agrees with them (`xev` in the VM), so nothing synthetic is added (B44). text-input-v3 is not used: Cmd+← is Home everywhere and Cmd+[ and ] are Back and Forward in browsers. Terminals paste with Ctrl+Shift+V, so kitty's Shift+Insert is not needed; VS Code copies and pastes with the Insert chords, which work in its terminal too. The tables are `data/keys/translation.kdl` (B45).

### Shortcuts and keybinds

1. The shortcut set is data: each action has an id, a label and a default Mac chord; overrides live in `settings.kdl` and apply live.
2. Defaults follow macOS: Cmd+Tab, Cmd+`, Cmd+Space, Cmd+H, Cmd+Option+H, Cmd+M, Cmd+Q, Cmd+W, Cmd+Option+Esc, Ctrl+Cmd+Q, Ctrl+Cmd+F, Ctrl+↑, Ctrl+↓, Ctrl+←/→, Ctrl+1…9, Shift+Cmd+3/4/5, Ctrl+Cmd+Space; desktop actions with no Mac chord sit on chords apps do not use.
3. Bindings match physical modifiers before translation.
4. A chord bound twice is refused with a message naming both actions; the last good set stays in force.
5. The switcher opens on Cmd+Tab and commits when Cmd is released, whatever was pressed in between.
6. Media, volume and brightness keys work while locked and while a game is in front, and volume and brightness repeat.
7. While a game is in front, only the escape set of desktop chords is live: Cmd+Tab, Mission Control, lock, and the screenshot chords (Shift+Cmd+3, 4 and 5, with or without Ctrl, and Cmd+Ctrl+Escape to stop a recording; P1 `capture.md`); every other chord goes to the game (B30).
8. Escape closes an open shell overlay and is consumed only then.
9. Bound actions run in the compositor; only app launches start a process.

**v1 lessons kept.** Inventory §16: a plain release bind was suppressed after Super+Tab, so commit needed a transparent non-consuming bind; the compositor sees releases directly. Inventory §11: every v1 action keeps its capability; its chord is kept where macOS has one or nothing else reaches the action (P5 review), so redetect keeps Ctrl+Option+Cmd+R and Settings, Monitor, Files, a terminal, pin and focus by direction have none by default; v1's Big Picture chord goes with Isle's console mode (B22).

**Settled in P5.** The set is the `shortcuts` section, a field per action; actions later packets build are reserved, so their chords never reach apps. The layout chord is Ctrl+Space, live only with two or more layouts. Zoom's chords are macOS's, off by default. A held chord is told when its modifiers are released, which the switcher commits on.

### Keyboard-shortcuts-inhibit and injected input

A VM or remote desktop with focus gets every key, including Cmd chords; keys
Isle types on the user's behalf reach the right window untouched.

1. While a focused surface holds a keyboard-shortcuts inhibitor (or an X11 client an xwayland-keyboard-grab), all keys pass untranslated and desktop chords are off, except one escape chord that is never inhibited; the user is shown that it is active, and it ends when focus leaves.
2. Keys from Isle (on-screen keyboard, emoji typing, paste into the previous window) enter as their own source, untranslated, aimed at a named window; paste sends that window's own paste chord (Ctrl+Shift+V in a terminal).
3. Third-party clients may inject through virtual-keyboard; their keys are not translated.

**v1 lessons kept.** D37: v1 typed through `wtype`, a process per key; injection is in-process here.

**Open for its packet.** The escape chord, which must not be Cmd+Ctrl+Escape (stop a recording, P1 `capture.md`) or Cmd+Option+Esc (Force Quit). Inhibitors are granted always, shown by the island's glyph and a notice in the window (P1, `window-chrome.md`). Whether libei is served for portal RemoteDesktop.

**Settled in P5.** The escape chord is Cmd+Escape: it takes the keys back and gives them back; focus leaving takes them back until the window returns. Xwayland's keyboard grabs count as inhibitors. Over a game the escape set survives its inhibitor (B46). Isle's own keys are aimed at a window by id; text goes through a keymap made for it, since text-input-v3 with the compositor as input method would take dead keys from GTK. libei is P11's.

### Gaming display path

A full-screen game reaches the screen without composition, with VRR, and
with tearing where the user allows it (B6).

1. A full-screen window whose buffer the plane accepts is scanned out directly on the primary plane; a shell surface over it forces composition only while visible.
2. VRR per output has three modes: off, on while a full-screen app is in front (the default), and always; the mode is a setting (B30).
3. Tearing is allowed per window through wp-tearing-control and X11 tearing-control, only when the window is full screen and scanned out directly, and only when the tearing setting is on; async page flip comes from Isle's Smithay fork.
4. Explicit sync (linux-drm-syncobj), fifo-v1, commit-timing-v1, pointer constraints, relative pointer, DRM lease for VR headsets, and every global nested gamescope requires (smithay-and-gaming.md §2.2) are served.
5. Frame scheduling on NVIDIA does not rely on presentation timestamps or early commit returns.
6. Overlay planes stay off on NVIDIA until tested on the host, as cosmic-comp does (smithay-and-gaming.md §2); primary-plane scanout stays on.
7. The compositor process carries an NVIDIA application profile with `GLVidHeapReuseRatio` set to 0 under its own process name.
8. The compositor offers a game profile that turns off its animations, blur and shadows and drops window places, and restores them on leaving without a reload; when it applies is gaming.md's policy.

**v1 lessons kept.** Inventory §10: v1's game mode turned off animations, blur, shadows, gaps and borders and left tearing to the user; leaving needed a full reload, which the live profile replaces. smithay-and-gaming.md §2.1: NVIDIA needs its own timing, plane and VRAM workarounds; cosmic-comp and niri both carry them.

**Open for its packet.** Cursor-only updates under VRR. Async flips on Blackwell with 615.x.

### Session and robustness

1. A small supervisor is the session process started by the greeter; it runs the compositor inside the logind session scope and exports the session environment (Wayland and X11 displays, toolkit backends, NVIDIA variables when the driver is present) to systemd and D-Bus activation.
2. Each shell surface's event handling and drawing sits behind a panic boundary; a panicking surface is dropped and rebuilt from state, one that keeps panicking is held off with a backoff and reported, and a panicking service on the worker is restarted there.
3. A compositor-core panic ends the session; the supervisor keeps the crash report (panic, backtrace, recent log) in the state directory and starts a new session, backing off after repeated quick deaths and never giving up.
4. A session that crashed while locked comes back locked; a failing lock surface is replaced by a minimal lock that still takes the password; unlocked content is never shown because the lock failed.
5. A watchdog thread logs any main-loop stall over one frame's budget, with the event source that was running.
6. A client's protocol error disconnects that client only.
7. Session restore relaunches apps by app and Space on a new session only, re-hides those that were hidden, skips apps autostart starts, and moves their windows to their Spaces as they map.
8. Log out asks every app to quit as Cmd+Q does, then ends the session.
9. Started by greetd in greeter mode, the compositor lights the outputs, takes input with each keyboard's profile, and shows the lock screen's surface as the login prompt, drawn with the shipped tokens and the choices the user's session published for it (the wallpaper, light or dark, the accent, the keyboard layouts and per-device profiles, the main screen), else shipped defaults, and never the user's token file (`platform.md` Session start 2); it serves no app windows, Spaces, places or session restore, and exits once greetd starts the user's session (B26).

**v1 lessons kept.** D17: outside the logind session scope NetworkManager, udisks and logind refuse or demand admin. D15, D43: a dead locker stranded the session and the restart loop gave up; logs were lost in the runtime directory. D53: restore by app and Space; a shell restart replays nothing. D66: Rust-side faults land in something restartable, not the whole desktop.

**Settled in P2.** A dispatch stuck for ten seconds writes a crash record and aborts the main thread, so its stack is in the core dump and the supervisor starts a fresh session; `debug { watchdog-abort #false }` turns that off (B35). **Open.** GPU reset recovery, which killed Denial's session (smithay-and-gaming.md §4.5). Where the session publishes the greeter's choices so greetd's user can read them.

### Shell surface hosting

The mechanics `shell.md`'s surfaces rely on.

1. Shell surfaces are composited in fixed layers: background, desktop, windows, top, overlay, lock; each surface is given its layer by its design.
2. Each output has its own instances of per-output surfaces; events go to one.
3. A shell surface takes keyboard focus never, on demand, or exclusively; when it closes, focus returns to the window that had it in the same frame.
4. The compositor exposes to the shell the window list (app, title, Space, output, state, MRU), live window textures drawn only while a surface shows them, and actions: focus, raise app, hide, minimise, close, move to Space or output, set box. The same list and actions are reachable over D-Bus for the CLI and tests.

**v1 lessons kept.** D55: a capture against a vanishing output crashed Hyprland, and v1 paid for previews nobody saw; previews exist only while visible and are never capture sessions. D49: one surface per output, events once.

### Wire protocols

| Protocol | Why |
|---|---|
| core, xdg-shell, xdg-dialog, xdg-activation | every client; windows, modal dialogs, focus requests with tokens and launch feedback |
| xdg-decoration, KDE server-decoration | negotiating server-side title bars |
| xdg-foreign, xdg-output, xdg-system-bell | portal dialogs parented to the app (platform.md); output names for older tools; bells reach the shell |
| xdg-toplevel-icon, xdg-toplevel-tag | icons for apps with no entry; stable window tags for places and PiP |
| linux-dmabuf v4+, linux-drm-syncobj | GPU buffers; explicit sync, needed on NVIDIA (smithay-and-gaming.md §2) |
| wp-viewporter, wp-fractional-scale | per-output scale |
| wp-presentation-time, wp-fifo, wp-commit-timing, wp-tearing-control (fork), wp-content-type | game and video pacing; Vulkan FIFO present cost; per-window tearing (B6); content hints from games and video players |
| wp-single-pixel-buffer, wp-alpha-modifier, wp-cursor-shape | cheaper surfaces; cursors without client buffers |
| pointer-constraints, relative-pointer, pointer-gestures, pointer-warp, tablet | games; forwarded gestures; drawing tablets |
| keyboard-shortcuts-inhibit, xwayland-shell, xwayland-keyboard-grab | VMs and remote desktops; the X11 WM (B5) |
| text-input-v3, input-method-v2, virtual-keyboard | input methods; the B21 text-field signal; third-party injection |
| idle-inhibit, ext-idle-notify | video players; clients that watch idle |
| ext-image-capture-source, ext-image-copy-capture, ext-foreign-toplevel-list | standard capture tools (B13) |
| ext-data-control, primary-selection, ext-background-effect | clipboard managers; primary selection; blur behind apps' glass |
| security-context, wp-drm-lease | sandboxed clients refused privileged globals; VR headsets |
| org_kde_kwin_appmenu v2, gtk_shell1 | global menu from Qt 6.9+, Chromium 137+ and GTK (B18, macos-model.md §1.3) |
| wlr-layer-shell | other programs' overlays and tools (B28) |

Not served: ext-session-lock (the lock is Isle's, B28), wlr-foreign-toplevel-management (the shell is in-process), wlr-output-management (output settings go through B10's D-Bus interface), wlr-gamma-control (night light is the compositor's), colour management (the HDR packet, B23).

## For the design process

What this document leaves to P1, per feature; "proposal" is the answer it
gave before.

- **Stacking**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/window-chrome.md`, `wallpaper.md`); recorded
  in DESIGN.md at gate 6.
- **Snapping**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/window-chrome.md`); recorded in DESIGN.md at
  gate 6.
- **Spaces**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/mission-control.md`); recorded in DESIGN.md
  at gate 6.
- **Split View**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/mission-control.md`); recorded in DESIGN.md
  at gate 6.
- **Stage Manager**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/mission-control.md`); recorded in DESIGN.md
  at gate 6.
- **Picture-in-picture**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/pip.md`); recorded in DESIGN.md.
- **Dropdown terminal**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/dropdown-terminal.md`); recorded in
  DESIGN.md at gate 6.
- **Server-side decorations**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/window-chrome.md`); recorded in DESIGN.md at
  gate 6.
- **Keyboard-shortcuts-inhibit**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/island.md`, `window-chrome.md`); recorded in
  DESIGN.md at gate 6.
- **Shell surface hosting**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/competition.md`); recorded in DESIGN.md at
  gate 6.

## Acceptance bars

- Every v1 capability in the parity map exists or is reshaped as the map says.
- A main window with a place appears at it in its first presented frame, with no move.
- No window is placed, recorded or left off every output after any hotplug, KVM switch or scale change.
- After a KVM switch away and back with the outputs asleep, the screen is lit with every window on its Space within ten seconds of the KVM returning, without input; redetect recovers it in one keypress otherwise.
- With a static desktop the compositor presents no frames beyond those its own damage asks for (a clock tick).
- On the host, a full-screen Vulkan and a Proton game are scanned out directly with VRR across 48–144 Hz, checked from DRM plane state; tearing engages once the fork carries it.
- Nested gamescope starts on the host at the output's refresh.
- Steam's client, its context menus, Big Picture and a Proton game run through the WM: menus open where clicked and stay open; X11 Chromium moves and resizes by its own controls.
- The translation test suite covers every table entry, focus changes mid-chord and inhibit, with no stuck modifiers; Cmd+C, Cmd+V, Cmd+W and Cmd+Q behave as a Mac user expects in Firefox, Chromium, kitty, a GTK 4 app, a Qt 6 app and VS Code; Cmd+Q on a keep-running app leaves it not running; with a game in front only the escape set reaches the desktop.
- The switcher commits on Cmd release in every one of a hundred scripted attempts.
- A panic injected into any shell surface is recovered within one second with no client affected; one injected into the core while locked comes back to a locked session with the crash report kept.
- Over a 24-hour soak the main loop logs no stall over a frame's budget in ordinary use, and compositor VRAM stays within 10% of its value after the first hour.
- Changing any setting in this area applies without a restart.

## Parity map

| v1 capability (inventory) | Here |
|---|---|
| Session environment, start hook, generated config fragments, hyprpm (§3.1, §3.2) | Session 1; settings apply live (B10); no plugins remain, so the plugin version check goes |
| `isle-session` restart loop, session scope, logs (§12, D17, D43) | Session 1, 3, 4 |
| Everything floats; the shell's own windows float centred (§3.3, §3.4) | Stacking 1, 6; Apps 1 |
| Gaps clearing the island (§3.1) | Stacking 7 |
| dwindle fallback, swap in tiling, float toggle (§3.1, v1 keymap) | Reshaped: the layout trait (Stacking 1); B8 rejects dynamic tiling |
| Window places at open, places poll, stray rescue (§3.4, §3.6, D40, D57, D91) | Stacking 2–5; Outputs 4 |
| Window, layer and workspace animations (§3.1) | Stacking 8, Spaces 3; curves and choreography are P1's |
| Pin, focus by direction (§11) | Stacking 9 |
| Floating snap, drag-to-edge tiling, snap ghost, snap keys (§3.1, §3.4) | Snapping |
| Recency, app grouping, `steam_app` and gamescope names (§3.6, D75, D92) | Apps 1, 2 |
| Kept running, raise not relaunch, close by address (§3.6, D63) | Apps 4, 6, 8, 9; Shell surface hosting 3 |
| `focus_on_activate` (§3.1) | Apps 11, reshaped to tokens and attention |
| Minimise and client minimise requests, zoom, bar double-click (§3.4) | Minimise and maximise |
| Workspaces, chords, move window to workspace (§3.6) | Spaces 1, 2 |
| Fullscreen gets its own space (§3.6, D48) | Spaces 5–7 |
| Special workspaces hidden, terminal, steam (§3.6) | Apps 5, Dropdown terminal, Spaces 5; Steam's Big Picture is a full-screen game, Isle's console mode is dropped (B22) |
| `pip` rule and mover; `isle-dropdown` (§3.3, §3.10) | Picture-in-picture; Dropdown terminal |
| hyprbars, border, resize on border (§3.1, §3.5) | Server-side decorations, drawn in Freya (B25) |
| X11 rules, `_NET_WM_MOVERESIZE`, Steam's X11 menus (§3.3, §3.4, §3.8, D40) | Xwayland window manager |
| Display arrangement, redetect, stranded heal (§3.7, D56, D76) | Outputs 1–3, 5, 6 |
| Primary monitor, shell follows focus (§3.7) | Shell surface hosting 2; the choice in shell.md |
| Night light, magnifier (§3.9, §3.10, D50) | Outputs 8, 9 |
| Brightness, capture as compositor children (§3.7, §4) | platform.md; frames come from the compositor (B13) |
| Idle dim, lock, DPMS, inhibitors (§3.10, §4, D18, D76) | Idle, DPMS and sleep |
| Mouse and touchpad, click to focus, `no_warps`, hardware cursors, no middle paste, Super+drag (§3.1, §3.9, D27) | Pointer, touchpad and gestures 1–4 |
| Game-mode hot zone (§4) | Pointer, touchpad and gestures 5 |
| Three-finger gestures (§3.1) | Pointer, touchpad and gestures 6, 7 |
| Per-device keyboard model, Mac variant, Num Lock, device lists, Detect submap (§3.9, §11, D32) | Keyboards 1–4, 6; PC keyboards gain the Alt and Super swap (Keyboards 3, B30) |
| xremap Mac profile, symbols through XKB, keycheck (§11, D7, D58–D61) | Cmd translation; Keyboards 5; acceptance bars |
| keymap.json actions, media keys, default chords, switcher release commit, Escape (§4, §11, §16) | Shortcuts and keybinds; the Big Picture chord goes (B22) |
| Lock's Caps Lock polling (§4) | Keyboards 7 |
| OSK through wtype, paste into the previous window (§3.6, §4, D37) | Keyboard-shortcuts-inhibit and injected input 2 |
| Tearing, VRR, direct scanout, game-mode compositor profile (§3.1, §3.3, §10) | Gaming display path; Stacking 4 |
| Mission Control moves, close, live previews (§4, D19, D55) | Spaces 2; Shell surface hosting 4 |
| Session restore, logout (§3.10, D53) | Session 7, 8 |
| `allow_session_lock_restore` (§3.1, D43) | Session 4 |

## Not here

- `shell.md`: the island, the menu bar and its menus, the switcher, Mission Control and App Exposé, the Stage Manager surface, the launcher, Notification Centre, Control Centre, the lock screen's face, the idle dimmer's drawing, hot-corner actions, which output carries the shell.
- `gaming.md`: game detection, Game Mode (the Gaming Focus) and when the game profile applies, Steam and its Big Picture as a full-screen game, controllers. There is no console mode and no pad-to-keys bridge (B22).
- `apps.md`: the Settings pages for displays, keyboards, mouse and shortcuts; Isle's own apps and their unified toolbar (B25); theming installed apps, including GTK's button layout and Chromium flags.
- `platform.md`: the settings store and its D-Bus interface (B10, B27), services (B11), the lock's and the greeter's PAM conversations (B12, B26), the portal backend on the compositor's service worker and the capture pipeline (B13, B26), install and update (B29), the NVIDIA profile file and restart-owed checks.
- The HDR packet (B23): colour management, HDR metadata to the output, SDR beside HDR and gamescope pass-through, after Isle replaces v1.
- P1, the design process: what title bars, previews and transitions look like, and every question under For the design process.
